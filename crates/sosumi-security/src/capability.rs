//! The optional-capability registry.
//!
//! Sosumi is designed so the core application never hard-depends on DRM,
//! Spatial Audio, Discord, or MPRIS. Each of those is a [`Capability`] with a
//! runtime-detected [`CapabilityState`]. The Settings screen renders the
//! registry directly, so adding a capability is a one-line change.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sosumi_diagnostics::{Health, HealthReport, HealthStatus};

/// A well-known optional capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Apple's Android `FairPlay` libraries, for lossless and hi-res.
    FairPlay,
    /// An external object-based renderer, for Spatial Audio.
    SpatialAudio,
    /// Discord Rich Presence.
    Discord,
    /// MPRIS2 D-Bus integration.
    Mpris,
}

impl Capability {
    /// Every capability, in the order Settings should show them.
    pub const ALL: [Self; 4] = [Self::FairPlay, Self::SpatialAudio, Self::Mpris, Self::Discord];

    /// Stable machine id, matching [`sosumi_diagnostics::Health::id`].
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::FairPlay => "fairplay",
            Self::SpatialAudio => "spatial",
            Self::Discord => "discord",
            Self::Mpris => "mpris",
        }
    }

    /// Display name used in the UI.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::FairPlay => "Lossless & Hi-Res",
            Self::SpatialAudio => "Spatial Audio",
            Self::Discord => "Discord Rich Presence",
            Self::Mpris => "System Media Controls",
        }
    }

    /// The user-facing quality tier this capability unlocks, if any.
    #[must_use]
    pub const fn unlocks(self) -> Option<&'static str> {
        match self {
            Self::FairPlay => Some("Lossless, Hi-Res Lossless"),
            Self::SpatialAudio => Some("Spatial Audio"),
            Self::Discord | Self::Mpris => None,
        }
    }
}

/// What state a capability is in right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CapabilityState {
    /// Installed and working.
    Ready { detail: String },
    /// Not installed or not configured. The user can enable it.
    Available { detail: String, action: String },
    /// Installed but currently failing.
    Degraded { detail: String },
    /// Not applicable on this system or build.
    Unsupported { detail: String },
}

impl CapabilityState {
    /// The detail line, whatever the variant.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Ready { detail }
            | Self::Available { detail, .. }
            | Self::Degraded { detail }
            | Self::Unsupported { detail } => detail,
        }
    }

    /// Whether the user can act to change this state.
    #[must_use]
    pub const fn is_actionable(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    /// The action hint, when there is one.
    #[must_use]
    pub fn action(&self) -> Option<&str> {
        match self {
            Self::Available { action, .. } => Some(action),
            _ => None,
        }
    }

    const fn status(&self) -> HealthStatus {
        match self {
            Self::Ready { .. } => HealthStatus::Ready,
            Self::Available { .. } | Self::Unsupported { .. } => HealthStatus::Unavailable,
            Self::Degraded { .. } => HealthStatus::Degraded,
        }
    }
}

/// Tracks the state of every capability.
#[derive(Debug, Clone, Default)]
pub struct CapabilityRegistry {
    states: BTreeMap<Capability, CapabilityState>,
}

impl CapabilityRegistry {
    /// An empty registry. Every capability reads as unsupported until probed.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a capability's state.
    pub fn set(&mut self, capability: Capability, state: CapabilityState) {
        self.states.insert(capability, state);
    }

    /// Probes and records a capability in one step.
    pub fn probe<F>(&mut self, capability: Capability, probe: F) -> &CapabilityState
    where
        F: FnOnce() -> CapabilityState,
    {
        self.states.entry(capability).or_insert_with(probe)
    }

    /// Looks up a capability's state.
    #[must_use]
    pub fn get(&self, capability: Capability) -> Option<&CapabilityState> {
        self.states.get(&capability)
    }

    /// True only when the capability is installed and working.
    #[must_use]
    pub fn is_ready(&self, capability: Capability) -> bool {
        self.states
            .get(&capability)
            .is_some_and(|state| matches!(state, CapabilityState::Ready { .. }))
    }

    /// Renders the registry as the report the UI consumes.
    #[must_use]
    pub fn health_report(&self) -> HealthReport {
        HealthReport::new(
            Capability::ALL
                .iter()
                .filter_map(|capability| {
                    let state = self.states.get(capability)?;
                    let mut health = Health {
                        id: capability.id().to_owned(),
                        name: capability.name().to_owned(),
                        status: state.status(),
                        detail: state.detail().to_owned(),
                        action: state.action().map(str::to_owned),
                    };
                    if let Some(unlocks) = capability.unlocks() {
                        if health.detail.is_empty() {
                            health.detail = format!("Enables {unlocks}");
                        }
                    }
                    Some(health)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_have_unique_ids() {
        let mut ids: Vec<_> = Capability::ALL.iter().map(|c| c.id()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }

    #[test]
    fn unknown_capability_is_not_ready() {
        let registry = CapabilityRegistry::new();
        assert!(!registry.is_ready(Capability::FairPlay));
    }

    #[test]
    fn ready_capability_appears_in_report() {
        let mut registry = CapabilityRegistry::new();
        registry.set(
            Capability::Mpris,
            CapabilityState::Ready { detail: "Connected to session bus".into() },
        );
        let report = registry.health_report();
        assert!(report.get("mpris").is_some_and(|h| h.status.is_ready()));
        assert!(report.get("fairplay").is_none());
    }

    #[test]
    fn available_capability_exposes_its_action() {
        let state = CapabilityState::Available {
            detail: "Not installed".into(),
            action: "Install from Settings".into(),
        };
        assert!(state.is_actionable());
        assert_eq!(state.action(), Some("Install from Settings"));
    }

    #[test]
    fn probe_does_not_overwrite_an_existing_state() {
        let mut registry = CapabilityRegistry::new();
        registry.set(Capability::Discord, CapabilityState::Unsupported { detail: "no".into() });
        let state = registry.probe(Capability::Discord, || CapabilityState::Ready {
            detail: "should not be used".into(),
        });
        assert!(matches!(state, CapabilityState::Unsupported { .. }));
    }
}
