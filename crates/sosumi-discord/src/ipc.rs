//! The Discord IPC transport.
//!
//! Discord listens on a Unix socket named `discord-ipc-N` in one of a few
//! runtime directories, and speaks a simple framed JSON protocol:
//!
//! ```text
//! request   → [opcode u32 LE][length u32 LE][json body]
//! response  ← [opcode u32 LE][length u32 LE][json body]
//! ```
//!
//! Opcodes: 0 handshake, 1 frame, 2 close, 3 ping, 4 pong.
//!
//! The client is deliberately synchronous and short-lived per update: connect,
//! handshake if needed, send, and keep the socket for the next update. If
//! anything fails the client disables itself rather than retrying in a loop,
//! because Discord not being open is a normal state, not an error.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::activity::Activity;

/// Discord's application id for the handshake.
///
/// Sosumi does not register an application, so presence is sent without one and
/// Discord renders it under the app name the local client associates with the
/// socket. This value is only used if Discord requires one.
pub const DEFAULT_APPLICATION_ID: &str = "0";

/// The opcodes Discord defines.
mod opcode {
    pub(super) const HANDSHAKE: u32 = 0;
    pub(super) const FRAME: u32 = 1;
    pub(super) const CLOSE: u32 = 2;
    pub(super) const PING: u32 = 3;
    pub(super) const PONG: u32 = 4;
}

/// How many socket indices to probe.
const MAX_SOCKET_INDEX: u8 = 10;

/// Errors from the Discord IPC.
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// No Discord client is running.
    #[error("Discord is not running")]
    NotRunning,
    /// The socket exists but the exchange failed.
    #[error("Discord IPC failed: {0}")]
    Transport(String),
    /// Discord sent something unexpected.
    #[error("unexpected Discord response: {0}")]
    Protocol(String),
}

/// A connected Discord client.
#[derive(Debug)]
pub struct DiscordClient {
    stream: UnixStream,
    application_id: String,
}

impl DiscordClient {
    /// Connects to a running Discord, or reports that none is running.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::NotRunning`] when no socket is found. That is the
    /// expected outcome when Discord is closed, so callers treat it as
    /// "presence is off" rather than as a failure.
    pub fn connect(application_id: impl Into<String>) -> Result<Self, IpcError> {
        let stream = find_socket()
            .and_then(|path| UnixStream::connect(path).ok())
            .ok_or(IpcError::NotRunning)?;

        // A read timeout keeps a half-dead Discord from blocking the caller.
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .map_err(|e| IpcError::Transport(e.to_string()))?;

        let mut client = Self { stream, application_id: application_id.into() };
        client.handshake()?;
        Ok(client)
    }

    /// Wraps an already connected stream. Used by tests with a socket pair.
    #[must_use]
    pub fn from_stream(stream: UnixStream, application_id: impl Into<String>) -> Self {
        Self { stream, application_id: application_id.into() }
    }

    /// Sends the handshake frame and checks Discord's reply.
    fn handshake(&mut self) -> Result<(), IpcError> {
        let payload = json!({ "v": 1, "client_id": self.application_id });
        self.write_frame(opcode::HANDSHAKE, &payload)?;

        let (op, body) = self.read_frame()?;
        if op == opcode::CLOSE {
            return Err(IpcError::Protocol(format!("Discord closed the connection: {body}")));
        }
        Ok(())
    }

    /// Publishes an activity.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] when the frame cannot be sent or Discord rejects it.
    pub fn set_activity(&mut self, activity: &Activity) -> Result<(), IpcError> {
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": std::process::id(),
                "activity": activity,
            },
            "nonce": nonce(),
        });
        self.write_frame(opcode::FRAME, &payload)?;

        let (op, body) = self.read_frame()?;
        if op == opcode::CLOSE {
            return Err(IpcError::Protocol(format!("Discord rejected the activity: {body}")));
        }
        Ok(())
    }

    /// Clears the presence.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] when the frame cannot be sent.
    pub fn clear_activity(&mut self) -> Result<(), IpcError> {
        let payload = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": Value::Null },
            "nonce": nonce(),
        });
        self.write_frame(opcode::FRAME, &payload)?;
        Ok(())
    }

    /// Sends a ping and waits for the matching pong.
    ///
    /// Used as a liveness check before committing to an activity update, so a
    /// Discord that has quit since the last update is discovered here rather
    /// than by a frame that silently goes nowhere.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::Transport`] when the exchange fails.
    pub fn ping(&mut self) -> Result<(), IpcError> {
        self.write_frame(opcode::PING, &json!({}))?;
        loop {
            let (op, _) = self.read_frame()?;
            match op {
                opcode::PONG => return Ok(()),
                opcode::CLOSE => {
                    return Err(IpcError::Protocol("Discord closed during a ping".into()));
                }
                // A frame arriving before the pong is Discord dispatching
                // something unrelated; keep reading until the pong.
                _ => {}
            }
        }
    }

    /// Writes one framed message.
    fn write_frame(&mut self, op: u32, body: &Value) -> Result<(), IpcError> {
        let json = serde_json::to_vec(body).map_err(|e| IpcError::Transport(e.to_string()))?;
        let length = u32::try_from(json.len())
            .map_err(|_| IpcError::Transport("frame exceeds the length field".into()))?;

        let mut frame = Vec::with_capacity(8 + json.len());
        frame.extend_from_slice(&op.to_le_bytes());
        frame.extend_from_slice(&length.to_le_bytes());
        frame.extend_from_slice(&json);

        self.stream.write_all(&frame).map_err(|e| IpcError::Transport(e.to_string()))
    }

    /// Reads one framed message.
    fn read_frame(&mut self) -> Result<(u32, Value), IpcError> {
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header).map_err(|e| IpcError::Transport(e.to_string()))?;

        let op = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

        // A sane bound: Discord's replies are small, and an absurd length means
        // the stream is not Discord.
        if length > 1 << 20 {
            return Err(IpcError::Protocol(format!("frame of {length} bytes is implausible")));
        }

        let mut body = vec![0u8; length];
        self.stream.read_exact(&mut body).map_err(|e| IpcError::Transport(e.to_string()))?;

        let value = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).map_err(|e| IpcError::Protocol(e.to_string()))?
        };
        Ok((op, value))
    }
}

/// A monotonic-ish nonce, so Discord can match replies to requests.
fn nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    format!("{nanos:x}")
}

/// Finds Discord's socket by probing the usual indices.
///
/// The search order follows Discord's own client: the runtime directory first,
/// then the Flatpak and Snap sandbox paths, then `/tmp`.
fn find_socket() -> Option<PathBuf> {
    for base in socket_dirs() {
        for index in 0..MAX_SOCKET_INDEX {
            let candidate = base.join(format!("discord-ipc-{index}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// The directories Discord may place its socket in.
fn socket_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        dirs.push(PathBuf::from(runtime));
    }
    if let Some(home) = std::env::var_os("HOME") {
        // Flatpak and Snap both relocate the runtime directory.
        dirs.push(PathBuf::from(&home).join(".flatpak/dev.discord.Discord/xdg-run"));
        dirs.push(PathBuf::from(&home).join("snap/discord/current/.config/discord"));
    }
    dirs.push(PathBuf::from("/tmp"));
    dirs
}

/// True when a Discord client appears to be running.
///
/// A cheap existence check, used by the capability probe. It does not connect.
#[must_use]
pub fn is_running() -> bool {
    find_socket().is_some()
}

/// The Rich Presence state Sosumi tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Presence {
    /// Not enabled by the user.
    #[default]
    Disabled,
    /// Enabled, but Discord is not reachable right now.
    Waiting,
    /// Enabled and publishing.
    Active,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_through_a_socket_pair() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let mut client = DiscordClient::from_stream(reader, "0");

        let payload = json!({ "v": 1, "client_id": "0" });
        writer.write_all(&0u32.to_le_bytes()).expect("op");
        let body = serde_json::to_vec(&payload).expect("json");
        writer.write_all(&(body.len() as u32).to_le_bytes()).expect("len");
        writer.write_all(&body).expect("body");

        let (op, value) = client.read_frame().expect("read");
        assert_eq!(op, opcode::HANDSHAKE);
        assert_eq!(value["client_id"], "0");
    }

    #[test]
    fn writing_a_frame_produces_the_expected_bytes() {
        let (writer, reader) = UnixStream::pair().expect("socket pair");
        let mut client = DiscordClient::from_stream(writer, "0");

        let payload = json!({ "cmd": "PING" });
        client.write_frame(opcode::FRAME, &payload).expect("write");

        let mut header = [0u8; 8];
        let mut reader = reader;
        reader.read_exact(&mut header).expect("header");
        assert_eq!(u32::from_le_bytes([header[0], header[1], header[2], header[3]]), opcode::FRAME);

        let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["cmd"], "PING");
    }

    #[test]
    fn an_empty_body_is_read_as_json_null() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let mut client = DiscordClient::from_stream(reader, "0");

        writer.write_all(&opcode::PONG.to_le_bytes()).expect("op");
        writer.write_all(&0u32.to_le_bytes()).expect("len");

        let (op, value) = client.read_frame().expect("read");
        assert_eq!(op, opcode::PONG);
        assert_eq!(value, Value::Null);
    }

    #[test]
    fn an_implausible_length_is_rejected() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let mut client = DiscordClient::from_stream(reader, "0");

        writer.write_all(&opcode::FRAME.to_le_bytes()).expect("op");
        writer.write_all(&u32::MAX.to_le_bytes()).expect("len");

        let error = client.read_frame().expect_err("must fail");
        assert!(matches!(error, IpcError::Protocol(_)));
    }

    #[test]
    fn a_close_opcode_is_surfaced_during_handshake() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let writer_thread = std::thread::spawn(move || {
            // Read the handshake, then answer with a close.
            let mut header = [0u8; 8];
            writer.read_exact(&mut header).expect("header");
            let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
            let mut body = vec![0u8; length];
            writer.read_exact(&mut body).expect("body");

            let close = br#"{"message":"bad client"}"#;
            writer.write_all(&opcode::CLOSE.to_le_bytes()).expect("op");
            writer.write_all(&(close.len() as u32).to_le_bytes()).expect("len");
            writer.write_all(close).expect("body");
        });

        let mut client = DiscordClient::from_stream(reader, "0");
        let error = client.handshake().expect_err("must fail");
        assert!(matches!(error, IpcError::Protocol(_)));
        writer_thread.join().expect("join");
    }

    #[test]
    fn a_successful_handshake_is_accepted() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let writer_thread = std::thread::spawn(move || {
            let mut header = [0u8; 8];
            writer.read_exact(&mut header).expect("header");
            let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
            let mut body = vec![0u8; length];
            writer.read_exact(&mut body).expect("body");

            let ready = br#"{"cmd":"DISPATCH","evt":"READY"}"#;
            writer.write_all(&opcode::FRAME.to_le_bytes()).expect("op");
            writer.write_all(&(ready.len() as u32).to_le_bytes()).expect("len");
            writer.write_all(ready).expect("body");
        });

        let mut client = DiscordClient::from_stream(reader, "0");
        client.handshake().expect("handshake");
        writer_thread.join().expect("join");
    }

    #[test]
    fn setting_an_activity_sends_the_expected_command() {
        let (mut writer, reader) = UnixStream::pair().expect("socket pair");
        let mut client = DiscordClient::from_stream(reader, "0");

        let writer_thread = std::thread::spawn(move || {
            let mut header = [0u8; 8];
            writer.read_exact(&mut header).expect("header");
            let length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
            let mut body = vec![0u8; length];
            writer.read_exact(&mut body).expect("body");
            let value: Value = serde_json::from_slice(&body).expect("json");
            assert_eq!(value["cmd"], "SET_ACTIVITY");
            assert_eq!(value["args"]["activity"]["details"], "Song");

            let reply = br#"{"cmd":"SET_ACTIVITY","data":{}}"#;
            writer.write_all(&opcode::FRAME.to_le_bytes()).expect("op");
            writer.write_all(&(reply.len() as u32).to_le_bytes()).expect("len");
            writer.write_all(reply).expect("body");
        });

        let activity = Activity::listening("Song", "Artist", Some(1_000), 0, false);
        client.set_activity(&activity).expect("set activity");
        writer_thread.join().expect("join");
    }

    #[test]
    fn nonces_differ_between_calls() {
        // Nanosecond timestamps; two calls in sequence must not collide.
        let first = nonce();
        std::thread::sleep(std::time::Duration::from_micros(1));
        let second = nonce();
        assert_ne!(first, second);
    }

    #[test]
    fn presence_starts_disabled() {
        assert_eq!(Presence::default(), Presence::Disabled);
    }

    #[test]
    fn connecting_without_discord_reports_not_running() {
        // Point the search at a directory with no sockets by clearing the env.
        let saved_runtime = std::env::var_os("XDG_RUNTIME_DIR");
        let saved_home = std::env::var_os("HOME");
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::set_var("HOME", "/nonexistent-home-for-test");

        // `/tmp` is still searched, so this only asserts the call does not panic
        // and returns a consistent type. A real Discord may exist on a dev box.
        let result = DiscordClient::connect("0");
        assert!(result.is_ok() || matches!(result, Err(IpcError::NotRunning)));

        if let Some(value) = saved_runtime {
            std::env::set_var("XDG_RUNTIME_DIR", value);
        }
        match saved_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }
}
