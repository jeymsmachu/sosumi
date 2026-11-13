//! The user's library.
//!
//! This holds what the user saved: songs, albums, and playlists. Apple serves
//! that over the API, but a client must keep a local copy because the library
//! is the screen the app opens to, and a round trip before every launch feels
//! broken on a slow connection.
//!
//! The store is a plain in-memory index persisted as one JSON document. A
//! relational engine would be premature: a large library is tens of thousands of
//! entries, and a map lookup beats a query planner at that size. If it ever
//! stops being enough, the boundaries here are what change.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod index;

pub use index::{Library, LibraryEntry, LibraryError, LibraryKind};
