//! Where the optional runtime payload lives, and whether it is trustworthy.
//!
//! Apple's libraries are not in this repository and are never distributed with
//! it. When a user chooses to enable the lossless tier, the app fetches them to
//! a directory of its own, and every file is checked against a pinned SHA-256
//! before it is used.
//!
//! Three properties are deliberate:
//!
//! 1. **The payload is not in the repository.** A clone of Sosumi contains no
//!    Apple code, so the licence position is clear and a contributor never
//!    accidentally commits a binary.
//! 2. **A file that fails its checksum is refused, not warned about.** The
//!    loader would otherwise `dlopen` whatever happens to be on disk, and the
//!    code that runs has full access to the process and to the user's session.
//! 3. **Absence is a normal state, not an error.** The app runs without any of
//!    this; the capability reports itself as available-to-install.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A file the runtime needs, with the hash it must have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedFile {
    /// File name, relative to the payload directory.
    pub name: String,
    /// Expected SHA-256, lower-case hex.
    pub sha256: String,
    /// Where a fetch may obtain it. Pinned to a commit, never a branch.
    pub sources: Vec<String>,
}

/// The payload a machine needs before the lossless tier can work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeManifest {
    /// Human-readable tier this enables, for the capability screen.
    pub tier: String,
    /// The files, in no particular order.
    pub files: Vec<PinnedFile>,
}

impl RuntimeManifest {
    /// Parses a manifest.
    ///
    /// # Errors
    ///
    /// Fails when the document is not a manifest.
    pub fn from_toml(text: &str) -> Result<Self, RuntimeError> {
        toml::from_str(text).map_err(|e| RuntimeError::BadManifest(e.to_string()))
    }
}

/// What state the payload is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStatus {
    /// Every pinned file is present and matches its hash.
    Ready,
    /// Nothing is installed yet. The user can install it.
    NotInstalled,
    /// Some files are present but at least one is missing or altered.
    ///
    /// Kept distinct from `NotInstalled` because the remedy differs: a partial
    /// payload needs repair, and telling the user to "install" something already
    /// half-present leads to a confusing loop.
    Damaged {
        /// The files that are missing or do not match, and why.
        problems: Vec<String>,
    },
    /// The platform cannot run the payload at all.
    Unsupported {
        /// Why, in one line.
        reason: String,
    },
}

impl RuntimeStatus {
    /// True only when the payload is complete and verified.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// The resolved on-disk layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLayout {
    /// Directory holding Apple's libraries, equivalent to Android's `system/lib64`.
    pub lib64: PathBuf,
    /// Directory holding the session store: `mpl_db`, `MUSIC_TOKEN`, `STOREFRONT_ID`.
    pub session: PathBuf,
    /// The manifest that describes what belongs in `lib64`.
    pub manifest: RuntimeManifest,
}

impl RuntimeLayout {
    /// Builds a layout under `base`.
    ///
    /// Note there is no `linker_dir`: `libhybris` is *embedded* and loaded from an
    /// anonymous memfd, so nothing is extracted to disk and the host's dynamic
    /// loader is never asked to search for it.
    #[must_use]
    pub fn under(base: impl Into<PathBuf>, manifest: RuntimeManifest) -> Self {
        let base = base.into();
        Self { lib64: base.join("lib64"), session: base.join("session"), manifest }
    }

    /// The name the embedded blob is registered under.
    ///
    /// Not a path: the loader writes the embedded image to a memfd and opens it
    /// through `/proc/self/fd/N`. The string is only used for log messages and to
    /// name the memfd.
    pub const EMBEDDED_CORE_NAME: &'static str = "libhybris-core.so";

    /// Verifies every pinned file, returning what is wrong.
    ///
    /// # Errors
    ///
    /// Never fails: an unreadable file is reported as a problem rather than
    /// raised, because "cannot read" and "hash mismatch" need the same remedy.
    #[must_use]
    pub fn verify(&self) -> RuntimeStatus {
        // A manifest with no files describes nothing, so it cannot be satisfied.
        // Without this check the loop below finds no problems and the payload
        // reads as `Ready`, which would let the loader run against an empty
        // directory — the exact case the verification exists to prevent.
        if self.manifest.files.is_empty() {
            return RuntimeStatus::NotInstalled;
        }

        let mut problems = Vec::new();

        if !cfg!(target_arch = "x86_64") {
            // Apple's Android libraries are x86-64; on another architecture the
            // payload cannot load however complete it is.
            return RuntimeStatus::Unsupported {
                reason: format!(
                    "Apple's libraries are x86-64 and this machine is {}",
                    std::env::consts::ARCH
                ),
            };
        }

        for file in &self.manifest.files {
            let path = self.lib64.join(&file.name);
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let actual = sha256_hex(&bytes);
                    if !actual.eq_ignore_ascii_case(&file.sha256) {
                        problems.push(format!("{} does not match its pinned checksum", file.name));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    problems.push(format!("{} is missing", file.name));
                }
                Err(error) => {
                    problems.push(format!("{} could not be read: {error}", file.name));
                }
            }
        }

        if problems.is_empty() {
            RuntimeStatus::Ready
        } else if self.manifest.files.iter().all(|file| !self.lib64.join(&file.name).exists()) {
            // Nothing present at all is the first-run state. This counts only
            // *missing* files: a file that exists but fails its checksum is
            // damage, not absence, and reporting it as "not installed" would tell
            // the user to install something that is already there.
            RuntimeStatus::NotInstalled
        } else {
            RuntimeStatus::Damaged { problems }
        }
    }

    /// True when the session directory holds a completed sign-in.
    ///
    /// The C library writes `MUSIC_TOKEN` last, so its presence is what marks a
    /// session as usable rather than partially written.
    #[must_use]
    pub fn has_session(&self) -> bool {
        self.session.join("MUSIC_TOKEN").is_file()
    }

    /// Creates the directories a first run needs.
    ///
    /// # Errors
    ///
    /// Propagates filesystem errors.
    pub fn ensure_directories(&self) -> Result<(), RuntimeError> {
        for dir in [&self.lib64, &self.session] {
            std::fs::create_dir_all(dir)
                .map_err(|e| RuntimeError::Io(format!("{}: {e}", dir.display())))?;
        }
        Ok(())
    }
}

/// Errors from the runtime payload.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// The manifest could not be parsed.
    #[error("the runtime manifest is invalid: {0}")]
    BadManifest(String),
    /// A filesystem operation failed.
    #[error("{0}")]
    Io(String),
}

/// Lower-case hex SHA-256 of a byte slice.
///
/// Uses the `sha2` implementation already present for the sealing key, so there
/// is one hashing implementation in the tree rather than two.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Installs the runtime payload: the stubs this build carries, and optionally
/// the files a fetch has already retrieved.
///
/// The split matters for user experience. The stubs are ours and are written
/// from embedded bytes, so a user never needs a compiler. Apple's libraries
/// cannot be embedded, so they must be fetched — which is why this takes a
/// directory the caller has already populated rather than doing network I/O
/// itself. Keeping the network out of this crate means the install path is
/// testable without a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    /// Everything is in place.
    Installed {
        /// Stubs written from this build.
        stubs: usize,
        /// Files the caller had already fetched.
        provided: usize,
    },
    /// The payload is not complete, with what is missing.
    Incomplete {
        /// Each missing file, by name.
        missing: Vec<String>,
    },
}

/// The stub libraries, carried inside the binary.
///
/// `include_bytes!` from the build script's output, so they cannot drift from the
/// sources that produce them.
mod stubs {
    include!(concat!(env!("OUT_DIR"), "/stubs.rs"));
}

/// The file names of the stub libraries.
pub const STUB_FILES: [&str; 3] = ["libandroid.so", "libOpenSLES.so", "libdl.so"];

impl RuntimeLayout {
    /// Writes the stub libraries and reports what is still missing.
    ///
    /// Idempotent: a stub that is already present with the right bytes is left
    /// alone, so a repair run does not rewrite files needlessly.
    ///
    /// # Errors
    ///
    /// Propagates filesystem errors from writing the stubs.
    pub fn install_stubs(&self) -> Result<usize, RuntimeError> {
        self.ensure_directories()?;

        let carried: [(&str, &[u8]); 3] = [
            (STUB_FILES[0], stubs::LIBANDROID_SO),
            (STUB_FILES[1], stubs::LIBOPENSLES_SO),
            (STUB_FILES[2], stubs::LIBDL_SO),
        ];

        let mut written = 0;
        for (name, bytes) in carried {
            let path = self.lib64.join(name);
            // Compare before writing, so a re-run is cheap and the mtime of an
            // unchanged file is not disturbed.
            // A missing or unreadable file is simply "needs writing".
            if std::fs::read(&path).is_ok_and(|existing| existing == bytes) {
                continue;
            }
            std::fs::write(&path, bytes)
                .map_err(|e| RuntimeError::Io(format!("{}: {e}", path.display())))?;
            written += 1;
        }

        tracing::info!(target: "sosumi::fairplay", written, "stub libraries installed");
        Ok(written)
    }

    /// Reports what the payload is still missing after the stubs are written.
    ///
    /// # Errors
    ///
    /// Propagates filesystem errors from writing the stubs.
    pub fn install(&self) -> Result<InstallOutcome, RuntimeError> {
        let stubs = self.install_stubs()?;

        let missing: Vec<String> = self
            .manifest
            .files
            .iter()
            .filter(|file| !self.lib64.join(&file.name).is_file())
            .map(|file| file.name.clone())
            .collect();

        if missing.is_empty() {
            let provided = self.manifest.files.len();
            Ok(InstallOutcome::Installed { stubs, provided })
        } else {
            Ok(InstallOutcome::Incomplete { missing })
        }
    }
}

#[cfg(test)]
// Tests may panic; the workspace lint policy allows it under `cfg(test)` by the
// same convention every other crate in this workspace uses.
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn manifest_for(files: &[(&str, &str)]) -> RuntimeManifest {
        RuntimeManifest {
            tier: "Lossless & Hi-Res".to_owned(),
            files: files
                .iter()
                .map(|(name, sha)| PinnedFile {
                    name: (*name).to_owned(),
                    sha256: (*sha).to_owned(),
                    sources: Vec::new(),
                })
                .collect(),
        }
    }

    fn layout_with(files: &[(&str, &str)]) -> (tempfile::TempDir, RuntimeLayout) {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = RuntimeLayout::under(dir.path(), manifest_for(files));
        layout.ensure_directories().expect("directories");
        (dir, layout)
    }

    #[test]
    fn an_empty_payload_reads_as_not_installed_rather_than_damaged() {
        let (_dir, layout) = layout_with(&[("libCoreFP.so", "00"), ("libc.so", "11")]);
        // Nothing written at all is the first-run state.
        assert_eq!(layout.verify(), RuntimeStatus::NotInstalled);
    }

    #[test]
    fn an_empty_manifest_can_never_be_ready() {
        // The regression this guards: a manifest with no files made the loop in
        // `verify` find no problems, so the payload read as `Ready` and the
        // loader was allowed to run against an empty directory.
        let (_dir, layout) = layout_with(&[]);
        assert_eq!(layout.verify(), RuntimeStatus::NotInstalled);
        assert!(!layout.verify().is_ready());
    }

    #[test]
    fn installing_writes_the_stubs_from_this_build() {
        // The stubs are ours, so a user needs no compiler and no download to get
        // them. This is what makes the install path work on a fresh machine.
        let (_dir, layout) = layout_with(&[("libCoreFP.so", "00")]);
        let written = layout.install_stubs().expect("install");
        assert_eq!(written, STUB_FILES.len(), "every stub is written on a first run");

        for name in STUB_FILES {
            let path = layout.lib64.join(name);
            assert!(path.is_file(), "{name} must exist after install");
            let bytes = std::fs::read(&path).expect("read");
            // An ELF shared object, not an empty file: the loader would reject a
            // zero-byte stand-in at load time rather than here.
            assert_eq!(&bytes[..4], b"\x7fELF", "{name} must be a shared object");
            assert!(bytes.len() > 1024, "{name} is implausibly small");
        }
    }

    #[test]
    fn installing_twice_writes_nothing_the_second_time() {
        let (_dir, layout) = layout_with(&[]);
        let first = layout.install_stubs().expect("first install");
        assert!(first > 0);
        // Idempotent, so a repair run does not rewrite identical bytes and
        // disturb the mtime of files nothing changed.
        let second = layout.install_stubs().expect("second install");
        assert_eq!(second, 0);
    }

    #[test]
    fn an_incomplete_install_names_what_is_missing() {
        let (_dir, layout) = layout_with(&[("libCoreFP.so", "00"), ("libc.so", "11")]);
        match layout.install().expect("install") {
            InstallOutcome::Incomplete { missing } => {
                assert_eq!(missing.len(), 2);
                assert!(missing.contains(&"libCoreFP.so".to_owned()));
            }
            other @ InstallOutcome::Installed { .. } => {
                panic!("expected an incomplete install, got {other:?}")
            }
        }
    }

    #[test]
    fn a_complete_install_succeeds() {
        let content = b"library bytes";
        let hash = sha256_hex(content);
        let (_dir, layout) = layout_with(&[("libCoreFP.so", &hash)]);
        std::fs::write(layout.lib64.join("libCoreFP.so"), content).expect("write");

        match layout.install().expect("install") {
            InstallOutcome::Installed { stubs, provided } => {
                assert_eq!(provided, 1);
                assert_eq!(stubs, STUB_FILES.len());
            }
            other @ InstallOutcome::Incomplete { .. } => {
                panic!("expected a complete install, got {other:?}")
            }
        }
        // And the payload now verifies, which is the property that matters.
        assert!(layout.verify().is_ready(), "a complete install must verify");
    }

    #[test]
    fn every_stub_name_is_a_plain_file_name() {
        // The names are joined onto a directory, so a path separator or a parent
        // reference here would write outside the payload directory.
        for name in STUB_FILES {
            assert!(!name.contains('/'), "{name} must not contain a path separator");
            assert_ne!(name, "..");
            assert_eq!(name.rsplit_once('.').map(|(_, ext)| ext), Some("so"));
        }
    }

    #[test]
    fn a_matching_payload_reads_as_ready() {
        let content = b"pretend this is a library";
        let hash = sha256_hex(content);
        let (dir, layout) = layout_with(&[("libCoreFP.so", &hash)]);
        std::fs::write(layout.lib64.join("libCoreFP.so"), content).expect("write");
        let _ = dir;

        assert_eq!(layout.verify(), RuntimeStatus::Ready);
        assert!(layout.verify().is_ready());
    }

    #[test]
    fn an_altered_file_is_refused_rather_than_warned_about() {
        let (dir, layout) = layout_with(&[("libCoreFP.so", "deadbeef")]);
        std::fs::write(layout.lib64.join("libCoreFP.so"), b"tampered").expect("write");
        let _ = dir;

        match layout.verify() {
            RuntimeStatus::Damaged { problems } => {
                assert_eq!(problems.len(), 1);
                assert!(problems[0].contains("checksum"));
            }
            other => panic!("expected damage, got {other:?}"),
        }
    }

    #[test]
    fn a_partial_payload_is_damaged_not_absent() {
        let good = sha256_hex(b"present");
        let (dir, layout) = layout_with(&[("a.so", &good), ("b.so", "beef")]);
        std::fs::write(layout.lib64.join("a.so"), b"present").expect("write");
        let _ = dir;

        // One present and one absent is repairable damage, and the remedy differs
        // from a fresh install, so the states are kept apart.
        assert!(matches!(layout.verify(), RuntimeStatus::Damaged { .. }));
    }

    #[test]
    fn verification_accepts_an_upper_case_manifest_hash() {
        let content = b"library";
        let hash = sha256_hex(content).to_uppercase();
        let (dir, layout) = layout_with(&[("a.so", &hash)]);
        std::fs::write(layout.lib64.join("a.so"), content).expect("write");
        let _ = dir;
        assert!(layout.verify().is_ready());
    }

    #[test]
    fn a_session_is_only_recognised_once_its_token_exists() {
        let (_dir, layout) = layout_with(&[]);
        assert!(!layout.has_session());

        std::fs::write(layout.session.join("mpl_db"), b"partial").expect("write");
        assert!(!layout.has_session(), "an unfinished sign-in is not a session");

        std::fs::write(layout.session.join("MUSIC_TOKEN"), b"token").expect("write");
        assert!(layout.has_session());
    }

    #[test]
    fn a_manifest_parses_from_toml() {
        let text = r#"
            tier = "Lossless & Hi-Res"
            [[files]]
            name = "libCoreFP.so"
            sha256 = "abc123"
            sources = ["https://example.invalid/libCoreFP.so"]
        "#;
        let manifest = RuntimeManifest::from_toml(text).expect("parse");
        assert_eq!(manifest.tier, "Lossless & Hi-Res");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].sources.len(), 1);
    }

    #[test]
    fn a_broken_manifest_is_reported() {
        assert!(RuntimeManifest::from_toml("this is not toml").is_err());
    }

    #[test]
    fn the_embedded_core_is_named_not_pathed() {
        // It is loaded from a memfd, so the constant must not look like a path.
        assert!(!RuntimeLayout::EMBEDDED_CORE_NAME.contains('/'));
        assert_eq!(
            RuntimeLayout::EMBEDDED_CORE_NAME.rsplit_once('.').map(|(_, ext)| ext),
            Some("so")
        );
    }
}
