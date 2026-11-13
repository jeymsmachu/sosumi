//! Security primitives and optional capabilities.
//!
//! Two responsibilities live here, both deliberately isolated from the rest of
//! the app:
//!
//! 1. **Capability registry** — what optional components are installed. The app
//!    builds and runs with none of them; the registry is how the UI knows what
//!    to offer. See [`CapabilityRegistry`].
//! 2. **Content keys** — the software Widevine CDM used for the baseline AAC
//!    tier. Key material never leaves this crate.
//!
//! Offline downloads are re-sealed under a locally generated key; see [`sealing`].

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod capability;
pub mod sealing;
pub mod widevine;

pub use capability::{Capability, CapabilityRegistry, CapabilityState};
pub use sealing::{SealedFile, Sealer};
