//! Audio decoding and output.
//!
//! The split is deliberate: [`decode`] turns bytes into PCM and never touches a
//! device, and [`output`] moves PCM to the sound card and never knows what codec
//! it came from. That makes the decoder testable without hardware, and the
//! output path testable with a synthetic source.
//!
//! This crate has no concept of DRM, providers, or quality tiers. It receives
//! decrypted frames and a [`sosumi_provider_api::StreamInfo`].

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)
)]

pub mod decode;
pub mod output;
pub mod player;

pub use decode::{Decoder, PcmFormat};
pub use output::{OutputDevice, OutputError};
pub use player::{Player, PlayerState};
