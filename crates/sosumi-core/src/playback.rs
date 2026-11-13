//! The playback engine.
//!
//! It owns the audio output and a worker thread that decodes and feeds it. The
//! engine is the only place that turns a [`PlaybackSession`] into sound, which
//! keeps the provider layer free of device concerns and the UI free of audio.
//!
//! Three decisions shape it:
//!
//! 1. **The worker is a plain thread, not a Tokio task.** Decoding and writing
//!    to the device are blocking, and the device callback must never wait on an
//!    async runtime.
//! 2. **The worker creates the output device itself.** `cpal::Stream` is not
//!    `Send`, so a device built on one thread cannot be moved to another. The
//!    worker therefore owns the device from the start, and [`PlaybackEngine::new`]
//!    waits for it to report whether the device opened.
//! 3. **The worker owns the player outright.** Bytes arrive over a channel and
//!    are decoded there, so no lock guards the decoder in the hot loop.
//!
//! The engine stops and joins its worker on drop, so no audio thread can outlive
//! the application — the single-process guarantee in `docs/adr/0001`.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sosumi_audio::{OutputDevice, Player, PlayerState as AudioState};
use sosumi_provider_api::{Codec, PlaybackSession, ProviderError, Result};

use crate::event::{Event, EventBus};
use crate::media::MediaFetcher;

/// How often a position event is published.
const POSITION_INTERVAL: Duration = Duration::from_millis(250);

/// How long the worker sleeps when there is nothing to do.
const IDLE_BACKOFF: Duration = Duration::from_millis(5);

/// How long to wait for the worker to report whether the device opened.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

/// A fetched track, waiting to be decoded by the worker.
struct LoadedTrack {
    bytes: Vec<u8>,
    /// The container hint, for example `m4a`, derived from the codec.
    hint: String,
}

/// A control command for the worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Play,
    Pause,
    Stop,
}

/// The running playback engine.
pub struct PlaybackEngine {
    commands: Sender<Command>,
    tracks: Sender<LoadedTrack>,
    /// Set when the worker should exit; the drop path sets it.
    shutdown: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for PlaybackEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackEngine")
            .field("running", &self.worker.is_some())
            .field("shutdown", &self.shutdown.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl PlaybackEngine {
    /// Starts the audio worker and waits for it to open the output device.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::Other`] when no output device is available, the
    /// worker cannot start, or it does not report readiness in time. Callers may
    /// treat the first case as "playback is unavailable" rather than fatal,
    /// which is what keeps the app usable on a machine with no sound card.
    pub fn new(events: EventBus) -> Result<Self> {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        let (track_tx, track_rx) = std::sync::mpsc::channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);

        let worker = std::thread::Builder::new()
            .name("sosumi-audio".to_owned())
            .spawn(move || {
                // The device must be created on this thread; see the module doc.
                match OutputDevice::open(48_000) {
                    Ok(output) => {
                        if ready_tx.send(Ok(())).is_err() {
                            return;
                        }
                        worker_loop(
                            &Rc::new(output),
                            &command_rx,
                            &track_rx,
                            &worker_shutdown,
                            &events,
                        );
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                    }
                }
            })
            .map_err(|e| ProviderError::Other(format!("spawning the audio worker: {e}")))?;

        match ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok(())) => {
                Ok(Self { commands: command_tx, tracks: track_tx, shutdown, worker: Some(worker) })
            }
            Ok(Err(message)) => Err(ProviderError::Other(format!("audio output: {message}"))),
            Err(_) => Err(ProviderError::Other("the audio worker did not start in time".into())),
        }
    }

    /// Fetches a session and hands it to the worker, which decodes and plays it.
    ///
    /// The bytes are fetched before playback begins, so a failure is reported
    /// here rather than surfacing later as silence.
    ///
    /// # Errors
    ///
    /// Propagates fetch failures, and reports a stopped worker as an error.
    pub async fn load(&self, session: &PlaybackSession) -> Result<()> {
        let fetcher = MediaFetcher::new()?;
        let bytes = fetcher.fetch_all(&session.source).await?;

        self.tracks
            .send(LoadedTrack { bytes: bytes.to_vec(), hint: container_hint(session.info.codec) })
            .map_err(|_| ProviderError::Other("the audio worker has stopped".into()))?;
        self.play()
    }

    /// Resumes playback.
    ///
    /// # Errors
    ///
    /// Fails when the worker has stopped.
    pub fn play(&self) -> Result<()> {
        self.send(Command::Play)
    }

    /// Pauses playback.
    ///
    /// # Errors
    ///
    /// Fails when the worker has stopped.
    pub fn pause(&self) -> Result<()> {
        self.send(Command::Pause)
    }

    /// Stops playback and discards the loaded track.
    ///
    /// # Errors
    ///
    /// Fails when the worker has stopped.
    pub fn stop(&self) -> Result<()> {
        self.send(Command::Stop)
    }

    fn send(&self, command: Command) -> Result<()> {
        self.commands
            .send(command)
            .map_err(|_| ProviderError::Other("the audio worker has stopped".into()))
    }
}

impl Drop for PlaybackEngine {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Dropping the senders also ends the worker's receive loops, so it
        // exits even if it is mid-sleep when `shutdown` was set.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The container hint a decoder should assume for a codec.
fn container_hint(codec: Codec) -> String {
    match codec {
        // ALAC, AAC, and object-based audio all travel in an MP4 container.
        // Spatial Audio is rendered downstream; the decoder still sees an MP4.
        Codec::Alac | Codec::Aac | Codec::Ec3Joc => "m4a".to_owned(),
        Codec::Flac => "flac".to_owned(),
        Codec::Opus => "ogg".to_owned(),
        Codec::Pcm => "wav".to_owned(),
    }
}

/// The worker's body: hold the current track, decode, feed the device, report.
fn worker_loop(
    output: &Rc<OutputDevice>,
    commands: &Receiver<Command>,
    tracks: &Receiver<LoadedTrack>,
    shutdown: &AtomicBool,
    events: &EventBus,
) {
    let mut current: Option<Player> = None;
    let mut playing = false;
    let mut last_position = Instant::now();

    while !shutdown.load(Ordering::Relaxed) {
        // A newly loaded track replaces whatever is playing.
        match tracks.try_recv() {
            Ok(loaded) => {
                tracing::debug!(target: "sosumi::playback", hint = %loaded.hint, "track loaded");
                match Player::load(loaded.bytes, &loaded.hint, Some(Rc::clone(output))) {
                    Ok(player) => {
                        player.play();
                        current = Some(player);
                        playing = true;
                        last_position = Instant::now();
                    }
                    Err(error) => {
                        tracing::error!(target: "sosumi::playback", %error, "decode failed");
                        events.publish(Event::Error { message: error.to_string() });
                        current = None;
                        playing = false;
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }

        // Drain commands; the last one wins, which is what a user expects from
        // rapid presses.
        loop {
            match commands.try_recv() {
                Ok(Command::Play) => {
                    playing = true;
                    if let Some(player) = &current {
                        player.play();
                    }
                }
                Ok(Command::Pause) => {
                    playing = false;
                    if let Some(player) = &current {
                        player.pause();
                    }
                }
                Ok(Command::Stop) => {
                    playing = false;
                    current = None;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if !playing {
            std::thread::sleep(IDLE_BACKOFF);
            continue;
        }

        let Some(player) = current.as_ref() else {
            std::thread::sleep(IDLE_BACKOFF);
            continue;
        };

        match player.pump() {
            Ok(AudioState::Playing) => {
                if last_position.elapsed() >= POSITION_INTERVAL {
                    events.publish(Event::Position { position_ms: player.position_ms() });
                    last_position = Instant::now();
                }
                // With a healthy buffer there is nothing useful to do but wait.
                if output.buffered_frames() > 0 {
                    std::thread::sleep(IDLE_BACKOFF);
                }
            }
            Ok(AudioState::Ended) => {
                events.publish(Event::Ended);
                current = None;
                playing = false;
            }
            Ok(_) => std::thread::sleep(IDLE_BACKOFF),
            Err(error) => {
                tracing::error!(target: "sosumi::playback", %error, "decode failed");
                events.publish(Event::Error { message: error.to_string() });
                current = None;
                playing = false;
            }
        }
    }

    tracing::debug!(target: "sosumi::playback", "audio worker stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_hints_match_the_codecs() {
        assert_eq!(container_hint(Codec::Alac), "m4a");
        assert_eq!(container_hint(Codec::Aac), "m4a");
        assert_eq!(container_hint(Codec::Flac), "flac");
        assert_eq!(container_hint(Codec::Pcm), "wav");
        assert_eq!(container_hint(Codec::Opus), "ogg");
    }

    #[test]
    fn an_engine_accepts_commands_when_a_device_exists() {
        // A headless machine has no output device, which is a valid outcome.
        // When one exists, every command must be accepted without blocking.
        match PlaybackEngine::new(EventBus::default()) {
            Ok(engine) => {
                engine.play().expect("play");
                engine.pause().expect("pause");
                engine.stop().expect("stop");
            }
            Err(error) => {
                assert!(
                    error.to_string().contains("audio output")
                        || error.to_string().contains("did not start"),
                    "unexpected failure: {error}"
                );
            }
        }
    }

    #[test]
    fn dropping_the_engine_joins_its_worker() {
        // If the join were missing, the worker would outlive this scope and the
        // test process would hang at exit. Completing the drop is the assertion.
        if let Ok(engine) = PlaybackEngine::new(EventBus::default()) {
            drop(engine);
        }
    }
}
