//! Health and capability reporting.
//!
//! The UI shows a single capabilities screen. Every optional subsystem
//! (`FairPlay`, Spatial Audio, Discord, MPRIS) reports through this type, so the
//! app can build and run with none of them present.

use serde::{Deserialize, Serialize};

/// Whether a subsystem is usable right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Working and ready.
    Ready,
    /// Not installed or not configured. User action can enable it.
    Unavailable,
    /// Present but currently failing.
    Degraded,
}

impl HealthStatus {
    /// True only when the subsystem is fully ready.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// One subsystem's state, with a human-readable reason and an optional
/// remediation hint shown in the capabilities screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    /// Stable machine identifier, e.g. `fairplay`.
    pub id: String,
    /// Display name, e.g. "Lossless & Hi-Res".
    pub name: String,
    pub status: HealthStatus,
    /// One short line explaining the current state.
    pub detail: String,
    /// What the user can do to improve it, when there is something to do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

impl Health {
    /// A ready subsystem.
    #[must_use]
    pub fn ready(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: HealthStatus::Ready,
            detail: String::new(),
            action: None,
        }
    }

    /// A subsystem that is not available, with a reason.
    #[must_use]
    pub fn unavailable(
        id: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: HealthStatus::Unavailable,
            detail: detail.into(),
            action: None,
        }
    }

    /// Attaches a remediation hint.
    #[must_use]
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// The aggregate capability report the UI renders.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    pub entries: Vec<Health>,
}

impl HealthReport {
    /// Builds a report from entries.
    #[must_use]
    pub fn new(entries: Vec<Health>) -> Self {
        Self { entries }
    }

    /// Looks up one subsystem by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Health> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// True when every subsystem is ready.
    #[must_use]
    pub fn all_ready(&self) -> bool {
        self.entries.iter().all(|entry| entry.status.is_ready())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lookup_and_readiness() {
        let report = HealthReport::new(vec![
            Health::ready("playback", "Playback"),
            Health::unavailable("fairplay", "Lossless", "Apple libraries not installed")
                .with_action("Install from Settings"),
        ]);

        assert!(!report.all_ready());
        assert!(report.get("playback").is_some_and(|h| h.status.is_ready()));
        assert_eq!(
            report.get("fairplay").and_then(|h| h.action.as_deref()),
            Some("Install from Settings")
        );
    }

    #[test]
    fn empty_report_is_ready() {
        assert!(HealthReport::default().all_ready());
    }
}
