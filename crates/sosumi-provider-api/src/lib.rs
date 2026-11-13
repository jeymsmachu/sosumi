//! The provider contract.
//!
//! Everything above this crate — the player, the library, the UI — depends only
//! on the types here. Adding a new source of music means implementing
//! [`MediaProvider`] and nothing else.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod error;
pub mod model;
pub mod provider;
pub mod stream;

pub use error::{ProviderError, Result};
pub use model::{Album, Artist, Artwork, Lyrics, Page, Playlist, ProviderId, Track, TrackId};
pub use provider::{MediaProvider, SearchQuery, SearchResults};
pub use stream::{
    AudioQuality, AudioSource, Codec, PlaybackSession, SpatialFormat, StreamInfo, StreamRequest,
};
