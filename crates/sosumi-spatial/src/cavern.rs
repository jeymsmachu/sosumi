//! The Cavern renderer adapter.
//!
//! Cavern's `CavernPipeServer` listens on a Unix socket and renders object-based
//! audio to binaural PCM. The protocol, as Cavern documents it:
//!
//! ```text
//! handshake   → 8 bytes: [bitDepth u8][mandatoryFrames u8][channels u16 LE][updateRate u32 LE]
//! request     → repeated: [length i32 LE][bytes]  … terminated by [length = -1]
//! response    ← repeated: [length i32 LE][f32 PCM] … terminated by [length = -1]
//! ```
//!
//! Output is raw interleaved little-endian `f32` at 48 kHz.
//!
//! # Why a subprocess
//!
//! Cavern is .NET and has its own licence. It is not linked into this binary and
//! is never bundled. This module only locates an existing installation and
//! talks to it. That keeps Sosumi's own licence clean and its build free of a
//! .NET dependency; see `docs/LEGAL-SCOPE.md`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sosumi_provider_api::SpatialFormat;

use crate::{RenderError, RenderTarget, Renderer};

/// The socket Cavern opens by default.
pub const DEFAULT_SOCKET: &str = "/tmp/CoreFxPipe_CavernPipe";

/// The binary name to look for on `PATH`.
pub const DEFAULT_BINARY: &str = "CavernPipeServer";

/// Environment override for the binary's location, for a non-standard install.
pub const BINARY_ENV: &str = "SOSUMI_CAVERN";

/// How long to wait for the socket to appear.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Renders through an installed `CavernPipeServer`.
#[derive(Debug)]
pub struct CavernRenderer {
    binary: Option<PathBuf>,
    socket: PathBuf,
}

impl CavernRenderer {
    /// Locates a renderer using the environment, `PATH`, and the default socket.
    #[must_use]
    pub fn detect() -> Self {
        let binary = std::env::var_os(BINARY_ENV)
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| which(DEFAULT_BINARY));
        Self { binary, socket: PathBuf::from(DEFAULT_SOCKET) }
    }

    /// Builds a renderer with an explicit binary path.
    ///
    /// A `None` path forces the unavailable state, which is what the tests use.
    #[must_use]
    pub fn with_binary(binary: Option<impl Into<PathBuf>>) -> Self {
        let binary = binary.and_then(|path| {
            let path = path.into();
            path.is_file().then_some(path)
        });
        Self { binary, socket: PathBuf::from(DEFAULT_SOCKET) }
    }

    /// The path to the binary, when one was found.
    #[must_use]
    pub fn binary(&self) -> Option<&Path> {
        self.binary.as_deref()
    }

    /// The socket that will be used.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// Connects to a running instance, waiting briefly for the socket.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::NotInstalled`] when there is no binary, and
    /// [`RenderError::Failed`] when the socket does not appear.
    pub fn connect(&self) -> Result<UnixStream, RenderError> {
        if self.binary.is_none() {
            return Err(RenderError::NotInstalled);
        }
        let deadline = std::time::Instant::now() + CONNECT_TIMEOUT;
        loop {
            match UnixStream::connect(&self.socket) {
                Ok(stream) => return Ok(stream),
                Err(_) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => {
                    return Err(RenderError::Failed(format!(
                        "socket {} did not appear: {error}",
                        self.socket.display()
                    )));
                }
            }
        }
    }

    /// Renders a stream by sending handshake, then input, then reading output.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] on a missing renderer or a protocol failure.
    pub fn render_stream(
        &self,
        inputs: &[Vec<u8>],
        target: RenderTarget,
    ) -> Result<Vec<f32>, RenderError> {
        let mut stream = self.connect()?;
        write_handshake(&mut stream, target)?;

        for chunk in inputs {
            write_chunk(&mut stream, chunk)?;
        }
        write_eof(&mut stream)?;

        read_pcm(&mut stream)
    }
}

impl Renderer for CavernRenderer {
    fn name(&self) -> &'static str {
        "cavern"
    }

    fn is_available(&self) -> bool {
        self.binary.is_some()
    }

    fn target(&self) -> RenderTarget {
        RenderTarget::default()
    }

    fn render(&self, encoded: &[u8], _format: &SpatialFormat) -> Result<Vec<f32>, RenderError> {
        self.render_stream(&[encoded.to_vec()], self.target())
    }
}

/// Writes Cavern's 8-byte handshake header.
fn write_handshake(stream: &mut UnixStream, target: RenderTarget) -> Result<(), RenderError> {
    let mut header = [0u8; 8];
    header[0] = 32; // bit depth: float32
    header[1] = 1; // mandatory frames
    header[2..4].copy_from_slice(&target.channels.to_le_bytes());
    header[4..8].copy_from_slice(&1024u32.to_le_bytes()); // update rate
    stream.write_all(&header).map_err(|e| RenderError::Failed(format!("handshake: {e}")))
}

/// Writes one length-prefixed chunk.
fn write_chunk(stream: &mut UnixStream, data: &[u8]) -> Result<(), RenderError> {
    let length = i32::try_from(data.len())
        .map_err(|_| RenderError::Failed("chunk exceeds the i32 length field".into()))?;
    stream
        .write_all(&length.to_le_bytes())
        .and_then(|()| stream.write_all(data))
        .map_err(|e| RenderError::Failed(format!("write chunk: {e}")))
}

/// Writes the end-of-stream sentinel, `-1`.
fn write_eof(stream: &mut UnixStream) -> Result<(), RenderError> {
    stream
        .write_all(&(-1i32).to_le_bytes())
        .map_err(|e| RenderError::Failed(format!("write EOF: {e}")))
}

/// Reads length-prefixed PCM chunks until the EOS sentinel.
fn read_pcm(stream: &mut UnixStream) -> Result<Vec<f32>, RenderError> {
    let mut out = Vec::new();
    let mut length_bytes = [0u8; 4];

    loop {
        stream
            .read_exact(&mut length_bytes)
            .map_err(|e| RenderError::Failed(format!("read chunk length: {e}")))?;
        let length = i32::from_le_bytes(length_bytes);
        if length < 0 {
            break;
        }
        let length = length as usize;
        let mut buffer = vec![0u8; length];
        stream
            .read_exact(&mut buffer)
            .map_err(|e| RenderError::Failed(format!("read chunk body: {e}")))?;

        for sample in buffer.chunks_exact(4) {
            out.push(f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]));
        }
    }

    Ok(out)
}

/// Finds an executable on `PATH`, without spawning a shell.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(name)).find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_binary_is_reported_as_unavailable() {
        let renderer = CavernRenderer::with_binary(None::<PathBuf>);
        assert!(!renderer.is_available());
        assert!(renderer.binary().is_none());
    }

    #[test]
    fn a_nonexistent_path_is_not_accepted() {
        let renderer = CavernRenderer::with_binary(Some("/nonexistent/CavernPipeServer"));
        assert!(!renderer.is_available(), "a path that is not a file must not count");
    }

    #[test]
    fn an_existing_binary_is_accepted() {
        // A file we know exists and can execute: this test binary's own shell.
        let renderer = CavernRenderer::with_binary(Some("/bin/sh"));
        if Path::new("/bin/sh").is_file() {
            assert!(renderer.is_available());
            assert_eq!(renderer.binary(), Some(Path::new("/bin/sh")));
        }
    }

    #[test]
    fn connecting_without_a_binary_fails_as_not_installed() {
        let renderer = CavernRenderer::with_binary(None::<PathBuf>);
        assert!(matches!(renderer.connect(), Err(RenderError::NotInstalled)));
    }

    #[test]
    fn the_default_socket_matches_caverns() {
        assert_eq!(DEFAULT_SOCKET, "/tmp/CoreFxPipe_CavernPipe");
    }

    #[test]
    fn the_handshake_encodes_the_target_layout() {
        let (mut writer, mut reader) = UnixStream::pair().expect("socket pair");
        write_handshake(&mut writer, RenderTarget { sample_rate: 48_000, channels: 2 })
            .expect("handshake");

        let mut header = [0u8; 8];
        reader.read_exact(&mut header).expect("read");
        assert_eq!(header[0], 32, "bit depth must be float32");
        assert_eq!(u16::from_le_bytes([header[2], header[3]]), 2);
        assert_eq!(u32::from_le_bytes([header[4], header[5], header[6], header[7]]), 1024);
    }

    #[test]
    fn a_chunk_round_trips_through_the_wire_format() {
        let (mut writer, mut reader) = UnixStream::pair().expect("socket pair");
        write_chunk(&mut writer, b"payload").expect("write");

        let mut length = [0u8; 4];
        reader.read_exact(&mut length).expect("read length");
        assert_eq!(i32::from_le_bytes(length), 7);

        let mut body = [0u8; 7];
        reader.read_exact(&mut body).expect("read body");
        assert_eq!(&body, b"payload");
    }

    #[test]
    fn eof_is_the_negative_one_sentinel() {
        let (mut writer, mut reader) = UnixStream::pair().expect("socket pair");
        write_eof(&mut writer).expect("write eof");

        let mut sentinel = [0u8; 4];
        reader.read_exact(&mut sentinel).expect("read");
        assert_eq!(i32::from_le_bytes(sentinel), -1);
    }

    #[test]
    fn pcm_is_read_as_little_endian_f32() {
        let (mut writer, mut reader) = UnixStream::pair().expect("socket pair");
        let writer_thread = std::thread::spawn(move || {
            let samples: Vec<f32> = vec![0.5, -0.25, 1.0];
            let mut bytes = Vec::new();
            for sample in samples {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            write_chunk(&mut writer, &bytes).expect("chunk");
            write_eof(&mut writer).expect("eof");
        });

        let pcm = read_pcm(&mut reader).expect("read pcm");
        writer_thread.join().expect("join");
        assert_eq!(pcm, vec![0.5, -0.25, 1.0]);
    }
}
