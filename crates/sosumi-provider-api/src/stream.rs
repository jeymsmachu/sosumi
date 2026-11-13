//! Audio quality, codecs, and stream descriptions.

use serde::{Deserialize, Serialize};

/// The codec of an audio stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    /// AAC-LC, the universal baseline.
    Aac,
    /// Apple Lossless.
    Alac,
    /// Free Lossless Audio Codec.
    Flac,
    /// Opus.
    Opus,
    /// Enhanced AC-3 with Joint Object Coding — the carrier for Spatial Audio.
    Ec3Joc,
    /// Raw linear PCM.
    Pcm,
}

impl Codec {
    /// True when the codec preserves the original samples exactly.
    #[must_use]
    pub const fn is_lossless(self) -> bool {
        matches!(self, Self::Alac | Self::Flac | Self::Pcm)
    }

    /// Lower-case name for display and filenames.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Aac => "AAC",
            Self::Alac => "ALAC",
            Self::Flac => "FLAC",
            Self::Opus => "Opus",
            Self::Ec3Joc => "Spatial Audio",
            Self::Pcm => "PCM",
        }
    }
}

/// The audio quality tier a user can request.
///
/// `Spatial` is deliberately not called "Atmos" anywhere in the codebase;
/// see `docs/LEGAL-SCOPE.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioQuality {
    /// AAC 64 kbps.
    Low,
    /// AAC 256 kbps. The default when nothing else is available.
    #[default]
    High,
    /// Lossless ALAC up to 48 kHz.
    Lossless,
    /// Hi-Res lossless ALAC above 48 kHz.
    HiResLossless,
    /// Object-based Spatial Audio, rendered binaurally.
    Spatial,
}

impl AudioQuality {
    /// Whether serving this tier requires an optional, opt-in component.
    #[must_use]
    pub const fn requires_capability(self) -> Option<&'static str> {
        match self {
            Self::Low | Self::High => None,
            Self::Lossless | Self::HiResLossless => Some("fairplay"),
            Self::Spatial => Some("spatial"),
        }
    }

    /// The capability id this tier depends on, if any. Matches
    /// [`sosumi_diagnostics::Health::id`].
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::High => "High Quality",
            Self::Lossless => "Lossless",
            Self::HiResLossless => "Hi-Res Lossless",
            Self::Spatial => "Spatial Audio",
        }
    }
}

/// The layout of a spatial stream, when it is object-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialFormat {
    /// Number of bed channels, e.g. 8 for 7.1.
    pub bed_channels: u8,
    /// Whether the stream carries dynamic objects beyond the bed.
    pub has_objects: bool,
}

/// What the provider will actually deliver for a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamInfo {
    pub codec: Codec,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Bit depth, for lossless codecs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<u16>,
    /// Nominal bitrate in kbps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    /// Present when the stream is object-based.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spatial: Option<SpatialFormat>,
}

impl StreamInfo {
    /// A short badge for the player bar, e.g. "ALAC 24-bit/96 kHz".
    #[must_use]
    pub fn badge(&self) -> String {
        match (self.bit_depth, self.codec) {
            (Some(depth), Codec::Alac | Codec::Flac) => {
                format!("{} {depth}-bit/{} kHz", self.codec.as_str(), self.sample_rate / 1000)
            }
            _ => self.codec.as_str().to_owned(),
        }
    }
}

/// What the caller asks for when opening a stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamRequest {
    pub track: super::TrackId,
    /// Storefront, e.g. `us`. Apple uses this to pick a catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storefront: Option<String>,
    /// The highest quality the caller is willing to accept.
    pub quality: AudioQuality,
}

/// Where a stream's bytes come from.
///
/// Encrypted sources use a decrypting reader, so the audio layer never handles
/// key material or provider-specific DRM.
pub enum AudioSource {
    /// Continuous bytes from an HTTP(S) URL. The reader seeks with `Range`.
    Http {
        url: String,
        /// Extra headers the provider requires, e.g. authorization.
        headers: Vec<(String, String)>,
    },
    /// Bytes produced by a decrypting reader.
    Decrypting(Box<dyn super::stream::source::AudioReader>),
}

impl std::fmt::Debug for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AudioSource").field(&self.kind()).finish()
    }
}

/// An open, playable stream for one track.
pub struct PlaybackSession {
    /// The track this session belongs to.
    pub track: super::TrackId,
    /// What the bytes actually are.
    pub info: StreamInfo,
    /// Where to read them from.
    pub source: AudioSource,
    /// Nominal duration in milliseconds, when known.
    pub duration_ms: Option<u64>,
}

impl std::fmt::Debug for PlaybackSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackSession")
            .field("track", &self.track)
            .field("info", &self.info)
            .field("source", &self.source.kind())
            .field("duration_ms", &self.duration_ms)
            .finish()
    }
}

impl AudioSource {
    /// A label for logs, without exposing URLs or headers.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Http { .. } => "http",
            Self::Decrypting(_) => "decrypting",
        }
    }
}

/// Support module for streaming bytes out of a session.
pub mod source {
    use std::pin::Pin;

    use tokio::io::AsyncRead;

    /// A reader that yields decrypted audio bytes.
    ///
    /// Implementors own their key material. Callers only ever see plaintext
    /// frames, which keeps DRM details out of the audio engine.
    pub trait AudioReader: Send + 'static {
        /// Takes the reader. Called once, by the decoder.
        fn into_reader(self: Box<Self>) -> Pin<Box<dyn AsyncRead + Send>>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_reports_its_capability() {
        assert_eq!(AudioQuality::High.requires_capability(), None);
        assert_eq!(AudioQuality::Lossless.requires_capability(), Some("fairplay"));
        assert_eq!(AudioQuality::Spatial.requires_capability(), Some("spatial"));
    }

    #[test]
    fn lossless_badge_shows_depth_and_rate() {
        let info = StreamInfo {
            codec: Codec::Alac,
            sample_rate: 96_000,
            bit_depth: Some(24),
            bitrate_kbps: None,
            spatial: None,
        };
        assert_eq!(info.badge(), "ALAC 24-bit/96 kHz");
    }

    #[test]
    fn aac_badge_is_just_the_codec() {
        let info = StreamInfo {
            codec: Codec::Aac,
            sample_rate: 44_100,
            bit_depth: None,
            bitrate_kbps: Some(256),
            spatial: None,
        };
        assert_eq!(info.badge(), "AAC");
        assert!(!info.codec.is_lossless());
    }

    #[test]
    fn spatial_is_ordered_above_hi_res() {
        assert!(AudioQuality::Spatial > AudioQuality::HiResLossless);
        assert!(AudioQuality::HiResLossless > AudioQuality::Lossless);
    }
}
