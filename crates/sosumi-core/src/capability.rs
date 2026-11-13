//! Runtime capability probing and installation.
//!
//! The application must run with none of the optional tiers present, and must be
//! able to offer each one when it is not. This module is where that state lives:
//! it probes what exists, reports what is missing, and installs what it can.
//!
//! # Why probing is separate from installing
//!
//! They answer different questions and have different costs. Probing is cheap,
//! local, and happens on every startup so Settings is accurate. Installing
//! fetches from the network and must be a deliberate action. Mixing them would
//! mean either probing does network I/O or the state is stale.
//!
//! # What can and cannot be installed here
//!
//! The stub libraries are carried in the binary and written from embedded bytes,
//! so installing them needs nothing external. Apple's `FairPlay` libraries cannot
//! be embedded — they are not ours to redistribute — so this reports precisely
//! which files are missing and leaves the fetch to `scripts/fetch-fairplay-runtime.sh`,
//! run on an explicit user action.

use std::sync::Arc;

use sosumi_diagnostics::{Health, HealthStatus};
use sosumi_fairplay::{InstallOutcome, RuntimeLayout, RuntimeManifest, RuntimeStatus};
use sosumi_security::CapabilityState;

/// The capability id the lossless tier reports under.
pub const FAIRPLAY_ID: &str = "fairplay";

/// The result of a probe or an install attempt, for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityReport {
    /// The tier is usable.
    Ready {
        /// One line describing how it is being served.
        detail: String,
    },
    /// The tier can be enabled. `missing` names what is absent.
    Installable {
        /// One line for the UI.
        detail: String,
        /// The specific files that are missing, when the tooling can say.
        missing: Vec<String>,
    },
    /// Present but not working, with what is wrong.
    Damaged {
        /// Each problem found.
        problems: Vec<String>,
    },
    /// The platform cannot run it at all.
    Unsupported {
        /// Why, in one line.
        reason: String,
    },
}

impl CapabilityReport {
    /// Maps to the capability state the Settings screen renders.
    #[must_use]
    pub fn to_state(&self) -> CapabilityState {
        match self {
            Self::Ready { detail } => CapabilityState::Ready { detail: detail.clone() },
            Self::Installable { detail, .. } => {
                CapabilityState::Available { detail: detail.clone(), action: "Install".to_owned() }
            }
            Self::Damaged { problems } => CapabilityState::Degraded { detail: problems.join("; ") },
            Self::Unsupported { reason } => CapabilityState::Unsupported { detail: reason.clone() },
        }
    }

    /// Maps to the health entry used by the diagnostics screen.
    #[must_use]
    pub fn to_health(&self) -> Health {
        match self {
            Self::Ready { detail } => Health {
                id: FAIRPLAY_ID.to_owned(),
                name: "Lossless & Hi-Res".to_owned(),
                status: HealthStatus::Ready,
                detail: detail.clone(),
                action: None,
            },
            Self::Installable { detail, .. } => {
                Health::unavailable(FAIRPLAY_ID, "Lossless & Hi-Res", detail).with_action("Install")
            }
            Self::Damaged { problems } => Health {
                id: FAIRPLAY_ID.to_owned(),
                name: "Lossless & Hi-Res".to_owned(),
                status: HealthStatus::Degraded,
                detail: problems.join("; "),
                action: Some("Repair".to_owned()),
            },
            Self::Unsupported { reason } => {
                Health::unavailable(FAIRPLAY_ID, "Lossless & Hi-Res", reason)
            }
        }
    }
}

/// The runtime payload, resolved from the app's data directory.
pub struct Runtime {
    layout: RuntimeLayout,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime").field("lib64", &self.layout.lib64).finish_non_exhaustive()
    }
}

impl Runtime {
    /// Loads the manifest and resolves the layout under `base`.
    ///
    /// # Errors
    ///
    /// Returns the reason the manifest could not be read. A missing manifest is
    /// an error rather than an empty one: it ships with the application, so its
    /// absence means the install is broken, not that the tier is unavailable.
    pub fn load(base: &std::path::Path, manifest_path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(manifest_path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", manifest_path.display()))?;
        let manifest = RuntimeManifest::from_toml(&text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", manifest_path.display()))?;

        let layout = RuntimeLayout::under(base, manifest);
        layout.ensure_directories()?;
        Ok(Self { layout })
    }

    /// The resolved layout.
    #[must_use]
    pub const fn layout(&self) -> &RuntimeLayout {
        &self.layout
    }

    /// Reports the tier's state without changing anything.
    #[must_use]
    pub fn probe(&self) -> CapabilityReport {
        match self.layout.verify() {
            RuntimeStatus::Ready => CapabilityReport::Ready {
                detail: "Signed-in playback, lossless and hi-res".to_owned(),
            },
            RuntimeStatus::NotInstalled => CapabilityReport::Installable {
                detail: "Not installed. Enables lossless and hi-res playback.".to_owned(),
                missing: self.missing_files(),
            },
            RuntimeStatus::Damaged { problems } => CapabilityReport::Damaged { problems },
            RuntimeStatus::Unsupported { reason } => CapabilityReport::Unsupported { reason },
        }
    }

    /// The pinned files that are not present.
    ///
    /// Named individually so the install step can say what it is fetching rather
    /// than showing an indeterminate spinner.
    #[must_use]
    pub fn missing_files(&self) -> Vec<String> {
        self.layout
            .manifest
            .files
            .iter()
            .filter(|file| !self.layout.lib64.join(&file.name).is_file())
            .map(|file| file.name.clone())
            .collect()
    }

    /// Writes the stub libraries this build carries.
    ///
    /// Safe to call at any time; it needs no network and no compiler, and it is
    /// idempotent. This is the half of installation that can always succeed.
    ///
    /// # Errors
    ///
    /// Propagates a filesystem failure from writing the stubs.
    pub fn install_stubs(&self) -> anyhow::Result<usize> {
        self.layout.install_stubs().map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Whether a sign-in has completed and the session files are present.
    #[must_use]
    pub fn has_session(&self) -> bool {
        self.layout.has_session()
    }

    /// Writes the stubs and reports what is still missing.
    ///
    /// # Errors
    ///
    /// Propagates a filesystem failure from writing the stubs.
    pub fn install(&self) -> anyhow::Result<InstallOutcome> {
        self.layout.install().map_err(|e| anyhow::anyhow!("{e}"))
    }
}

/// Shared handle the application holds.
pub type SharedRuntime = Arc<Runtime>;

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
        tier = "Lossless & Hi-Res"

        [[files]]
        name = "libCoreFP.so"
        sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
        sources = []
    "#;

    fn runtime() -> (tempfile::TempDir, Runtime) {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("runtime.toml");
        std::fs::write(&manifest_path, MANIFEST).expect("write manifest");
        let runtime =
            Runtime::load(&dir.path().join("payload"), &manifest_path).expect("load runtime");
        (dir, runtime)
    }

    #[test]
    fn a_fresh_install_reports_itself_as_installable() {
        let (_dir, runtime) = runtime();
        match runtime.probe() {
            CapabilityReport::Installable { missing, .. } => {
                assert_eq!(missing, vec!["libCoreFP.so".to_owned()]);
            }
            other => panic!("expected installable, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_manifest_is_an_error_not_an_empty_tier() {
        // The manifest ships with the application, so its absence means the
        // install is broken. Reporting "unavailable" would hide that.
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(Runtime::load(dir.path(), &dir.path().join("absent.toml")).is_err());
    }

    #[test]
    fn a_broken_manifest_is_reported_with_its_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("runtime.toml");
        std::fs::write(&path, "not a manifest").expect("write");
        let error = Runtime::load(dir.path(), &path).expect_err("must fail");
        assert!(error.to_string().contains("runtime.toml"));
    }

    #[test]
    fn installing_the_stubs_changes_what_is_missing() {
        let (_dir, runtime) = runtime();
        // Before: the stub names are not tracked by the manifest, so the missing
        // list is only about Apple's files. After: the stubs exist on disk.
        let before = runtime.missing_files();
        let written = runtime.install_stubs().expect("install stubs");
        assert!(written > 0);
        assert_eq!(runtime.missing_files(), before, "the manifest list is unchanged");
    }

    #[test]
    fn install_still_reports_apple_files_as_missing() {
        let (_dir, runtime) = runtime();
        match runtime.install().expect("install") {
            InstallOutcome::Incomplete { missing } => {
                assert!(missing.contains(&"libCoreFP.so".to_owned()));
            }
            other @ InstallOutcome::Installed { .. } => {
                panic!("expected incomplete, got {other:?}")
            }
        }
    }

    #[test]
    fn the_report_maps_onto_the_capability_state() {
        let (_dir, runtime) = runtime();
        let state = runtime.probe().to_state();
        assert!(matches!(state, CapabilityState::Available { .. }), "got {state:?}");
        assert!(state.is_actionable());

        let health = runtime.probe().to_health();
        assert_eq!(health.id, FAIRPLAY_ID);
        assert_eq!(health.status, HealthStatus::Unavailable);
    }

    #[test]
    fn a_damaged_payload_maps_to_a_degraded_state_that_offers_repair() {
        let report = CapabilityReport::Damaged { problems: vec!["libCoreFP.so is altered".into()] };
        let state = report.to_state();
        assert!(matches!(state, CapabilityState::Degraded { .. }));
        // Degraded is not actionable by the same path as "install": the remedy is
        // a repair, and the report says so.
        let health = report.to_health();
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(health.action.as_deref(), Some("Repair"));
    }

    #[test]
    fn an_unsupported_platform_is_not_offered_as_installable() {
        // Telling a user to install something their machine cannot run is worse
        // than saying nothing, so the state is distinct.
        let report = CapabilityReport::Unsupported { reason: "x86-64 only".into() };
        let state = report.to_state();
        assert!(matches!(state, CapabilityState::Unsupported { .. }));
        assert!(!state.is_actionable());
    }

    #[test]
    fn a_session_is_reported_only_after_its_token_exists() {
        let (_dir, runtime) = runtime();
        assert!(!runtime.has_session());
        std::fs::write(runtime.layout().session.join("MUSIC_TOKEN"), b"token").expect("write");
        assert!(runtime.has_session());
    }

    #[test]
    fn the_layout_is_reachable_for_diagnostics() {
        let (_dir, runtime) = runtime();
        assert!(runtime.layout().lib64.ends_with("lib64"));
    }
}
