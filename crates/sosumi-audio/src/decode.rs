//! PCM decoding through Symphonia.
//!
//! The decoder is device-free: it takes a byte source and yields interleaved
//! `f32` frames. That keeps it testable without a sound card, which is why the
//! tests here synthesise a WAV file rather than mocking anything.
//!
//! Symfonia decodes AAC and ALAC with the `all` feature set enabled in the
//! workspace manifest, which is what the Apple provider delivers.

use std::io::Cursor;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder as SymphoniaDecoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// The sample format a decoder produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of interleaved channels.
    pub channels: u16,
}

/// Errors from decoding.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The container or codec is not supported.
    #[error("unsupported format: {0}")]
    Unsupported(String),
    /// The stream ended unexpectedly or is corrupt.
    #[error("decoding failed: {0}")]
    Failed(String),
    /// The format changed mid-stream, which we do not handle yet.
    #[error("stream format changed mid-track")]
    FormatChanged,
    /// No audio track was found.
    #[error("no audio track in stream")]
    NoTrack,
}

impl From<SymphoniaError> for DecodeError {
    fn from(err: SymphoniaError) -> Self {
        match err {
            SymphoniaError::IoError(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                Self::Failed("unexpected end of stream".into())
            }
            other => Self::Failed(other.to_string()),
        }
    }
}

/// Decodes an in-memory audio stream into interleaved `f32` samples.
///
/// Construct once per track, then call [`Decoder::decode_next`] until it returns
/// `Ok(None)` (end of stream).
pub struct Decoder {
    format: Box<dyn FormatReader>,
    codec: Box<dyn SymphoniaDecoder>,
    track_id: u32,
    format_hint: PcmFormat,
}

impl std::fmt::Debug for Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoder").field("format", &self.format_hint).finish_non_exhaustive()
    }
}

impl Decoder {
    /// Builds a decoder over an in-memory buffer.
    ///
    /// `extension_hint` (for example `"m4a"`) guides the container probe; pass
    /// an empty string to let Symphonia sniff it.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Unsupported`] when no decoder can be built for the
    /// stream, and [`DecodeError::NoTrack`] when there is no audio track.
    pub fn new(data: Vec<u8>, extension_hint: &str) -> Result<Self, DecodeError> {
        let source = MediaSourceStream::new(
            Box::new(Cursor::new(data)),
            MediaSourceStreamOptions::default(),
        );

        let mut hint = Hint::new();
        if !extension_hint.is_empty() {
            hint.with_extension(extension_hint);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, source, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

        let format = probed.format;
        let track = format.default_track().ok_or(DecodeError::NoTrack)?;
        let track_id = track.id;

        let codec = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

        let params = &track.codec_params;
        let sample_rate = params.sample_rate.unwrap_or(44_100);
        let channels = params.channels.map_or(2, |c| c.count() as u16);

        let format_hint = PcmFormat { sample_rate, channels };
        Ok(Self { format, codec, track_id, format_hint })
    }

    /// The output format, known after construction.
    #[must_use]
    pub const fn format(&self) -> PcmFormat {
        self.format_hint
    }

    /// Decodes the next packet.
    ///
    /// Returns `Ok(None)` at a clean end of stream. A recoverable decode error on
    /// a single packet is skipped rather than failing the whole track, because
    /// streaming audio occasionally has a damaged frame.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::FormatChanged`] if the track changes mid-stream,
    /// and [`DecodeError::Failed`] for an unrecoverable error.
    pub fn decode_next(&mut self) -> Result<Option<DecodedPacket>, DecodeError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => return Err(DecodeError::FormatChanged),
                Err(other) => return Err(other.into()),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.codec.decode(&packet) {
                Ok(audio) => return Ok(Some(convert(&audio))),
                // A single bad frame is skipped; the stream continues.
                Err(SymphoniaError::DecodeError(_)) => {}
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(other) => return Err(other.into()),
            }
        }
    }

    /// Decodes the entire stream into one interleaved buffer.
    ///
    /// # Errors
    ///
    /// Propagates any [`DecodeError`] from [`Decoder::decode_next`].
    pub fn decode_all(&mut self) -> Result<Vec<f32>, DecodeError> {
        let mut out = Vec::new();
        while let Some(packet) = self.decode_next()? {
            out.extend_from_slice(&packet.samples);
        }
        Ok(out)
    }
}

/// One decoded packet.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedPacket {
    /// Interleaved `f32` samples.
    pub samples: Vec<f32>,
    /// Sample rate of this packet.
    pub sample_rate: u32,
    /// Channel count of this packet.
    pub channels: u16,
}

/// Normalises a Symphonia buffer into interleaved `f32`.
fn convert(audio: &AudioBufferRef<'_>) -> DecodedPacket {
    let sample_rate = audio.spec().rate;
    let channels = audio.spec().channels.count() as u16;
    let frames = audio.frames();
    let mut samples = vec![0.0f32; frames * channels as usize];

    // Symphonia hands back a typed buffer; we normalise the common cases to f32.
    macro_rules! interleave {
        ($buf:expr) => {{
            for channel in 0..channels as usize {
                let plane = $buf.chan(channel);
                for (frame, sample) in plane.iter().enumerate() {
                    samples[frame * channels as usize + channel] =
                        symphonia::core::conv::FromSample::from_sample(*sample);
                }
            }
        }};
    }

    match audio {
        AudioBufferRef::F32(buf) => interleave!(buf),
        AudioBufferRef::F64(buf) => interleave!(buf),
        AudioBufferRef::S16(buf) => interleave!(buf),
        AudioBufferRef::S24(buf) => interleave!(buf),
        AudioBufferRef::S32(buf) => interleave!(buf),
        AudioBufferRef::U8(buf) => interleave!(buf),
        AudioBufferRef::U16(buf) => interleave!(buf),
        AudioBufferRef::U24(buf) => interleave!(buf),
        AudioBufferRef::U32(buf) => interleave!(buf),
        AudioBufferRef::S8(buf) => interleave!(buf),
    }

    DecodedPacket { samples, sample_rate, channels }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a minimal 16-bit PCM WAV so the decoder has something real to read.
    fn synth_wav(sample_rate: u32, channels: u16, frames: u32, amplitude: f32) -> Vec<u8> {
        let bits = 16u16;
        let block_align = channels * bits / 8;
        let byte_rate = sample_rate * u32::from(block_align);
        let data_len = frames * u32::from(block_align);

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());

        for frame in 0..frames {
            // A 440 Hz-ish sine, so the samples are non-trivial.
            let t = frame as f32 / sample_rate as f32;
            let value = amplitude * (t * 440.0 * std::f32::consts::TAU).sin();
            let scaled = (value * f32::from(i16::MAX)) as i16;
            for _ in 0..channels {
                out.extend_from_slice(&scaled.to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn decodes_a_synthesised_wav() {
        let wav = synth_wav(44_100, 2, 4_410, 0.5);
        let mut decoder = Decoder::new(wav, "wav").expect("decoder");
        assert_eq!(decoder.format().sample_rate, 44_100);
        assert_eq!(decoder.format().channels, 2);

        let samples = decoder.decode_all().expect("decode");
        assert!(!samples.is_empty());
        // Stereo: sample count is a multiple of two.
        assert_eq!(samples.len() % 2, 0);
        // The signal should actually carry energy.
        let peak = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak > 0.1, "expected audible signal, peak was {peak}");
    }

    #[test]
    fn reports_the_right_channel_count_for_mono() {
        let wav = synth_wav(22_050, 1, 2_205, 0.25);
        let decoder = Decoder::new(wav, "wav").expect("decoder");
        assert_eq!(decoder.format().channels, 1);
        assert_eq!(decoder.format().sample_rate, 22_050);
    }

    #[test]
    fn garbage_is_rejected_as_unsupported() {
        let err = Decoder::new(vec![0u8; 512], "wav").expect_err("must fail");
        assert!(matches!(err, DecodeError::Unsupported(_)));
    }

    #[test]
    fn empty_input_is_rejected() {
        assert!(Decoder::new(Vec::new(), "wav").is_err());
    }

    #[test]
    fn short_stream_reports_clean_end_of_stream() {
        let wav = synth_wav(44_100, 2, 100, 0.3);
        let mut decoder = Decoder::new(wav, "wav").expect("decoder");
        let mut packets = 0;
        while decoder.decode_next().expect("decode").is_some() {
            packets += 1;
            assert!(packets < 1_000, "decoder failed to terminate");
        }
        assert!(packets >= 1);
    }
}
