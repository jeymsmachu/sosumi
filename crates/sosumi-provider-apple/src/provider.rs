//! The [`MediaProvider`] implementation for Apple Music.
//!
//! This module is wiring, not logic: the parsing lives in [`super::hls`] and
//! [`super::catalog`], and the stream acquisition in [`super::webplayback`].
//! What is here is the policy — which storefront, which token, and how a
//! capability gap becomes an actionable error.

use async_trait::async_trait;
use parking_lot::RwLock;

use sosumi_provider_api::{
    Album, Artist, AudioQuality, AudioSource, Codec, Lyrics, MediaProvider, Page, PlaybackSession,
    Playlist, ProviderError, ProviderId, Result, SearchQuery, SearchResults, StreamInfo,
    StreamRequest, Track, TrackId,
};

use crate::catalog::{self, Document};
use crate::client::HttpClient;
use crate::hls;
use crate::webplayback::{self, SubDownloadResponse};

/// Credentials the provider needs. The music-user token identifies the
/// subscriber; the developer token identifies the app.
#[derive(Debug, Clone, Default)]
pub struct AppleCredentials {
    /// The subscriber's Music-User-Token.
    pub music_user_token: Option<String>,
    /// The app's developer token, refreshed from the web player.
    pub developer_token: Option<String>,
    /// The storefront, for example `us`.
    pub storefront: String,
}

/// The Apple Music provider.
pub struct AppleMusicProvider {
    client: HttpClient,
    credentials: RwLock<AppleCredentials>,
}

impl std::fmt::Debug for AppleMusicProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let credentials = self.credentials.read();
        f.debug_struct("AppleMusicProvider")
            .field("storefront", &credentials.storefront)
            .field("signed_in", &credentials.music_user_token.is_some())
            .finish_non_exhaustive()
    }
}

impl AppleMusicProvider {
    /// Builds a provider. It is usable before sign-in for catalog reads that do
    /// not need a subscriber token, but playback requires one.
    ///
    /// # Errors
    ///
    /// Fails when the HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: HttpClient::new()?,
            credentials: RwLock::new(AppleCredentials {
                storefront: catalog::DEFAULT_STOREFRONT.to_owned(),
                ..AppleCredentials::default()
            }),
        })
    }

    /// Replaces the credentials, for example after a sign-in.
    pub fn set_credentials(&self, credentials: AppleCredentials) {
        *self.credentials.write() = credentials;
    }

    /// A snapshot of the current credentials.
    #[must_use]
    pub fn credentials(&self) -> AppleCredentials {
        self.credentials.read().clone()
    }

    /// The storefront in use.
    fn storefront(&self) -> String {
        let storefront = self.credentials.read().storefront.clone();
        if storefront.is_empty() {
            catalog::DEFAULT_STOREFRONT.to_owned()
        } else {
            storefront
        }
    }

    /// The developer token, discovering one from the web player if needed.
    async fn developer_token(&self) -> Result<String> {
        if let Some(token) = self.credentials.read().developer_token.clone() {
            return Ok(token);
        }
        let token = self.discover_developer_token().await?;
        self.credentials.write().developer_token = Some(token.clone());
        Ok(token)
    }

    /// Scrapes the token from the web player bundle.
    async fn discover_developer_token(&self) -> Result<String> {
        let page = self.client.get_text(crate::token::PLAYER_PAGE, None).await?;
        let path = crate::token::find_script_path(&page).ok_or_else(|| {
            ProviderError::Malformed("web player script not referenced by the page".into())
        })?;
        let script = self.client.get_text(&crate::token::script_url(path), None).await?;
        crate::token::find_token(&script).map(str::to_owned).ok_or_else(|| {
            ProviderError::Malformed("no developer token in web player script".into())
        })
    }

    /// The subscriber token, or an actionable error.
    fn music_user_token(&self) -> Result<String> {
        self.credentials.read().music_user_token.clone().ok_or(ProviderError::NotAuthenticated)
    }

    /// Maps a quality the caller asked for onto the codec actually selected.
    #[allow(dead_code)]
    fn stream_info(quality: AudioQuality, variant: &hls::Variant) -> StreamInfo {
        let _ = quality;
        let codec = if variant.is_lossless() {
            Codec::Alac
        } else if variant.spatial {
            Codec::Ec3Joc
        } else {
            Codec::Aac
        };

        let bit_depth = codec.is_lossless().then_some(16);
        StreamInfo {
            codec,
            sample_rate: variant.sample_rate.unwrap_or(44_100),
            bit_depth,
            bitrate_kbps: u32::try_from(variant.bandwidth / 1000).ok(),
            spatial: (codec == Codec::Ec3Joc).then_some(sosumi_provider_api::SpatialFormat {
                bed_channels: u8::try_from(variant.channels.unwrap_or(8)).unwrap_or(8),
                has_objects: true,
            }),
        }
    }
}

impl Default for AppleMusicProvider {
    fn default() -> Self {
        // `HttpClient::new` only fails if the TLS backend cannot initialise,
        // which is unrecoverable; surface it as a panic-free fallback.
        Self::new().unwrap_or_else(|_| Self {
            client: HttpClient::from_client(reqwest::Client::new()),
            credentials: RwLock::new(AppleCredentials {
                storefront: catalog::DEFAULT_STOREFRONT.to_owned(),
                ..AppleCredentials::default()
            }),
        })
    }
}

/// The provider's stable id, allocated once.
static APPLE_PROVIDER_ID: std::sync::LazyLock<ProviderId> =
    std::sync::LazyLock::new(|| ProviderId(ProviderId::APPLE.to_owned()));

#[async_trait]
impl MediaProvider for AppleMusicProvider {
    fn id(&self) -> &ProviderId {
        &APPLE_PROVIDER_ID
    }

    /// Copies the account credentials into the provider.
    ///
    /// This is what makes sign-in actually take effect: the core stores the
    /// session, then hands it here, and every subsequent request carries the
    /// subscriber's token. Without this the UI would report success and playback
    /// would still fail.
    fn set_account(&self, account: sosumi_provider_api::Account) {
        let mut credentials = self.credentials.write();
        credentials.music_user_token = account.user_token;
        if account.developer_token.is_some() {
            credentials.developer_token = account.developer_token;
        }
        if !account.storefront.is_empty() {
            credentials.storefront = account.storefront;
        }
        tracing::debug!(
            target: "sosumi::apple",
            signed_in = credentials.music_user_token.is_some(),
            storefront = %credentials.storefront,
            "account updated"
        );
    }

    fn name(&self) -> &'static str {
        "Apple Music"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search(&self, query: SearchQuery) -> Result<SearchResults> {
        let storefront = query.storefront.unwrap_or_else(|| self.storefront());
        let token = self.developer_token().await?;
        let url = catalog::search_url(&storefront, &query.text, "songs,albums,artists,playlists");

        let response: catalog::SearchResponse = self.client.get_json(&url, Some(&token)).await?;
        let sections = response.results;

        Ok(SearchResults {
            tracks: sections
                .songs
                .map(|doc| catalog::to_page(doc, catalog::to_track).items)
                .unwrap_or_default(),
            albums: sections
                .albums
                .map(|doc| catalog::to_page(doc, catalog::to_album).items)
                .unwrap_or_default(),
            artists: sections
                .artists
                .map(|doc| catalog::to_page(doc, catalog::to_artist).items)
                .unwrap_or_default(),
            playlists: sections
                .playlists
                .map(|doc| catalog::to_page(doc, catalog::to_playlist).items)
                .unwrap_or_default(),
        })
    }

    async fn track(&self, id: &TrackId) -> Result<Track> {
        let storefront = self.storefront();
        let token = self.developer_token().await?;
        let url = catalog::resource_url(&storefront, "songs", catalog::bare_id(id));
        let document: Document<catalog::TrackAttributes> =
            self.client.get_json(&url, Some(&token)).await?;
        let resource =
            document.data.first().ok_or_else(|| ProviderError::NotFound(id.as_str().to_owned()))?;
        Ok(catalog::to_track(resource))
    }

    async fn album(&self, id: &str, storefront: Option<&str>) -> Result<(Album, Vec<Track>)> {
        let storefront = storefront.map_or_else(|| self.storefront(), str::to_owned);
        let token = self.developer_token().await?;

        let url = catalog::resource_url(&storefront, "albums", id);
        let document: Document<catalog::AlbumAttributes> =
            self.client.get_json(&url, Some(&token)).await?;
        let album = document
            .data
            .first()
            .map(catalog::to_album)
            .ok_or_else(|| ProviderError::NotFound(format!("album {id}")))?;

        let tracks_url = format!("{url}/tracks");
        let tracks: Document<catalog::TrackAttributes> =
            self.client.get_json(&tracks_url, Some(&token)).await?;

        Ok((album, catalog::to_page(tracks, catalog::to_track).items))
    }

    async fn playlist(
        &self,
        id: &str,
        storefront: Option<&str>,
    ) -> Result<(Playlist, Page<Track>)> {
        let storefront = storefront.map_or_else(|| self.storefront(), str::to_owned);
        let token = self.developer_token().await?;

        let url = catalog::resource_url(&storefront, "playlists", id);
        let document: Document<catalog::PlaylistAttributes> =
            self.client.get_json(&url, Some(&token)).await?;
        let playlist = document
            .data
            .first()
            .map(catalog::to_playlist)
            .ok_or_else(|| ProviderError::NotFound(format!("playlist {id}")))?;

        let tracks_url = format!("{url}/tracks?limit=100");
        let tracks: Document<catalog::TrackAttributes> =
            self.client.get_json(&tracks_url, Some(&token)).await?;

        Ok((playlist, catalog::to_page(tracks, catalog::to_track)))
    }

    async fn artist(&self, id: &str, storefront: Option<&str>) -> Result<(Artist, Vec<Track>)> {
        let storefront = storefront.map_or_else(|| self.storefront(), str::to_owned);
        let token = self.developer_token().await?;

        let url = catalog::resource_url(&storefront, "artists", id);
        let document: Document<catalog::ArtistAttributes> =
            self.client.get_json(&url, Some(&token)).await?;
        let artist = document
            .data
            .first()
            .map(catalog::to_artist)
            .ok_or_else(|| ProviderError::NotFound(format!("artist {id}")))?;

        let songs_url = format!("{url}/view/top-songs");
        let songs: Document<catalog::TrackAttributes> =
            self.client.get_json(&songs_url, Some(&token)).await?;

        Ok((artist, catalog::to_page(songs, catalog::to_track).items))
    }

    async fn lyrics(&self, id: &TrackId, storefront: Option<&str>) -> Result<Lyrics> {
        let storefront = storefront.map_or_else(|| self.storefront(), str::to_owned);
        let token = self.developer_token().await?;
        let url = format!(
            "https://amp-api.music.apple.com/v1/catalog/{storefront}/songs/{}/lyrics",
            catalog::bare_id(id)
        );

        let response: catalog::LyricsResponse =
            self.client.get_json(&url, Some(&token)).await.map_err(|err| match err {
                ProviderError::NotFound(_) => {
                    ProviderError::NotFound(format!("lyrics for {}", id.as_str()))
                }
                other => other,
            })?;

        Ok(Lyrics { plain: response.lyrics.unwrap_or_default(), timed: Vec::new(), language: None })
    }

    async fn open(&self, req: StreamRequest) -> Result<PlaybackSession> {
        let music_user_token = self.music_user_token()?;
        let developer_token = self.developer_token().await?;
        let storefront = req.storefront.clone().unwrap_or_else(|| self.storefront());
        let adam_id = catalog::bare_id(&req.track);

        let qualities = webplayback::qualities_for(req.quality);
        let url = webplayback::sub_download_url(&storefront, adam_id, &qualities);
        let response: SubDownloadResponse =
            self.client.get_json(&url, Some(&developer_token)).await?;

        let asset = response.best_for(req.quality).ok_or_else(|| {
            ProviderError::NotEntitled(format!(
                "no playable asset for quality {}",
                req.quality.label()
            ))
        })?;
        let playlist_url = asset
            .url
            .clone()
            .ok_or_else(|| ProviderError::Malformed("asset has no playlist URL".into()))?;

        let media = self.client.get_text(&playlist_url, Some(&developer_token)).await?;
        let media = hls::parse_media(&media, &playlist_url)
            .map_err(|e| ProviderError::Malformed(e.to_string()))?;

        let info = stream_info_from_media(req.quality, &media);
        let headers = vec![
            ("Authorization".to_owned(), format!("Bearer {developer_token}")),
            ("Music-User-Token".to_owned(), music_user_token),
        ];

        Ok(PlaybackSession {
            track: req.track,
            info,
            source: AudioSource::Http { url: media.segments[0].clone(), headers },
            duration_ms: media.duration_ms,
        })
    }
}

/// The provider's stable id.
#[allow(dead_code)]
const APPLE_ID: &str = ProviderId::APPLE;

/// Derives stream info from a parsed media playlist.
fn stream_info_from_media(quality: AudioQuality, media: &hls::MediaPlaylist) -> StreamInfo {
    let codec = match &media.key {
        Some((hls::KeyMethod::Drm { system: hls::DrmSystem::FairPlay, .. }, _)) => match quality {
            AudioQuality::Lossless | AudioQuality::HiResLossless => Codec::Alac,
            AudioQuality::Spatial => Codec::Ec3Joc,
            _ => Codec::Aac,
        },
        _ => Codec::Aac,
    };

    StreamInfo {
        codec,
        sample_rate: 44_100,
        bit_depth: codec.is_lossless().then_some(16),
        bitrate_kbps: Some(256),
        spatial: (codec == Codec::Ec3Joc)
            .then_some(sosumi_provider_api::SpatialFormat { bed_channels: 8, has_objects: true }),
    }
}
