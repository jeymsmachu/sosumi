//! Apple Music API types.
//!
//! These are deliberately a small subset of the catalog response. Unknown
//! fields are ignored so Apple can add attributes without breaking playback.

use serde::{Deserialize, Serialize};

use crate::model::AudioVariant;

/// A JSON:API resource returned by Apple Music.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource<T> {
    /// Catalog or library identifier.
    pub id: String,
    /// Resource type, for example `songs`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Resource attributes.
    pub attributes: T,
}

/// A collection of resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceCollection<T> {
    /// The resources in this page.
    pub data: Vec<Resource<T>>,
    /// The next page URL, when present.
    #[serde(default)]
    pub next: Option<String>,
}

/// Artwork metadata returned by Apple Music.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artwork {
    /// URL template, usually containing `{w}` and `{h}`.
    pub url: String,
    /// Intrinsic width.
    #[serde(default)]
    pub width: Option<u32>,
    /// Intrinsic height.
    #[serde(default)]
    pub height: Option<u32>,
    /// Background color as an Apple hex string.
    #[serde(default)]
    pub bg_color: Option<String>,
    /// Primary text color.
    #[serde(default)]
    pub text_color1: Option<String>,
    /// Secondary text color.
    #[serde(default)]
    pub text_color2: Option<String>,
    /// Tertiary text color.
    #[serde(default)]
    pub text_color3: Option<String>,
}

impl Artwork {
    /// Expands the artwork template to the requested pixel size.
    #[must_use]
    pub fn url_for(&self, width: u32, height: u32) -> String {
        self.url
            .replace("{w}", &width.to_string())
            .replace("{h}", &height.to_string())
    }
}

/// Song attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongAttributes {
    /// Song title.
    pub name: String,
    /// Artist display name.
    #[serde(default)]
    pub artist_name: String,
    /// Album display name.
    #[serde(default)]
    pub album_name: Option<String>,
    /// Artwork.
    #[serde(default)]
    pub artwork: Option<Artwork>,
    /// Duration in milliseconds.
    #[serde(default)]
    pub duration_in_millis: Option<u64>,
    /// Track number on the album.
    #[serde(default)]
    pub track_number: Option<u32>,
    /// Disc number.
    #[serde(default)]
    pub disc_number: Option<u32>,
    /// International Standard Recording Code.
    #[serde(default)]
    pub isrc: Option<String>,
    /// Whether Apple has lyrics for this song.
    #[serde(default)]
    pub has_lyrics: Option<bool>,
    /// Apple Music web URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Audio traits such as `lossless`, `hi-res-lossless`, `atmos`.
    #[serde(default)]
    pub audio_traits: Option<Vec<String>>,
    /// Preview assets.
    #[serde(default)]
    pub previews: Option<Vec<Preview>>,
}

impl SongAttributes {
    /// Best audio variant advertised by the catalog metadata.
    #[must_use]
    pub fn best_audio_variant(&self) -> AudioVariant {
        let traits = self.audio_traits.as_deref().unwrap_or_default();
        if traits.iter().any(|t| t.contains("hi-res-lossless")) {
            AudioVariant::HiResLossless
        } else if traits.iter().any(|t| t.contains("atmos")) {
            AudioVariant::DolbyAtmos
        } else if traits.iter().any(|t| t.contains("lossless")) {
            AudioVariant::Lossless
        } else if traits.iter().any(|t| t.contains("surround")) {
            AudioVariant::DolbyAudio
        } else {
            AudioVariant::Aac
        }
    }
}

/// An audio preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// Preview URL.
    pub url: String,
    /// Preview duration in milliseconds.
    #[serde(default)]
    pub duration_in_millis: Option<u64>,
}

/// Album attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumAttributes {
    /// Album title.
    pub name: String,
    /// Artist display name.
    #[serde(default)]
    pub artist_name: String,
    /// Artwork.
    #[serde(default)]
    pub artwork: Option<Artwork>,
    /// Number of tracks.
    #[serde(default)]
    pub track_count: Option<u32>,
    /// Release date (YYYY-MM-DD).
    #[serde(default)]
    pub release_date: Option<String>,
    /// Copyright line.
    #[serde(default)]
    pub copyright: Option<String>,
    /// Genre names.
    #[serde(default)]
    pub genre_names: Vec<String>,
    /// Apple Music web URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Whether this is a single.
    #[serde(default)]
    pub is_single: Option<bool>,
    /// Whether this is a compilation.
    #[serde(default)]
    pub is_compilation: Option<bool>,
}

/// Artist attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistAttributes {
    /// Artist name.
    pub name: String,
    /// Genre names.
    #[serde(default)]
    pub genre_names: Vec<String>,
    /// Artwork.
    #[serde(default)]
    pub artwork: Option<Artwork>,
    /// Apple Music web URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Editorial notes.
    #[serde(default)]
    pub editorial_notes: Option<EditorialNotes>,
}

/// Editorial notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorialNotes {
    /// Short description.
    #[serde(default)]
    pub short: Option<String>,
    /// Standard description.
    #[serde(default)]
    pub standard: Option<String>,
    /// Name of the notes author.
    #[serde(default)]
    pub name: Option<String>,
}

/// Playlist attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistAttributes {
    /// Playlist title.
    pub name: String,
    /// Curator name.
    #[serde(default)]
    pub curator_name: String,
    /// Description, when present.
    #[serde(default)]
    pub description: Option<EditorialNotes>,
    /// Artwork.
    #[serde(default)]
    pub artwork: Option<Artwork>,
    /// Number of tracks.
    #[serde(default)]
    pub track_count: Option<u32>,
    /// Apple Music web URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// A page of search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results grouped by resource type.
    pub results: SearchResults,
}

/// Search results grouped by resource type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    /// Song results.
    #[serde(default)]
    pub songs: Option<ResourceCollection<SongAttributes>>,
    /// Album results.
    #[serde(default)]
    pub albums: Option<ResourceCollection<AlbumAttributes>>,
    /// Artist results.
    #[serde(default)]
    pub artists: Option<ResourceCollection<ArtistAttributes>>,
    /// Playlist results.
    #[serde(default)]
    pub playlists: Option<ResourceCollection<PlaylistAttributes>>,
}

/// Resource types accepted by the search endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    /// Songs.
    Songs,
    /// Albums.
    Albums,
    /// Artists.
    Artists,
    /// Playlists.
    Playlists,
}

impl SearchType {
    /// The value Apple Music expects in the `types` query parameter.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Songs => "songs",
            Self::Albums => "albums",
            Self::Artists => "artists",
            Self::Playlists => "playlists",
        }
    }
}

/// Storefront returned by `/v1/me/storefront`.
pub type StorefrontResponse = ResourceCollection<StorefrontAttributes>;

/// Storefront attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorefrontAttributes {
    /// Storefront identifier, for example `us`.
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Default language tag.
    #[serde(default)]
    pub default_language_tag: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Artwork, SongAttributes};
    use crate::model::AudioVariant;

    #[test]
    fn artwork_template_expands() {
        let artwork = Artwork {
            url: "https://example.com/{w}x{h}bb.jpg".to_owned(),
            width: Some(3000),
            height: Some(3000),
            bg_color: None,
            text_color1: None,
            text_color2: None,
            text_color3: None,
        };

        assert_eq!(
            artwork.url_for(600, 600),
            "https://example.com/600x600bb.jpg"
        );
    }

    #[test]
    fn best_variant_prefers_hi_res_lossless() {
        let song = SongAttributes {
            name: "Track".to_owned(),
            artist_name: "Artist".to_owned(),
            album_name: None,
            artwork: None,
            duration_in_millis: None,
            track_number: None,
            disc_number: None,
            isrc: None,
            has_lyrics: None,
            url: None,
            audio_traits: Some(vec![
                "lossless".to_owned(),
                "hi-res-lossless".to_owned(),
                "atmos".to_owned(),
            ]),
            previews: None,
        };

        assert_eq!(song.best_audio_variant(), AudioVariant::HiResLossless);
    }
}
