//! The bridge API: the types and functions Dart sees.
//!
//! Everything here is deliberately plain. Domain types from `sosumi-core` are
//! not exposed directly, because they carry concerns — borrowed readers, trait
//! objects, key material — that must not cross into a garbage-collected runtime.
//! Each type below is a flat, owned projection.
//!
//! The `frb_expand` cfg is emitted by the `#[frb]` macros; it is not a real
//! feature of this crate, so the unexpected-cfg lint is silenced here.
#![allow(unexpected_cfgs)]

use std::sync::Arc;

use flutter_rust_bridge::frb;

use crate::frb_generated::StreamSink;

use sosumi_core::event::Event as CoreEvent;
use sosumi_core::queue::RepeatMode as CoreRepeat;
use sosumi_core::{Session as CoreSession, Sosumi};
use sosumi_provider_api::{
    AudioQuality as CoreQuality, ProviderId, SearchQuery as CoreSearchQuery, Track as CoreTrack,
    TrackId as CoreTrackId,
};
use sosumi_provider_apple::AppleMusicProvider;
use sosumi_provider_local::LocalProvider;
use sosumi_storage::Paths;

// ── Errors ──────────────────────────────────────────────────────────────────

/// A failure crossing the bridge, flattened to something Dart can display.
#[derive(Debug, Clone)]
pub struct BridgeError {
    /// A stable machine-readable kind: `not_authenticated`, `capability`,
    /// `network`, `not_found`, or `other`.
    pub kind: String,
    /// A message suitable for showing to a user.
    pub message: String,
    /// What the user can do, when the failure is actionable.
    pub remedy: Option<String>,
}

impl BridgeError {
    /// Wraps a provider error, preserving its kind and remedy across the bridge.
    ///
    /// Private: `ProviderError` is a core type and must not appear in a public
    /// signature, or the generated bindings would try to marshal it.
    fn from_provider(err: &sosumi_provider_api::ProviderError) -> Self {
        use sosumi_provider_api::ProviderError as P;
        match err {
            P::NotAuthenticated => Self {
                kind: "not_authenticated".into(),
                message: "Sign in to Apple Music to continue".into(),
                remedy: Some("Open Settings and sign in".into()),
            },
            P::CapabilityMissing { feature, remedy } => Self {
                kind: "capability".into(),
                message: format!("{feature} is not installed"),
                remedy: Some(remedy.clone()),
            },
            P::Network(message) => Self {
                kind: "network".into(),
                message: message.clone(),
                remedy: Some("Check your connection".into()),
            },
            P::NotFound(what) => Self {
                kind: "not_found".into(),
                message: format!("{what} was not found"),
                remedy: None,
            },
            P::NotEntitled(what) => Self {
                kind: "not_entitled".into(),
                message: what.clone(),
                remedy: Some("This may not be available in your storefront".into()),
            },
            other => Self { kind: "other".into(), message: other.to_string(), remedy: None },
        }
    }
}

impl From<sosumi_provider_api::ProviderError> for BridgeError {
    fn from(err: sosumi_provider_api::ProviderError) -> Self {
        Self::from_provider(&err)
    }
}

impl From<anyhow::Error> for BridgeError {
    fn from(err: anyhow::Error) -> Self {
        Self { kind: "other".into(), message: err.to_string(), remedy: None }
    }
}

// ── Data transfer objects ───────────────────────────────────────────────────

/// A track, flattened for Dart.
#[derive(Debug, Clone, Default)]
pub struct TrackDto {
    /// `provider:locals`, for example `apple:1440833098`.
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub animated_artwork_url: Option<String>,
    pub dominant_color: Option<String>,
    pub duration_ms: Option<u32>,
}

impl From<&CoreTrack> for TrackDto {
    fn from(track: &CoreTrack) -> Self {
        Self {
            id: track.id.as_str().to_owned(),
            title: track.title.clone(),
            artist: track.artist.name.clone(),
            album: track.album.as_ref().map(|a| a.title.clone()),
            artwork_url: track.artwork.as_ref().map(|a| a.url.clone()),
            animated_artwork_url: track.artwork.as_ref().and_then(|a| a.animated_url.clone()),
            dominant_color: track.artwork.as_ref().and_then(|a| a.dominant_color.clone()),
            duration_ms: track.duration_ms.and_then(|ms| u32::try_from(ms).ok()),
        }
    }
}

/// A playlist, flattened for Dart.
#[derive(Debug, Clone, Default)]
pub struct PlaylistDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub artwork_url: Option<String>,
    pub track_count: Option<u32>,
}

/// One grouped search result set.
#[derive(Debug, Clone, Default)]
pub struct SearchResultsDto {
    pub tracks: Vec<TrackDto>,
    pub albums: Vec<AlbumDto>,
    pub artists: Vec<ArtistDto>,
    pub playlists: Vec<PlaylistDto>,
}

impl SearchResultsDto {
    /// True when nothing matched.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
    }
}

/// An album, flattened for Dart.
#[derive(Debug, Clone, Default)]
pub struct AlbumDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artwork_url: Option<String>,
    pub release_date: Option<String>,
    pub track_count: Option<u32>,
}

/// An artist, flattened for Dart.
#[derive(Debug, Clone, Default)]
pub struct ArtistDto {
    pub id: String,
    pub name: String,
    pub artwork_url: Option<String>,
}

/// One timed lyric line.
#[derive(Debug, Clone, Default)]
pub struct LyricLineDto {
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub text: String,
    pub translation: Option<String>,
}

/// Lyrics, synced and plain.
#[derive(Debug, Clone, Default)]
pub struct LyricsDto {
    pub plain: String,
    pub lines: Vec<LyricLineDto>,
    pub language: Option<String>,
}

/// What the user is signed in as.
#[derive(Debug, Clone, Default)]
pub struct SessionDto {
    pub signed_in: bool,
    pub playable: bool,
    pub storefront: String,
}

impl From<&CoreSession> for SessionDto {
    fn from(session: &CoreSession) -> Self {
        Self {
            signed_in: session.is_signed_in(),
            playable: session.is_playable(),
            storefront: session.storefront.clone(),
        }
    }
}

/// An event pushed to the UI.
///
/// A flat struct rather than an enum with data, because `flutter_rust_bridge`
/// maps a Dart-compatible enum without payloads most cleanly. `kind` selects the
/// variant and the optional fields carry its payload.
#[derive(Debug, Clone, Default)]
pub struct EventDto {
    /// One of `track_changed`, `playing`, `paused`, `stopped`, `position`,
    /// `ended`, `queue_changed`, `quality_changed`, `auth_changed`,
    /// `capabilities_changed`, or `error`.
    pub kind: String,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<u32>,
    pub position_ms: Option<u32>,
    pub queue_length: Option<u32>,
    pub message: Option<String>,
    pub signed_in: Option<bool>,
}

impl From<CoreEvent> for EventDto {
    fn from(event: CoreEvent) -> Self {
        let mut dto = Self::default();
        match event {
            CoreEvent::TrackChanged { track, title, artist, duration_ms, artwork_url } => {
                dto.kind = "track_changed".into();
                dto.track_id = Some(track.as_str().to_owned());
                dto.title = Some(title);
                dto.artist = Some(artist);
                dto.duration_ms = duration_ms.and_then(|ms| u32::try_from(ms).ok());
                dto.artwork_url = artwork_url;
            }
            CoreEvent::Playing => dto.kind = "playing".into(),
            CoreEvent::Paused => dto.kind = "paused".into(),
            CoreEvent::Stopped => dto.kind = "stopped".into(),
            CoreEvent::Ended => dto.kind = "ended".into(),
            CoreEvent::Position { position_ms } => {
                dto.kind = "position".into();
                dto.position_ms = u32::try_from(position_ms).ok();
            }
            CoreEvent::QueueChanged { length } => {
                dto.kind = "queue_changed".into();
                dto.queue_length = u32::try_from(length).ok();
            }
            CoreEvent::QualityChanged { quality } => {
                dto.kind = "quality_changed".into();
                dto.message = Some(quality);
            }
            CoreEvent::AuthChanged { signed_in } => {
                dto.kind = "auth_changed".into();
                dto.signed_in = Some(signed_in);
            }
            CoreEvent::CapabilitiesChanged => dto.kind = "capabilities_changed".into(),
            CoreEvent::Error { message } => {
                dto.kind = "error".into();
                dto.message = Some(message);
            }
        }
        dto
    }
}

// ── The engine ──────────────────────────────────────────────────────────────

/// The application handle the UI holds for its whole lifetime.
///
/// Opaque to Dart: the UI calls methods, never touches fields. That keeps the
/// core free to change shape without a binding change.
#[frb(opaque)]
pub struct Engine {
    inner: Arc<Sosumi>,
    runtime: tokio::runtime::Runtime,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

/// Builds the engine. The UI calls this once, at startup.
///
/// The returned handle owns a Tokio runtime, so the whole application is one
/// process with two cooperating runtimes: Dart's isolate loop and Tokio. That
/// is the single-process design in `docs/adr/0001`.
///
/// # Errors
///
/// Fails when the storage layout cannot be created or the runtime cannot start.
pub fn create_engine() -> Result<Engine, BridgeError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| BridgeError { kind: "other".into(), message: e.to_string(), remedy: None })?;

    let paths = Paths::resolve()?;
    let mut app = Sosumi::new(paths)?;

    // The concrete providers are chosen here, at the outermost layer, so the
    // core stays free of any knowledge of Apple or of the filesystem.
    app.register(Arc::new(AppleMusicProvider::new()?));

    let music_dirs = default_music_directories();
    if !music_dirs.is_empty() {
        app.register(Arc::new(LocalProvider::new(music_dirs)));
    }

    // Start audio here, at the composition root, so playback is available as
    // soon as the UI is ready. A machine with no output device is not an error:
    // the app still browses, and the core reports the reason as an event.
    app.start_audio();

    // Restore a previous sign-in so the user does not authenticate every launch.
    // A failure is not fatal: it means the store is unreadable, and signing in
    // again repairs it.
    if let Ok(sealer) = machine_sealer() {
        if let Err(error) = app.restore_session(sealer) {
            tracing::warn!(target: "sosumi::bridge", %error, "could not restore the session");
        }
    }

    Ok(Engine { inner: Arc::new(app), runtime })
}

/// The sealing key for the local store.
fn machine_sealer() -> anyhow::Result<sosumi_security::Sealer> {
    let key = sosumi_security::sealing::machine_key()?;
    sosumi_security::Sealer::new(&key, "local")
}

/// The directories to scan for local music.
///
/// The XDG music directory first, then the conventional fallback. A directory
/// that does not exist is harmless: the provider skips it.
fn default_music_directories() -> Vec<std::path::PathBuf> {
    let mut directories = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        let music = std::env::var_os("XDG_MUSIC_DIR")
            .map_or_else(|| home.join("Music"), std::path::PathBuf::from);
        directories.push(music);
    }

    directories
}

impl Engine {
    /// Runs a future on the engine's runtime.
    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    /// Searches the catalog.
    ///
    /// # Errors
    ///
    /// Fails with a mapped [`BridgeError`].
    pub fn search(&self, query: String) -> Result<SearchResultsDto, BridgeError> {
        let results = self.block_on(self.inner.search(CoreSearchQuery::new(query)))?;
        Ok(SearchResultsDto {
            tracks: results.tracks.iter().map(TrackDto::from).collect(),
            albums: results
                .albums
                .iter()
                .map(|album| AlbumDto {
                    id: album.id.clone(),
                    title: album.title.clone(),
                    artist: album.artist.name.clone(),
                    artwork_url: album.artwork.as_ref().map(|a| a.url.clone()),
                    release_date: album.release_date.clone(),
                    track_count: album.track_count,
                })
                .collect(),
            artists: results
                .artists
                .iter()
                .map(|artist| ArtistDto {
                    id: artist.id.clone(),
                    name: artist.name.clone(),
                    artwork_url: artist.artwork.as_ref().map(|a| a.url.clone()),
                })
                .collect(),
            playlists: results
                .playlists
                .iter()
                .map(|playlist| PlaylistDto {
                    id: playlist.id.clone(),
                    title: playlist.title.clone(),
                    description: playlist.description.clone(),
                    artwork_url: playlist.artwork.as_ref().map(|a| a.url.clone()),
                    track_count: playlist.track_count,
                })
                .collect(),
        })
    }

    /// Fetches one track's metadata.
    ///
    /// # Errors
    ///
    /// Fails with a mapped [`BridgeError`].
    pub fn track(&self, id: String) -> Result<TrackDto, BridgeError> {
        let id = CoreTrackId(id);
        let track = self.block_on(self.inner.track(&id))?;
        Ok(TrackDto::from(&track))
    }

    /// Opens a stream for a track and marks it as current.
    ///
    /// Returns the track as the player should display it. The audio itself is
    /// driven from Rust; the UI never receives a URL.
    ///
    /// # Errors
    ///
    /// Fails with a mapped [`BridgeError`], including the `capability` kind when
    /// the requested quality needs an optional component.
    pub fn play_track(&self, id: String) -> Result<TrackDto, BridgeError> {
        let id = CoreTrackId(id);
        self.block_on(self.inner.start(&id))?;
        let track = self.block_on(self.inner.track(&id))?;
        Ok(TrackDto::from(&track))
    }

    /// Replaces the queue and returns its length.
    pub fn set_queue(&self, track_ids: Vec<String>) {
        self.inner.set_queue(track_ids.into_iter().map(CoreTrackId).collect());
    }

    /// Appends a track to the queue, returning the new length.
    pub fn enqueue(&self, id: String) -> usize {
        self.inner.enqueue(CoreTrackId(id));
        self.inner.queue_len()
    }

    /// The number of queued tracks.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.inner.queue_len()
    }

    /// Advances the queue, returning the new current track when there is one.
    ///
    /// # Errors
    ///
    /// Fails with a mapped [`BridgeError`] when opening the next stream fails.
    pub fn next(&self) -> Result<Option<TrackDto>, BridgeError> {
        let session = self.block_on(self.inner.next())?;
        let Some(session) = session else {
            return Ok(None);
        };
        let track = self.block_on(self.inner.track(&session.track))?;
        Ok(Some(TrackDto::from(&track)))
    }

    /// Sets the quality tier.
    ///
    /// Returns the tier's label so the UI can confirm what it selected.
    pub fn set_quality(&self, quality: &str) -> Result<String, BridgeError> {
        let parsed = parse_quality(quality).ok_or_else(|| BridgeError {
            kind: "other".into(),
            message: format!("unknown quality {quality}"),
            remedy: None,
        })?;
        self.inner.set_quality(parsed);
        Ok(parsed.label().to_owned())
    }

    /// The current quality label.
    #[must_use]
    pub fn quality(&self) -> String {
        self.inner.quality().label().to_owned()
    }

    /// Fetches lyrics for a track.
    ///
    /// # Errors
    ///
    /// Fails with a mapped [`BridgeError`].
    pub fn lyrics(&self, id: String) -> Result<LyricsDto, BridgeError> {
        let id = CoreTrackId(id);
        let lyrics = self.block_on(self.inner.lyrics(&id))?;
        Ok(LyricsDto {
            plain: lyrics.plain,
            language: lyrics.language,
            lines: lyrics
                .timed
                .into_iter()
                .map(|line| LyricLineDto {
                    start_ms: line.start_ms,
                    end_ms: line.end_ms,
                    text: line.text,
                    translation: line.translation,
                })
                .collect(),
        })
    }

    /// The current session.
    #[must_use]
    pub fn session(&self) -> SessionDto {
        SessionDto::from(&self.inner.session())
    }

    /// Records a signed-in session and persists it.
    ///
    /// The session is handed to every provider immediately, so a search or a
    /// playback request made straight after this call is authenticated. It is
    /// also written to the encrypted store, so a restart does not require
    /// signing in again.
    ///
    /// # Errors
    ///
    /// Fails when the store cannot be written. The in-memory session is still
    /// applied, so a failed write costs persistence rather than the sign-in.
    pub fn set_session(
        &self,
        music_user_token: String,
        storefront: String,
    ) -> Result<SessionDto, BridgeError> {
        let mut session = CoreSession::new();
        session.set_music_user_token(music_user_token);
        session.set_storefront(storefront);

        self.inner.set_session(session)?;

        if let Ok(sealer) = machine_sealer() {
            self.inner.persist_session(sealer)?;
        }

        Ok(SessionDto::from(&self.inner.session()))
    }

    /// Signs out and clears the persisted session.
    pub fn sign_out(&self) {
        let mut session = CoreSession::new();
        session.set_storefront(self.inner.session().storefront);
        let _ = self.inner.set_session(session);
        if let Ok(sealer) = machine_sealer() {
            let _ = self.inner.persist_session(sealer);
        }
    }

    /// Starts the audio engine.
    ///
    /// Idempotent. The bridge already starts audio when it creates the engine,
    /// so this exists for a caller that stopped it and wants it back.
    pub fn start_audio(&self) {
        self.inner.start_audio();
    }

    /// True when audio output is available.
    #[must_use]
    pub fn audio_ready(&self) -> bool {
        self.inner.audio_ready()
    }

    /// Pauses playback.
    ///
    /// # Errors
    ///
    /// Fails with the `capability` kind when no audio device is available.
    pub fn pause(&self) -> Result<(), BridgeError> {
        self.inner.pause()?;
        Ok(())
    }

    /// Resumes playback.
    ///
    /// # Errors
    ///
    /// Fails with the `capability` kind when no audio device is available.
    pub fn resume(&self) -> Result<(), BridgeError> {
        self.inner.resume()?;
        Ok(())
    }

    /// Stops playback and unloads the track.
    ///
    /// # Errors
    ///
    /// Fails with the `capability` kind when no audio device is available.
    pub fn stop(&self) -> Result<(), BridgeError> {
        self.inner.stop()?;
        Ok(())
    }

    /// The storefront in use.
    #[must_use]
    pub fn storefront(&self) -> String {
        self.inner.session().storefront
    }

    /// Sets the repeat mode. Accepts `off`, `all`, or `one`.
    ///
    /// Returns the mode that was applied, so the UI can reflect it.
    pub fn set_repeat(&self, mode: &str) -> String {
        let parsed = parse_repeat(mode);
        let label = match parsed {
            CoreRepeat::All => "all",
            CoreRepeat::One => "one",
            CoreRepeat::Off => "off",
        };
        self.inner.set_repeat(parsed);
        label.to_owned()
    }

    /// Streams core events to the UI.
    ///
    /// The Rust core publishes to a broadcast bus; this method subscribes and
    /// forwards onto a Dart `Stream`. Call it once: the UI fans the single
    /// subscription out itself, which keeps one consumer on the Rust side.
    ///
    /// The stream ends when the engine is dropped.
    pub fn watch_events(&self, sink: StreamSink<EventDto>) {
        let mut receiver = self.inner.subscribe();
        self.runtime.spawn(async move {
            while let Some(event) = receiver.next().await {
                if sink.add(EventDto::from(event)).is_err() {
                    break;
                }
            }
        });
    }

    /// The operating system this build targets.
    #[must_use]
    pub fn platform(&self) -> String {
        std::env::consts::OS.to_owned()
    }
}

/// Parses a quality name from Dart.
fn parse_quality(value: &str) -> Option<CoreQuality> {
    match value {
        "low" => Some(CoreQuality::Low),
        "high" => Some(CoreQuality::High),
        "lossless" => Some(CoreQuality::Lossless),
        "hi_res_lossless" => Some(CoreQuality::HiResLossless),
        "spatial" => Some(CoreQuality::Spatial),
        _ => None,
    }
}

/// Parses a repeat mode name from Dart.
///
/// Private: returns a core type, which must not appear in a public signature.
fn parse_repeat(value: &str) -> CoreRepeat {
    match value {
        "all" => CoreRepeat::All,
        "one" => CoreRepeat::One,
        _ => CoreRepeat::Off,
    }
}

/// The provider ids this build knows about.
#[must_use]
pub fn known_providers() -> Vec<String> {
    vec![ProviderId::APPLE.to_owned(), ProviderId::LOCAL.to_owned()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::ProviderError;

    #[test]
    fn quality_names_round_trip() {
        for (name, expected) in [
            ("low", CoreQuality::Low),
            ("high", CoreQuality::High),
            ("lossless", CoreQuality::Lossless),
            ("hi_res_lossless", CoreQuality::HiResLossless),
            ("spatial", CoreQuality::Spatial),
        ] {
            assert_eq!(parse_quality(name), Some(expected));
        }
        assert_eq!(parse_quality("nonsense"), None);
    }

    #[test]
    fn repeat_names_parse_with_a_safe_default() {
        assert_eq!(parse_repeat("all"), CoreRepeat::All);
        assert_eq!(parse_repeat("one"), CoreRepeat::One);
        assert_eq!(parse_repeat("anything else"), CoreRepeat::Off);
    }

    #[test]
    fn auth_failure_becomes_an_actionable_error() {
        let error = BridgeError::from_provider(&ProviderError::NotAuthenticated);
        assert_eq!(error.kind, "not_authenticated");
        assert!(error.remedy.is_some());
    }

    #[test]
    fn a_missing_capability_preserves_its_remedy() {
        let error = BridgeError::from_provider(&ProviderError::CapabilityMissing {
            feature: "fairplay",
            remedy: "Install from Settings".into(),
        });
        assert_eq!(error.kind, "capability");
        assert_eq!(error.remedy.as_deref(), Some("Install from Settings"));
    }

    #[test]
    fn a_position_event_carries_its_offset() {
        let dto = EventDto::from(CoreEvent::Position { position_ms: 1_234 });
        assert_eq!(dto.kind, "position");
        assert_eq!(dto.position_ms, Some(1_234));
    }

    #[test]
    fn a_track_change_event_carries_display_fields() {
        let dto = EventDto::from(CoreEvent::TrackChanged {
            track: CoreTrackId("apple:1".into()),
            title: "Song".into(),
            artist: "Artist".into(),
            duration_ms: Some(1_000),
            artwork_url: Some("https://art".into()),
        });
        assert_eq!(dto.kind, "track_changed");
        assert_eq!(dto.title.as_deref(), Some("Song"));
        assert_eq!(dto.artist.as_deref(), Some("Artist"));
        assert_eq!(dto.duration_ms, Some(1_000));
    }

    #[test]
    fn a_bare_track_event_has_no_extra_payload() {
        let dto = EventDto::from(CoreEvent::Playing);
        assert_eq!(dto.kind, "playing");
        assert!(dto.track_id.is_none());
    }

    #[test]
    fn empty_search_results_report_empty() {
        assert!(SearchResultsDto::default().is_empty());
    }

    #[test]
    fn the_provider_list_is_stable() {
        assert_eq!(known_providers(), vec!["apple", "local"]);
    }
}
