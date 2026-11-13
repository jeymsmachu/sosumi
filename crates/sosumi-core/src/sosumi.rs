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

use crate::event::{Event, EventBus, EventReceiver};
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
}

impl std::fmt::Debug for Sosumi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sosumi")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("active", &*self.active_provider.read())
            .field("queue_len", &self.queue.read().len())
            .field("signed_in", &self.session.read().is_signed_in())
            .field("quality", &*self.quality.read())
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
        })
    }

    /// Registers a provider. Registering a provider with a known id replaces it.
    pub fn register(&mut self, provider: Arc<dyn MediaProvider>) {
        self.providers.insert(provider.id().clone(), provider);
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

    /// Replaces the session, for example after sign-in.
    pub fn set_session(&self, session: Session) {
        let signed_in = session.is_signed_in();
        *self.session.write() = session;
        self.events.publish(Event::AuthChanged { signed_in });
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
    /// # Errors
    ///
    /// Propagates provider failures. On failure an [`Event::Error`] is also
    /// published so the UI can show it without the caller having to.
    pub async fn start(&self, id: &TrackId) -> Result<PlaybackSession> {
        match self.open_stream(id).await {
            Ok(session) => {
                let metadata = self.provider()?.track(id).await.ok();
                self.publish_track_change(id, metadata.as_ref(), &session);
                Ok(session)
            }
            Err(err) => {
                self.events.publish(Event::Error { message: err.to_string() });
                Err(err)
            }
        }
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
        let signed_in = session.is_signed_in();
        *self.session.write() = session;
        self.events.publish(Event::AuthChanged { signed_in });
        Ok(())
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
        app.set_session(session);
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
}
