//! HTTP client shared by the Apple modules.
//!
//! One place to own transport policy: timeouts, connection reuse, a user agent
//! that looks like a browser (Apple serves different responses otherwise), and
//! the mapping from `reqwest` errors to [`ProviderError`].
//!
//! TLS verification is always on. There is deliberately no switch to disable it,
//! which matches the promise in `docs/SECURITY.md`.

use std::time::Duration;

use serde::de::DeserializeOwned;

use sosumi_provider_api::ProviderError;

/// A browser-like user agent. Apple's web endpoints behave differently for
/// clients that do not present one.
const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 ",
    "(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"
);

/// The origin Apple's developer token is scoped to.
///
/// The token carries `root_https_origin: ["apple.com"]`, and the API rejects a
/// request without a matching `Origin`. This is not optional.
const ORIGIN: &str = "https://music.apple.com";

/// The referer the web player sends. Sent alongside the origin for consistency.
const REFERER: &str = "https://music.apple.com/";

/// A pooled HTTP client.
#[derive(Debug, Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    /// Builds a client with sane timeouts and connection reuse.
    ///
    /// # Errors
    ///
    /// Fails only if the TLS backend cannot be initialised.
    pub fn new() -> Result<Self, ProviderError> {
        let inner = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Wraps an existing client, for tests that inject a mock transport.
    #[must_use]
    pub const fn from_client(inner: reqwest::Client) -> Self {
        Self { inner }
    }

    /// The underlying client, for callers that need a less common verb.
    #[must_use]
    pub const fn raw(&self) -> &reqwest::Client {
        &self.inner
    }

    /// Issues a GET and decodes the JSON body.
    ///
    /// # Errors
    ///
    /// Returns a mapped [`ProviderError`] for transport, status, or parse
    /// failures.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<T, ProviderError> {
        let response = self.send_get(url, bearer).await?;
        response.json::<T>().await.map_err(|e| ProviderError::Malformed(e.to_string()))
    }

    /// Issues a GET and returns the body as text.
    ///
    /// Used for playlists, which are not JSON.
    ///
    /// # Errors
    ///
    /// Returns a mapped [`ProviderError`] for transport or status failures.
    pub async fn get_text(&self, url: &str, bearer: Option<&str>) -> Result<String, ProviderError> {
        let response = self.send_get(url, bearer).await?;
        response.text().await.map_err(|e| ProviderError::Network(e.to_string()))
    }

    /// Builds and sends a GET with the headers Apple's API requires.
    ///
    /// Apple's developer token is scoped to `root_https_origin: ["apple.com"]`,
    /// so the API rejects any request that does not carry an `Origin` header
    /// naming the web player. Omitting it returns a bare 401, which reads as
    /// "not signed in" and is easy to misdiagnose as an auth problem.
    async fn send_get(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<reqwest::Response, ProviderError> {
        let mut request = self
            .inner
            .get(url)
            .header(reqwest::header::ORIGIN, ORIGIN)
            .header(reqwest::header::REFERER, REFERER);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.map_err(|e| map_transport(&e))?;
        check_status(response).await
    }

    /// Issues a POST with a JSON body and decodes the JSON response.
    ///
    /// # Errors
    ///
    /// Returns a mapped [`ProviderError`] for transport, status, or parse
    /// failures.
    pub async fn post_json<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
        body: &B,
    ) -> Result<T, ProviderError> {
        let mut request = self
            .inner
            .post(url)
            .header(reqwest::header::ORIGIN, ORIGIN)
            .header(reqwest::header::REFERER, REFERER)
            .json(body);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.map_err(|e| map_transport(&e))?;
        let response = check_status(response).await?;
        response.json::<T>().await.map_err(|e| ProviderError::Malformed(e.to_string()))
    }

    /// Issues a POST with a raw body and returns the raw response bytes.
    ///
    /// Used for the licence exchange, which speaks protobuf rather than JSON.
    ///
    /// # Errors
    ///
    /// Returns a mapped [`ProviderError`] for transport or status failures.
    pub async fn post_bytes(
        &self,
        url: &str,
        bearer: Option<&str>,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<Vec<u8>, ProviderError> {
        let mut request = self
            .inner
            .post(url)
            .header(reqwest::header::ORIGIN, ORIGIN)
            .header(reqwest::header::REFERER, REFERER)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.map_err(|e| map_transport(&e))?;
        let response = check_status(response).await?;
        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| ProviderError::Network(e.to_string()))
    }
}

/// Maps a transport failure to a provider error.
fn map_transport(err: &reqwest::Error) -> ProviderError {
    if let Some(status) = err.status() {
        return ProviderError::Upstream { status: status.as_u16(), message: err.to_string() };
    }
    ProviderError::Network(err.to_string())
}

/// Turns a non-success status into a typed error.
async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, ProviderError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    if status.as_u16() == 404 {
        return Err(ProviderError::NotFound(response.url().to_string()));
    }
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(ProviderError::NotAuthenticated);
    }
    let message = response.text().await.unwrap_or_default();
    Err(ProviderError::Upstream { status: status.as_u16(), message })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_builds() {
        assert!(HttpClient::new().is_ok());
    }

    #[test]
    fn user_agent_identifies_a_browser() {
        assert!(USER_AGENT.contains("Mozilla"));
        assert!(USER_AGENT.contains("Linux"));
    }
}
