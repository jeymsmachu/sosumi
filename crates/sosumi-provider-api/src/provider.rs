//! The [`MediaProvider`] trait.

use async_trait::async_trait;

use crate::error::Result;
use crate::model::{Album, Artist, Lyrics, Page, Playlist, ProviderId, Track, TrackId};
use crate::stream::{PlaybackSession, StreamRequest};

/// A free-text search, with optional paging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    /// Storefront to search in, e.g. `us`.
    pub storefront: Option<String>,
    /// Provider cursor from a previous page.
    pub cursor: Option<String>,
}

impl SearchQuery {
    /// A first-page query in the default storefront.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), storefront: None, cursor: None }
    }
}

/// Everything a catalog search can return, in one round trip.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

impl SearchResults {
    /// True when the query matched nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
    }
}

/// A source of music: a streaming catalog, local files, or anything else.
///
/// Providers are object-safe and `Send + Sync` so the core can hold them behind
/// a `dyn` and switch at runtime. Methods that a provider cannot serve return
/// [`crate::ProviderError::CapabilityMissing`] rather than being absent, so the
/// UI never needs to special-case a provider.
#[async_trait]
pub trait MediaProvider: Send + Sync + 'static {
    /// Stable id, matching the prefix used in [`TrackId`]s.
    fn id(&self) -> &ProviderId;

    /// Human-readable name for the UI.
    fn name(&self) -> &str;

    /// Whether the provider can serve requests right now. Cheap and
    /// non-blocking; it must not perform network I/O.
    fn is_available(&self) -> bool;

    /// Searches the catalog.
    async fn search(&self, query: SearchQuery) -> Result<SearchResults>;

    /// Fetches one track's metadata.
    async fn track(&self, id: &TrackId) -> Result<Track>;

    /// Fetches an album and its tracks.
    async fn album(&self, id: &str, storefront: Option<&str>) -> Result<(Album, Vec<Track>)>;

    /// Fetches a playlist and its tracks.
    async fn playlist(&self, id: &str, storefront: Option<&str>)
        -> Result<(Playlist, Page<Track>)>;

    /// Fetches an artist's top songs and albums.
    async fn artist(&self, id: &str, storefront: Option<&str>) -> Result<(Artist, Vec<Track>)>;

    /// Fetches synced lyrics, if the provider has them.
    async fn lyrics(&self, id: &TrackId, storefront: Option<&str>) -> Result<Lyrics>;

    /// Opens a playable stream.
    ///
    /// The provider picks the best codec it can deliver at or below
    /// `req.quality`, and fails with [`crate::ProviderError::CapabilityMissing`]
    /// when the requested tier needs a component that is not installed.
    async fn open(&self, req: StreamRequest) -> Result<PlaybackSession>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_search_results_report_empty() {
        assert!(SearchResults::default().is_empty());
    }

    #[test]
    fn search_query_defaults_to_no_storefront_or_cursor() {
        let query = SearchQuery::new("radiohead");
        assert_eq!(query.text, "radiohead");
        assert!(query.storefront.is_none());
        assert!(query.cursor.is_none());
    }
}
