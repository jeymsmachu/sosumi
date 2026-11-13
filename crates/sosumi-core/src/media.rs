//! Resolving a stream description into decodable bytes.
//!
//! A provider describes *where* audio comes from; this module turns that into
//! bytes the decoder can read. The two cases that matter:
//!
//! - **HLS**: Apple serves a track as many small segments that are byte ranges
//!   of one fragmented MP4. Concatenating them in order reconstructs the file,
//!   which is why the whole track is materialised before decoding. A fully
//!   streaming path is a later refinement; correctness first.
//! - **HTTP**: a single URL, read whole.
//!
//! Nothing here understands DRM. An encrypted source arrives as a decrypting
//! reader and is read through the same interface.

use std::time::Duration;

use bytes::Bytes;
use sosumi_provider_api::{AudioSource, ProviderError, Result};

/// Maps a transport error to a provider error, matching the API client's rules.
fn map_transport(error: &reqwest::Error) -> ProviderError {
    if let Some(status) = error.status() {
        return ProviderError::Upstream { status: status.as_u16(), message: error.to_string() };
    }
    ProviderError::Network(error.to_string())
}

/// A shared client for fetching media.
#[derive(Debug, Clone)]
pub struct MediaFetcher {
    client: reqwest::Client,
}

impl MediaFetcher {
    /// Builds a fetcher with connection reuse and a generous read timeout.
    ///
    /// The timeout is longer than the API client's: a lossless track is tens of
    /// megabytes, and a slow connection is still a working one.
    ///
    /// # Errors
    ///
    /// Fails only if the TLS backend cannot be initialised.
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(8)
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        Ok(Self { client })
    }

    /// Materialises a source into one byte buffer.
    ///
    /// # Errors
    ///
    /// Returns a network error when a request fails, and a malformed error when
    /// a source type cannot be read yet.
    pub async fn fetch_all(&self, source: &AudioSource) -> Result<Bytes> {
        match source {
            AudioSource::Http { url, headers } => {
                if let Some(path) = url.strip_prefix("file://") {
                    return self.read_local(path).await;
                }
                self.get(url, headers).await
            }
            AudioSource::Hls { segments, headers } => self.fetch_hls(segments, headers).await,
            AudioSource::Decrypting(_) => Err(ProviderError::Other(
                "decrypting sources are read by the caller, not fetched".into(),
            )),
        }
    }

    /// Reads a local file.
    async fn read_local(&self, path: &str) -> Result<Bytes> {
        tokio::fs::read(path)
            .await
            .map(Bytes::from)
            .map_err(|e| ProviderError::Network(format!("reading {path}: {e}")))
    }

    /// Fetches one URL.
    async fn get(&self, url: &str, headers: &[(String, String)]) -> Result<Bytes> {
        let mut request = self.client.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send().await.map_err(|e| map_transport(&e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ProviderError::Upstream {
                status: status.as_u16(),
                message: format!("media request to {url} failed"),
            });
        }
        response.bytes().await.map_err(|e| ProviderError::Network(e.to_string()))
    }

    /// Fetches and concatenates HLS segments, in order.
    ///
    /// Segments are fetched sequentially on purpose. They are byte ranges of one
    /// file and are small, so the request overhead dominates, and a parallel
    /// fetch would need reordering buffers to reconstruct the stream.
    async fn fetch_hls(&self, segments: &[String], headers: &[(String, String)]) -> Result<Bytes> {
        if segments.is_empty() {
            return Err(ProviderError::Malformed("playlist has no segments".into()));
        }

        let mut buffer = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            let chunk = self.get(segment, headers).await.map_err(|error| {
                tracing::warn!(
                    target: "sosumi::media",
                    segment = index,
                    "segment fetch failed: {error}"
                );
                error
            })?;
            buffer.extend_from_slice(&chunk);
        }

        if buffer.is_empty() {
            return Err(ProviderError::Malformed("the stream produced no bytes".into()));
        }
        Ok(Bytes::from(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetcher() -> MediaFetcher {
        MediaFetcher::new().expect("client")
    }

    #[tokio::test]
    async fn a_local_file_is_read_whole() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("audio.bin");
        std::fs::write(&path, b"audio bytes").expect("write");

        let source =
            AudioSource::Http { url: format!("file://{}", path.display()), headers: Vec::new() };
        let bytes = fetcher().fetch_all(&source).await.expect("fetch");
        assert_eq!(&bytes[..], b"audio bytes");
    }

    #[tokio::test]
    async fn a_missing_local_file_reports_a_network_error() {
        let source =
            AudioSource::Http { url: "file:///nonexistent/audio.bin".into(), headers: Vec::new() };
        assert!(matches!(fetcher().fetch_all(&source).await, Err(ProviderError::Network(_))));
    }

    #[tokio::test]
    async fn an_empty_hls_playlist_is_rejected_before_any_request() {
        let source = AudioSource::Hls { segments: Vec::new(), headers: Vec::new() };
        assert!(matches!(fetcher().fetch_all(&source).await, Err(ProviderError::Malformed(_))));
    }

    #[tokio::test]
    async fn a_decrypting_source_is_refused_with_an_explanation() {
        struct Noop;
        impl sosumi_provider_api::stream::source::AudioReader for Noop {
            fn into_reader(self: Box<Self>) -> std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>> {
                Box::pin(tokio::io::empty())
            }
        }

        let source = AudioSource::Decrypting(Box::new(Noop));
        let error = fetcher().fetch_all(&source).await.expect_err("must fail");
        assert!(error.to_string().contains("read by the caller"));
    }
}
