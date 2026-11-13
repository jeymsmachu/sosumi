//! Discord Rich Presence.
//!
//! Discord exposes a local socket (a Unix socket on Linux) that accepts small
//! JSON frames. This module speaks that protocol directly rather than pulling
//! in a wrapper crate, because the protocol is small and stable and a direct
//! implementation is easier to test without a running Discord.
//!
//! Two facts shape the design:
//!
//! 1. **Discord may not be running.** Its socket is absent, and every operation
//!    must degrade to "presence is off" rather than to an error the user sees.
//! 2. **The presence is optional.** It is off by default, and the capability
//!    reports itself as available only when the socket is present.
//!
//! Nothing here contacts Discord's servers. The app talks to the local client,
//! which does that itself, which is why `docs/PRIVACY.md` can say the app has no
//! other destination.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod activity;
pub mod ipc;

pub use activity::Activity;
pub use ipc::{DiscordClient, Presence};
