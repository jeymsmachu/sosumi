//! Apple Music account session.
//!
//! The session pairs a signed developer token with the user's Music User Token.
//! Platform auth adapters are responsible for obtaining the Music User Token;
//! this type only validates and packages it for REST calls.

use crate::apple::AppleMusicClient;
use crate::apple::client::normalize_storefront;
use crate::apple::error::AppleMusicError;
use crate::apple::token::DeveloperToken;

/// A signed-in Apple Music account.
#[derive(Debug, Clone)]
pub struct AppleMusicSession {
    developer_token: DeveloperToken,
    music_user_token: String,
    storefront: String,
}

impl AppleMusicSession {
    /// Creates a session from an already-signed developer token and user token.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::InvalidStorefront`] for a non-two-letter
    /// storefront.
    pub fn new(
        developer_token: DeveloperToken,
        music_user_token: impl Into<String>,
        storefront: impl AsRef<str>,
    ) -> Result<Self, AppleMusicError> {
        Ok(Self {
            developer_token,
            music_user_token: music_user_token.into(),
            storefront: normalize_storefront(storefront)?,
        })
    }

    /// Builds an authenticated REST client.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn client(&self) -> Result<AppleMusicClient, AppleMusicError> {
        AppleMusicClient::new(self.developer_token.as_str(), &self.storefront)
            .map(|client| client.with_music_user_token(self.music_user_token.clone()))
    }

    /// The decoded developer token.
    #[must_use]
    pub const fn developer_token(&self) -> &DeveloperToken {
        &self.developer_token
    }

    /// The user's Music User Token.
    #[must_use]
    pub fn music_user_token(&self) -> &str {
        &self.music_user_token
    }

    /// The normalized storefront.
    #[must_use]
    pub fn storefront(&self) -> &str {
        &self.storefront
    }

    /// Whether the developer token needs refreshing.
    #[must_use]
    pub fn developer_token_expired(&self) -> bool {
        self.developer_token.is_expired_now()
    }

    /// Replaces the developer token after a refresh.
    pub fn set_developer_token(&mut self, developer_token: DeveloperToken) {
        self.developer_token = developer_token;
    }

    /// Replaces the Music User Token after re-authentication.
    pub fn set_music_user_token(&mut self, music_user_token: impl Into<String>) {
        self.music_user_token = music_user_token.into();
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use serde_json::json;

    use super::AppleMusicSession;
    use crate::apple::error::AppleMusicError;
    use crate::apple::token::DeveloperToken;

    fn developer_token() -> DeveloperToken {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&json!({
                "iss": "TEAMID1234",
                "iat": 1_700_000_000_u64,
                "exp": 4_000_000_000_u64
            }))
            .unwrap(),
        );
        DeveloperToken::parse(format!("{header}.{payload}.signature")).unwrap()
    }

    #[test]
    fn normalizes_storefront_and_builds_client() {
        let session = AppleMusicSession::new(developer_token(), "music-user-token", "US ")
            .expect("session must build");

        assert_eq!(session.storefront(), "us");
        assert_eq!(session.music_user_token(), "music-user-token");
        assert!(!session.developer_token_expired());
        assert!(session.client().is_ok());
    }

    #[test]
    fn rejects_invalid_storefront() {
        let error = AppleMusicSession::new(developer_token(), "music-user-token", "usa")
            .expect_err("invalid storefront must fail");

        assert!(matches!(error, AppleMusicError::InvalidStorefront(_)));
    }
}
