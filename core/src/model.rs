//! Shared domain models.

use serde::{Deserialize, Serialize};

/// Audio variants that a track can expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioVariant {
    /// Lossy stereo AAC.
    Aac,
    /// Apple Lossless up to 24-bit/48 kHz.
    Lossless,
    /// Apple Lossless up to 24-bit/192 kHz.
    HiResLossless,
    /// Dolby Atmos.
    DolbyAtmos,
    /// Dolby Audio surround.
    DolbyAudio,
}

impl AudioVariant {
    /// Human-readable label for this variant.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aac => "AAC",
            Self::Lossless => "Lossless",
            Self::HiResLossless => "Hi-Res Lossless",
            Self::DolbyAtmos => "Dolby Atmos",
            Self::DolbyAudio => "Dolby Audio",
        }
    }

    /// Whether the variant is lossless.
    #[must_use]
    pub const fn is_lossless(self) -> bool {
        matches!(self, Self::Lossless | Self::HiResLossless)
    }
}

/// Playback capabilities reported by a platform adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Lossless playback is available.
    pub lossless: bool,
    /// Hi-res lossless playback is available.
    pub hi_res_lossless: bool,
    /// Dolby Atmos playback is available.
    pub dolby_atmos: bool,
    /// Dolby Audio playback is available.
    pub dolby_audio: bool,
    /// Offline playback is available.
    pub offline: bool,
    /// Music videos are available.
    pub music_videos: bool,
    /// Radio stations are available.
    pub stations: bool,
}

impl Capabilities {
    /// No capabilities. Useful for an unconnected adapter.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            lossless: false,
            hi_res_lossless: false,
            dolby_atmos: false,
            dolby_audio: false,
            offline: false,
            music_videos: false,
            stations: false,
        }
    }

    /// The best variant supported by these capabilities, if any.
    #[must_use]
    pub const fn best_variant(self) -> Option<AudioVariant> {
        if self.hi_res_lossless {
            Some(AudioVariant::HiResLossless)
        } else if self.dolby_atmos {
            Some(AudioVariant::DolbyAtmos)
        } else if self.lossless {
            Some(AudioVariant::Lossless)
        } else if self.dolby_audio {
            Some(AudioVariant::DolbyAudio)
        } else {
            None
        }
    }
}

/// Playback lifecycle state shared by every adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    /// No playback session.
    #[default]
    Idle,
    /// A track is being prepared.
    Loading,
    /// Audio is playing.
    Playing,
    /// Playback is paused.
    Paused,
    /// The player is seeking.
    Seeking,
    /// The player is buffering.
    Stalled,
    /// The current track ended.
    Ended,
    /// The player is in an error state.
    Error,
}

/// A playable track in the core model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    /// Catalog or library identifier.
    pub id: String,
    /// Track title.
    pub title: String,
    /// Artist display name.
    pub artist: String,
    /// Album display name, when known.
    pub album: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Best known audio variant.
    pub variant: AudioVariant,
}

impl Track {
    /// Creates a track with the required fields.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        artist: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
            album: None,
            duration_ms,
            variant: AudioVariant::Aac,
        }
    }
}
