//! The encrypted secret store.
//!
//! Small values — tokens, session cookies — live in one JSON file whose payload
//! is sealed with [`sosumi_security::Sealer`]. The sealing key itself is held
//! by the OS keyring via the `keyring` crate; this module only ever handles the
//! sealed envelope, never the key.
//!
//! If the keyring is unavailable (headless CI, no Secret Service), the caller
//! may pass an explicit key. Sosumi never falls back to writing plaintext.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sosumi_security::sealing::SealedFile;
use sosumi_security::Sealer;

/// Errors from the secret store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The store file exists but could not be read or parsed.
    #[error("secret store is unreadable: {0}")]
    Unreadable(String),
    /// The store could not be decrypted — likely a different user or a lost key.
    #[error("secret store could not be decrypted")]
    Undecryptable,
    /// Writing failed.
    #[error("could not write secret store: {0}")]
    Io(#[from] std::io::Error),
}

/// A key/value store persisted as a single sealed JSON blob.
#[derive(Debug)]
pub struct SecretStore {
    path: PathBuf,
    sealer: Sealer,
    entries: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    entries: BTreeMap<String, String>,
}

impl SecretStore {
    /// Opens the store at `path`, decrypting it with `sealer`.
    ///
    /// A missing file yields an empty store, which is the first-run case.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the file exists but cannot be decrypted.
    pub fn open(path: impl Into<PathBuf>, sealer: Sealer) -> Result<Self, StoreError> {
        let path = path.into();
        let entries = match std::fs::read(&path) {
            Ok(bytes) => {
                let sealed =
                    SealedFile::from_bytes(&bytes).map_err(|_| StoreError::Undecryptable)?;
                let plain = sealer.open(&sealed).map_err(|_| StoreError::Undecryptable)?;
                let payload: Payload = serde_json::from_slice(&plain)
                    .map_err(|e| StoreError::Unreadable(e.to_string()))?;
                payload.entries
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(err) => return Err(StoreError::Io(err)),
        };
        Ok(Self { path, sealer, entries })
    }

    /// Returns a stored value.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Stores a value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Removes a value, returning it if present.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.remove(key)
    }

    /// True when the store holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The keys currently stored. Values are never exposed in bulk.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Persists the store, writing atomically so a crash cannot corrupt it.
    ///
    /// # Errors
    ///
    /// Propagates sealing and filesystem errors.
    pub fn save(&self) -> Result<(), StoreError> {
        let payload = Payload { entries: self.entries.clone() };
        let plain =
            serde_json::to_vec(&payload).map_err(|e| StoreError::Unreadable(e.to_string()))?;
        let sealed = self.sealer.seal(&plain).map_err(|e| StoreError::Unreadable(e.to_string()))?;
        write_atomic(&self.path, &sealed.to_bytes())?;
        Ok(())
    }

    /// The file backing this store.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Writes `bytes` to `path` via a sibling temp file and a rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let tmp = parent
        .join(format!(".{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("store")));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealer(id: &str) -> Sealer {
        Sealer::new(&[3u8; 32], id).expect("valid key")
    }

    #[test]
    fn missing_file_yields_empty_store() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = SecretStore::open(dir.path().join("secrets.json"), sealer("k")).expect("open");
        assert!(store.is_empty());
    }

    #[test]
    fn values_round_trip_through_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("secrets.json");

        let mut store = SecretStore::open(&path, sealer("k")).expect("open");
        store.set("music_user_token", "secret-value");
        store.save().expect("save");

        let reopened = SecretStore::open(&path, sealer("k")).expect("reopen");
        assert_eq!(reopened.get("music_user_token"), Some("secret-value"));
    }

    #[test]
    fn plaintext_never_hits_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("secrets.json");
        let mut store = SecretStore::open(&path, sealer("k")).expect("open");
        store.set("token", "PLEASE_DO_NOT_LEAK");
        store.save().expect("save");

        let raw = std::fs::read(&path).expect("read");
        let haystack = String::from_utf8_lossy(&raw);
        assert!(!haystack.contains("PLEASE_DO_NOT_LEAK"));
    }

    #[test]
    fn a_different_key_cannot_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("secrets.json");
        let mut store = SecretStore::open(&path, sealer("k")).expect("open");
        store.set("x", "y");
        store.save().expect("save");

        assert!(matches!(
            SecretStore::open(&path, sealer("other")),
            Err(StoreError::Undecryptable)
        ));
    }

    #[test]
    fn remove_reports_the_previous_value() {
        let mut store =
            SecretStore::open(tempfile::tempdir().expect("t").path().join("s.json"), sealer("k"))
                .expect("open");
        store.set("a", "1");
        assert_eq!(store.remove("a").as_deref(), Some("1"));
        assert_eq!(store.remove("a"), None);
        assert!(store.is_empty());
    }
}
