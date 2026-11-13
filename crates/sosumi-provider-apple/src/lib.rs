//! The Apple Music provider.
//!
//! This is the only crate that knows what Apple is. Everything above it speaks
//! [`sosumi_provider_api`] vocabulary.
//!
//! The modules are ordered by how much they know:
//!
//! - [`hls`] and [`token`] are pure parsers with no I/O. They are the most
//!   heavily tested because they are where Apple's formats are interpreted.
//! - [`catalog`] is a thin, typed wrapper over `amp-api.music.apple.com`.
//! - [`webplayback`] acquires playable stream URLs and licence keys.
//! - [`provider`] wires them into a [`sosumi_provider_api::MediaProvider`].

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod catalog;
pub mod client;
pub mod hls;
pub mod provider;
pub mod token;
pub mod webplayback;

pub use provider::AppleMusicProvider;
