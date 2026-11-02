//! Developer token handling.
//!
//! The client never needs Apple's private key. A deployment-owned token service
//! signs the ES256 JWT and the client only decodes the public payload to know
//! when it should refresh.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use crate::apple::error::AppleMusicError;

/// Claims Apple expects in a MusicKit developer token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeveloperTokenClaims {
    /// Apple Developer Team ID.
    pub iss: String,
    /// Issued-at time, in Unix seconds.
    pub iat: u64,
    /// Expiration time, in Unix seconds.
    pub exp: u64,
    /// Optional web origins allowed to use this token.
    #[serde(default)]
    pub origin: Option<Vec<String>>,
}

/// A developer token plus its decoded public claims.
#[derive(Debug, Clone)]
pub struct DeveloperToken {
    raw: String,
    claims: DeveloperTokenClaims,
}

impl DeveloperToken {
    /// Parses a JWT and decodes its claims.
    ///
    /// This does not verify the signature; Apple's server remains the authority.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::InvalidDeveloperToken`] when the token is not
    /// a three-part JWT or its payload is not valid base64url JSON.
    pub fn parse(raw: impl Into<String>) -> Result<Self, AppleMusicError> {
        let raw = raw.into();
        let payload = raw
            .split('.')
            .nth(1)
            .ok_or_else(|| AppleMusicError::InvalidDeveloperToken("missing JWT payload".into()))?;
        let bytes = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|error| AppleMusicError::InvalidDeveloperToken(error.to_string()))?;
        let claims = serde_json::from_slice::<DeveloperTokenClaims>(&bytes)
            .map_err(|error| AppleMusicError::InvalidDeveloperToken(error.to_string()))?;

        Ok(Self { raw, claims })
    }

    /// The raw JWT, suitable for an `Authorization: Bearer ...` header.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The decoded public claims.
    #[must_use]
    pub const fn claims(&self) -> &DeveloperTokenClaims {
        &self.claims
    }

    /// Whether the token has expired at the given Unix timestamp.
    #[must_use]
    pub const fn is_expired_at(&self, now_unix_seconds: u64) -> bool {
        now_unix_seconds >= self.claims.exp
    }

    /// Whether the token has expired right now.
    #[must_use]
    pub fn is_expired_now(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        self.is_expired_at(now)
    }

    /// Seconds until expiration, saturating at zero.
    #[must_use]
    pub const fn seconds_until_expiry(&self, now_unix_seconds: u64) -> u64 {
        self.claims.exp.saturating_sub(now_unix_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::DeveloperToken;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use serde_json::json;

    fn token_with_claims(claims: serde_json::Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"ES256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn decodes_claims_and_detects_expiry() {
        let token = token_with_claims(json!({
            "iss": "TEAMID1234",
            "iat": 1_700_000_000_u64,
            "exp": 1_700_003_600_u64
        }));
        let parsed = DeveloperToken::parse(token).expect("token must parse");

        assert_eq!(parsed.claims().iss, "TEAMID1234");
        assert_eq!(parsed.seconds_until_expiry(1_700_000_000), 3_600);
        assert!(!parsed.is_expired_at(1_700_003_599));
        assert!(parsed.is_expired_at(1_700_003_600));
    }

    #[test]
    fn rejects_non_jwt_values() {
        assert!(DeveloperToken::parse("not-a-token").is_err());
    }
}
