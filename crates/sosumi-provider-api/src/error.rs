//! Provider errors.

use sosumi_diagnostics::HealthStatus;

/// Convenience alias for provider results.
pub type Result<T> = std::result::Result<T, ProviderError>;

/// Anything a provider can fail at.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The requested item does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// The user is not signed in, or their session expired.
    #[error("not authenticated")]
    NotAuthenticated,

    /// The subscription does not cover this request (quality, region, …).
    #[error("not entitled: {0}")]
    NotEntitled(String),

    /// The provider cannot serve this request without an optional component
    /// that is not installed. The UI offers to install it.
    #[error("{feature} is not available")]
    CapabilityMissing {
        /// Machine id matching [`sosumi_diagnostics::Health::id`].
        feature: &'static str,
        /// What the user should do.
        remedy: String,
    },

    /// The upstream service rejected the request.
    #[error("upstream error ({status}): {message}")]
    Upstream {
        /// HTTP status, when there was one.
        status: u16,
        message: String,
    },

    /// The network is unreachable.
    #[error("network error: {0}")]
    Network(String),

    /// A response could not be understood.
    #[error("malformed response: {0}")]
    Malformed(String),

    /// A hard error with context.
    #[error("{0}")]
    Other(String),
}

impl ProviderError {
    /// Maps an error to a health status for the capabilities screen.
    #[must_use]
    pub const fn health_status(&self) -> HealthStatus {
        match self {
            Self::NotAuthenticated
            | Self::CapabilityMissing { .. }
            | Self::NotFound(_)
            | Self::NotEntitled(_) => HealthStatus::Unavailable,
            Self::Upstream { .. } | Self::Network(_) | Self::Malformed(_) | Self::Other(_) => {
                HealthStatus::Degraded
            }
        }
    }

    /// True when the failure is worth retrying without user action.
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Upstream { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_and_capability_failures_are_user_actionable() {
        assert_eq!(ProviderError::NotAuthenticated.health_status(), HealthStatus::Unavailable);
        assert_eq!(
            ProviderError::CapabilityMissing { feature: "fairplay", remedy: "Install".into() }
                .health_status(),
            HealthStatus::Unavailable
        );
    }

    #[test]
    fn network_failures_are_transient() {
        assert!(ProviderError::Network("reset".into()).is_transient());
        assert!(!ProviderError::NotFound("x".into()).is_transient());
    }
}
