//! Diagnostics for Sosumi: logging setup, error primitives, and health reporting.
//!
//! This crate is the root of the dependency graph. It must never depend on another
//! Sosumi crate, so that every layer can log and report errors without cycles.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod health;
pub mod logging;

pub use health::{Health, HealthReport, HealthStatus};
