//! Provider-agnostic domain model.
//!
//! These types describe music as the UI understands it. They are deliberately
//! free of Apple-specific concepts: an `apple:1234` id and a local file id are
//! both just [`TrackId`]s here.

use serde::{Deserialize, Serialize};

/// Identifies which provider a resource came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderId(pub String);

impl ProviderId {
    /// The Apple Music provider id.
    pub const APPLE: &'static str = "apple";
    /// The local-files provider id.
    pub const LOCAL: &'static str = "local";

    /// Borrows the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProviderId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A track identifier, namespaced by provider (`apple:1440833098`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrackId(pub String);

impl TrackId {
    /// Builds an id from a provider and an opaque provider-local key.
    #[must_use]
    pub fn new(provider: &ProviderId, local: impl AsRef<str>) -> Self {
        Self(format!("{}:{}", provider.as_str(), local.as_ref()))
    }

    /// Borrows the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TrackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cover art at several resolutions, plus an optional animated variant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artwork {
    /// URL of a reasonably sized image, already templated for the current DPI.
    pub url: String,
    /// URL of a small thumbnail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// HLS playlist for animated artwork, when the item has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animated_url: Option<String>,
    /// Dominant colour as `#rrggbb`, used to tint the glass UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_color: Option<String>,
    /// Pixel width of [`Artwork::url`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Pixel height of [`Artwork::url`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// An artist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<Artwork>,
}

/// An album.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: Artist,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<Artwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_count: Option<u32>,
}

/// A playlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<Artwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_count: Option<u32>,
}

/// Synced lyrics.
///
/// `timed` carries word-level or line-level timing depending on what the
/// provider supplied; `plain` is always present as a fallback.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lyrics {
    /// Unsynced lyrics text.
    pub plain: String,
    /// Synced lines, when available.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub timed: Vec<LyricLine>,
    /// BCP-47 language tag of the lyrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

/// One timed lyric line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricLine {
    /// Start time in milliseconds.
    pub start_ms: u64,
    /// End time in milliseconds, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u64>,
    /// The line text.
    pub text: String,
    /// Optional translation, aligned line-for-line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

/// A playable track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub artist: Artist,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Album>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork: Option<Artwork>,
    /// Duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// International Standard Recording Code, used to match across providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    /// Track number within its disc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_number: Option<u32>,
    /// Disc number, for multi-disc releases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disc_number: Option<u32>,
}

/// A slice of a list result, with an optional cursor for the next page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Provider cursor for the next page, when more results exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// Total matching items, when the provider reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl<T> Page<T> {
    /// A page with no continuation.
    #[must_use]
    pub fn single(items: Vec<T>) -> Self {
        Self { items, next: None, total: None }
    }

    /// True when a further page can be fetched.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.next.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_ids_are_provider_namespaced() {
        let id = TrackId::new(&ProviderId::from(ProviderId::APPLE), "1440833098");
        assert_eq!(id.as_str(), "apple:1440833098");
    }

    #[test]
    fn page_single_has_no_continuation() {
        let page = Page::single(vec![1, 2, 3]);
        assert!(!page.has_more());
        assert_eq!(page.items.len(), 3);
    }
}
