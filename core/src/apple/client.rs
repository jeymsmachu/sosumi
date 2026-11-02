//! Apple Music REST client.

use reqwest::{Client, Method, RequestBuilder, StatusCode, Url, header};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::apple::error::{ApiErrorBody, AppleMusicError};
use crate::apple::models::{
    AlbumAttributes, ArtistAttributes, PlaylistAttributes, Resource, ResourceCollection,
    SearchResponse, SearchType, SongAttributes, StorefrontAttributes,
};

/// Apple Music REST client for catalog, library, and private lyric endpoints.
#[derive(Debug, Clone)]
pub struct AppleMusicClient {
    http: Client,
    api_base: Url,
    amp_base: Url,
    developer_token: String,
    music_user_token: Option<String>,
    storefront: String,
}

impl AppleMusicClient {
    /// Creates a client against the production Apple Music API.
    ///
    /// # Errors
    ///
    /// Returns an error when the storefront or HTTP client is invalid.
    pub fn new(
        developer_token: impl Into<String>,
        storefront: impl Into<String>,
    ) -> Result<Self, AppleMusicError> {
        let api_base = Url::parse("https://api.music.apple.com")
            .map_err(|error| AppleMusicError::InvalidUrl(error.to_string()))?;
        let amp_base = Url::parse("https://amp-api.music.apple.com")
            .map_err(|error| AppleMusicError::InvalidUrl(error.to_string()))?;
        Self::with_endpoints(developer_token, storefront, api_base, amp_base)
    }

    /// Creates a client against custom endpoints. Used by tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the storefront or HTTP client is invalid.
    pub fn with_endpoints(
        developer_token: impl Into<String>,
        storefront: impl Into<String>,
        api_base: Url,
        amp_base: Url,
    ) -> Result<Self, AppleMusicError> {
        let storefront = normalize_storefront(storefront.into())?;
        let http = Client::builder()
            .user_agent(concat!("sosumi/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(AppleMusicError::from)?;

        Ok(Self {
            http,
            api_base,
            amp_base,
            developer_token: developer_token.into(),
            music_user_token: None,
            storefront,
        })
    }

    /// Sets the Music User Token used for personal endpoints.
    #[must_use]
    pub fn with_music_user_token(mut self, music_user_token: impl Into<String>) -> Self {
        self.music_user_token = Some(music_user_token.into());
        self
    }

    /// Replaces the Music User Token.
    pub fn set_music_user_token(&mut self, music_user_token: Option<String>) {
        self.music_user_token = music_user_token;
    }

    /// The normalized storefront.
    #[must_use]
    pub fn storefront(&self) -> &str {
        &self.storefront
    }

    /// Searches the catalog.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError`] for transport and Apple API failures.
    pub async fn search(
        &self,
        term: &str,
        types: &[SearchType],
        limit: Option<u32>,
    ) -> Result<SearchResponse, AppleMusicError> {
        let types = types
            .iter()
            .map(|search_type| search_type.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let mut request = self.request(
            Method::GET,
            &format!("/v1/catalog/{}/search", self.storefront),
        )?;
        request = request.query(&[("term", term), ("types", types.as_str())]);
        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)]);
        }

        self.send_json(request).await
    }

    /// Fetches one catalog song.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::NotFound`] when Apple omits the song.
    pub async fn song(&self, id: &str) -> Result<Resource<SongAttributes>, AppleMusicError> {
        let response: ResourceCollection<SongAttributes> = self
            .send_json(self.request(
                Method::GET,
                &format!("/v1/catalog/{}/songs/{id}", self.storefront),
            )?)
            .await?;

        single_resource(response)
    }

    /// Fetches one catalog album.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::NotFound`] when Apple omits the album.
    pub async fn album(&self, id: &str) -> Result<Resource<AlbumAttributes>, AppleMusicError> {
        let response: ResourceCollection<AlbumAttributes> = self
            .send_json(self.request(
                Method::GET,
                &format!("/v1/catalog/{}/albums/{id}", self.storefront),
            )?)
            .await?;

        single_resource(response)
    }

    /// Fetches an album's tracks.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError`] for transport and Apple API failures.
    pub async fn album_tracks(
        &self,
        id: &str,
        limit: Option<u32>,
    ) -> Result<ResourceCollection<SongAttributes>, AppleMusicError> {
        let mut request = self.request(
            Method::GET,
            &format!("/v1/catalog/{}/albums/{id}/tracks", self.storefront),
        )?;
        request = add_limit(request, limit);

        self.send_json(request).await
    }

    /// Fetches one catalog artist.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::NotFound`] when Apple omits the artist.
    pub async fn artist(&self, id: &str) -> Result<Resource<ArtistAttributes>, AppleMusicError> {
        let response: ResourceCollection<ArtistAttributes> = self
            .send_json(self.request(
                Method::GET,
                &format!("/v1/catalog/{}/artists/{id}", self.storefront),
            )?)
            .await?;

        single_resource(response)
    }

    /// Fetches an artist's albums.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError`] for transport and Apple API failures.
    pub async fn artist_albums(
        &self,
        id: &str,
        limit: Option<u32>,
    ) -> Result<ResourceCollection<AlbumAttributes>, AppleMusicError> {
        let mut request = self.request(
            Method::GET,
            &format!("/v1/catalog/{}/artists/{id}/albums", self.storefront),
        )?;
        request = add_limit(request, limit);

        self.send_json(request).await
    }

    /// Fetches one catalog playlist.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::NotFound`] when Apple omits the playlist.
    pub async fn playlist(
        &self,
        id: &str,
    ) -> Result<Resource<PlaylistAttributes>, AppleMusicError> {
        let response: ResourceCollection<PlaylistAttributes> = self
            .send_json(self.request(
                Method::GET,
                &format!("/v1/catalog/{}/playlists/{id}", self.storefront),
            )?)
            .await?;

        single_resource(response)
    }

    /// Fetches a playlist's tracks.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError`] for transport and Apple API failures.
    pub async fn playlist_tracks(
        &self,
        id: &str,
        limit: Option<u32>,
    ) -> Result<ResourceCollection<SongAttributes>, AppleMusicError> {
        let mut request = self.request(
            Method::GET,
            &format!("/v1/catalog/{}/playlists/{id}/tracks", self.storefront),
        )?;
        request = add_limit(request, limit);

        self.send_json(request).await
    }

    /// Fetches the signed-in user's library songs.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::MissingUserToken`] without a Music User Token.
    pub async fn library_songs(
        &self,
        limit: Option<u32>,
    ) -> Result<ResourceCollection<SongAttributes>, AppleMusicError> {
        self.require_user_token()?;
        let mut request = self.request(Method::GET, "/v1/me/library/songs")?;
        request = add_limit(request, limit);

        self.send_json(request).await
    }

    /// Fetches the user's recently played tracks.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::MissingUserToken`] without a Music User Token.
    pub async fn recently_played_tracks(
        &self,
        limit: Option<u32>,
    ) -> Result<ResourceCollection<SongAttributes>, AppleMusicError> {
        self.require_user_token()?;
        let mut request = self.request(Method::GET, "/v1/me/recent/played/tracks")?;
        request = add_limit(request, limit);

        self.send_json(request).await
    }

    /// Fetches the user's storefront.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::MissingUserToken`] without a Music User Token.
    pub async fn user_storefront(&self) -> Result<Resource<StorefrontAttributes>, AppleMusicError> {
        self.require_user_token()?;
        let response: ResourceCollection<StorefrontAttributes> = self
            .send_json(self.request(Method::GET, "/v1/me/storefront")?)
            .await?;

        single_resource(response)
    }

    /// Fetches timed lyrics as TTML.
    ///
    /// Apple Music does not expose this as a documented public endpoint. The
    /// `amp-api` host requires the same Music User Token the web player uses.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::MissingUserToken`] without a Music User Token
    /// and [`AppleMusicError::NotFound`] when Apple returns no TTML body.
    pub async fn lyrics(&self, song_id: &str, language: &str) -> Result<String, AppleMusicError> {
        let music_user_token = self.require_user_token()?;
        let url = self
            .amp_base
            .join(&format!(
                "/v1/catalog/{}/songs/{song_id}/lyrics",
                self.storefront
            ))
            .map_err(|error| AppleMusicError::InvalidUrl(error.to_string()))?;

        let response = self
            .http
            .get(url)
            .query(&[("l", language), ("extend", "ttmlLocalizations")])
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.developer_token),
            )
            .header(header::REFERER, "https://music.apple.com/")
            .header(header::ORIGIN, "https://music.apple.com")
            .header(
                header::COOKIE,
                format!("media-user-token={music_user_token}"),
            )
            .send()
            .await?;

        let status = response.status();
        let retry_after = retry_after_seconds(&response);
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(api_error(status, &body, retry_after));
        }

        let value: Value = serde_json::from_str(&body).map_err(AppleMusicError::from)?;
        let attributes = value.pointer("/data/0/attributes");
        let ttml = attributes
            .and_then(|attributes| attributes.get("ttml"))
            .and_then(Value::as_str)
            .or_else(|| {
                attributes
                    .and_then(|attributes| attributes.get("ttmlLocalizations"))
                    .and_then(Value::as_str)
            });

        ttml.map(str::to_owned).ok_or(AppleMusicError::NotFound)
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder, AppleMusicError> {
        let url = self
            .api_base
            .join(path)
            .map_err(|error| AppleMusicError::InvalidUrl(error.to_string()))?;
        let mut request = self
            .http
            .request(method, url)
            .header(header::ACCEPT, "application/json")
            .bearer_auth(&self.developer_token);

        if let Some(music_user_token) = &self.music_user_token {
            request = request.header("Music-User-Token", music_user_token);
        }

        Ok(request)
    }

    fn require_user_token(&self) -> Result<&str, AppleMusicError> {
        self.music_user_token
            .as_deref()
            .ok_or(AppleMusicError::MissingUserToken)
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, AppleMusicError> {
        let response = request.send().await?;
        let status = response.status();
        let retry_after = retry_after_seconds(&response);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(api_error(status, &body, retry_after));
        }

        Ok(response.json().await?)
    }
}

pub(crate) fn normalize_storefront(storefront: impl AsRef<str>) -> Result<String, AppleMusicError> {
    let storefront = storefront.as_ref().trim().to_ascii_lowercase();
    if storefront.len() != 2
        || !storefront
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Err(AppleMusicError::InvalidStorefront(storefront));
    }

    Ok(storefront)
}

fn add_limit(request: RequestBuilder, limit: Option<u32>) -> RequestBuilder {
    match limit {
        Some(limit) => request.query(&[("limit", limit)]),
        None => request,
    }
}

fn single_resource<T>(response: ResourceCollection<T>) -> Result<Resource<T>, AppleMusicError> {
    response
        .data
        .into_iter()
        .next()
        .ok_or(AppleMusicError::NotFound)
}

fn retry_after_seconds(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

fn api_error(status: StatusCode, body: &str, retry_after_secs: Option<u64>) -> AppleMusicError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return AppleMusicError::RateLimited { retry_after_secs };
    }

    let parsed: ApiErrorBody = serde_json::from_str(body).unwrap_or_default();
    let (code, message) = parsed
        .first()
        .unwrap_or(("UNKNOWN", "unknown Apple Music error"));

    AppleMusicError::Api {
        status: status.as_u16(),
        code: Some(code.to_owned()),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::AppleMusicClient;
    use crate::apple::models::SearchType;

    fn client_for(server: &MockServer) -> AppleMusicClient {
        AppleMusicClient::with_endpoints(
            "test-developer-token",
            "US ",
            reqwest::Url::parse(&server.uri()).unwrap(),
            reqwest::Url::parse(&server.uri()).unwrap(),
        )
        .expect("test client must build")
        .with_music_user_token("test-user-token")
    }

    #[tokio::test]
    async fn search_sends_headers_and_parses_songs() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/catalog/us/search"))
            .and(query_param("term", "daft punk"))
            .and(query_param("types", "songs"))
            .and(query_param("limit", "1"))
            .and(header("authorization", "Bearer test-developer-token"))
            .and(header("music-user-token", "test-user-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": {
                    "songs": {
                        "data": [{
                            "id": "1",
                            "type": "songs",
                            "attributes": {
                                "name": "One More Time",
                                "artistName": "Daft Punk",
                                "durationInMillis": 320_000,
                                "audioTraits": ["lossless"],
                                "artwork": {
                                    "url": "https://example.com/{w}x{h}bb.jpg",
                                    "width": 3000,
                                    "height": 3000
                                }
                            }
                        }]
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let response = client
            .search("daft punk", &[SearchType::Songs], Some(1))
            .await
            .expect("search must succeed");
        let song = response
            .results
            .songs
            .expect("songs must be present")
            .data
            .into_iter()
            .next()
            .expect("one song must be present");

        assert_eq!(song.id, "1");
        assert_eq!(song.attributes.name, "One More Time");
        assert_eq!(song.attributes.best_audio_variant().label(), "Lossless");
    }

    #[tokio::test]
    async fn lyrics_uses_amp_api_and_media_user_token_cookie() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/catalog/us/songs/42/lyrics"))
            .and(query_param("l", "en"))
            .and(query_param("extend", "ttmlLocalizations"))
            .and(header("authorization", "Bearer test-developer-token"))
            .and(header("cookie", "media-user-token=test-user-token"))
            .and(header("referer", "https://music.apple.com/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{
                    "id": "42",
                    "type": "songs",
                    "attributes": {
                        "ttml": "<tt>timed lyrics</tt>"
                    }
                }]
            })))
            .mount(&server)
            .await;

        let client = client_for(&server);
        let ttml = client
            .lyrics("42", "en")
            .await
            .expect("lyrics must succeed");

        assert_eq!(ttml, "<tt>timed lyrics</tt>");
    }

    #[tokio::test]
    async fn missing_user_token_is_reported_before_the_request() {
        let server = MockServer::start().await;
        let client = AppleMusicClient::with_endpoints(
            "token",
            "us",
            reqwest::Url::parse(&server.uri()).unwrap(),
            reqwest::Url::parse(&server.uri()).unwrap(),
        )
        .expect("test client must build");

        let error = client
            .library_songs(None)
            .await
            .expect_err("personal request must fail without a token");

        assert!(matches!(
            error,
            crate::apple::error::AppleMusicError::MissingUserToken
        ));
    }

    #[test]
    fn rejects_invalid_storefront() {
        let error = AppleMusicClient::new("token", "usa")
            .expect_err("three-letter storefronts are invalid");

        assert!(matches!(
            error,
            crate::apple::error::AppleMusicError::InvalidStorefront(_)
        ));
    }
}
