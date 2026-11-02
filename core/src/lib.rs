//! Platform-neutral core for sosumi.
//!
//! This crate owns catalog-independent domain state: the queue, playback
//! capabilities, and shared models. Platform playback adapters (MusicKit,
//! MusicKit for Android, and the Linux/Android TV compatibility path) live
//! outside this crate.
//!
//! The core deliberately has no dependency on Apple frameworks, Flutter, or
//! any DRM implementation.

#![forbid(unsafe_code)]

pub mod apple;
pub mod error;
pub mod model;
pub mod queue;

pub use error::{CoreError, Result};
pub use model::{AudioVariant, Capabilities, PlaybackState, Track};
pub use queue::{PlayQueue, RepeatMode};

use serde::{Deserialize, Serialize};

/// Core crate version.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// ABI version for native bridge consumers.
pub const ABI_VERSION: u32 = 1;

/// Serializable view of the core state, suitable for a future Dart bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSnapshot {
    /// Core version.
    pub version: String,
    /// Native ABI version.
    pub abi_version: u32,
    /// Number of queued tracks.
    pub queue_len: usize,
    /// Current queue index, if any.
    pub current_index: Option<usize>,
    /// Capabilities reported by the active adapter.
    pub capabilities: Capabilities,
}

/// The root core object.
#[derive(Debug, Clone, Default)]
pub struct Core {
    queue: PlayQueue,
    capabilities: Capabilities,
}

impl Core {
    /// Creates an empty core with no playback capabilities.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a core with the given capabilities.
    #[must_use]
    pub fn with_capabilities(capabilities: Capabilities) -> Self {
        Self {
            queue: PlayQueue::new(),
            capabilities,
        }
    }

    /// The playback queue.
    #[must_use]
    pub const fn queue(&self) -> &PlayQueue {
        &self.queue
    }

    /// Mutable access to the playback queue.
    pub const fn queue_mut(&mut self) -> &mut PlayQueue {
        &mut self.queue
    }

    /// Capabilities reported by the active adapter.
    #[must_use]
    pub const fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    /// Replaces the capability set after an adapter connects.
    pub const fn set_capabilities(&mut self, capabilities: Capabilities) {
        self.capabilities = capabilities;
    }

    /// Produces a serializable snapshot for the UI layer.
    #[must_use]
    pub fn snapshot(&self) -> CoreSnapshot {
        CoreSnapshot {
            version: CORE_VERSION.to_owned(),
            abi_version: ABI_VERSION,
            queue_len: self.queue.len(),
            current_index: self.queue.current_index(),
            capabilities: self.capabilities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ABI_VERSION, CORE_VERSION, Capabilities, Core, CoreSnapshot};
    use crate::model::AudioVariant;

    #[test]
    fn snapshot_starts_empty() {
        let core = Core::new();
        let snapshot = core.snapshot();

        assert_eq!(snapshot.version, CORE_VERSION);
        assert_eq!(snapshot.abi_version, ABI_VERSION);
        assert_eq!(snapshot.queue_len, 0);
        assert_eq!(snapshot.current_index, None);
    }

    #[test]
    fn best_variant_uses_hi_res_first() {
        let capabilities = Capabilities {
            lossless: true,
            hi_res_lossless: true,
            dolby_atmos: true,
            ..Capabilities::none()
        };

        assert_eq!(
            capabilities.best_variant(),
            Some(AudioVariant::HiResLossless)
        );
    }

    #[test]
    fn snapshot_serializes() {
        let snapshot = Core::new().snapshot();
        let json = serde_json::to_string(&snapshot).expect("snapshot must serialize");
        let decoded: CoreSnapshot = serde_json::from_str(&json).expect("snapshot must deserialize");

        assert_eq!(decoded, snapshot);
    }
}
