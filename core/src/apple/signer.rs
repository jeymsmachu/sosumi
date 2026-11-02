//! ES256 developer-token signing.
//!
//! This is the server-side half of MusicKit authentication. The `.p8` key must
//! live in the token service, never in the Flutter client.

use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::SecretKey;
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use serde::Serialize;

use crate::apple::error::AppleMusicError;
use crate::apple::token::{DeveloperToken, DeveloperTokenClaims};

/// Apple's maximum developer-token lifetime: six months.
pub const MAX_TOKEN_TTL: Duration = Duration::from_secs(15_777_000);

/// Signs MusicKit developer tokens with a P-256 private key.
pub struct DeveloperTokenSigner {
    signing_key: SigningKey,
    team_id: String,
    key_id: String,
    ttl: Duration,
}

impl DeveloperTokenSigner {
    /// Creates a signer from an Apple `.p8` PKCS#8 PEM key.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::TokenSigning`] when the PEM is invalid or the
    /// requested TTL is outside Apple's permitted range.
    pub fn from_pkcs8_pem(
        pem: &str,
        team_id: impl Into<String>,
        key_id: impl Into<String>,
        ttl: Duration,
    ) -> Result<Self, AppleMusicError> {
        if ttl.is_zero() || ttl > MAX_TOKEN_TTL {
            return Err(AppleMusicError::TokenSigning(format!(
                "token TTL must be between 1 and {} seconds",
                MAX_TOKEN_TTL.as_secs()
            )));
        }

        let secret = SecretKey::from_pkcs8_pem(pem)
            .map_err(|error| AppleMusicError::TokenSigning(error.to_string()))?;
        let signing_key = SigningKey::from(secret);

        Ok(Self {
            signing_key,
            team_id: team_id.into(),
            key_id: key_id.into(),
            ttl,
        })
    }

    /// Issues a token valid at the given Unix timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::TokenSigning`] if JSON encoding or signing
    /// fails.
    pub fn issue_at(&self, now_unix_seconds: u64) -> Result<DeveloperToken, AppleMusicError> {
        self.issue_at_with_origin(now_unix_seconds, None)
    }

    /// Issues a token with an optional web-origin restriction.
    ///
    /// # Errors
    ///
    /// Returns [`AppleMusicError::TokenSigning`] if JSON encoding or signing
    /// fails.
    pub fn issue_at_with_origin(
        &self,
        now_unix_seconds: u64,
        origin: Option<Vec<String>>,
    ) -> Result<DeveloperToken, AppleMusicError> {
        #[derive(Serialize)]
        struct Header<'a> {
            alg: &'a str,
            kid: &'a str,
            typ: &'a str,
        }

        let header = Header {
            alg: "ES256",
            kid: &self.key_id,
            typ: "JWT",
        };
        let claims = DeveloperTokenClaims {
            iss: self.team_id.clone(),
            iat: now_unix_seconds,
            exp: now_unix_seconds.saturating_add(self.ttl.as_secs()),
            origin,
        };

        let header = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&header)
                .map_err(|error| AppleMusicError::TokenSigning(error.to_string()))?,
        );
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&claims)
                .map_err(|error| AppleMusicError::TokenSigning(error.to_string()))?,
        );
        let signing_input = format!("{header}.{payload}");
        let signature: Signature = self.signing_key.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        DeveloperToken::parse(format!("{signing_input}.{signature}"))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DeveloperTokenSigner;

    // Test-only key. Not an Apple credential and not used outside tests.
    const TEST_PEM: &str = r"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgiRFa7Qxgy29AatuV
IZ4yRLlLU7kw3xrnGNZUC9Rg7mahRANCAARtjWb/1LMS1yzDipVqtIFTpJKdDlIa
tg+p0LD0NrKwvSMtzkNfT1fEDsmFvyuFGGQYPY3BNhj3Lgp3HRrHv/Kf
-----END PRIVATE KEY-----";

    #[test]
    fn signs_a_parseable_es256_token() {
        let signer = DeveloperTokenSigner::from_pkcs8_pem(
            TEST_PEM,
            "TEAMID1234",
            "KEYID12345",
            Duration::from_secs(3_600),
        )
        .expect("test key must parse");

        let token = signer.issue_at(1_700_000_000).expect("token must sign");
        let claims = token.claims();

        assert_eq!(claims.iss, "TEAMID1234");
        assert_eq!(claims.iat, 1_700_000_000);
        assert_eq!(claims.exp, 1_700_003_600);
        assert_eq!(token.as_str().split('.').count(), 3);
    }

    #[test]
    fn rejects_ttl_above_apple_maximum() {
        let error = DeveloperTokenSigner::from_pkcs8_pem(
            TEST_PEM,
            "TEAMID1234",
            "KEYID12345",
            Duration::from_secs(15_777_001),
        )
        .err()
        .expect("TTL above six months must fail");

        assert!(matches!(
            error,
            crate::apple::error::AppleMusicError::TokenSigning(_)
        ));
    }
}
