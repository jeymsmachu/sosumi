//! Audio output through cpal.
//!
//! The output path is deliberately thin: it owns a device, a stream, and a
//! ring buffer, and it copies `f32` frames from the buffer to the device. It has
//! no knowledge of codecs, tracks, or providers.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;

/// Errors from the output path.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    /// No output device is available.
    #[error("no audio output device")]
    NoDevice,
    /// The device rejected the configuration.
    #[error("device does not support the requested format: {0}")]
    Config(String),
    /// The stream could not be built or started.
    #[error("could not start output stream: {0}")]
    Stream(String),
}

/// A ring buffer of interleaved `f32` samples shared with the audio callback.
struct Ring {
    data: Vec<f32>,
    read: usize,
    write: usize,
    channels: u16,
}

impl Ring {
    fn with_capacity(frames: usize, channels: u16) -> Self {
        let capacity = frames * channels as usize;
        Self { data: vec![0.0; capacity], read: 0, write: 0, channels }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    /// Writes interleaved frames, returning how many frames fit.
    fn push(&mut self, samples: &[f32]) -> usize {
        let channels = self.channels as usize;
        let frames = samples.len() / channels;
        let free = self.free_frames();
        let to_write = frames.min(free);
        for sample in samples.iter().take(to_write * channels) {
            self.data[self.write] = *sample;
            self.write = (self.write + 1) % self.len();
        }
        to_write
    }

    /// Reads `frames` frames into `out`, zero-filling any shortfall so playback
    /// underruns are silence rather than noise.
    fn pop_into(&mut self, out: &mut [f32]) {
        let channels = self.channels as usize;
        let want_frames = out.len() / channels;
        for frame in 0..want_frames {
            let available = self.available_frames() > 0;
            for channel in 0..channels {
                let value = if available {
                    let sample = self.data[self.read];
                    self.read = (self.read + 1) % self.len();
                    sample
                } else {
                    0.0
                };
                out[frame * channels + channel] = value;
            }
        }
    }

    fn available_frames(&self) -> usize {
        let write = self.write;
        let read = self.read;
        if write >= read {
            (write - read) / self.channels as usize
        } else {
            (self.len() - read + write) / self.channels as usize
        }
    }

    fn free_frames(&self) -> usize {
        let capacity = self.len() / self.channels as usize;
        capacity.saturating_sub(self.available_frames()).saturating_sub(1)
    }
}

impl std::fmt::Debug for Ring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ring")
            .field("capacity", &self.data.len())
            .field("channels", &self.channels)
            .field("available", &self.available_frames())
            .field("read", &self.read)
            .field("write", &self.write)
            .finish()
    }
}

/// An open output device with a live stream.
pub struct OutputDevice {
    ring: std::sync::Arc<Mutex<Ring>>,
    _stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
}

impl std::fmt::Debug for OutputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputDevice")
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .finish_non_exhaustive()
    }
}

impl OutputDevice {
    /// Opens the default output device and starts a stream.
    ///
    /// `buffer_frames` is the ring capacity per channel; roughly 100 ms of audio
    /// is a reasonable default.
    ///
    /// # Errors
    ///
    /// Returns [`OutputError`] when there is no device, or the device rejects
    /// the format.
    pub fn open(buffer_frames: usize) -> Result<Self, OutputError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(OutputError::NoDevice)?;
        let supported =
            device.default_output_config().map_err(|e| OutputError::Config(e.to_string()))?;

        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let config: StreamConfig = supported.clone().into();
        let ring = std::sync::Arc::new(Mutex::new(Ring::with_capacity(buffer_frames, channels)));
        let callback_ring = std::sync::Arc::clone(&ring);

        let error_callback =
            |err| tracing::error!(target: "sosumi::audio", %err, "output stream error");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |out: &mut [f32], _| callback_ring.lock().pop_into(out),
                error_callback,
                None,
            ),
            other => {
                return Err(OutputError::Config(format!(
                    "unsupported sample format {other:?}; only f32 is handled"
                )));
            }
        }
        .map_err(|e| OutputError::Stream(e.to_string()))?;

        stream.play().map_err(|e| OutputError::Stream(e.to_string()))?;

        Ok(Self { ring, _stream: stream, sample_rate, channels })
    }

    /// The device's sample rate.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The device's channel count.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Queues interleaved frames, returning how many frames were accepted.
    ///
    /// A short return means the buffer is full; the caller should retry after
    /// the stream has drained.
    pub fn write(&self, samples: &[f32]) -> usize {
        self.ring.lock().push(samples)
    }

    /// Frames queued but not yet played.
    #[must_use]
    pub fn buffered_frames(&self) -> usize {
        self.ring.lock().available_frames()
    }

    /// Discards everything queued. Used when seeking or skipping.
    pub fn flush(&self) {
        let mut ring = self.ring.lock();
        ring.read = ring.write;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_round_trips_frames() {
        let mut ring = Ring::with_capacity(8, 2);
        let written = ring.push(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(written, 2);

        let mut out = [0.0f32; 4];
        ring.pop_into(&mut out);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn ring_underrun_yields_silence() {
        let mut ring = Ring::with_capacity(4, 1);
        ring.push(&[0.5, 0.5]);
        let mut out = [9.0f32; 4];
        ring.pop_into(&mut out);
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn ring_rejects_overflow_without_corrupting() {
        let mut ring = Ring::with_capacity(4, 1);
        let written = ring.push(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert!(written <= 3, "must leave room for its own bookkeeping");
        assert_eq!(ring.available_frames(), written);
    }

    #[test]
    fn ring_wraps_around_correctly() {
        let mut ring = Ring::with_capacity(4, 1);
        ring.push(&[1.0, 2.0]);
        let mut out = [0.0f32; 2];
        ring.pop_into(&mut out);
        assert_eq!(out, [1.0, 2.0]);

        // Write past the end and read it back.
        ring.push(&[3.0, 4.0, 5.0]);
        let mut out2 = [0.0f32; 3];
        ring.pop_into(&mut out2);
        assert_eq!(out2, [3.0, 4.0, 5.0]);
    }

    #[test]
    fn flush_discards_queued_frames() {
        let mut ring = Ring::with_capacity(8, 1);
        ring.push(&[1.0, 2.0, 3.0]);
        ring.read = ring.write;
        assert_eq!(ring.available_frames(), 0);
    }
}
