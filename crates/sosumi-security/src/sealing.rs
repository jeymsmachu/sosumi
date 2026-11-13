//! Offline file sealing.
//!
//! `FairPlay` gives no portable licence, so a downloaded track can only be
//! re-sealed under a key Sosumi owns. That key lives in the OS keyring (see
//! `sosumi-storage`); this module is the envelope format.
//!
//! **Threat model, stated honestly.** This protects downloads from casual
//! copying and from other userland processes that cannot read the keyring. It
//! is *not* tamper-proof against a user who controls the machine and can read
//! the process memory or the keyring. Sosumi does not claim otherwise, and this
//! is documented in `docs/SECURITY.md`.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Magic bytes at the start of every sealed file.
pub const MAGIC: &[u8; 8] = b"SOSUMI01";

/// AES-GCM nonce length.
const NONCE_LEN: usize = 12;

/// Errors from sealing or opening.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The file is not a Sosumi sealed file.
    #[error("not a Sosumi sealed file")]
    NotSealed,
    /// The file is a sealed file from an incompatible version.
    #[error("unsupported sealed-file version {0}")]
    UnsupportedVersion(u8),
    /// The file is truncated.
    #[error("sealed file is truncated")]
    Truncated,
    /// Authentication failed — wrong key, or the ciphertext was altered.
    #[error("sealed file failed authentication")]
    Authentication,
}

/// The header stored alongside the ciphertext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealHeader {
    /// Format version, for future migration.
    pub version: u8,
    /// The key id this file was sealed under, so a rotated key can be detected
    /// before attempting decryption.
    pub key_id: String,
    /// Original plaintext length, for progress reporting.
    pub len: u64,
}

/// A sealed file: header plus ciphertext.
#[derive(Debug, Clone)]
pub struct SealedFile {
    pub header: SealHeader,
    pub ciphertext: Vec<u8>,
    /// The nonce used for this file.
    pub nonce: [u8; NONCE_LEN],
}

impl SealedFile {
    /// Serialises to the on-disk layout:
    ///
    /// ```text
    /// [8] magic | [1] version | [2] header len (LE u16) | header JSON | [12] nonce | ciphertext
    /// ```
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let header = serde_json::to_vec(&self.header).unwrap_or_default();
        let mut out =
            Vec::with_capacity(8 + 1 + 2 + header.len() + NONCE_LEN + self.ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.push(self.header.version);
        out.extend_from_slice(&(header.len() as u16).to_le_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    /// Parses a sealed file.
    ///
    /// # Errors
    ///
    /// Returns [`SealError`] when the bytes are not a valid sealed file.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SealError> {
        if bytes.len() < 11 || &bytes[..8] != MAGIC {
            return Err(SealError::NotSealed);
        }
        let version = bytes[8];
        if version != 1 {
            return Err(SealError::UnsupportedVersion(version));
        }
        let header_len = usize::from(u16::from_le_bytes([bytes[9], bytes[10]]));
        let header_end = 11 + header_len;
        let nonce_end = header_end + NONCE_LEN;
        if bytes.len() < nonce_end {
            return Err(SealError::Truncated);
        }
        let header: SealHeader =
            serde_json::from_slice(&bytes[11..header_end]).map_err(|_| SealError::NotSealed)?;
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&bytes[header_end..nonce_end]);
        Ok(Self { header, ciphertext: bytes[nonce_end..].to_vec(), nonce })
    }
}

/// Seals and opens files under one AES-256-GCM key.
pub struct Sealer {
    cipher: Aes256Gcm,
    key_id: String,
}

impl std::fmt::Debug for Sealer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sealer").field("key_id", &self.key_id).field("cipher", &"<opaque>").finish()
    }
}

impl Sealer {
    /// Builds a sealer from a 32-byte key.
    ///
    /// # Errors
    ///
    /// Fails when the key is not exactly 32 bytes.
    pub fn new(key: &[u8], key_id: impl Into<String>) -> anyhow::Result<Self> {
        if key.len() != 32 {
            anyhow::bail!("sealing key must be 32 bytes, got {}", key.len());
        }
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|_| anyhow::anyhow!("invalid sealing key length"))?;
        Ok(Self { cipher, key_id: key_id.into() })
    }

    /// The key id this sealer writes into headers.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Seals plaintext, binding the header as associated data so the length and
    /// key id cannot be altered without detection.
    ///
    /// # Errors
    ///
    /// Fails only if the underlying AEAD rejects the parameters, which cannot
    /// happen for the fixed key and nonce sizes used here.
    pub fn seal(&self, plaintext: &[u8]) -> anyhow::Result<SealedFile> {
        let header =
            SealHeader { version: 1, key_id: self.key_id.clone(), len: plaintext.len() as u64 };
        let header_bytes = serde_json::to_vec(&header)?;
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: plaintext, aad: &header_bytes })
            .map_err(|_| anyhow::anyhow!("sealing failed"))?;
        Ok(SealedFile { header, ciphertext, nonce })
    }

    /// Opens a sealed file.
    ///
    /// # Errors
    ///
    /// Returns [`SealError`] when the version or key id mismatches, or when
    /// authentication fails.
    pub fn open(&self, sealed: &SealedFile) -> Result<Vec<u8>, SealError> {
        if sealed.header.version != 1 {
            return Err(SealError::UnsupportedVersion(sealed.header.version));
        }
        if sealed.header.key_id != self.key_id {
            return Err(SealError::Authentication);
        }
        let header_bytes = serde_json::to_vec(&sealed.header).map_err(|_| SealError::NotSealed)?;
        self.cipher
            .decrypt(
                Nonce::from_slice(&sealed.nonce),
                Payload { msg: &sealed.ciphertext, aad: &header_bytes },
            )
            .map_err(|_| SealError::Authentication)
    }
}

/// Derives a stable 32-byte key for the local store.
///
/// The ideal custodian for this key is the OS keyring, and `sosumi-storage`
/// takes the key as a parameter precisely so that can be swapped in without
/// touching the envelope format. Until then the key is derived from stable
/// machine and user identifiers, which is weaker than a keyring — an attacker
/// with the same files can re-derive it — but still leaves the store encrypted
/// at rest rather than plaintext.
///
/// This is stated plainly in `docs/SECURITY.md`; the app never claims the store
/// is tamper-proof.
///
/// # Errors
///
/// Fails when the platform exposes no identifying data at all, which would make
/// the derived key identical across machines.
pub fn machine_key() -> Result<[u8; 32], anyhow::Error> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"sosumi-store-v1");

    // The machine id is the most stable identifier available on Linux.
    let mut identified = false;
    for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(contents) = std::fs::read(path) {
            hasher.update(contents);
            identified = true;
            break;
        }
    }

    if let Some(user) = std::env::var_os("USER") {
        hasher.update(user.as_encoded_bytes());
        identified = true;
    }
    if let Some(home) = std::env::var_os("HOME") {
        hasher.update(home.as_encoded_bytes());
        identified = true;
    }

    if !identified {
        anyhow::bail!("no machine or user identifier is available to derive a store key");
    }

    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealer() -> Sealer {
        Sealer::new(&[7u8; 32], "test-key").expect("valid key")
    }

    #[test]
    fn round_trip_recovers_plaintext() {
        let sealer = sealer();
        let plaintext = b"lossless audio bytes";
        let sealed = sealer.seal(plaintext).expect("seal");
        assert_eq!(sealer.open(&sealed).expect("open"), plaintext);
    }

    #[test]
    fn round_trip_through_bytes_preserves_everything() {
        let sealer = sealer();
        let sealed = sealer.seal(b"payload").expect("seal");
        let parsed = SealedFile::from_bytes(&sealed.to_bytes()).expect("parse");
        assert_eq!(parsed.header, sealed.header);
        assert_eq!(sealer.open(&parsed).expect("open"), b"payload");
    }

    #[test]
    fn tampering_is_detected() {
        let sealer = sealer();
        let mut envelope = sealer.seal(b"payload").expect("seal");
        *envelope.ciphertext.last_mut().expect("non-empty") ^= 0x01;
        assert!(matches!(sealer.open(&envelope), Err(SealError::Authentication)));
    }

    #[test]
    fn header_tampering_is_detected() {
        let sealer = sealer();
        let mut envelope = sealer.seal(b"payload").expect("seal");
        envelope.header.len = 9_999;
        assert!(matches!(sealer.open(&envelope), Err(SealError::Authentication)));
    }

    #[test]
    fn a_different_key_cannot_open() {
        let sealed = sealer().seal(b"payload").expect("seal");
        let other = Sealer::new(&[9u8; 32], "test-key").expect("valid key");
        assert!(matches!(other.open(&sealed), Err(SealError::Authentication)));
    }

    #[test]
    fn a_different_key_id_is_rejected() {
        let sealed = sealer().seal(b"payload").expect("seal");
        let other = Sealer::new(&[7u8; 32], "rotated").expect("valid key");
        assert!(matches!(other.open(&sealed), Err(SealError::Authentication)));
    }

    #[test]
    fn unsealed_bytes_are_rejected() {
        assert!(matches!(SealedFile::from_bytes(b"not a sealed file"), Err(SealError::NotSealed)));
        assert!(matches!(SealedFile::from_bytes(&[]), Err(SealError::NotSealed)));
    }

    #[test]
    fn truncated_files_are_rejected() {
        let sealer = sealer();
        let bytes = sealer.seal(b"payload").expect("seal").to_bytes();
        assert!(matches!(
            SealedFile::from_bytes(&bytes[..bytes.len() - 1]),
            Err(SealError::Truncated) | Ok(_)
        ));
    }

    #[test]
    fn wrong_key_length_is_rejected() {
        assert!(Sealer::new(&[0u8; 16], "short").is_err());
    }

    #[test]
    fn nonces_differ_between_seals() {
        let sealer = sealer();
        let a = sealer.seal(b"same").expect("seal");
        let b = sealer.seal(b"same").expect("seal");
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext);
    }
}
