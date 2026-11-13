//! The application facade.
//!
//! [`Sosumi`] is the one type the UI talks to. It owns the providers, the
//! queue, the audio player, the session, and the event bus, and it exposes a
//! small set of intents — search, play, pause, skip — rather than its internals.
//!
//! Keeping every entry point here means the FFI bridge has one thing to
//! translate, and the UI can never reach past the facade into a subsystem.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use sosumi_provider_api::{
    AudioQuality, Lyrics, MediaProvider, Page, PlaybackSession, Playlist, ProviderError,
    ProviderId, Result, SearchQuery, SearchResults, StreamRequest, Track, TrackId,
};
use sosumi_storage::{Paths, SecretStore};

use crate::capability::{CapabilityReport, SharedRuntime};
use crate::event::{Event, EventBus, EventReceiver};
use crate::playback::PlaybackEngine;
use crate::queue::Queue;
use crate::session::Session;

/// The application.
pub struct Sosumi {
    providers: HashMap<ProviderId, Arc<dyn MediaProvider>>,
    active_provider: RwLock<ProviderId>,
    queue: RwLock<Queue>,
    session: RwLock<Session>,
    events: EventBus,
    paths: Paths,
    quality: RwLock<AudioQuality>,
    /// The track currently loaded. A [`PlaybackSession`] is single-use because
    /// it owns a reader, so only its identity is retained here.
    current: RwLock<Option<TrackId>>,
    /// The audio engine. Absent when no output device is available, which keeps
    /// the app usable for browsing on a machine with no sound card. Held behind
    /// an `Arc` so a caller can clone it out and await without the lock.
    playback: RwLock<Option<Arc<PlaybackEngine>>>,
    /// The optional lossless runtime, when its manifest was found.
    ///
    /// `None` when the manifest is missing, which means the install is broken
    /// rather than that the tier is unavailable, so the capability reports an
    /// error instead of offering an install that cannot work.
    runtime: Option<SharedRuntime>,
}

impl std::fmt::Debug for Sosumi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sosumi")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("active", &*self.active_provider.read())
            .field("queue_len", &self.queue.read().len())
            .field("signed_in", &self.session.read().is_signed_in())
            .field("quality", &*self.quality.read())
            .field("audio", &self.playback.read().is_some())
            .finish_non_exhaustive()
    }
}

impl Sosumi {
    /// Builds the application from its resolved paths.
    ///
    /// # Errors
    ///
    /// Fails when the storage layout cannot be created.
    pub fn new(paths: Paths) -> anyhow::Result<Self> {
        paths.ensure()?;
        Ok(Self {
            providers: HashMap::new(),
            active_provider: RwLock::new(ProviderId::from(ProviderId::APPLE)),
            queue: RwLock::new(Queue::new()),
            session: RwLock::new(Session::new()),
            events: EventBus::default(),
            paths,
            quality: RwLock::new(AudioQuality::High),
            current: RwLock::new(None),
            playback: RwLock::new(None),
            runtime: None,
        })
    }

    /// Starts the audio engine.
    ///
    /// Absent by default rather than at construction, because a machine with no
    /// output device must still be able to browse. A failure here is logged and
    /// reported as an event, not raised to the caller.
    ///
    /// Idempotent: a second call is a no-op, so a caller cannot accidentally
    /// replace a running engine.
    pub fn start_audio(&self) {
        if self.playback.read().is_some() {
            return;
        }
        match PlaybackEngine::new(self.events.clone()) {
            Ok(engine) => {
                tracing::info!(target: "sosumi::core", "audio engine started");
                *self.playback.write() = Some(Arc::new(engine));
            }
            Err(error) => {
                tracing::warn!(target: "sosumi::core", %error, "audio engine unavailable");
                self.events
                    .publish(Event::Error { message: format!("Playback is unavailable: {error}") });
            }
        }
    }

    /// True when audio output is available.
    #[must_use]
    pub fn audio_ready(&self) -> bool {
        self.playback.read().is_some()
    }

    /// Pauses playback.
    ///
    /// # Errors
    ///
    /// Fails when the audio engine is not running.
    pub fn pause(&self) -> Result<()> {
        self.playback_engine()
            .ok_or_else(|| ProviderError::CapabilityMissing {
                feature: "audio",
                remedy: "No audio output device is available".into(),
            })?
            .pause()?;
        self.events.publish(Event::Paused);
        Ok(())
    }

    /// Resumes playback.
    ///
    /// # Errors
    ///
    /// Fails when the audio engine is not running.
    pub fn resume(&self) -> Result<()> {
        self.playback_engine()
            .ok_or_else(|| ProviderError::CapabilityMissing {
                feature: "audio",
                remedy: "No audio output device is available".into(),
            })?
            .play()?;
        self.events.publish(Event::Playing);
        Ok(())
    }

    /// Stops playback and unloads the track.
    ///
    /// # Errors
    ///
    /// Fails when the audio engine is not running.
    pub fn stop(&self) -> Result<()> {
        self.playback_engine()
            .ok_or_else(|| ProviderError::CapabilityMissing {
                feature: "audio",
                remedy: "No audio output device is available".into(),
            })?
            .stop()?;
        *self.current.write() = None;
        self.events.publish(Event::Stopped);
        Ok(())
    }

    /// Attaches the optional lossless runtime.
    ///
    /// Separate from the constructor because the manifest lives on disk and a
    /// caller may legitimately not have it: the app runs without this.
    pub fn set_runtime(&mut self, runtime: SharedRuntime) {
        self.runtime = Some(runtime);
        self.events.publish(Event::CapabilitiesChanged);
    }

    /// The runtime, when its manifest was found.
    #[must_use]
    pub fn runtime(&self) -> Option<&SharedRuntime> {
        self.runtime.as_ref()
    }

    /// Reports the lossless tier's state.
    ///
    /// Returns an unsupported state when there is no runtime attached, which is a
    /// broken install rather than an uninstalled tier, and says so.
    #[must_use]
    pub fn lossless_capability(&self) -> CapabilityReport {
        match &self.runtime {
            Some(runtime) => runtime.probe(),
            None => CapabilityReport::Unsupported {
                reason: "the runtime manifest is missing from this installation".to_owned(),
            },
        }
    }

    /// Writes the stub libraries and reports what is still missing.
    ///
    /// # Errors
    ///
    /// Fails when there is no runtime attached, or the stubs cannot be written.
    pub fn install_lossless_stubs(&self) -> Result<usize> {
        let runtime = self.runtime.as_ref().ok_or_else(|| ProviderError::CapabilityMissing {
            feature: "fairplay",
            remedy: "The runtime manifest is missing from this installation".to_owned(),
        })?;
        let written = runtime.install_stubs().map_err(|e| ProviderError::Other(e.to_string()))?;
        self.events.publish(Event::CapabilitiesChanged);
        Ok(written)
    }

    /// Registers a provider. Registering a provider with a known id replaces it.
    ///
    /// The provider immediately receives the current account, so a provider
    /// registered after sign-in is usable without waiting for the next change.
    pub fn register(&mut self, provider: Arc<dyn MediaProvider>) {
        provider.set_account(self.account());
        self.providers.insert(provider.id().clone(), provider);
    }

    /// The account credentials, as providers see them.
    fn account(&self) -> sosumi_provider_api::Account {
        let session = self.session.read();
        sosumi_provider_api::Account {
            user_token: session.music_user_token.clone(),
            developer_token: session.developer_token.clone(),
            storefront: session.storefront.clone(),
        }
    }

    /// The event bus, for wiring UI and integrations.
    #[must_use]
    pub fn events(&self) -> &EventBus {
        &self.events
    }

    /// Subscribes to events.
    #[must_use]
    pub fn subscribe(&self) -> EventReceiver {
        self.events.subscribe()
    }

    /// The storage layout in use.
    #[must_use]
    pub const fn paths(&self) -> &Paths {
        &self.paths
    }

    /// The session.
    #[must_use]
    pub fn session(&self) -> Session {
        self.session.read().clone()
    }

    /// Replaces the in-memory session and hands it to every provider.
    ///
    /// This is what makes sign-in take effect for the current run: storing the
    /// session without telling the providers would leave them serving
    /// unauthenticated requests while the UI reported success. It does not write
    /// to the encrypted store — call [`Sosumi::persist_session`] for that, and the
    /// two are separate so a failed write costs persistence rather than the
    /// current sign-in.
    ///
    /// # Errors
    ///
    /// Currently infallible; the result is kept so a future provider that can
    /// reject an account does not force a signature change at every call site.
    pub fn set_session(&self, session: Session) -> Result<()> {
        let signed_in = session.is_signed_in();
        let account = sosumi_provider_api::Account {
            user_token: session.music_user_token.clone(),
            developer_token: session.developer_token.clone(),
            storefront: session.storefront.clone(),
        };
        for provider in self.providers.values() {
            provider.set_account(account.clone());
        }
        *self.session.write() = session;
        self.events.publish(Event::AuthChanged { signed_in });
        Ok(())
    }

    /// Persists the current session to the encrypted store.
    ///
    /// # Errors
    ///
    /// Fails when the sealer cannot be built or the store cannot be written.
    pub fn persist_session(&self, sealer: sosumi_security::Sealer) -> Result<()> {
        let mut store = SecretStore::open(self.paths.secret_store(), sealer)
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        self.session.read().save(&mut store).map_err(|e| ProviderError::Other(e.to_string()))
    }

    /// The active quality.
    #[must_use]
    pub fn quality(&self) -> AudioQuality {
        *self.quality.read()
    }

    /// Sets the active quality and announces it.
    pub fn set_quality(&self, quality: AudioQuality) {
        *self.quality.write() = quality;
        self.events.publish(Event::QualityChanged { quality: quality.label().to_owned() });
    }

    /// Selects the active provider.
    ///
    /// # Errors
    ///
    /// Returns [`ProviderError::NotFound`] when the id is not registered.
    pub fn set_active_provider(&self, id: &ProviderId) -> Result<()> {
        if !self.providers.contains_key(id) {
            return Err(ProviderError::NotFound(format!("provider {id}")));
        }
        *self.active_provider.write() = id.clone();
        Ok(())
    }

    /// The active provider.
    fn provider(&self) -> Result<&Arc<dyn MediaProvider>> {
        let id = self.active_provider.read().clone();
        self.providers.get(&id).ok_or_else(|| ProviderError::NotFound(format!("provider {id}")))
    }

    /// Searches the active provider.
    ///
    /// # Errors
    ///
    /// Propagates provider failures.
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let provider = self.provider()?;
        let mut query = query;
        if query.storefront.is_none() {
            query.storefront = Some(self.session.read().storefront.clone());
        }
        provider.search(query).await
    }

    /// Fetches a track's metadata.
    ///
    /// # Errors
    ///
    /// Propagates provider failures.
    pub async fn track(&self, id: &TrackId) -> Result<Track> {
        self.provider()?.track(id).await
    }

    /// Fetches a playlist and its tracks.
    ///
    /// # Errors
    ///
    /// Propagates provider failures.
    pub async fn playlist(&self, id: &str) -> Result<(Playlist, Page<Track>)> {
        let storefront = self.session.read().storefront.clone();
        self.provider()?.playlist(id, Some(&storefront)).await
    }

    /// Fetches lyrics.
    ///
    /// # Errors
    ///
    /// Propagates provider failures.
    pub async fn lyrics(&self, id: &TrackId) -> Result<Lyrics> {
        let storefront = self.session.read().storefront.clone();
        self.provider()?.lyrics(id, Some(&storefront)).await
    }

    /// Opens a stream for a track at the current quality.
    ///
    /// # Errors
    ///
    /// Propagates provider failures, including
    /// [`ProviderError::CapabilityMissing`] when the requested tier needs an
    /// optional component.
    pub async fn open_stream(&self, id: &TrackId) -> Result<PlaybackSession> {
        let storefront = self.session.read().storefront.clone();
        let quality = self.quality();
        let provider = self.provider()?;
        let session = provider
            .open(StreamRequest { track: id.clone(), storefront: Some(storefront), quality })
            .await?;
        *self.current.write() = Some(id.clone());
        Ok(session)
    }

    /// The track whose stream was most recently opened.
    #[must_use]
    pub fn loaded_track(&self) -> Option<TrackId> {
        self.current.read().clone()
    }

    /// Replaces the queue and starts at the first track.
    pub fn set_queue(&self, tracks: Vec<TrackId>) {
        self.queue.write().set_items(tracks);
        self.events.publish(Event::QueueChanged { length: self.queue.read().len() });
    }

    /// Appends a track to the queue.
    pub fn enqueue(&self, track: TrackId) {
        self.queue.write().push(track);
        self.events.publish(Event::QueueChanged { length: self.queue.read().len() });
    }

    /// Sets the repeat mode.
    pub fn set_repeat(&self, mode: crate::queue::RepeatMode) {
        self.queue.write().set_repeat(mode);
    }

    /// The repeat mode.
    #[must_use]
    pub fn repeat(&self) -> crate::queue::RepeatMode {
        self.queue.read().repeat()
    }

    /// Enables or disables shuffle.
    pub fn set_shuffled(&self, shuffled: bool) {
        let mut queue = self.queue.write();
        if shuffled {
            queue.shuffle();
        } else {
            queue.unshuffle();
        }
    }

    /// True when shuffle is on.
    #[must_use]
    pub fn is_shuffled(&self) -> bool {
        self.queue.read().is_shuffled()
    }

    /// The current track in the queue.
    #[must_use]
    pub fn current_track(&self) -> Option<TrackId> {
        self.queue.read().current().cloned()
    }

    /// The number of tracks queued.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.read().len()
    }

    /// Advances the queue and opens the next stream, announcing the change.
    ///
    /// Returns `None` when the queue ended and repeat is off.
    ///
    /// # Errors
    ///
    /// Propagates provider failures from opening the next stream.
    pub async fn next(&self) -> Result<Option<PlaybackSession>> {
        let next_id = self.queue.write().advance().cloned();
        let Some(id) = next_id else {
            self.events.publish(Event::Ended);
            return Ok(None);
        };
        self.start(&id).await.map(Some)
    }

    /// Opens a specific track and announces it as the current one.
    ///
    /// When audio is available the stream is also loaded and started, so a
    /// caller only has to name a track. Without an audio device the metadata is
    /// still published, which is what lets the UI browse on a silent machine.
    ///
    /// # Errors
    ///
    /// Propagates provider failures. On failure an [`Event::Error`] is also
    /// published so the UI can show it without the caller having to.
    pub async fn start(&self, id: &TrackId) -> Result<PlaybackSession> {
        match self.open_stream(id).await {
            Ok(session) => {
                let metadata = self.provider()?.track(id).await.ok();
                self.publish_track_change(id, metadata.as_ref(), &session);

                // The engine is held behind a lock, and loading awaits a network
                // fetch. Cloning it out first means the guard is never held
                // across an await point, which is what avoids a deadlock with
                // any other thread touching playback.
                if let Some(engine) = self.playback_engine() {
                    if let Err(error) = engine.load(&session).await {
                        tracing::warn!(target: "sosumi::core", %error, "could not start audio");
                        self.events.publish(Event::Error { message: error.to_string() });
                    }
                }
                Ok(session)
            }
            Err(err) => {
                self.events.publish(Event::Error { message: err.to_string() });
                Err(err)
            }
        }
    }

    /// Clones the audio engine handle out of the lock.
    ///
    /// A shared handle rather than the guard, so callers can await without
    /// holding the playback lock.
    fn playback_engine(&self) -> Option<Arc<PlaybackEngine>> {
        self.playback.read().clone()
    }

    /// Announces the current track.
    fn publish_track_change(&self, id: &TrackId, track: Option<&Track>, session: &PlaybackSession) {
        let (title, artist, artwork) = match track {
            Some(track) => (
                track.title.clone(),
                track.artist.name.clone(),
                track
                    .artwork
                    .as_ref()
                    .and_then(|a| a.animated_url.clone().or_else(|| Some(a.url.clone()))),
            ),
            None => (id.as_str().to_owned(), String::new(), None),
        };

        self.events.publish(Event::TrackChanged {
            track: id.clone(),
            title,
            artist,
            duration_ms: session.duration_ms.or_else(|| track.and_then(|t| t.duration_ms)),
            artwork_url: artwork,
        });
    }

    /// Loads the session from the encrypted store.
    ///
    /// # Errors
    ///
    /// Fails when the sealer cannot be built or the store cannot be opened.
    pub fn restore_session(&self, sealer: sosumi_security::Sealer) -> Result<()> {
        let store = SecretStore::open(self.paths.secret_store(), sealer)
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let session = Session::load(&store);
        // `set_session` publishes the session to the providers, which is what
        // makes a restored session usable immediately.
        self.set_session(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Sosumi {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = Paths::under(dir.path().to_path_buf());
        // Leak the tempdir so the app's paths stay valid for the test's life.
        std::mem::forget(dir);
        Sosumi::new(paths).expect("app")
    }

    #[test]
    fn a_fresh_app_is_signed_out_at_high_quality() {
        let app = app();
        assert!(!app.session().is_signed_in());
        assert_eq!(app.quality(), AudioQuality::High);
        assert_eq!(app.queue_len(), 0);
        assert!(app.current_track().is_none());
    }

    #[test]
    fn setting_quality_announces_the_change() {
        let app = app();
        let mut events = app.subscribe();
        app.set_quality(AudioQuality::Lossless);
        match events.try_next_event() {
            Some(Event::QualityChanged { quality }) => assert_eq!(quality, "Lossless"),
            other => panic!("expected QualityChanged, got {other:?}"),
        }
    }

    #[test]
    fn setting_the_session_announces_auth_state() {
        let app = app();
        let mut events = app.subscribe();
        let mut session = Session::new();
        session.set_music_user_token("mut");
        app.set_session(session).expect("set session");
        assert!(app.session().is_signed_in());
        assert!(matches!(events.try_next_event(), Some(Event::AuthChanged { signed_in: true })));
    }

    #[test]
    fn queueing_announces_the_new_length() {
        let app = app();
        let mut events = app.subscribe();
        let tracks = vec![
            TrackId::new(&ProviderId::from(ProviderId::APPLE), "1"),
            TrackId::new(&ProviderId::from(ProviderId::APPLE), "2"),
        ];
        app.set_queue(tracks);
        assert_eq!(app.queue_len(), 2);
        assert_eq!(app.current_track().unwrap().as_str(), "apple:1");
        assert!(matches!(events.try_next_event(), Some(Event::QueueChanged { length: 2 })));
    }

    #[test]
    fn an_unregistered_provider_is_rejected() {
        let app = app();
        let local = ProviderId::from(ProviderId::LOCAL);
        assert!(matches!(app.set_active_provider(&local), Err(ProviderError::NotFound(_))));
    }

    #[test]
    fn search_without_a_provider_reports_not_found() {
        let app = app();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let result = runtime.block_on(app.search(SearchQuery::new("radiohead")));
        assert!(matches!(result, Err(ProviderError::NotFound(_))));
    }

    #[test]
    fn next_on_an_empty_queue_announces_the_end() {
        let app = app();
        let mut events = app.subscribe();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let result = runtime.block_on(app.next()).expect("no provider needed for an empty queue");
        assert!(result.is_none());
        assert!(matches!(events.try_next_event(), Some(Event::Ended)));
    }

    #[test]
    fn an_empty_queue_reports_no_current_track() {
        let app = app();
        assert!(app.current_track().is_none());
    }

    #[test]
    fn audio_is_absent_until_started() {
        let app = app();
        assert!(!app.audio_ready());
    }

    #[test]
    fn signing_in_hands_the_credentials_to_every_provider() {
        // The regression this guards: `set_session` once stored the token
        // without telling the provider, so the UI reported a successful sign-in
        // while every request still failed as unauthenticated.
        use async_trait::async_trait;
        use parking_lot::Mutex;
        use sosumi_provider_api::{
            Account, Lyrics, MediaProvider, Page, Playlist, ProviderId, SearchQuery, SearchResults,
            StreamRequest, Track,
        };

        #[derive(Default)]
        struct Recorder {
            account: Mutex<Option<Account>>,
        }

        #[async_trait]
        impl MediaProvider for Recorder {
            fn id(&self) -> &ProviderId {
                static ID: std::sync::LazyLock<ProviderId> =
                    std::sync::LazyLock::new(|| ProviderId("recorder".to_owned()));
                &ID
            }
            fn name(&self) -> &'static str {
                "Recorder"
            }
            fn is_available(&self) -> bool {
                true
            }
            fn set_account(&self, account: Account) {
                *self.account.lock() = Some(account);
            }
            async fn search(&self, _: SearchQuery) -> sosumi_provider_api::Result<SearchResults> {
                Ok(SearchResults::default())
            }
            async fn track(
                &self,
                _: &sosumi_provider_api::TrackId,
            ) -> sosumi_provider_api::Result<Track> {
                unimplemented!()
            }
            async fn album(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> sosumi_provider_api::Result<(sosumi_provider_api::Album, Vec<Track>)> {
                unimplemented!()
            }
            async fn playlist(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> sosumi_provider_api::Result<(Playlist, Page<Track>)> {
                unimplemented!()
            }
            async fn artist(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> sosumi_provider_api::Result<(sosumi_provider_api::Artist, Vec<Track>)>
            {
                unimplemented!()
            }
            async fn lyrics(
                &self,
                _: &sosumi_provider_api::TrackId,
                _: Option<&str>,
            ) -> sosumi_provider_api::Result<Lyrics> {
                unimplemented!()
            }
            async fn open(
                &self,
                _: StreamRequest,
            ) -> sosumi_provider_api::Result<sosumi_provider_api::PlaybackSession> {
                unimplemented!()
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = Paths::under(dir.path().to_path_buf());
        std::mem::forget(dir);
        let mut app = Sosumi::new(paths).expect("app");

        let recorder = Arc::new(Recorder::default());
        app.register(Arc::clone(&recorder) as Arc<dyn MediaProvider>);

        let mut session = Session::new();
        session.set_music_user_token("secret-token");
        session.set_storefront("gb");
        app.set_session(session).expect("set session");

        let seen = recorder.account.lock().clone().expect("the provider must be told");
        assert_eq!(seen.user_token.as_deref(), Some("secret-token"));
        assert_eq!(seen.storefront, "gb");
        assert!(seen.is_signed_in());
    }

    #[test]
    fn a_provider_registered_after_sign_in_still_gets_the_account() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = Paths::under(dir.path().to_path_buf());
        std::mem::forget(dir);
        let mut app = Sosumi::new(paths).expect("app");

        let mut session = Session::new();
        session.set_music_user_token("token");
        app.set_session(session).expect("set session");

        // The local provider needs no credentials, but it must not panic when
        // the core hands them over during registration.
        app.register(Arc::new(sosumi_provider_local::LocalProvider::new(Vec::new())));
        assert_eq!(app.session().music_user_token.as_deref(), Some("token"));
    }

    #[test]
    fn transport_commands_without_a_device_report_a_capability_gap() {
        // A machine with no output device still runs the app; transport is the
        // one thing that cannot work, and it must say so rather than panic.
        let app = app();
        if app.audio_ready() {
            return;
        }
        let error = app.pause().expect_err("pause must fail without audio");
        assert!(matches!(error, ProviderError::CapabilityMissing { feature: "audio", .. }));
        assert!(app.resume().is_err());
        assert!(app.stop().is_err());
    }

    #[test]
    fn starting_audio_publishes_transport_events() {
        let app = app();
        app.start_audio();
        if !app.audio_ready() {
            return;
        }

        let mut events = app.subscribe();
        app.pause().expect("pause");
        assert!(matches!(events.try_next_event(), Some(Event::Paused)));

        app.resume().expect("resume");
        assert!(matches!(events.try_next_event(), Some(Event::Playing)));

        app.stop().expect("stop");
        assert!(matches!(events.try_next_event(), Some(Event::Stopped)));
        assert!(app.loaded_track().is_none(), "stopping must unload the track");
    }
}
