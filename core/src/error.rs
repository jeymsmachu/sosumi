//! Error types for the sosumi core.

use thiserror::Error;

/// Errors produced by platform-neutral core operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    /// The queue has no entries.
    #[error("the queue is empty")]
    EmptyQueue,

    /// An index did not point into the queue.
    #[error("index {index} is out of bounds for a queue of length {len}")]
    IndexOutOfBounds {
        /// The requested index.
        index: usize,
        /// The current queue length.
        len: usize,
    },

    /// A capability was requested that the active playback adapter does not have.
    #[error("capability is not available: {0}")]
    UnsupportedCapability(String),
}

/// Convenient result alias for core operations.
pub type Result<T, E = CoreError> = std::result::Result<T, E>;
