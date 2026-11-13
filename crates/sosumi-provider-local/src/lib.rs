//! The local-files provider.
//!
//! It serves audio the user already owns, under the same [`MediaProvider`]
//! contract as Apple Music. That is the point of the contract: the player, the
//! queue, and the UI treat a local FLAC and a streamed ALAC identically.
//!
//! Scope is deliberately small. It indexes a set of directories, recognises
//! audio by extension, derives a title from the filename, and serves the file
//! over the `file://`-equivalent HTTP source the audio layer understands.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use sosumi_provider_api::{
    Album, Artist, AudioQuality, AudioSource, Codec, Lyrics, MediaProvider, Page, PlaybackSession,
    Playlist, ProviderError, ProviderId, Result, SearchQuery, SearchResults, StreamInfo,
    StreamRequest, Track, TrackId,
};

/// Audio file extensions this provider recognises.
const AUDIO_EXTENSIONS: &[&str] = &["flac", "alac", "m4a", "aac", "mp3", "opus", "wav", "ogg"];

/// One indexed file.
#[derive(Debug, Clone)]
struct IndexedFile {
    path: PathBuf,
    title: String,
    /// Present when the file lives in a subdirectory, which is treated as the
    /// album. A flat directory yields no album.
    album: Option<String>,
}

/// Serves audio files from local directories.
#[derive(Debug)]
pub struct LocalProvider {
    roots: Vec<PathBuf>,
    files: Vec<IndexedFile>,
}

impl LocalProvider {
    /// Builds a provider over the given directories.
    ///
    /// Directories that do not exist are skipped rather than treated as an
    /// error: a user's library list may outlive a removable drive.
    #[must_use]
    pub fn new(roots: Vec<PathBuf>) -> Self {
        let mut provider = Self { roots, files: Vec::new() };
        provider.reindex();
        provider
    }

    /// Rescans the roots.
    pub fn reindex(&mut self) {
        self.files.clear();
        // Clone the roots so the scan can borrow `self` mutably below.
        for root in self.roots.clone() {
            self.scan(&root);
        }
        // A stable order keeps the UI from reshuffling between scans.
        self.files.sort_by_key(|file| file.title.to_lowercase());
    }

    fn scan(&mut self, root: &Path) {
        if !root.is_dir() {
            tracing::debug!(target: "sosumi::local", path = %root.display(), "skipping missing root");
            return;
        }

        for entry in walkdir::WalkDir::new(root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if !AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()) {
                continue;
            }

            let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_owned();

            let album = path
                .parent()
                .filter(|parent| *parent != root)
                .and_then(Path::file_name)
                .and_then(|n| n.to_str())
                .map(str::to_owned);

            self.files.push(IndexedFile { path: path.to_path_buf(), title, album });
        }
    }

    /// The number of indexed files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// True when nothing was indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Resolves a track id back to an indexed file.
    fn file_for(&self, id: &TrackId) -> Result<&IndexedFile> {
        let key = id.as_str().strip_prefix("local:").unwrap_or(id.as_str());
        self.files
            .iter()
            .enumerate()
            .find(|(index, _)| index.to_string() == key)
            .map(|(_, file)| file)
            .ok_or_else(|| ProviderError::NotFound(id.as_str().to_owned()))
    }

    /// Maps an indexed file into the domain model.
    fn track_for(index: usize, file: &IndexedFile) -> Track {
        let id = TrackId::new(&ProviderId::from(ProviderId::LOCAL), index.to_string());
        Track {
            id,
            title: file.title.clone(),
            artist: Artist {
                id: "local".to_owned(),
                name: file.album.clone().unwrap_or_else(|| "Local Files".to_owned()),
                artwork: None,
            },
            album: file.album.as_ref().map(|album| Album {
                id: album.clone(),
                title: album.clone(),
                artist: Artist { id: "local".into(), name: "Local Files".into(), artwork: None },
                artwork: None,
                release_date: None,
                track_count: None,
            }),
            artwork: None,
            duration_ms: None,
            isrc: None,
            track_number: None,
            disc_number: None,
        }
    }
}

/// Guesses a codec from a file extension.
fn codec_for(path: &Path) -> Codec {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("flac") => Codec::Flac,
        Some("alac" | "m4a") => Codec::Alac,
        Some("opus" | "ogg") => Codec::Opus,
        Some("wav") => Codec::Pcm,
        _ => Codec::Aac,
    }
}

#[async_trait]
impl MediaProvider for LocalProvider {
    fn id(&self) -> &ProviderId {
        static ID: std::sync::LazyLock<ProviderId> =
            std::sync::LazyLock::new(|| ProviderId(ProviderId::LOCAL.to_owned()));
        &ID
    }

    fn name(&self) -> &'static str {
        "Local Files"
    }

    fn is_available(&self) -> bool {
        !self.files.is_empty()
    }

    async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let needle = query.text.to_lowercase();
        let mut tracks = Vec::new();

        for (index, file) in self.files.iter().enumerate() {
            let haystack = format!(
                "{} {}",
                file.title.to_lowercase(),
                file.album.as_deref().unwrap_or_default().to_lowercase()
            );
            if haystack.contains(&needle) {
                tracks.push(Self::track_for(index, file));
            }
        }

        Ok(SearchResults { tracks, ..SearchResults::default() })
    }

    async fn track(&self, id: &TrackId) -> Result<Track> {
        let key = id.as_str().strip_prefix("local:").unwrap_or(id.as_str());
        let index: usize =
            key.parse().map_err(|_| ProviderError::NotFound(id.as_str().to_owned()))?;
        let file = self.file_for(id)?;
        Ok(Self::track_for(index, file))
    }

    async fn album(&self, id: &str, _storefront: Option<&str>) -> Result<(Album, Vec<Track>)> {
        let mut tracks = Vec::new();
        for (index, file) in self.files.iter().enumerate() {
            if file.album.as_deref() == Some(id) {
                tracks.push(Self::track_for(index, file));
            }
        }
        if tracks.is_empty() {
            return Err(ProviderError::NotFound(format!("album {id}")));
        }
        let album =
            tracks[0].album.clone().ok_or_else(|| ProviderError::NotFound(id.to_owned()))?;
        Ok((album, tracks))
    }

    async fn playlist(
        &self,
        id: &str,
        _storefront: Option<&str>,
    ) -> Result<(Playlist, Page<Track>)> {
        // Local files have no playlists of their own; a directory is the
        // closest equivalent, and it is served as an album instead.
        Err(ProviderError::NotFound(format!("playlist {id}")))
    }

    async fn artist(&self, id: &str, _storefront: Option<&str>) -> Result<(Artist, Vec<Track>)> {
        Err(ProviderError::NotFound(format!("artist {id}")))
    }

    async fn lyrics(&self, _id: &TrackId, _storefront: Option<&str>) -> Result<Lyrics> {
        // Local files carry no lyrics from this provider. A sidecar-file reader
        // can be added without changing the contract.
        Err(ProviderError::NotFound("lyrics".to_owned()))
    }

    async fn open(&self, req: StreamRequest) -> Result<PlaybackSession> {
        let file = self.file_for(&req.track)?;
        let codec = codec_for(&file.path);
        let url = format!("file://{}", file.path.display());

        Ok(PlaybackSession {
            track: req.track,
            info: StreamInfo {
                codec,
                sample_rate: 44_100,
                bit_depth: codec.is_lossless().then_some(16),
                bitrate_kbps: None,
                spatial: None,
            },
            source: AudioSource::Http { url, headers: Vec::new() },
            duration_ms: None,
        })
    }
}

/// Returns `true` when `quality` is servable from local files at any tier.
///
/// Local files are never gated by a capability: the audio is already on disk,
/// so a lossless request is satisfied whenever the file itself is lossless.
#[must_use]
pub const fn supports_quality(_quality: AudioQuality) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, LocalProvider) {
        let dir = tempfile::tempdir().expect("tempdir");
        let album = dir.path().join("In Rainbows");
        std::fs::create_dir_all(&album).expect("mkdir");

        std::fs::write(album.join("01 15 Step.flac"), b"x").expect("write");
        std::fs::write(album.join("02 Bodysnatchers.m4a"), b"x").expect("write");
        std::fs::write(dir.path().join("loose track.mp3"), b"x").expect("write");
        // A non-audio file must be ignored.
        std::fs::write(dir.path().join("cover.jpg"), b"x").expect("write");

        let provider = LocalProvider::new(vec![dir.path().to_path_buf()]);
        (dir, provider)
    }

    #[test]
    fn indexes_only_audio_files() {
        let (_dir, provider) = library();
        assert_eq!(provider.len(), 3, "the JPG must not be indexed");
    }

    #[test]
    fn a_missing_root_is_skipped_not_fatal() {
        let provider = LocalProvider::new(vec![PathBuf::from("/nonexistent/path/xyz")]);
        assert!(provider.is_empty());
        assert!(!provider.is_available());
    }

    #[test]
    fn a_subdirectory_becomes_the_album() {
        let (_dir, provider) = library();
        let track = provider.files.iter().find(|f| f.title == "01 15 Step").expect("track");
        assert_eq!(track.album.as_deref(), Some("In Rainbows"));
    }

    #[test]
    fn a_top_level_file_has_no_album() {
        let (_dir, provider) = library();
        let track = provider.files.iter().find(|f| f.title == "loose track").expect("track");
        assert!(track.album.is_none());
    }

    #[test]
    fn codec_is_guessed_from_the_extension() {
        assert_eq!(codec_for(Path::new("a.flac")), Codec::Flac);
        assert_eq!(codec_for(Path::new("a.m4a")), Codec::Alac);
        assert_eq!(codec_for(Path::new("a.opus")), Codec::Opus);
        assert_eq!(codec_for(Path::new("a.wav")), Codec::Pcm);
        assert_eq!(codec_for(Path::new("a.mp3")), Codec::Aac);
    }

    #[tokio::test]
    async fn search_matches_title_and_album() {
        let (_dir, provider) = library();

        let by_title = provider.search(SearchQuery::new("bodysnatchers")).await.expect("search");
        assert_eq!(by_title.tracks.len(), 1);

        let by_album = provider.search(SearchQuery::new("rainbows")).await.expect("search");
        assert_eq!(by_album.tracks.len(), 2);
    }

    #[tokio::test]
    async fn search_is_case_insensitive() {
        let (_dir, provider) = library();
        let results = provider.search(SearchQuery::new("STEP")).await.expect("search");
        assert_eq!(results.tracks.len(), 1);
    }

    #[tokio::test]
    async fn opening_a_track_yields_a_file_source() {
        let (_dir, provider) = library();
        let search = provider.search(SearchQuery::new("15 Step")).await.expect("search");
        let track = &search.tracks[0];

        let session = provider
            .open(StreamRequest {
                track: track.id.clone(),
                storefront: None,
                quality: AudioQuality::Lossless,
            })
            .await
            .expect("open");

        assert_eq!(session.info.codec, Codec::Flac);
        match session.source {
            AudioSource::Http { url, .. } => assert!(url.starts_with("file://")),
            AudioSource::Decrypting(_) => panic!("expected a file source"),
        }
    }

    #[tokio::test]
    async fn an_unknown_track_is_not_found() {
        let (_dir, provider) = library();
        let id = TrackId::new(&ProviderId::from(ProviderId::LOCAL), "9999");
        assert!(matches!(provider.track(&id).await, Err(ProviderError::NotFound(_))));
    }

    #[tokio::test]
    async fn playlists_are_not_served() {
        let (_dir, provider) = library();
        assert!(provider.playlist("anything", None).await.is_err());
    }

    #[test]
    fn local_files_never_need_a_capability() {
        for quality in [
            AudioQuality::Low,
            AudioQuality::High,
            AudioQuality::Lossless,
            AudioQuality::HiResLossless,
            AudioQuality::Spatial,
        ] {
            assert!(supports_quality(quality));
        }
    }
}
