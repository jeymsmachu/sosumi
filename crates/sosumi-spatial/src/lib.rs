//! Object-based audio rendering.
//!
//! Spatial Audio arrives as an E-AC-3 stream carrying Joint Object Coding: the
//! audio, plus a description of where each sound sits and how it moves. Turning
//! that into something a pair of headphones can play is *rendering*, and it is
//! a different problem from decoding.
//!
//! Sosumi does not embed a renderer. The one that does this properly —
//! [Cavern](https://github.com/VoidXH/Cavern) — is a separate program with its
//! own licence, which forbids sale and requires attribution. So the renderer is
//! a [`Renderer`] behind a trait, chosen at runtime:
//!
//! - [`CavernRenderer`] drives an installed `CavernPipeServer` over its local
//!   socket. This is the real object-based path.
//! - [`DownmixRenderer`] is the honest fallback: no object rendering, just a
//!   correct channel downmix, so audio always plays.
//!
//! Nothing here bundles, downloads, or auto-installs a renderer. Detection is a
//! path check, and the UI offers the install explicitly.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod cavern;
pub mod downmix;

use sosumi_diagnostics::{Health, HealthStatus};
use sosumi_provider_api::SpatialFormat;

pub use cavern::CavernRenderer;
pub use downmix::DownmixRenderer;

/// The capability id this subsystem reports under.
pub const CAPABILITY_ID: &str = "spatial";

/// What a renderer produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    /// Output sample rate in Hz.
    pub sample_rate: u32,
    /// Output channels, interleaved. Binaural rendering targets 2.
    pub channels: u16,
}

impl Default for RenderTarget {
    fn default() -> Self {
        // Binaural output for headphones, which is the only target that makes
        // sense without knowing the listener's speaker layout.
        Self { sample_rate: 48_000, channels: 2 }
    }
}

/// Errors from rendering.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The renderer is not installed.
    #[error("no spatial renderer is installed")]
    NotInstalled,
    /// The renderer was present but failed.
    #[error("renderer failed: {0}")]
    Failed(String),
    /// The input stream is not something this renderer handles.
    #[error("unsupported input: {0}")]
    Unsupported(String),
}

/// A renderer turns an encoded object-based stream into channel audio.
///
/// Implementors own their process, socket, or buffers. Callers hand them bytes
/// and receive PCM; the transport is private.
pub trait Renderer: Send + Sync + 'static {
    /// A short name for logs and diagnostics.
    fn name(&self) -> &'static str;

    /// True when this renderer can run now.
    fn is_available(&self) -> bool;

    /// The output layout this renderer produces.
    fn target(&self) -> RenderTarget;

    /// Renders one chunk of encoded object audio to interleaved `f32`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] when the renderer is unavailable or rejects the
    /// input.
    fn render(&self, encoded: &[u8], format: &SpatialFormat) -> Result<Vec<f32>, RenderError>;
}

/// Picks the best available renderer, or reports honestly that there is none.
///
/// The selection is deliberate and ordered: a real object renderer if one is
/// installed, otherwise the downmix, which always works.
#[must_use]
pub fn select_renderer(cavern: CavernRenderer) -> Box<dyn Renderer> {
    if cavern.is_available() {
        tracing::info!(target: "sosumi::spatial", "using Cavern for object rendering");
        Box::new(cavern)
    } else {
        tracing::info!(
            target: "sosumi::spatial",
            "no object renderer installed; falling back to a channel downmix"
        );
        Box::new(DownmixRenderer)
    }
}

/// Reports the renderer's state for the capabilities screen.
#[must_use]
pub fn health(renderer: &dyn Renderer) -> Health {
    if renderer.is_available() {
        Health {
            id: CAPABILITY_ID.to_owned(),
            name: "Spatial Audio".to_owned(),
            status: HealthStatus::Ready,
            detail: format!("Rendering through {}", renderer.name()),
            action: None,
        }
    } else {
        Health::unavailable(
            CAPABILITY_ID,
            "Spatial Audio",
            "No object-based renderer is installed; Spatial Audio tracks play as a downmix.",
        )
        .with_action("Install the renderer from Settings")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_target_is_binaural() {
        let target = RenderTarget::default();
        assert_eq!(target.channels, 2);
        assert_eq!(target.sample_rate, 48_000);
    }

    #[test]
    fn a_missing_cavern_selects_the_downmix() {
        // Point the renderer at a path that cannot exist.
        let cavern = CavernRenderer::with_binary(Some("/nonexistent/CavernPipeServer"));
        let renderer = select_renderer(cavern);
        assert_eq!(renderer.name(), "downmix");
    }

    #[test]
    fn health_reflects_the_renderer_in_use() {
        let renderer = DownmixRenderer;
        let health = health(&renderer);
        assert_eq!(health.id, "spatial");
        assert!(health.status.is_ready());
        assert!(health.detail.contains("downmix"));
    }
}
