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

use sosumi_core::capability::{CapabilityReport, Runtime, SharedRuntime};
use sosumi_core::event::Event as CoreEvent;
use sosumi_core::fairplay_sign_in::{SignIn, SignInEvent as CoreSignInEvent};
use sosumi_core::install::InstallEvent as CoreInstallEvent;
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

/// Maps a sign-in failure to something the form can show.
///
/// The variants are kept apart because the remedies differ: a missing component
/// needs an install, a rejected password needs the user to read Apple's message,
/// and a busy session needs a moment rather than a correction.
impl From<sosumi_fairplay::SignInError> for BridgeError {
    fn from(err: sosumi_fairplay::SignInError) -> Self {
        use sosumi_fairplay::SignInError as E;
        match &err {
            E::RuntimeMissing => Self {
                kind: "capability".into(),
                message: "The lossless component is not installed".into(),
                remedy: Some("Install it from Settings, then try again".into()),
            },
            E::AlreadyInitialised => Self {
                kind: "busy".into(),
                message: "A sign-in is already in progress".into(),
                remedy: Some("Wait for it to finish, or cancel it".into()),
            },
            E::Unanswered => Self {
                kind: "cancelled".into(),
                message: "Sign-in was cancelled".into(),
                remedy: None,
            },
            E::InteriorNul | E::Bridge(_) => {
                Self { kind: "other".into(), message: err.to_string(), remedy: None }
            }
        }
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

/// One step of a native sign-in.
#[derive(Debug, Clone, Default)]
pub struct SignInEventDto {
    /// One of `started`, `code_required`, `preparing`, `succeeded`, or `failed`.
    pub kind: String,
    /// A message fit for the sign-in form, on failure.
    pub message: Option<String>,
}

/// One step of a lossless install, as the Settings screen renders it.
#[derive(Debug, Clone, Default)]
pub struct InstallEventDto {
    /// One of `started`, `progress`, `verifying`, `finished`, `cancelled`, or
    /// `failed`.
    pub kind: String,
    /// The file being fetched, when the event names one.
    pub file: Option<String>,
    /// One-based position of `file` in the queue.
    pub index: u32,
    /// How many files are queued.
    pub files: u32,
    /// Bytes received of the current file.
    pub done: u64,
    /// The current file's size, when the server reported it.
    pub total: Option<u64>,
    /// A message fit for the screen, on failure.
    pub message: Option<String>,
}

impl InstallEventDto {
    /// Maps a core event into the flat shape Dart holds.
    fn from_core(event: CoreInstallEvent) -> Self {
        match event {
            CoreInstallEvent::Started { files } => Self {
                kind: "started".into(),
                files: u32::try_from(files).unwrap_or(u32::MAX),
                ..Self::default()
            },
            CoreInstallEvent::Progress { file, index, files, done, total } => Self {
                kind: "progress".into(),
                file: Some(file),
                index: u32::try_from(index).unwrap_or(u32::MAX),
                files: u32::try_from(files).unwrap_or(u32::MAX),
                done,
                total,
                message: None,
            },
            CoreInstallEvent::Verifying { file, index, files } => Self {
                kind: "verifying".into(),
                file: Some(file),
                index: u32::try_from(index).unwrap_or(u32::MAX),
                files: u32::try_from(files).unwrap_or(u32::MAX),
                ..Self::default()
            },
            CoreInstallEvent::Finished { fetched } => Self {
                kind: "finished".into(),
                index: u32::try_from(fetched).unwrap_or(u32::MAX),
                ..Self::default()
            },
            CoreInstallEvent::Cancelled => Self { kind: "cancelled".into(), ..Self::default() },
            CoreInstallEvent::Failed { message } => {
                Self { kind: "failed".into(), message: Some(message), ..Self::default() }
            }
        }
    }
}

impl SignInEventDto {
    fn from(event: CoreSignInEvent) -> Self {
        match event {
            CoreSignInEvent::Started => Self { kind: "started".into(), message: None },
            CoreSignInEvent::CodeRequired => Self { kind: "code_required".into(), message: None },
            CoreSignInEvent::Preparing => Self { kind: "preparing".into(), message: None },
            CoreSignInEvent::Succeeded => Self { kind: "succeeded".into(), message: None },
            CoreSignInEvent::Failed { message } => {
                Self { kind: "failed".into(), message: Some(message) }
            }
        }
    }
}

/// One optional capability, as the Settings screen renders it.
#[derive(Debug, Clone, Default)]
pub struct CapabilityDto {
    /// Stable machine id, for example `fairplay`.
    pub id: String,
    /// Display name, for example "Lossless & Hi-Res".
    pub name: String,
    /// One of `ready`, `available`, `degraded`, or `unsupported`.
    pub state: String,
    /// One line explaining the state.
    pub detail: String,
    /// What the user can do, when there is something to do.
    pub action: Option<String>,
    /// The specific files missing, for an installable tier.
    pub missing: Vec<String>,
    /// True when the tier is installed and working.
    pub ready: bool,
}

impl CapabilityDto {
    /// Projects a capability report into the flat shape Dart holds.
    fn from_report(report: &CapabilityReport, name: &str, id: &str) -> Self {
        let (state, detail, action, missing) = match report {
            CapabilityReport::Ready { detail } => {
                ("ready".to_owned(), detail.clone(), None, Vec::new())
            }
            CapabilityReport::Installable { detail, missing } => (
                "available".to_owned(),
                detail.clone(),
                Some("Install".to_owned()),
                missing.clone(),
            ),
            CapabilityReport::Damaged { problems } => {
                ("degraded".to_owned(), problems.join("; "), Some("Repair".to_owned()), Vec::new())
            }
            CapabilityReport::Unsupported { reason } => {
                ("unsupported".to_owned(), reason.clone(), None, Vec::new())
            }
        };

        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            state,
            detail,
            action,
            missing,
            ready: report.to_health().status.is_ready(),
        }
    }
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
    /// A long-lived channel of sign-in progress.
    ///
    /// Owned by the engine rather than by an attempt, so a subscriber cannot bind
    /// to the wrong attempt. Subscribing before starting is the natural order, and
    /// an attempt-scoped channel made that order lose every event: the
    /// subscription attached to nothing, then the attempt reported into a channel
    /// with no listeners, and the UI waited forever.
    sign_in_events: tokio::sync::broadcast::Sender<CoreSignInEvent>,
    /// The in-flight native sign-in, when there is one.
    sign_in: parking_lot::RwLock<Option<Arc<SignIn>>>,
    /// A long-lived channel of install progress.
    ///
    /// Owned by the engine for the same reason as the sign-in channel: a
    /// subscriber must be able to attach before the work starts, and the natural
    /// order for a UI is to listen first and then begin.
    install_events: tokio::sync::broadcast::Sender<CoreInstallEvent>,
    /// Set while a fetch is running, so the UI can cancel it and so a second
    /// install cannot start on top of the first.
    ///
    /// Behind an `Arc` because the spawned task clears it when it finishes, which
    /// is how a later start learns the slot is free.
    install_cancel: Arc<parking_lot::RwLock<Option<Arc<std::sync::atomic::AtomicBool>>>>,
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
    let mut app = Sosumi::new(paths.clone())?;

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

    // Attach the optional lossless runtime. A missing manifest is not fatal: the
    // app runs without the tier, and the capability reports the install as broken
    // rather than offering one that cannot work.
    match attach_runtime(&paths) {
        Ok(runtime) => app.set_runtime(runtime),
        Err(error) => {
            tracing::warn!(target: "sosumi::bridge", %error, "lossless runtime unavailable");
        }
    }

    // Restore a previous sign-in so the user does not authenticate every launch.
    // A failure is not fatal: it means the store is unreadable, and signing in
    // again repairs it.
    if let Ok(sealer) = machine_sealer() {
        if let Err(error) = app.restore_session(sealer) {
            tracing::warn!(target: "sosumi::bridge", %error, "could not restore the session");
        }
    }

    // Then prefer the native session, when there is one, over the restored copy.
    // Apple's library refreshes `MUSIC_TOKEN` on every launch, so the file on disk
    // is newer than what was persisted at the last sign-in; without this the app
    // would run on an expired token until the user signed in again. The developer
    // token, which the native flow never writes, is kept from the restored session.
    if let Some(native) =
        app.runtime().and_then(|runtime| native_session(&runtime.layout().session).ok())
    {
        let mut session = app.session();
        session.set_music_user_token(native.token);
        if let Some(storefront) = native.storefront {
            session.set_storefront(storefront);
        }
        if let Err(error) = app.set_session(session) {
            tracing::warn!(target: "sosumi::bridge", %error, "could not apply the native session");
        }
    }

    // Sized for the handful of events one attempt emits, with slack for a
    // subscriber that is briefly slow.
    let (sign_in_events, _) = tokio::sync::broadcast::channel(32);
    // Progress events arrive in bursts while a file streams, so this channel is
    // bigger; a subscriber that lags rejoins at the newest event, which is what a
    // progress bar wants anyway.
    let (install_events, _) = tokio::sync::broadcast::channel(256);

    Ok(Engine {
        inner: Arc::new(app),
        runtime,
        sign_in_events,
        sign_in: parking_lot::RwLock::new(None),
        install_events,
        install_cancel: Arc::new(parking_lot::RwLock::new(None)),
    })
}

/// Reads a small text file, trimming the trailing newline the C library writes.
///
/// Returns `None` when the file is absent or empty, which is the ordinary state
/// before a sign-in rather than an error.
fn read_trimmed(path: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// The session the native flow wrote to disk, or the reason it wrote none.
///
/// A struct rather than a [`CoreSession`] because the two fields have different
/// weight: the token is required and its absence is the failure, while the
/// storefront is optional and must *not* be defaulted here — the caller merges it
/// into a session that may already carry a better value, and inventing `us` would
/// overwrite it.
#[derive(Debug)]
struct NativeSession {
    /// The subscriber token. Present whenever the native flow completed.
    token: String,
    /// The two-letter storefront, when the file was readable and recognisable.
    storefront: Option<String>,
}

/// Reads what the native flow wrote to disk.
///
/// A free function rather than a method because two callers need it: adoption
/// after a sign-in, and the startup refresh. Keeping it in one place means the
/// on-disk contract is read the same way in both.
///
/// # Errors
///
/// Fails when `MUSIC_TOKEN` is absent or empty, which is the ordinary state
/// before any sign-in.
fn native_session(session_dir: &std::path::Path) -> Result<NativeSession, BridgeError> {
    let token = read_trimmed(&session_dir.join("MUSIC_TOKEN")).ok_or_else(|| BridgeError {
        kind: "not_authenticated".into(),
        message: "Apple did not return a session token".into(),
        remedy: Some("Try signing in again".into()),
    })?;

    let storefront = read_trimmed(&session_dir.join("STOREFRONT_ID"))
        .as_deref()
        .and_then(sosumi_provider_apple::storefront::region_for);

    Ok(NativeSession { token, storefront })
}

/// Locates the runtime manifest and resolves the payload layout.
///
/// The manifest ships beside the executable, so it is found relative to the
/// binary rather than the working directory: an app launched from a desktop entry
/// has no useful working directory, and a relative path would silently miss.
fn attach_runtime(paths: &Paths) -> anyhow::Result<SharedRuntime> {
    let manifest = discover_manifest()?;
    let runtime = Runtime::load(&paths.runtime(), &manifest)?;
    Ok(Arc::new(runtime))
}

/// Finds the runtime manifest.
///
/// Checked in order: an override for development, then beside the executable (the
/// packaged layout), then the source tree (a `flutter run` from the repository).
fn discover_manifest() -> anyhow::Result<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("SOSUMI_RUNTIME_MANIFEST") {
        let path = std::path::PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("SOSUMI_RUNTIME_MANIFEST points at {}, which is not a file", path.display());
    }

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // The packaged layout puts assets under the bundle's data directory.
            candidates.push(dir.join("data/flutter_assets/assets/runtime.toml"));
            candidates.push(dir.join("assets/runtime.toml"));
        }
    }
    // A development run from the repository.
    candidates.push(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/assets/runtime.toml"),
    );

    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| anyhow::anyhow!("the runtime manifest was not found beside the executable"))
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
    /// The token is merged into the existing session rather than replacing it,
    /// keeping the developer token the app discovered separately; replacing would
    /// leave a session that reports signed in but cannot play.
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
        let mut session = self.inner.session();
        session.set_music_user_token(music_user_token);
        session.set_storefront(storefront);

        self.inner.set_session(session)?;

        if let Ok(sealer) = machine_sealer() {
            self.inner.persist_session(sealer)?;
        }

        Ok(SessionDto::from(&self.inner.session()))
    }

    /// Signs out and clears the persisted session.
    ///
    /// The native flow's own token files are removed too. They are what the
    /// startup refresh reads, so leaving them behind would sign the user back in
    /// on the next launch — a sign-out that quietly undoes itself. The storefront
    /// is kept, since it says where the account was rather than who it was.
    pub fn sign_out(&self) {
        let mut session = CoreSession::new();
        session.set_storefront(self.inner.session().storefront);
        let _ = self.inner.set_session(session);
        if let Ok(sealer) = machine_sealer() {
            let _ = self.inner.persist_session(sealer);
        }
        if let Some(runtime) = self.inner.runtime() {
            let session_dir = &runtime.layout().session;
            for name in ["MUSIC_TOKEN", "STOREFRONT_ID"] {
                let _ = std::fs::remove_file(session_dir.join(name));
            }
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

    /// Reports the optional capabilities this machine has.
    ///
    /// Read-only and local, so the Settings screen can call it freely.
    #[must_use]
    pub fn capabilities(&self) -> Vec<CapabilityDto> {
        let mut report = Vec::with_capacity(4);

        report.push(CapabilityDto::from_report(
            &self.inner.lossless_capability(),
            "Lossless & Hi-Res",
            "fairplay",
        ));

        // Spatial Audio: whether an object renderer is installed. Probed by path
        // rather than started, so the check is cheap.
        //
        // `sosumi_spatial::health` reports a `Health`, not a `CapabilityReport`,
        // because the renderer makes no distinction between "installable" and
        // "degraded": the downmix always works, and the only question is whether
        // object rendering is available.
        let cavern = sosumi_spatial::CavernRenderer::detect();
        let spatial = sosumi_spatial::health(&cavern);
        report.push(CapabilityDto {
            id: "spatial".into(),
            name: "Spatial Audio".into(),
            state: if spatial.status.is_ready() { "ready" } else { "available" }.into(),
            detail: spatial.detail,
            action: spatial.action,
            missing: Vec::new(),
            ready: spatial.status.is_ready(),
        });

        // System media controls: present whenever a session bus is reachable.
        report.push(if sosumi_linux::mpris::session_bus_available() {
            CapabilityDto {
                id: "mpris".into(),
                name: "System Media Controls".into(),
                state: "ready".into(),
                detail: "Control playback from your desktop".into(),
                action: None,
                missing: Vec::new(),
                ready: true,
            }
        } else {
            CapabilityDto {
                id: "mpris".into(),
                name: "System Media Controls".into(),
                state: "unsupported".into(),
                detail: "No D-Bus session bus".into(),
                action: None,
                missing: Vec::new(),
                ready: false,
            }
        });

        // Discord presence: available when the local client is running.
        report.push(if sosumi_discord::ipc::is_running() {
            CapabilityDto {
                id: "discord".into(),
                name: "Discord Rich Presence".into(),
                state: "ready".into(),
                detail: "Show what you are listening to".into(),
                action: None,
                missing: Vec::new(),
                ready: true,
            }
        } else {
            CapabilityDto {
                id: "discord".into(),
                name: "Discord Rich Presence".into(),
                state: "available".into(),
                detail: "Discord is not running".into(),
                action: Some("Start Discord".into()),
                missing: Vec::new(),
                ready: false,
            }
        });

        report
    }

    /// Writes the stub libraries for the lossless tier.
    ///
    /// The half of installation that needs no network: the stubs are carried in
    /// this binary. Apple's own files still have to be fetched, and
    /// [`Engine::capabilities`] reports which ones are missing afterwards.
    ///
    /// # Errors
    ///
    /// Fails when the runtime is not attached or the stubs cannot be written.
    pub fn install_lossless_stubs(&self) -> Result<usize, BridgeError> {
        self.inner.install_lossless_stubs().map_err(Into::into)
    }

    /// Starts installing the lossless tier, reporting progress as it goes.
    ///
    /// Returns immediately; progress arrives on [`Engine::watch_install`]. The
    /// fetch runs on the engine's runtime, so a 60 MiB download never blocks the
    /// caller. Calling this while an install is already running is refused
    /// rather than queued: two concurrent writes into one payload directory is
    /// the kind of race that leaves a half-written library behind.
    ///
    /// # Errors
    ///
    /// Fails when the runtime is not attached or an install is already running.
    pub fn begin_lossless_install(&self) -> Result<(), BridgeError> {
        if self.install_cancel.read().is_some() {
            return Err(BridgeError {
                kind: "other".into(),
                message: "An install is already in progress".into(),
                remedy: None,
            });
        }

        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let slot = Arc::clone(&self.install_cancel);
        *slot.write() = Some(Arc::clone(&cancel));

        let inner = Arc::clone(&self.inner);
        let events = self.install_events.clone();

        self.runtime.spawn(async move {
            let sink = events.clone();
            let result = inner
                .install_lossless(cancel, move |event| {
                    let _ = sink.send(event);
                })
                .await;

            if let Err(error) = result {
                // Reported as an event rather than only a log: a fetch that fails
                // has to reach the screen, or the bar sits full and nothing says
                // why. `failed` rather than `cancelled`, so the UI can offer a
                // retry instead of looking as though the user stopped it.
                let _ = events.send(CoreInstallEvent::Failed { message: error.to_string() });
                tracing::warn!(target: "sosumi::bridge", %error, "lossless install failed");
            }

            // Released last, so a start attempt during the tail of this one is
            // still refused.
            *slot.write() = None;
        });

        Ok(())
    }

    /// Streams install progress to the UI.
    ///
    /// Emits until a terminal event. Subscribing before starting is safe, which
    /// is the order the UI uses.
    pub fn watch_install(&self, sink: StreamSink<InstallEventDto>) {
        let mut events = self.install_events.subscribe();
        self.runtime.spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        let terminal = matches!(
                            event,
                            CoreInstallEvent::Finished { .. }
                                | CoreInstallEvent::Cancelled
                                | CoreInstallEvent::Failed { .. }
                        );
                        if sink.add(InstallEventDto::from_core(event)).is_err() || terminal {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// Stops an install in progress.
    ///
    /// Files already fetched and verified are kept, so a cancel followed by
    /// another start resumes rather than beginning again.
    pub fn cancel_lossless_install(&self) {
        if let Some(cancel) = self.install_cancel.read().clone() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Whether a lossless sign-in has completed.
    #[must_use]
    pub fn lossless_signed_in(&self) -> bool {
        self.inner.runtime().is_some_and(|runtime| runtime.has_session())
    }

    /// How ready sign-in is, as one of `ready`, `installable`, `damaged`, or
    /// `unsupported`.
    ///
    /// Sign-in is done by Apple's own libraries, so those libraries must be
    /// installed before there is anything to sign in to. Asking first is what
    /// lets the form explain that up front, instead of collecting a password and
    /// then refusing it — which reads as a wrong password.
    ///
    /// A string rather than a bool because the states have different remedies:
    /// `installable` is the one Sosumi can fix, and `unsupported` is the one it
    /// must not pretend to.
    #[must_use]
    pub fn sign_in_readiness(&self) -> String {
        let Some(runtime) = self.inner.runtime() else {
            // No manifest at all is a broken install rather than a missing tier,
            // so it is not something an install press could fix.
            return "unsupported".into();
        };
        if runtime.has_session() {
            return "ready".into();
        }
        match runtime.probe() {
            CapabilityReport::Ready { .. } => "ready".into(),
            CapabilityReport::Installable { .. } => "installable".into(),
            CapabilityReport::Damaged { .. } => "damaged".into(),
            CapabilityReport::Unsupported { .. } => "unsupported".into(),
        }
    }

    /// Starts a native sign-in to Apple.
    ///
    /// Returns immediately; progress arrives on [`Engine::watch_sign_in`]. The
    /// work runs on a background thread because the underlying call blocks for
    /// the whole login, including while it waits for a verification code.
    ///
    /// The password is moved into that thread, used for one call, and dropped. It
    /// is never stored, logged, or returned.
    ///
    /// # Errors
    ///
    /// Fails when the runtime is not attached, when a sign-in is already running,
    /// or when the worker cannot start.
    pub fn begin_sign_in(&self, email: String, password: String) -> Result<(), BridgeError> {
        let runtime = self.inner.runtime().ok_or_else(|| BridgeError {
            kind: "capability".into(),
            message: "The lossless component is not installed".into(),
            remedy: Some("Install it from Settings first".into()),
        })?;

        let layout = runtime.layout().clone();
        let sign_in = SignIn::begin(layout, email, password, self.sign_in_events.clone())?;

        *self.sign_in.write() = Some(Arc::new(sign_in));
        Ok(())
    }

    /// Streams sign-in progress to the UI.
    ///
    /// Emits one event per state change until the attempt ends. A second call
    /// while an attempt is running subscribes to the same attempt rather than
    /// starting another.
    ///
    /// # Errors
    ///
    /// Fails when no sign-in is in progress.
    pub fn watch_sign_in(&self, sink: StreamSink<SignInEventDto>) {
        // Subscribes to the engine's channel, which exists whether or not an
        // attempt is running. That makes the call safe in any order: a
        // subscription made before the start receives the events of the attempt
        // that follows, which is the order the UI naturally uses.
        let mut events = self.sign_in_events.subscribe();
        self.runtime.spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        let terminal = matches!(
                            event,
                            CoreSignInEvent::Succeeded | CoreSignInEvent::Failed { .. }
                        );
                        if sink.add(SignInEventDto::from(event)).is_err() || terminal {
                            break;
                        }
                    }
                    // A lagged subscriber rejoins at the oldest retained event,
                    // which for a short stream is the whole stream.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// Supplies the verification code the bridge is waiting for.
    ///
    /// # Errors
    ///
    /// Fails when no sign-in is in progress or the attempt already ended.
    pub fn submit_sign_in_code(&self, code: String) -> Result<(), BridgeError> {
        let sign_in = self.sign_in.read().clone().ok_or_else(|| BridgeError {
            kind: "other".into(),
            message: "No sign-in is in progress".into(),
            remedy: None,
        })?;
        sign_in.submit_code(code).map_err(Into::into)
    }

    /// Reads the session the native sign-in wrote and hands it to the providers.
    ///
    /// This is the step that makes signing in actually take effect. The native
    /// bridge writes `MUSIC_TOKEN` and `STOREFRONT_ID` into its own session
    /// directory, and until those reach the providers every request stays
    /// unauthenticated — so a successful sign-in would look like a broken one,
    /// with search returning "not authenticated" and no explanation for why.
    ///
    /// The current session is *updated* rather than replaced. The native flow
    /// supplies a subscriber token and a storefront but no developer token, which
    /// the app obtains separately; replacing the session would drop that token and
    /// leave a session that reports signed in but cannot play.
    ///
    /// Returns the session as the app now sees it.
    ///
    /// # Errors
    ///
    /// Fails when the lossless component is not attached, or when it finished
    /// without writing a token.
    pub fn adopt_native_session(&self) -> Result<SessionDto, BridgeError> {
        let runtime = self.inner.runtime().ok_or_else(|| BridgeError {
            kind: "capability".into(),
            message: "The lossless component is not installed".into(),
            remedy: Some("Install it from Settings first".into()),
        })?;

        let native = native_session(&runtime.layout().session)?;

        // Merged with what is already known, so a developer token obtained on an
        // earlier run is not thrown away by a sign-in that never had one, and a
        // storefront the native flow did not report is left as it was.
        let mut session = self.inner.session();
        session.set_music_user_token(native.token);
        if let Some(storefront) = native.storefront {
            session.set_storefront(storefront);
        }

        // Propagated to the providers as well as stored, which is what makes the
        // *next* request authenticated rather than the next restart.
        self.inner.set_session(session)?;

        if let Ok(sealer) = machine_sealer() {
            let _ = self.inner.persist_session(sealer);
        }

        Ok(SessionDto::from(&self.inner.session()))
    }

    /// Gives up on a sign-in in progress.
    pub fn cancel_sign_in(&self) {
        if let Some(sign_in) = self.sign_in.read().clone() {
            sign_in.abandon();
        }
        *self.sign_in.write() = None;
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
    fn a_native_session_is_read_with_its_translated_storefront() {
        // The real files Apple's wrapper writes: `143467-2,31` is India, and the
        // token carries a trailing newline. Both have to survive the read, since a
        // numeric storefront would be sent as a path segment and a trailing
        // newline would make every request malformed.
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::write(dir.path().join("MUSIC_TOKEN"), "abc123\n").expect("a token");
        std::fs::write(dir.path().join("STOREFRONT_ID"), "143467-2,31").expect("a storefront");

        let session = native_session(dir.path()).expect("a native session");

        assert_eq!(session.token, "abc123");
        assert_eq!(session.storefront.as_deref(), Some("in"));
    }

    #[test]
    fn a_native_session_without_a_storefront_has_none() {
        // Not defaulted here: the caller merges into a session that may already
        // hold a better storefront, and inventing `us` would overwrite it.
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::write(dir.path().join("MUSIC_TOKEN"), "abc123").expect("a token");

        let session = native_session(dir.path()).expect("a native session");

        assert_eq!(session.token, "abc123");
        assert_eq!(session.storefront, None);
    }

    #[test]
    fn a_native_session_without_a_token_is_an_error() {
        // The ordinary pre-sign-in state, and the one that must not be read as
        // success: adopting a tokenless session would report signed in with
        // nothing to send.
        let dir = tempfile::tempdir().expect("a temp dir");
        std::fs::write(dir.path().join("STOREFRONT_ID"), "143441-1,29").expect("a storefront");

        let error = native_session(dir.path()).expect_err("no token, no session");

        assert_eq!(error.kind, "not_authenticated");
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
