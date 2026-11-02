//! Apple Music API integration.
//!
//! This module owns network access, JSON models, token signing/expiry parsing,
//! and session construction. It deliberately knows nothing about playback
//! adapters or DRM.

pub mod client;
pub mod error;
pub mod models;
pub mod session;
pub mod signer;
pub mod token;

pub use client::AppleMusicClient;
pub use error::AppleMusicError;
pub use models::{
    AlbumAttributes, ArtistAttributes, Artwork, EditorialNotes, PlaylistAttributes, Preview,
    Resource, ResourceCollection, SearchResponse, SearchResults, SearchType, SongAttributes,
    StorefrontAttributes,
};
pub use session::AppleMusicSession;
pub use signer::{DeveloperTokenSigner, MAX_TOKEN_TTL};
pub use token::{DeveloperToken, DeveloperTokenClaims};
