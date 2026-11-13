//! Desktop integration for Linux.
//!
//! Today this is MPRIS2: the D-Bus interface that lets a desktop's media applet,
//! media keys, and lock screen control playback. The interface is a
//! specification, so this is an implementation rather than a guess.
//!
//! The module is split so the parts can be tested without a bus:
//!
//! - [`mpris`] holds the interface traits and the state they are answered from.
//! - [`metadata`] converts a track into the `a{sv}` dictionary MPRIS expects.
//!
//! The service itself is only started when a session bus is present, which keeps
//! the app working in a container or over SSH.

#![deny(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)
)]

pub mod metadata;
pub mod mpris;

pub use mpris::{PlaybackStatus, PlayerState, Server};
