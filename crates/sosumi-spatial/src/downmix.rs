//! The fallback renderer.
//!
//! When no object renderer is installed, Spatial Audio must still play. This
//! downmix takes the bed channels of an object stream and folds them to stereo
//! with proper gain, so nothing is lost or clipped.
//!
//! It is named `downmix` rather than `spatial` on purpose: [`crate::health`]
//! reports it as the fallback so the UI can say plainly that object rendering is
//! not active, rather than implying Spatial Audio is fully working.

use sosumi_provider_api::SpatialFormat;

use crate::{RenderError, RenderTarget, Renderer};

/// A constant-power stereo downmix.
#[derive(Debug, Default)]
pub struct DownmixRenderer;

/// The gain applied to a stereo pair when folding a surround bed.
///
/// `1/sqrt(2)` is the constant-power coefficient: summing two uncorrelated
/// channels this way keeps perceived loudness constant instead of boosting
/// correlated content to the point of clipping.
const DOWNMIX_GAIN: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Channel order for a 7.1 bed, as E-AC-3 carries it.
///
/// `(left, right)` pairs give the destination stereo channel for each source.
const SURROUND_PAIRS: &[(usize, usize)] = &[
    (0, 1), // L, R
    (2, 3), // Ls, Rs
    (4, 5), // Lb, Rb
    (6, 7), // Lc, Rc (or height, depending on the layout)
];

impl Renderer for DownmixRenderer {
    fn name(&self) -> &'static str {
        "downmix"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn target(&self) -> RenderTarget {
        RenderTarget::default()
    }

    fn render(&self, encoded: &[u8], format: &SpatialFormat) -> Result<Vec<f32>, RenderError> {
        if encoded.len() % 4 != 0 {
            return Err(RenderError::Unsupported(
                "PCM input must be a whole number of f32 samples".into(),
            ));
        }
        if format.bed_channels == 0 {
            return Err(RenderError::Unsupported("bed has no channels".into()));
        }

        let input: Vec<f32> = encoded
            .chunks_exact(4)
            .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
            .collect();

        Ok(downmix(&input, format.bed_channels))
    }
}

/// Folds an interleaved surround bed down to interleaved stereo.
///
/// A mono source is duplicated rather than panned, which is the least
/// surprising choice and keeps the output a stable two channels.
#[must_use]
pub fn downmix(interleaved: &[f32], channels: u8) -> Vec<f32> {
    let channels = channels as usize;
    if channels == 0 {
        return Vec::new();
    }
    let frames = interleaved.len() / channels;
    let mut out = Vec::with_capacity(frames * 2);

    if channels == 1 {
        for &sample in interleaved.iter().take(frames) {
            out.push(sample);
            out.push(sample);
        }
        return out;
    }

    for frame in interleaved.chunks_exact(channels).take(frames) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;

        for &(left_index, right_index) in SURROUND_PAIRS {
            if right_index >= channels {
                break;
            }
            left += frame[left_index];
            right += frame[right_index];
        }

        // Channels above the 7.1 pairs contribute to the nearest side, so
        // nothing is silently dropped on a wider bed.
        for (index, &sample) in frame.iter().enumerate().skip(8) {
            if index % 2 == 0 {
                left += sample;
            } else {
                right += sample;
            }
        }

        out.push((left * DOWNMIX_GAIN).clamp(-1.0, 1.0));
        out.push((right * DOWNMIX_GAIN).clamp(-1.0, 1.0));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> DownmixRenderer {
        DownmixRenderer
    }

    fn stereo_format() -> SpatialFormat {
        SpatialFormat { bed_channels: 2, has_objects: true }
    }

    fn surround_format() -> SpatialFormat {
        SpatialFormat { bed_channels: 8, has_objects: true }
    }

    #[test]
    fn stereo_passes_through_at_reduced_gain() {
        let input: Vec<f32> = vec![1.0, 0.0];
        let mut bytes = Vec::new();
        for sample in &input {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        let out = renderer().render(&bytes, &stereo_format()).expect("render");
        assert_eq!(out.len(), 2);
        // Left keeps its content, right stays silent.
        assert!((out[0] - DOWNMIX_GAIN).abs() < 1e-6);
        assert!(out[1].abs() < 1e-6);
    }

    #[test]
    fn mono_is_duplicated_to_both_channels() {
        let out = downmix(&[0.5, 0.25], 1);
        assert_eq!(out, vec![0.5, 0.5, 0.25, 0.25]);
    }

    #[test]
    fn surround_folds_into_both_sides() {
        // 8 channels, first frame: a hard-left signal, everything else silent.
        let mut interleaved = vec![0.0f32; 8];
        interleaved[0] = 1.0;
        let out = downmix(&interleaved, 8);

        assert_eq!(out.len(), 2);
        assert!(out[0] > 0.5);
        assert!(out[1].abs() < 1e-6);
    }

    #[test]
    fn the_output_is_clamped_to_the_valid_range() {
        // Every channel at full scale must not overflow.
        let interleaved = vec![1.0f32; 8 * 4];
        let out = downmix(&interleaved, 8);
        assert!(out.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn a_zero_frame_input_yields_nothing() {
        assert!(downmix(&[], 8).is_empty());
    }

    #[test]
    fn zero_bed_channels_is_rejected() {
        let error = downmix(&[], 0);
        assert!(error.is_empty());
    }

    #[test]
    fn the_renderer_rejects_a_partial_sample() {
        let error = renderer().render(&[0u8; 3], &stereo_format()).expect_err("must fail");
        assert!(matches!(error, RenderError::Unsupported(_)));
    }

    #[test]
    fn the_renderer_rejects_an_empty_bed() {
        let format = SpatialFormat { bed_channels: 0, has_objects: true };
        let error = renderer().render(&[], &format).expect_err("must fail");
        assert!(matches!(error, RenderError::Unsupported(_)));
    }

    #[test]
    fn the_renderer_renders_a_surround_bed() {
        // Four frames of a 7.1 bed with a hard-left signal.
        let mut interleaved = vec![0.0f32; 8 * 4];
        for frame in 0..4 {
            interleaved[frame * 8] = 0.75;
        }
        let mut bytes = Vec::new();
        for sample in &interleaved {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        let out = renderer().render(&bytes, &surround_format()).expect("render");
        assert_eq!(out.len(), 4 * 2, "one stereo frame per input frame");
        assert!(out[0] > 0.0);
        assert!(out[1].abs() < 1e-6);
    }

    #[test]
    fn the_downmix_reports_itself_as_the_fallback() {
        assert_eq!(renderer().name(), "downmix");
        assert!(renderer().is_available());
    }

    #[test]
    fn channels_beyond_seven_one_still_contribute() {
        // 10 channels: content only on the two extra channels must not vanish.
        let mut interleaved = vec![0.0f32; 10];
        interleaved[8] = 1.0;
        interleaved[9] = 1.0;
        let out = downmix(&interleaved, 10);
        assert!(out[0] > 0.0);
        assert!(out[1] > 0.0);
    }
}
