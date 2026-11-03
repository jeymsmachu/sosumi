//! Flutter-facing Apple Music bridge.
//!
//! Keep this surface small and JSON-friendly. Network policy, token handling,
//! and MusicKit adapters remain outside this file.

use sosumi_core::apple::{
    AlbumAttributes, ArtistAttributes, Artwork, PlaylistAttributes, SearchType, SongAttributes,
};
use sosumi_core::apple::{AppleMusicClient, AppleMusicSession, DeveloperToken};

/// A track ready for Flutter rendering.
#[derive(Debug, Clone)]
pub struct TrackDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: u64,
    pub artwork_url: Option<String>,
    pub audio_variant: String,
    pub has_lyrics: bool,
}

/// An album ready for Flutter rendering.
#[derive(Debug, Clone)]
pub struct AlbumDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artwork_url: Option<String>,
    pub track_count: Option<u32>,
    pub release_date: Option<String>,
}

/// An artist ready for Flutter rendering.
#[derive(Debug, Clone)]
pub struct ArtistDto {
    pub id: String,
    pub name: String,
    pub artwork_url: Option<String>,
}

/// A playlist ready for Flutter rendering.
#[derive(Debug, Clone)]
pub struct PlaylistDto {
    pub id: String,
    pub title: String,
    pub curator: String,
    pub artwork_url: Option<String>,
    pub track_count: Option<u32>,
}

/// A catalog search result page.
#[derive(Debug, Clone, Default)]
pub struct SearchDto {
    pub songs: Vec<TrackDto>,
    pub albums: Vec<AlbumDto>,
    pub artists: Vec<ArtistDto>,
    pub playlists: Vec<PlaylistDto>,
}

/// Initializes Flutter-facing Rust helpers.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// The Rust core version.
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn core_version() -> String {
    sosumi_core::CORE_VERSION.to_owned()
}

/// Searches the Apple Music catalog.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn search_catalog(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    term: String,
    limit: u32,
) -> Result<SearchDto, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    let response = client
        .search(
            &term,
            &[
                SearchType::Songs,
                SearchType::Albums,
                SearchType::Artists,
                SearchType::Playlists,
            ],
            Some(limit),
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(SearchDto {
        songs: response
            .results
            .songs
            .map(|collection| {
                collection
                    .data
                    .into_iter()
                    .map(|resource| track(resource.id, &resource.attributes))
                    .collect()
            })
            .unwrap_or_default(),
        albums: response
            .results
            .albums
            .map(|collection| {
                collection
                    .data
                    .into_iter()
                    .map(|resource| album(resource.id, &resource.attributes))
                    .collect()
            })
            .unwrap_or_default(),
        artists: response
            .results
            .artists
            .map(|collection| {
                collection
                    .data
                    .into_iter()
                    .map(|resource| artist(resource.id, &resource.attributes))
                    .collect()
            })
            .unwrap_or_default(),
        playlists: response
            .results
            .playlists
            .map(|collection| {
                collection
                    .data
                    .into_iter()
                    .map(|resource| playlist(resource.id, &resource.attributes))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// Fetches one catalog song.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn catalog_song(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    song_id: String,
) -> Result<TrackDto, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    let song = client
        .song(&song_id)
        .await
        .map_err(|error| error.to_string())?;

    Ok(track(song.id, &song.attributes))
}

/// Fetches an album's tracks.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn album_tracks(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    album_id: String,
) -> Result<Vec<TrackDto>, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    let tracks = client
        .album_tracks(&album_id, Some(100))
        .await
        .map_err(|error| error.to_string())?;

    Ok(tracks
        .data
        .into_iter()
        .map(|resource| track(resource.id, &resource.attributes))
        .collect())
}

/// Fetches the user's library songs.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn library_songs(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    limit: u32,
) -> Result<Vec<TrackDto>, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    let songs = client
        .library_songs(Some(limit))
        .await
        .map_err(|error| error.to_string())?;

    Ok(songs
        .data
        .into_iter()
        .map(|resource| track(resource.id, &resource.attributes))
        .collect())
}

/// Fetches recently played tracks.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn recently_played(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    limit: u32,
) -> Result<Vec<TrackDto>, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    let songs = client
        .recently_played_tracks(Some(limit))
        .await
        .map_err(|error| error.to_string())?;

    Ok(songs
        .data
        .into_iter()
        .map(|resource| track(resource.id, &resource.attributes))
        .collect())
}

/// Fetches timed lyrics as TTML.
///
/// # Errors
///
/// Returns a display string when auth, transport, or Apple's API fails.
pub async fn lyrics(
    developer_token: String,
    music_user_token: String,
    storefront: String,
    song_id: String,
    language: String,
) -> Result<String, String> {
    let client = client(developer_token, music_user_token, storefront)?;
    client
        .lyrics(&song_id, &language)
        .await
        .map_err(|error| error.to_string())
}

fn client(
    developer_token: String,
    music_user_token: String,
    storefront: String,
) -> Result<AppleMusicClient, String> {
    let developer_token =
        DeveloperToken::parse(developer_token).map_err(|error| error.to_string())?;
    let session = AppleMusicSession::new(developer_token, music_user_token, storefront)
        .map_err(|error| error.to_string())?;
    session.client().map_err(|error| error.to_string())
}

fn track(id: String, attributes: &SongAttributes) -> TrackDto {
    TrackDto {
        id,
        title: attributes.name.clone(),
        artist: attributes.artist_name.clone(),
        album: attributes.album_name.clone(),
        duration_ms: attributes.duration_in_millis.unwrap_or(0),
        artwork_url: attributes
            .artwork
            .as_ref()
            .map(|artwork| artwork_url(artwork, 600)),
        audio_variant: attributes.best_audio_variant().label().to_owned(),
        has_lyrics: attributes.has_lyrics.unwrap_or(false),
    }
}

fn album(id: String, attributes: &AlbumAttributes) -> AlbumDto {
    AlbumDto {
        id,
        title: attributes.name.clone(),
        artist: attributes.artist_name.clone(),
        artwork_url: attributes
            .artwork
            .as_ref()
            .map(|artwork| artwork_url(artwork, 600)),
        track_count: attributes.track_count,
        release_date: attributes.release_date.clone(),
    }
}

fn artist(id: String, attributes: &ArtistAttributes) -> ArtistDto {
    ArtistDto {
        id,
        name: attributes.name.clone(),
        artwork_url: attributes
            .artwork
            .as_ref()
            .map(|artwork| artwork_url(artwork, 600)),
    }
}

fn playlist(id: String, attributes: &PlaylistAttributes) -> PlaylistDto {
    PlaylistDto {
        id,
        title: attributes.name.clone(),
        curator: attributes.curator_name.clone(),
        artwork_url: attributes
            .artwork
            .as_ref()
            .map(|artwork| artwork_url(artwork, 600)),
        track_count: attributes.track_count,
    }
}

fn artwork_url(artwork: &Artwork, size: u32) -> String {
    artwork.url_for(size, size)
}
