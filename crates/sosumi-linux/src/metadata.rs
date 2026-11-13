//! The MPRIS2 metadata dictionary.
//!
//! MPRIS carries track information as a D-Bus `a{sv}` — a map of string keys to
//! variant values. The keys are defined by the specification; getting one wrong
//! means an applet shows a blank field, so they are named here and tested.

use std::collections::HashMap;

use sosumi_provider_api::Track;

use zbus::zvariant::{OwnedValue, Value};

/// The D-Bus interface name for the player.
pub const INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

/// The root interface name.
pub const ROOT_INTERFACE: &str = "org.mpris.MediaPlayer2";

/// The object path MPRIS uses.
pub const OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";

/// The bus name prefix every MPRIS player uses.
pub const BUS_NAME_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// The `mpris:trackid` value.
///
/// MPRIS requires this to look like a D-Bus object path. A track that has no
/// path of its own gets one derived from its id, since applets key their state
/// on it and a repeated value makes them treat two tracks as one.
#[must_use]
pub fn track_object_path(track_id: &str) -> String {
    let sanitized: String =
        track_id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("/dev/sosumi/track/{sanitized}")
}

/// Builds the metadata dictionary for a track.
///
/// `length_us` is the duration in microseconds, which is the unit MPRIS uses;
/// passing milliseconds is an easy and invisible mistake, so the parameter is
/// named for its unit.
#[must_use]
pub fn metadata_for(track: &Track, length_us: Option<i64>) -> HashMap<String, OwnedValue> {
    let mut map = HashMap::new();

    insert_string(&mut map, "mpris:trackid", &track_object_path(track.id.as_str()));
    insert_string(&mut map, "xesam:title", &track.title);

    if !track.artist.name.is_empty() {
        insert_string_array(&mut map, "xesam:artist", std::slice::from_ref(&track.artist.name));
    }
    if let Some(album) = &track.album {
        insert_string(&mut map, "xesam:album", &album.title);
    }
    if let Some(isrc) = &track.isrc {
        insert_string(&mut map, "xesam:contentCreated", isrc);
    }
    if let Some(number) = track.track_number {
        insert_int(&mut map, "xesam:trackNumber", i32::try_from(number).unwrap_or(0));
    }
    if let Some(artwork) = &track.artwork {
        insert_string(&mut map, "mpris:artUrl", &artwork.url);
    }
    if let Some(length) = length_us {
        insert_i64(&mut map, "mpris:length", length);
    }

    map
}

fn insert_string(map: &mut HashMap<String, OwnedValue>, key: &str, value: &str) {
    if let Ok(owned) = OwnedValue::try_from(Value::from(value)) {
        map.insert(key.to_owned(), owned);
    }
}

fn insert_string_array(map: &mut HashMap<String, OwnedValue>, key: &str, values: &[String]) {
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    if let Ok(owned) = OwnedValue::try_from(Value::from(refs)) {
        map.insert(key.to_owned(), owned);
    }
}

fn insert_int(map: &mut HashMap<String, OwnedValue>, key: &str, value: i32) {
    if let Ok(owned) = OwnedValue::try_from(Value::from(value)) {
        map.insert(key.to_owned(), owned);
    }
}

fn insert_i64(map: &mut HashMap<String, OwnedValue>, key: &str, value: i64) {
    if let Ok(owned) = OwnedValue::try_from(Value::from(value)) {
        map.insert(key.to_owned(), owned);
    }
}

/// Converts a millisecond duration to the microseconds MPRIS wants.
///
/// Saturates rather than wrapping: MPRIS uses a signed 64-bit microsecond count,
/// and an absurd input must not produce a negative position.
#[must_use]
#[allow(clippy::cast_possible_wrap)] // Guarded by the bound check below.
pub const fn ms_to_us(ms: u64) -> i64 {
    match ms.checked_mul(1000) {
        Some(value) if value <= i64::MAX as u64 => value as i64,
        _ => i64::MAX,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::{Album, Artist, Artwork, ProviderId, TrackId};

    fn track() -> Track {
        Track {
            id: TrackId::new(&ProviderId::from(ProviderId::APPLE), "1440833098"),
            title: "Weird Fishes".into(),
            artist: Artist { id: "radiohead".into(), name: "Radiohead".into(), artwork: None },
            album: Some(Album {
                id: "1".into(),
                title: "In Rainbows".into(),
                artist: Artist { id: "radiohead".into(), name: "Radiohead".into(), artwork: None },
                artwork: None,
                release_date: None,
                track_count: None,
            }),
            artwork: Some(Artwork { url: "https://art/632.jpg".into(), ..Artwork::default() }),
            duration_ms: Some(318_186),
            isrc: Some("GBSTK0700041".into()),
            track_number: Some(3),
            disc_number: Some(1),
        }
    }

    #[test]
    fn track_id_becomes_a_valid_object_path() {
        let path = track_object_path("apple:1440833098");
        assert!(path.starts_with('/'));
        assert!(!path.contains(':'), "a D-Bus path cannot contain a colon");
        assert_eq!(path, "/dev/sosumi/track/apple_1440833098");
    }

    #[test]
    fn distinct_tracks_get_distinct_paths() {
        assert_ne!(track_object_path("apple:1"), track_object_path("apple:2"));
    }

    #[test]
    fn metadata_carries_the_fields_applets_read() {
        let metadata = metadata_for(&track(), Some(ms_to_us(318_186)));
        for key in [
            "mpris:trackid",
            "mpris:length",
            "mpris:artUrl",
            "xesam:title",
            "xesam:artist",
            "xesam:album",
        ] {
            assert!(metadata.contains_key(key), "missing {key}");
        }
    }

    #[test]
    fn duration_is_converted_to_microseconds() {
        assert_eq!(ms_to_us(1), 1_000);
        assert_eq!(ms_to_us(318_186), 318_186_000);
    }

    #[test]
    fn an_absurd_duration_saturates_rather_than_wrapping() {
        assert_eq!(ms_to_us(u64::MAX), i64::MAX);
    }

    #[test]
    fn a_track_without_optional_fields_still_produces_metadata() {
        let bare = Track {
            id: TrackId::new(&ProviderId::from(ProviderId::LOCAL), "3"),
            title: "Untitled".into(),
            artist: Artist { id: String::new(), name: String::new(), artwork: None },
            album: None,
            artwork: None,
            duration_ms: None,
            isrc: None,
            track_number: None,
            disc_number: None,
        };
        let metadata = metadata_for(&bare, None);
        assert!(metadata.contains_key("mpris:trackid"));
        assert!(metadata.contains_key("xesam:title"));
        assert!(!metadata.contains_key("xesam:artist"), "an unnamed artist must be omitted");
        assert!(!metadata.contains_key("mpris:length"));
    }
}
