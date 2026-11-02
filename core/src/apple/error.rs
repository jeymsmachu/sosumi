//! Errors from the Apple Music API layer.

use serde::Deserialize;
use thiserror::Error;

/// Errors returned by Apple Music API operations.
#[derive(Debug, Error)]
pub enum AppleMusicError {
    /// The HTTP request failed before a response was received.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The response body was not valid JSON for the expected type.
    #[error("invalid Apple Music response: {0}")]
    Json(#[from] serde_json::Error),

    /// A storefront was not a two-letter region code.
    #[error("invalid storefront: {0}")]
    InvalidStorefront(String),

    /// A base URL could not be built.
    #[error("invalid Apple Music URL: {0}")]
    InvalidUrl(String),

    /// A JWT developer token could not be parsed.
    #[error("invalid developer token: {0}")]
    InvalidDeveloperToken(String),

    /// A developer token could not be signed.
    #[error("could not sign developer token: {0}")]
    TokenSigning(String),

    /// The request needs a Music User Token.
    #[error("a Music User Token is required for this request")]
    MissingUserToken,

    /// The requested resource was not present in the response.
    #[error("the requested Apple Music resource was not found")]
    NotFound,

    /// Apple Music rejected the request with a structured error.
    #[error("Apple Music API returned {status}: {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Apple error code, when provided.
        code: Option<String>,
        /// Human-readable message.
        message: String,
    },

    /// Apple Music rate-limited the request.
    #[error("Apple Music rate limited the request")]
    RateLimited {
        /// Value of the `Retry-After` header, in seconds.
        retry_after_secs: Option<u64>,
    },
}

impl AppleMusicError {
    /// HTTP status code, when this error came from a response.
    #[must_use]
    pub const fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::RateLimited { .. } => Some(429),
            _ => None,
        }
    }

    /// Whether this error is an authentication or authorization failure.
    #[must_use]
    pub const fn is_auth_error(&self) -> bool {
        matches!(self.status(), Some(401 | 403))
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ApiErrorBody {
    #[serde(default)]
    pub(crate) errors: Vec<ApiErrorItem>,
}

impl ApiErrorBody {
    pub(crate) fn first(&self) -> Option<(&str, &str)> {
        self.errors
            .first()
            .map(|error| (error.code.as_deref().unwrap_or("UNKNOWN"), error.message()))
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorItem {
    #[serde(default)]
    pub(crate) code: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) detail: Option<String>,
}

impl ApiErrorItem {
    pub(crate) fn message(&self) -> &str {
        self.detail
            .as_deref()
            .or(self.title.as_deref())
            .unwrap_or("unknown Apple Music error")
    }
}
