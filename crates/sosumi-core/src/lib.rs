//! Application orchestration.
//!
//! This crate is where the pieces meet: it holds the providers, drives the
//! audio player, and tells the UI what happened. It is the only crate the FFI
//! bridge depends on, which keeps the boundary small — see
//! `docs/adr/0002-rust-core-flutter-ui.md`.
//!
//! Everything here is transport-agnostic. It knows nothing about Flutter, D-Bus,
//! or Discord; those attach as observers of [`Event`] and callers of [`Sosumi`].

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod event;
pub mod media;
pub mod playback;
pub mod queue;
pub mod session;
pub mod sosumi;

pub use event::{Event, EventBus, EventReceiver};
pub use media::MediaFetcher;
pub use playback::PlaybackEngine;
pub use queue::{Queue, RepeatMode};
pub use session::Session;
pub use sosumi::Sosumi;
