//! The MPRIS2 service.
//!
//! A desktop's media applet discovers players by bus name and then reads
//! properties and calls methods on two interfaces:
//! [`org.mpris.MediaPlayer2`] (identity, quit, raise) and
//! [`org.mpris.MediaPlayer2.Player`] (transport, metadata, position).
//!
//! This module serves both. Every property projects [`PlayerState`], which is a
//! plain snapshot, so the D-Bus layer has no state of its own and can be tested
//! by inspecting a snapshot rather than by talking to a bus.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use sosumi_provider_api::Track;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};
use zbus::{interface, Connection};

use crate::metadata::{self, INTERFACE, OBJECT_PATH, ROOT_INTERFACE};

/// The transport state, in MPRIS's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    /// Nothing loaded.
    #[default]
    Stopped,
    /// Loaded but not advancing.
    Paused,
    /// Advancing.
    Playing,
}

impl PlaybackStatus {
    /// The exact string MPRIS expects.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::Paused => "Paused",
            Self::Playing => "Playing",
        }
    }
}

/// A snapshot of what the player is doing, as MPRIS needs it.
#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    /// The transport state.
    pub status: PlaybackStatus,
    /// The current track, when there is one.
    pub track: Option<Track>,
    /// Duration in microseconds, when known.
    pub length_us: Option<i64>,
    /// Position in microseconds.
    pub position_us: i64,
    /// Volume in the range `0.0..=1.0`.
    pub volume: f64,
    /// Whether shuffle is on.
    pub shuffle: bool,
    /// The loop mode: `None`, `Track`, or `Playlist`.
    pub loop_status: String,
}

impl PlayerState {
    /// A stopped player at full volume.
    #[must_use]
    pub fn stopped() -> Self {
        Self { volume: 1.0, loop_status: "None".to_owned(), ..Self::default() }
    }

    /// True when a transport action applies.
    #[must_use]
    pub const fn can_control(&self) -> bool {
        self.track.is_some()
    }

    /// True when the position can be changed, which needs a known length.
    #[must_use]
    pub const fn can_seek(&self) -> bool {
        self.length_us.is_some()
    }
}

/// Commands the service forwards to the application.
///
/// The MPRIS layer does not play audio itself; it reports intent. That keeps it
/// independent of the player and testable with a recording sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Start or resume.
    Play,
    /// Pause.
    Pause,
    /// Toggle between play and pause.
    PlayPause,
    /// Stop and unload.
    Stop,
    /// Skip forward.
    Next,
    /// Skip back.
    Previous,
    /// Seek by a relative number of microseconds.
    Seek(i64),
    /// Seek to an absolute number of microseconds.
    SetPosition(i64),
    /// Quit the application.
    Quit,
}

/// Receives commands from the MPRIS interface.
pub type CommandSink = Arc<dyn Fn(Command) + Send + Sync>;

/// The MPRIS service, holding the shared state and the command sink.
#[derive(Clone)]
pub struct Server {
    state: Arc<RwLock<PlayerState>>,
    commands: CommandSink,
    identity: Arc<str>,
}

impl std::fmt::Debug for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Server").field("identity", &self.identity).finish_non_exhaustive()
    }
}

impl Server {
    /// Builds a service.
    ///
    /// The identity is the suffix of the bus name, for example
    /// `org.mpris.MediaPlayer2.sosumi`.
    #[must_use]
    pub fn new(identity: impl Into<Arc<str>>, commands: CommandSink) -> Self {
        Self {
            state: Arc::new(RwLock::new(PlayerState::stopped())),
            commands,
            identity: identity.into(),
        }
    }

    /// The MPRIS bus name this service claims.
    #[must_use]
    pub fn bus_name(&self) -> String {
        format!("{}{}", metadata::BUS_NAME_PREFIX, self.identity)
    }

    /// Replaces the published state.
    pub fn update(&self, state: PlayerState) {
        *self.state.write() = state;
    }

    /// A snapshot of the published state.
    #[must_use]
    pub fn snapshot(&self) -> PlayerState {
        self.state.read().clone()
    }

    /// Publishes the service on the session bus.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no session bus, which is the normal
    /// outcome in a container or over SSH. Callers treat that as "MPRIS is not
    /// available" rather than as a failure.
    pub async fn serve(self) -> anyhow::Result<Connection> {
        let bus_name = self.bus_name();

        let connection = Connection::session().await?;
        connection.request_name(bus_name.as_str()).await?;

        let root = RootInterface;
        let player = PlayerInterface { server: self };

        connection.object_server().at(OBJECT_PATH, root).await?;
        connection.object_server().at(OBJECT_PATH, player).await?;

        tracing::info!(bus = %bus_name, "MPRIS2 service published");
        Ok(connection)
    }
}

/// Whether a session bus is reachable.
///
/// Checked before starting the service so the app never blocks on D-Bus in an
/// environment that has none.
#[must_use]
pub fn session_bus_available() -> bool {
    std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
}

/// The `org.mpris.MediaPlayer2` interface.
///
/// The identity fields are constants for this application, so the struct carries
/// no state and the methods take `&self` only to satisfy the interface macro.
struct RootInterface;

#[interface(name = "org.mpris.MediaPlayer2")]
// Every method of a D-Bus interface takes `&self` by contract, even when the
// implementation is a constant. Allowing that here mirrors the protocol.
#[allow(clippy::unused_self)]
impl RootInterface {
    /// Raised to bring the player to the front.
    fn raise(&self) {
        // The window is owned by the UI; a raise request is forwarded once the
        // platform channel lands.
    }

    /// Quits the application.
    fn quit(&self) {
        // Quit is a window-manager concern; the UI handles it. Deliberately a
        // no-op here rather than killing the process from a D-Bus call.
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> String {
        "Sosumi".to_owned()
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> String {
        "sosumi".to_owned()
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        Vec::new()
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}

/// The `org.mpris.MediaPlayer2.Player` interface.
struct PlayerInterface {
    server: Server,
}

impl PlayerInterface {
    fn send(&self, command: Command) {
        (self.server.commands)(command);
    }
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
// The `#[interface]` macro fixes these signatures to the D-Bus specification:
// every method takes `&self`, and setters take owned values. They cannot be
// changed to satisfy a lint without breaking the protocol.
#[allow(clippy::unused_self, clippy::needless_pass_by_value)]
impl PlayerInterface {
    fn play(&self) {
        self.send(Command::Play);
    }

    fn pause(&self) {
        self.send(Command::Pause);
    }

    fn play_pause(&self) {
        self.send(Command::PlayPause);
    }

    fn stop(&self) {
        self.send(Command::Stop);
    }

    fn next(&self) {
        self.send(Command::Next);
    }

    fn previous(&self) {
        self.send(Command::Previous);
    }

    /// Seeks by a relative number of microseconds.
    ///
    /// MPRIS specifies that a seek issued while stopped is ignored, so the
    /// caller does not have to guard against it.
    fn seek(&self, offset: i64) {
        if self.server.snapshot().status == PlaybackStatus::Stopped {
            return;
        }
        self.send(Command::Seek(offset));
    }

    /// Sets the absolute position, keyed on the track it applies to.
    ///
    /// The `track_id` parameter exists so a stale call for a previous track can
    /// be ignored; MPRIS requires that check.
    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let state = self.server.snapshot();
        let Some(track) = &state.track else { return };
        let expected = metadata::track_object_path(track.id.as_str());
        if track_id.as_str() != expected {
            tracing::debug!("ignoring SetPosition for a track that is no longer playing");
            return;
        }
        self.send(Command::SetPosition(position.max(0)));
    }

    /// Opens a URI. Sosumi claims no schemes, so this is always refused.
    fn open_uri(&self, uri: String) -> bool {
        tracing::debug!(%uri, "refusing OpenUri: no schemes are claimed");
        false
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.server.snapshot().status.as_str().to_owned()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        self.server.snapshot().loop_status
    }

    #[zbus(property)]
    fn set_loop_status(&self, status: String) {
        let normalised = match status.as_str() {
            "Track" => "Track",
            "Playlist" => "Playlist",
            _ => "None",
        };
        self.server
            .update(PlayerState { loop_status: normalised.to_owned(), ..self.server.snapshot() });
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.server.snapshot().shuffle
    }

    #[zbus(property)]
    fn set_shuffle(&self, shuffle: bool) {
        self.server.update(PlayerState { shuffle, ..self.server.snapshot() });
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let state = self.server.snapshot();
        match &state.track {
            Some(track) => metadata::metadata_for(track, state.length_us),
            None => HashMap::new(),
        }
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.server.snapshot().volume
    }

    #[zbus(property)]
    fn set_volume(&self, volume: f64) {
        let clamped = volume.clamp(0.0, 1.0);
        self.server.update(PlayerState { volume: clamped, ..self.server.snapshot() });
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.server.snapshot().position_us
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.server.snapshot().can_control()
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.server.snapshot().can_control()
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.server.snapshot().can_control()
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.server.snapshot().can_control()
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.server.snapshot().length_us.is_some()
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        self.server.snapshot().can_control()
    }
}

/// The two interface names this service publishes. Used by tests and docs.
#[must_use]
pub const fn interfaces() -> [&'static str; 2] {
    [ROOT_INTERFACE, INTERFACE]
}

/// Converts a track id into a typed object path, for a caller holding a `Value`.
#[must_use]
pub fn track_path_value(track_id: &str) -> Value<'static> {
    Value::from(metadata::track_object_path(track_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::{Artist, ProviderId, TrackId};

    fn track() -> Track {
        Track {
            id: TrackId::new(&ProviderId::from(ProviderId::APPLE), "42"),
            title: "Song".into(),
            artist: Artist { id: "a".into(), name: "Artist".into(), artwork: None },
            album: None,
            artwork: None,
            duration_ms: Some(180_000),
            isrc: None,
            track_number: None,
            disc_number: None,
        }
    }

    fn server() -> (Server, Arc<RwLock<Vec<Command>>>) {
        let received = Arc::new(RwLock::new(Vec::new()));
        let sink = Arc::clone(&received);
        let server = Server::new("sosumi", Arc::new(move |command| sink.write().push(command)));
        (server, received)
    }

    #[test]
    fn the_bus_name_follows_the_specification() {
        let (server, _) = server();
        assert_eq!(server.bus_name(), "org.mpris.MediaPlayer2.sosumi");
    }

    #[test]
    fn a_fresh_server_is_stopped_at_full_volume() {
        let (server, _) = server();
        let state = server.snapshot();
        assert_eq!(state.status, PlaybackStatus::Stopped);
        assert_eq!(state.volume, 1.0);
        assert!(state.track.is_none());
    }

    #[test]
    fn status_strings_match_the_specification() {
        assert_eq!(PlaybackStatus::Playing.as_str(), "Playing");
        assert_eq!(PlaybackStatus::Paused.as_str(), "Paused");
        assert_eq!(PlaybackStatus::Stopped.as_str(), "Stopped");
    }

    #[test]
    fn updating_state_is_visible_in_the_snapshot() {
        let (server, _) = server();
        server.update(PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(track()),
            length_us: Some(180_000_000),
            ..PlayerState::stopped()
        });
        let state = server.snapshot();
        assert_eq!(state.status, PlaybackStatus::Playing);
        assert!(state.can_control());
        assert!(state.can_seek());
    }

    #[test]
    fn a_stopped_player_cannot_be_controlled() {
        let (server, _) = server();
        assert!(!server.snapshot().can_control());
    }

    #[test]
    fn commands_reach_the_sink() {
        let (server, received) = server();
        let interface = PlayerInterface { server: server.clone() };
        interface.play();
        interface.next();
        interface.previous();
        interface.pause();

        let commands = received.read().clone();
        assert_eq!(commands, vec![Command::Play, Command::Next, Command::Previous, Command::Pause]);
    }

    #[test]
    fn seek_is_ignored_while_stopped() {
        let (server, received) = server();
        let interface = PlayerInterface { server: server.clone() };
        interface.seek(1_000_000);
        assert!(received.read().is_empty(), "MPRIS ignores a seek while stopped");

        server.update(PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(track()),
            ..PlayerState::stopped()
        });
        interface.seek(1_000_000);
        assert_eq!(received.read().clone(), vec![Command::Seek(1_000_000)]);
    }

    #[test]
    fn set_position_for_a_stale_track_is_ignored() {
        let (server, received) = server();
        server.update(PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(track()),
            ..PlayerState::stopped()
        });
        let interface = PlayerInterface { server: server.clone() };

        let stale = ObjectPath::try_from("/dev/sosumi/track/apple_1").expect("path");
        interface.set_position(stale, 5_000_000);
        assert!(received.read().is_empty(), "a stale track id must not move playback");

        let current = ObjectPath::try_from("/dev/sosumi/track/apple_42").expect("path");
        interface.set_position(current, 5_000_000);
        assert_eq!(received.read().clone(), vec![Command::SetPosition(5_000_000)]);
    }

    #[test]
    fn a_negative_position_is_clamped_to_zero() {
        let (server, received) = server();
        server.update(PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(track()),
            ..PlayerState::stopped()
        });
        let interface = PlayerInterface { server: server.clone() };
        let current = ObjectPath::try_from("/dev/sosumi/track/apple_42").expect("path");
        interface.set_position(current, -99);
        assert_eq!(received.read().clone(), vec![Command::SetPosition(0)]);
    }

    #[test]
    fn volume_is_clamped_to_the_valid_range() {
        let (server, _) = server();
        let interface = PlayerInterface { server: server.clone() };
        interface.set_volume(5.0);
        assert_eq!(server.snapshot().volume, 1.0);
        interface.set_volume(-1.0);
        assert_eq!(server.snapshot().volume, 0.0);
    }

    #[test]
    fn loop_status_is_normalised() {
        let (server, _) = server();
        let interface = PlayerInterface { server: server.clone() };
        interface.set_loop_status("Playlist".into());
        assert_eq!(server.snapshot().loop_status, "Playlist");
        interface.set_loop_status("nonsense".into());
        assert_eq!(server.snapshot().loop_status, "None");
    }

    #[test]
    fn metadata_is_empty_when_nothing_is_playing() {
        let (server, _) = server();
        let interface = PlayerInterface { server };
        assert!(interface.metadata().is_empty());
    }

    #[test]
    fn metadata_is_populated_while_playing() {
        let (server, _) = server();
        server.update(PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(track()),
            length_us: Some(180_000_000),
            ..PlayerState::stopped()
        });
        let interface = PlayerInterface { server };
        let metadata = interface.metadata();
        assert!(metadata.contains_key("xesam:title"));
        assert!(metadata.contains_key("mpris:length"));
    }

    #[test]
    fn no_uri_schemes_are_claimed() {
        let (server, _) = server();
        let root = RootInterface;
        assert!(root.supported_uri_schemes().is_empty());
        let _ = server;
    }

    #[test]
    fn the_interface_names_are_the_specified_ones() {
        assert_eq!(interfaces(), ["org.mpris.MediaPlayer2", "org.mpris.MediaPlayer2.Player"]);
    }

    #[test]
    fn bus_availability_follows_the_environment() {
        // The test only asserts the function reflects the environment rather
        // than a constant, which is what callers rely on.
        let before = session_bus_available();
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/tmp/test");
        assert!(session_bus_available());
        if !before {
            std::env::remove_var("DBUS_SESSION_BUS_ADDRESS");
        }
    }
}
