//! The playback state machine.
//!
//! This is where decoding meets output. It owns a decoder, feeds an
//! [`OutputDevice`], and exposes a small, explicit state machine. Position is
//! tracked in decoded frames so reporting is exact rather than estimated from
//! wall-clock time.
//!
//! The player is deliberately synchronous and caller-driven: a caller pumps it
//! from its own loop. That keeps it testable without spawning anything, and it
//! avoids a hidden task that outlives its owner — consistent with the
//! single-process decision in `docs/adr/0001`.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::decode::{DecodeError, Decoder};
use crate::output::OutputDevice;

/// What the player is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerState {
    /// Nothing loaded.
    #[default]
    Idle,
    /// A track is loaded but not advancing.
    Paused,
    /// Frames are being emitted.
    Playing,
    /// The track reached its end.
    Ended,
}

/// A loaded track, ready to be pumped.
pub struct Player {
    decoder: Mutex<Decoder>,
    output: Option<Arc<OutputDevice>>,
    state: Mutex<PlayerState>,
    /// Decoded frames emitted so far, for position reporting.
    frames_played: Mutex<u64>,
    sample_rate: u32,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("sample_rate", &self.sample_rate)
            .field("state", &*self.state.lock())
            .field("frames_played", &*self.frames_played.lock())
            .finish_non_exhaustive()
    }
}

impl Player {
    /// Loads decoded audio, optionally attached to an output device.
    ///
    /// A `None` output makes the player a decoder harness: frames are consumed
    /// and counted but not sent anywhere. That is what the tests use.
    ///
    /// # Errors
    ///
    /// Propagates [`DecodeError`] from the decoder.
    pub fn load(
        data: Vec<u8>,
        hint: &str,
        output: Option<Arc<OutputDevice>>,
    ) -> Result<Self, DecodeError> {
        let decoder = Decoder::new(data, hint)?;
        let sample_rate = decoder.format().sample_rate;
        Ok(Self {
            decoder: Mutex::new(decoder),
            output,
            state: Mutex::new(PlayerState::Paused),
            frames_played: Mutex::new(0),
            sample_rate,
        })
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> PlayerState {
        *self.state.lock()
    }

    /// The source sample rate.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Elapsed playback position in milliseconds.
    #[must_use]
    pub fn position_ms(&self) -> u64 {
        let frames = *self.frames_played.lock();
        frames.saturating_mul(1_000) / u64::from(self.sample_rate.max(1))
    }

    /// Starts or resumes playback.
    pub fn play(&self) {
        let mut state = self.state.lock();
        if matches!(*state, PlayerState::Paused | PlayerState::Idle) {
            *state = PlayerState::Playing;
        }
    }

    /// Pauses playback.
    pub fn pause(&self) {
        let mut state = self.state.lock();
        if *state == PlayerState::Playing {
            *state = PlayerState::Paused;
        }
    }

    /// Pumps one decode step.
    ///
    /// Feeds one packet to the output and advances the position. Returns the
    /// state after the step.
    ///
    /// # Errors
    ///
    /// Propagates [`DecodeError`] from the decoder.
    pub fn pump(&self) -> Result<PlayerState, DecodeError> {
        if self.state() != PlayerState::Playing {
            return Ok(self.state());
        }

        let packet = self.decoder.lock().decode_next()?;
        let Some(packet) = packet else {
            *self.state.lock() = PlayerState::Ended;
            return Ok(PlayerState::Ended);
        };

        let channels = u64::from(packet.channels.max(1));
        if let Some(output) = &self.output {
            output.write(&packet.samples);
        }
        *self.frames_played.lock() += packet.samples.len() as u64 / channels;
        Ok(PlayerState::Playing)
    }

    /// Resets the position counter. The decoder cannot rewind, so this is used
    /// when a new track is loaded.
    pub fn reset_position(&self) {
        *self.frames_played.lock() = 0;
    }

    /// Discards queued output audio.
    pub fn flush_output(&self) {
        if let Some(output) = &self.output {
            output.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_wav(sample_rate: u32, channels: u16, frames: u32) -> Vec<u8> {
        let bits = 16u16;
        let block_align = channels * bits / 8;
        let data_len = frames * u32::from(block_align);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for frame in 0..frames {
            let value = ((frame as f32 * 0.05).sin() * 20_000.0) as i16;
            for _ in 0..channels {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    fn player() -> Player {
        let wav = synth_wav(44_100, 2, 44_100);
        Player::load(wav, "wav", None).expect("player")
    }

    #[test]
    fn starts_paused_at_zero() {
        let player = player();
        assert_eq!(player.state(), PlayerState::Paused);
        assert_eq!(player.position_ms(), 0);
    }

    #[test]
    fn pumping_while_paused_does_not_advance() {
        let player = player();
        player.pump().expect("pump");
        assert_eq!(player.position_ms(), 0);
    }

    #[test]
    fn playback_advances_position() {
        let player = player();
        player.play();
        for _ in 0..10 {
            player.pump().expect("pump");
        }
        assert!(player.position_ms() > 0, "position must advance while playing");
    }

    #[test]
    fn reaches_end_and_stops_advancing() {
        let wav = synth_wav(44_100, 2, 1_000);
        let player = Player::load(wav, "wav", None).expect("player");
        player.play();

        let mut guard = 0;
        while player.pump().expect("pump") == PlayerState::Playing {
            guard += 1;
            assert!(guard < 10_000, "player never reached the end");
        }
        assert_eq!(player.state(), PlayerState::Ended);

        let position = player.position_ms();
        player.pump().expect("pump after end");
        assert_eq!(player.position_ms(), position, "ended player must not advance");
    }

    #[test]
    fn pause_stops_advancing() {
        let player = player();
        player.play();
        player.pump().expect("pump");
        let position = player.position_ms();
        player.pause();
        player.pump().expect("pump");
        assert_eq!(player.position_ms(), position);
        assert_eq!(player.state(), PlayerState::Paused);
    }

    #[test]
    fn reset_position_returns_to_zero() {
        let player = player();
        player.play();
        player.pump().expect("pump");
        player.reset_position();
        assert_eq!(player.position_ms(), 0);
    }
}
