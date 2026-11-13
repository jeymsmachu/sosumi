//! The Rich Presence activity payload.
//!
//! This mirrors Discord's `Activity` shape. Only the fields Sosumi uses are
//! modelled; Discord ignores what it does not recognise, so an unknown field is
//! not a compatibility risk.

use serde::{Deserialize, Serialize};

/// Discord's activity type. `Listening` is the right one for a music player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityType {
    /// "Playing".
    Playing = 0,
    /// "Listening to". This is what a music app should use.
    Listening = 2,
    /// "Watching".
    Watching = 3,
}

/// A timestamp, in milliseconds since the Unix epoch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamps {
    /// When the track started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    /// When the track ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
}

/// One image reference, with an optional hover tooltip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// The image key or an external `mp:` URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    /// Tooltip shown when hovering the large image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

/// A button on the presence card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub url: String,
}

/// What Discord displays.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// Always `Listening` for a player.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<i32>,
    /// The track's title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// The artist, or the album when an artist is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Asset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<Button>>,
    /// The paused indicator Discord shows over the artwork.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub instance: bool,
}

impl Activity {
    /// An activity for a track, with timing for the progress bar.
    ///
    /// `now_ms` is passed in rather than read from the clock so the result is
    /// deterministic and testable.
    #[must_use]
    pub fn listening(
        title: &str,
        subtitle: &str,
        duration_ms: Option<u64>,
        now_ms: i64,
        paused: bool,
    ) -> Self {
        let timestamps = duration_ms.map(|duration| {
            let duration = i64::try_from(duration).unwrap_or(i64::MAX);
            if paused {
                // A paused track shows elapsed time but no end, so Discord does
                // not animate a countdown that is not running.
                Timestamps { start: Some(now_ms), end: None }
            } else {
                Timestamps { start: Some(now_ms), end: Some(now_ms + duration) }
            }
        });

        Self {
            activity_type: Some(ActivityType::Listening as i32),
            details: non_empty(title),
            state: non_empty(subtitle),
            timestamps,
            assets: None,
            buttons: None,
            instance: paused,
        }
    }

    /// Attaches artwork.
    #[must_use]
    pub fn with_artwork(mut self, url: &str, title: &str) -> Self {
        self.assets = Some(Asset {
            large_image: Some(url.to_owned()),
            large_text: non_empty(title),
            small_image: None,
            small_text: None,
        });
        self
    }

    /// Attaches a button.
    #[must_use]
    pub fn with_button(mut self, label: &str, url: &str) -> Self {
        self.buttons
            .get_or_insert_with(Vec::new)
            .push(Button { label: label.to_owned(), url: url.to_owned() });
        self
    }
}

/// `Some` only when the string carries content.
fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_playing_activity_has_a_start_and_an_end() {
        let activity = Activity::listening("Song", "Artist", Some(180_000), 1_000_000, false);
        let timestamps = activity.timestamps.expect("timestamps");
        assert_eq!(timestamps.start, Some(1_000_000));
        assert_eq!(timestamps.end, Some(1_180_000));
    }

    #[test]
    fn a_paused_activity_has_no_end() {
        let activity = Activity::listening("Song", "Artist", Some(180_000), 1_000_000, true);
        let timestamps = activity.timestamps.expect("timestamps");
        assert_eq!(timestamps.start, Some(1_000_000));
        assert_eq!(timestamps.end, None);
        assert!(activity.instance, "paused must set the instance flag");
    }

    #[test]
    fn an_unknown_duration_yields_no_timestamps() {
        let activity = Activity::listening("Song", "Artist", None, 1_000_000, false);
        assert!(activity.timestamps.is_none());
    }

    #[test]
    fn empty_strings_are_omitted_rather_than_sent() {
        let activity = Activity::listening("Song", "   ", None, 0, false);
        assert_eq!(activity.details.as_deref(), Some("Song"));
        assert!(activity.state.is_none(), "a blank artist must not be sent");
    }

    #[test]
    fn the_activity_type_is_listening() {
        let activity = Activity::listening("Song", "Artist", None, 0, false);
        assert_eq!(activity.activity_type, Some(ActivityType::Listening as i32));
        assert_eq!(ActivityType::Listening as i32, 2);
    }

    #[test]
    fn artwork_is_attached_as_the_large_image() {
        let activity = Activity::listening("Song", "Artist", None, 0, false)
            .with_artwork("https://art/632.jpg", "Album");
        let assets = activity.assets.expect("assets");
        assert_eq!(assets.large_image.as_deref(), Some("https://art/632.jpg"));
        assert_eq!(assets.large_text.as_deref(), Some("Album"));
    }

    #[test]
    fn buttons_accumulate() {
        let activity = Activity::listening("Song", "Artist", None, 0, false)
            .with_button("Listen", "https://example/1")
            .with_button("Artist", "https://example/2");
        assert_eq!(activity.buttons.expect("buttons").len(), 2);
    }

    #[test]
    fn absent_fields_are_not_serialised() {
        let activity = Activity::listening("Song", "", None, 0, false);
        let json = serde_json::to_string(&activity).expect("serialise");
        assert!(!json.contains("state"), "a blank field must not appear: {json}");
        assert!(!json.contains("timestamps"));
        assert!(!json.contains("instance"), "a false instance flag must be omitted");
    }

    #[test]
    fn a_full_activity_serialises_with_the_expected_keys() {
        let activity = Activity::listening("Song", "Artist", Some(1_000), 0, false)
            .with_artwork("https://art", "Album");
        let json = serde_json::to_string(&activity).expect("serialise");
        assert!(json.contains("\"type\":2"));
        assert!(json.contains("\"details\":\"Song\""));
        assert!(json.contains("\"state\":\"Artist\""));
        assert!(json.contains("large_image"));
    }
}
