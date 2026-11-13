//! Storage: filesystem layout, atomic writes, and encrypted local state.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod paths;
pub mod store;

pub use paths::Paths;
pub use store::SecretStore;
