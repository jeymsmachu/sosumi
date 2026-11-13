//! The Apple Music catalog API (`amp-api.music.apple.com`).
//!
//! This module owns the wire types and the mapping into
//! [`sosumi_provider_api`]'s model. The mapping is a pure function, so it is
//! tested against captured JSON without a network.

use serde::{Deserialize, Serialize};

use sosumi_provider_api::{Album, Artist, Artwork, Page, Playlist, ProviderId, Track, TrackId};

/// The catalog API base URL.
pub const API_BASE: &str = "https://amp-api.music.apple.com/v1/catalog";

/// The storefront used when the caller does not specify one.
pub const DEFAULT_STOREFRONT: &str = "us";

// ── Wire types ──────────────────────────────────────────────────────────────
//
// Apple's API is a JSON:API dialect. Relationships and attributes are nested
// under `data[].attributes` and `data[].relationships`. We model only what we
// consume, and keep these types private to this module.

/// A JSON:API document.
#[derive(Debug, Deserialize)]
pub(crate) struct Document<T> {
    pub(crate) data: Vec<Resource<T>>,
    #[serde(default)]
    pub(crate) next: Option<String>,
}

/// A JSON:API resource.
#[derive(Debug, Deserialize)]
pub(crate) struct Resource<T> {
    pub(crate) id: String,
    pub(crate) attributes: T,
}

/// A track's attributes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct TrackAttributes {
    pub(crate) name: String,
    pub(crate) artist_name: Option<String>,
    pub(crate) album_name: Option<String>,
    pub(crate) artwork: Option<ArtworkAttributes>,
    pub(crate) duration_in_millis: Option<u64>,
    pub(crate) isrc: Option<String>,
    pub(crate) track_number: Option<u32>,
    pub(crate) disc_number: Option<u32>,
}

/// An album's attributes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct AlbumAttributes {
    pub(crate) name: String,
    pub(crate) artist_name: Option<String>,
    pub(crate) artwork: Option<ArtworkAttributes>,
    pub(crate) release_date: Option<String>,
    pub(crate) track_count: Option<u32>,
}

/// A playlist's attributes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PlaylistAttributes {
    pub(crate) name: String,
    pub(crate) description: Option<DescriptionAttributes>,
    pub(crate) artwork: Option<ArtworkAttributes>,
    pub(crate) track_count: Option<u32>,
}

/// A playlist description, which Apple nests as an object.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct DescriptionAttributes {
    pub(crate) standard: Option<String>,
}

/// An artist's attributes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ArtistAttributes {
    pub(crate) name: String,
    pub(crate) artwork: Option<ArtworkAttributes>,
}

/// Artwork attributes, including the URL template.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct ArtworkAttributes {
    pub(crate) url: String,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    /// A solid colour Apple provides for tinting.
    pub(crate) bg_color: Option<String>,
}

// ── Mapping ─────────────────────────────────────────────────────────────────

/// Maps a track resource into the domain model.
pub(crate) fn to_track(resource: &Resource<TrackAttributes>) -> Track {
    let attrs = &resource.attributes;
    let artist = Artist {
        id: attrs.artist_name.clone().unwrap_or_default(),
        name: attrs.artist_name.clone().unwrap_or_else(|| "Unknown Artist".into()),
        artwork: None,
    };
    let album = attrs.album_name.as_ref().map(|name| Album {
        id: String::new(),
        title: name.clone(),
        artist: artist.clone(),
        artwork: attrs.artwork.as_ref().map(to_artwork),
        release_date: None,
        track_count: None,
    });

    Track {
        id: TrackId::new(&ProviderId::from(ProviderId::APPLE), &resource.id),
        title: attrs.name.clone(),
        artist,
        album,
        artwork: attrs.artwork.as_ref().map(to_artwork),
        duration_ms: attrs.duration_in_millis,
        isrc: attrs.isrc.clone(),
        track_number: attrs.track_number,
        disc_number: attrs.disc_number,
    }
}

/// Maps an album resource into the domain model.
pub(crate) fn to_album(resource: &Resource<AlbumAttributes>) -> Album {
    let attrs = &resource.attributes;
    Album {
        id: resource.id.clone(),
        title: attrs.name.clone(),
        artist: Artist {
            id: String::new(),
            name: attrs.artist_name.clone().unwrap_or_else(|| "Unknown Artist".into()),
            artwork: None,
        },
        artwork: attrs.artwork.as_ref().map(to_artwork),
        release_date: attrs.release_date.clone(),
        track_count: attrs.track_count,
    }
}

/// Maps a playlist resource into the domain model.
pub(crate) fn to_playlist(resource: &Resource<PlaylistAttributes>) -> Playlist {
    let attrs = &resource.attributes;
    Playlist {
        id: resource.id.clone(),
        title: attrs.name.clone(),
        description: attrs.description.as_ref().and_then(|d| d.standard.clone()),
        artwork: attrs.artwork.as_ref().map(to_artwork),
        track_count: attrs.track_count,
    }
}

/// Maps an artist resource into the domain model.
pub(crate) fn to_artist(resource: &Resource<ArtistAttributes>) -> Artist {
    let attrs = &resource.attributes;
    Artist {
        id: resource.id.clone(),
        name: attrs.name.clone(),
        artwork: attrs.artwork.as_ref().map(to_artwork),
    }
}

/// Maps artwork attributes, preserving the template for later resizing.
pub(crate) fn to_artwork(attrs: &ArtworkAttributes) -> Artwork {
    let template = sosumi_metadata::ArtworkTemplate::new(&attrs.url);
    Artwork {
        url: template.at_size(632),
        thumbnail_url: Some(template.at_size(64)),
        animated_url: None,
        dominant_color: attrs.bg_color.as_deref().and_then(sosumi_metadata::normalize_color),
        width: attrs.width,
        height: attrs.height,
    }
}

/// Turns a JSON:API document into a page of domain items.
pub(crate) fn to_page<T, U>(document: Document<T>, map: impl Fn(&Resource<T>) -> U) -> Page<U> {
    Page { items: document.data.iter().map(map).collect(), next: document.next, total: None }
}

/// Builds the search URL for a query.
#[must_use]
pub fn search_url(storefront: &str, query: &str, types: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    format!("{API_BASE}/{storefront}/search?term={encoded}&types={types}&limit=25")
}

/// Builds a catalog resource URL.
#[must_use]
pub fn resource_url(storefront: &str, kind: &str, id: &str) -> String {
    format!("{API_BASE}/{storefront}/{kind}/{id}")
}

/// The storefront, or the default when omitted.
#[must_use]
pub fn storefront_or_default(storefront: Option<&str>) -> &str {
    storefront.unwrap_or(DEFAULT_STOREFRONT)
}

/// Strips the `apple:` prefix from a track id to recover Apple's numeric id.
#[must_use]
pub fn bare_id(track: &TrackId) -> &str {
    track.as_str().split_once(':').map_or(track.as_str(), |(_, rest)| rest)
}

/// A search response, with one section per result type.
#[derive(Debug, Deserialize)]
pub(crate) struct SearchResponse {
    pub(crate) results: SearchSections,
}

/// The per-type sections of a search response.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct SearchSections {
    pub(crate) songs: Option<Document<TrackAttributes>>,
    pub(crate) albums: Option<Document<AlbumAttributes>>,
    pub(crate) artists: Option<Document<ArtistAttributes>>,
    pub(crate) playlists: Option<Document<PlaylistAttributes>>,
}

/// A lyrics response from Apple's private endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LyricsResponse {
    #[serde(default)]
    pub(crate) lyrics: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK_JSON: &str = r##"{
      "id": "1440833098",
      "attributes": {
        "name": "Weird Fishes / Arpeggi",
        "artistName": "Radiohead",
        "albumName": "In Rainbows",
        "durationInMillis": 318186,
        "isrc": "GBSTK0700041",
        "trackNumber": 3,
        "discNumber": 1,
        "artwork": {
          "url": "https://is1-ssl.mzstatic.com/image/thumb/Features/v4/{w}x{h}bb.jpg",
          "width": 3000,
          "height": 3000,
          "bgColor": "#0A0A0A"
        }
      }
    }"##;

    #[test]
    fn track_maps_into_the_domain_model() {
        let resource: Resource<TrackAttributes> = serde_json::from_str(TRACK_JSON).expect("parse");
        let track = to_track(&resource);

        assert_eq!(track.id.as_str(), "apple:1440833098");
        assert_eq!(track.title, "Weird Fishes / Arpeggi");
        assert_eq!(track.artist.name, "Radiohead");
        assert_eq!(track.album.as_ref().expect("album").title, "In Rainbows");
        assert_eq!(track.duration_ms, Some(318_186));
        assert_eq!(track.isrc.as_deref(), Some("GBSTK0700041"));
        assert_eq!(track.track_number, Some(3));
    }

    #[test]
    fn artwork_is_expanded_to_a_concrete_size() {
        let resource: Resource<TrackAttributes> = serde_json::from_str(TRACK_JSON).expect("parse");
        let track = to_track(&resource);
        let artwork = track.artwork.expect("artwork");
        assert!(artwork.url.contains("632x632"));
        assert!(!artwork.url.contains("{w}"));
        assert_eq!(artwork.dominant_color.as_deref(), Some("#0a0a0a"));
    }

    #[test]
    fn missing_artist_is_named_not_blank() {
        let json = r#"{"id":"1","attributes":{"name":"Untitled"}}"#;
        let resource: Resource<TrackAttributes> = serde_json::from_str(json).expect("parse");
        let track = to_track(&resource);
        assert_eq!(track.artist.name, "Unknown Artist");
    }

    #[test]
    fn search_url_encodes_the_query() {
        let url = search_url("us", "radiohead & friends", "songs,albums");
        assert!(
            url.contains("term=radiohead+%26+friends")
                || url.contains("term=radiohead%20%26%20friends")
        );
        assert!(url.contains("types=songs,albums"));
    }

    #[test]
    fn resource_url_includes_storefront_and_kind() {
        assert_eq!(
            resource_url("gb", "albums", "123"),
            "https://amp-api.music.apple.com/v1/catalog/gb/albums/123"
        );
    }

    #[test]
    fn storefront_defaults_when_absent() {
        assert_eq!(storefront_or_default(None), "us");
        assert_eq!(storefront_or_default(Some("jp")), "jp");
    }

    #[test]
    fn bare_id_strips_the_provider_prefix() {
        let id = TrackId::new(&ProviderId::from(ProviderId::APPLE), "1440833098");
        assert_eq!(bare_id(&id), "1440833098");
    }

    #[test]
    fn a_document_maps_to_a_page_with_a_cursor() {
        let json = r#"{
          "data": [{"id":"1","attributes":{"name":"A"}},{"id":"2","attributes":{"name":"B"}}],
          "next": "/v1/catalog/us/songs?offset=2"
        }"#;
        let document: Document<TrackAttributes> = serde_json::from_str(json).expect("parse");
        let page = to_page(document, to_track);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next.as_deref(), Some("/v1/catalog/us/songs?offset=2"));
        assert!(page.has_more());
    }

    #[test]
    fn playlist_description_is_unwrapped() {
        let json = r#"{"id":"p1","attributes":{"name":"Mix","description":{"standard":"For focus"},"trackCount":12}}"#;
        let resource: Resource<PlaylistAttributes> = serde_json::from_str(json).expect("parse");
        let playlist = to_playlist(&resource);
        assert_eq!(playlist.description.as_deref(), Some("For focus"));
        assert_eq!(playlist.track_count, Some(12));
    }

    #[test]
    fn search_response_tolerates_missing_sections() {
        let json = r#"{"results":{"songs":{"data":[]}}}"#;
        let response: SearchResponse = serde_json::from_str(json).expect("parse");
        assert!(response.results.songs.is_some());
        assert!(response.results.albums.is_none());
    }
}
