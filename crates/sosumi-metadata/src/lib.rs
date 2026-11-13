//! Metadata helpers: artwork URL templating, colour extraction, and filename
//! templates for downloads.
//!
//! Nothing here talks to a network. It turns provider-supplied strings into the
//! shapes the rest of the app wants.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod artwork;
pub mod naming;

pub use artwork::ArtworkTemplate;
pub use naming::FilenameTemplate;
