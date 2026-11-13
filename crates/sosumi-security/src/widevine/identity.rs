//! Device identity for the Widevine CDM.
//!
//! A licence server only accepts a device it recognises, so Sosumi neither
//! ships nor silently generates an identity. The user supplies one; this module
//! loads it, validates it, and exposes exactly the operations the licence
//! handshake needs (sign, and unwrap the session key).
//!
//! Files, in the directory `sosumi-storage` gives us:
//!
//! - `device_private_key` — a PEM PKCS#1 or PKCS#8 RSA private key
//! - `device_client_id_blob` — a serialised `ClientIdentification`
//!
//! Nothing here reads the environment or the network; the caller decides the
//! directory, which keeps this testable and keeps path logic in one place.

use std::path::{Path, PathBuf};

use prost::Message;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use sha1::{Digest, Sha1};

use super::license::ClientIdentification;

/// Reasons an identity cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// No identity is configured. The UI turns this into a setup action.
    #[error("no Widevine device identity is configured")]
    NotConfigured,
    /// The private key file could not be parsed.
    #[error("device private key is not a valid RSA key")]
    InvalidKey,
    /// The client-id blob could not be parsed.
    #[error("device client-id blob is malformed")]
    InvalidClientId,
    /// The key is too small to be accepted by a licence server.
    #[error("device key is {0} bits; at least 1024 is required")]
    KeyTooSmall(usize),
    /// Underlying I/O failed.
    #[error("identity I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A loaded, validated device identity.
pub struct DeviceIdentity {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    client_id_blob: Vec<u8>,
    client_id: ClientIdentification,
}

impl std::fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceIdentity")
            .field("key_bits", &self.public_key.n().bits())
            .field("client_id_blob_len", &self.client_id_blob.len())
            .field("private_key", &"<opaque>")
            .field("public_key", &"<opaque>")
            .field("client_id", &"<opaque>")
            .finish()
    }
}

/// The file name of the private key inside the identity directory.
pub const KEY_FILE: &str = "device_private_key";
/// The file name of the client-id blob inside the identity directory.
pub const CLIENT_ID_FILE: &str = "device_client_id_blob";

impl DeviceIdentity {
    /// Builds an identity from an already-parsed key and client-id blob.
    ///
    /// # Errors
    ///
    /// Fails when the client-id blob is not a valid `ClientIdentification`, or
    /// the key is smaller than 1024 bits.
    pub fn from_parts(
        private_key: RsaPrivateKey,
        client_id_blob: Vec<u8>,
    ) -> Result<Self, IdentityError> {
        let bits = private_key.n().bits();
        if bits < 1024 {
            return Err(IdentityError::KeyTooSmall(bits));
        }
        let client_id = ClientIdentification::decode(client_id_blob.as_slice())
            .map_err(|_| IdentityError::InvalidClientId)?;
        let public_key = RsaPublicKey::from(&private_key);
        Ok(Self { private_key, public_key, client_id_blob, client_id })
    }

    /// Loads an identity from a directory.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::NotConfigured`] when the directory has no
    /// identity, and a specific error when a file is present but unusable.
    pub fn load(directory: &Path) -> Result<Self, IdentityError> {
        let key_path = directory.join(KEY_FILE);
        let client_id_path = directory.join(CLIENT_ID_FILE);

        if !key_path.exists() || !client_id_path.exists() {
            return Err(IdentityError::NotConfigured);
        }

        let pem = std::fs::read_to_string(&key_path)?;
        let client_id_blob = std::fs::read(&client_id_path)?;
        let private_key = parse_private_key(&pem)?;
        Self::from_parts(private_key, client_id_blob)
    }

    /// The path to a file that reports whether an identity is present.
    #[must_use]
    pub fn probe(directory: &Path) -> PathBuf {
        directory.join(KEY_FILE)
    }

    /// The RSA private key, for the licence handshake.
    #[must_use]
    pub(crate) fn private_key(&self) -> &RsaPrivateKey {
        &self.private_key
    }

    /// The RSA public key, for tests and diagnostics.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn public_key(&self) -> &RsaPublicKey {
        &self.public_key
    }

    /// Builds the `ClientIdentification` to embed in a licence request.
    #[must_use]
    pub fn client_identification(&self) -> ClientIdentification {
        self.client_id.clone()
    }

    /// The raw client-id blob.
    #[must_use]
    pub fn client_id_blob(&self) -> &[u8] {
        &self.client_id_blob
    }

    /// Signs a licence request with the device key (RSASSA-PKCS1-v1.5, SHA-1,
    /// which is what Widevine's protocol specifies).
    ///
    /// # Errors
    ///
    /// Fails only if the key is unusable for signing, which validation prevents.
    pub(crate) fn sign(&self, message: &[u8]) -> Result<Vec<u8>, rsa::Error> {
        let mut hasher = Sha1::new();
        hasher.update(message);
        let digest = hasher.finalize();
        self.private_key.sign(Pkcs1v15Sign::new::<Sha1>(), &digest)
    }
}

/// Parses a PEM private key, accepting both PKCS#1 and PKCS#8 encodings.
///
/// Widely distributed identities use either form, so both are supported rather
/// than forcing users to convert their file.
fn parse_private_key(pem: &str) -> Result<RsaPrivateKey, IdentityError> {
    if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(pem) {
        return Ok(key);
    }
    RsaPrivateKey::from_pkcs8_pem(pem).map_err(|_| IdentityError::InvalidKey)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::pkcs8::EncodePrivateKey;

    fn client_id_blob() -> Vec<u8> {
        use super::super::license::{ClientIdentification, ClientIdentificationType};
        ClientIdentification {
            r#type: ClientIdentificationType::DeviceCertificate as i32,
            token: vec![1, 2, 3],
            client_info: vec![],
            provider_client_token: None,
            license_counter: 0,
            client_capabilities: None,
        }
        .encode_to_vec()
    }

    fn write_identity(dir: &Path, pem: &str) {
        std::fs::write(dir.join(KEY_FILE), pem).expect("write key");
        std::fs::write(dir.join(CLIENT_ID_FILE), client_id_blob()).expect("write blob");
    }

    #[test]
    fn missing_directory_reports_not_configured() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(matches!(DeviceIdentity::load(dir.path()), Err(IdentityError::NotConfigured)));
    }

    #[test]
    fn pkcs1_pem_is_accepted() {
        let key = RsaPrivateKey::new(&mut OsRng, 1024).expect("keygen");
        let pem = key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).expect("encode");
        let dir = tempfile::tempdir().expect("tempdir");
        write_identity(dir.path(), &pem);
        let identity = DeviceIdentity::load(dir.path()).expect("load");
        assert_eq!(identity.client_id_blob(), client_id_blob());
    }

    #[test]
    fn pkcs8_pem_is_accepted() {
        let key = RsaPrivateKey::new(&mut OsRng, 1024).expect("keygen");
        let pem = key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).expect("encode");
        let dir = tempfile::tempdir().expect("tempdir");
        write_identity(dir.path(), &pem);
        assert!(DeviceIdentity::load(dir.path()).is_ok());
    }

    #[test]
    fn garbage_key_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_identity(dir.path(), "not a pem");
        assert!(matches!(DeviceIdentity::load(dir.path()), Err(IdentityError::InvalidKey)));
    }

    #[test]
    fn malformed_client_id_is_rejected() {
        let key = RsaPrivateKey::new(&mut OsRng, 1024).expect("keygen");
        let pem = key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).expect("encode");
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(KEY_FILE), pem.as_bytes()).expect("write");
        std::fs::write(dir.path().join(CLIENT_ID_FILE), [0xff, 0xff, 0xff]).expect("write");
        assert!(matches!(DeviceIdentity::load(dir.path()), Err(IdentityError::InvalidClientId)));
    }

    #[test]
    fn small_keys_are_rejected() {
        let key = RsaPrivateKey::new(&mut OsRng, 512).expect("keygen");
        let err = DeviceIdentity::from_parts(key, client_id_blob()).expect_err("small");
        assert!(matches!(err, IdentityError::KeyTooSmall(512)));
    }
}
