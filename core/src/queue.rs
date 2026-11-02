//! Deterministic playback queue.
//!
//! The queue is platform-neutral. Platform adapters mirror it into MusicKit or
//! the compatibility player, but this type remains the source of truth for
//! ordering, repeat, and the current index.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::model::Track;

/// How playback should proceed at the end of the queue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    /// Stop at the end.
    #[default]
    Off,
    /// Wrap to the beginning.
    All,
    /// Repeat the current track.
    One,
}

/// A queue of tracks with a current position.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayQueue {
    entries: Vec<Track>,
    current: Option<usize>,
    repeat: RepeatMode,
}

impl PlayQueue {
    /// Creates an empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a queue from tracks and selects the first track.
    #[must_use]
    pub fn with_tracks(entries: Vec<Track>) -> Self {
        let current = if entries.is_empty() { None } else { Some(0) };
        Self {
            entries,
            current,
            repeat: RepeatMode::Off,
        }
    }

    /// The queued tracks.
    #[must_use]
    pub fn entries(&self) -> &[Track] {
        &self.entries
    }

    /// The number of queued tracks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The current track, if any.
    #[must_use]
    pub fn current(&self) -> Option<&Track> {
        self.current.and_then(|index| self.entries.get(index))
    }

    /// The current index, if any.
    #[must_use]
    pub const fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// The active repeat mode.
    #[must_use]
    pub const fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    /// Replaces the queue and selects the first track.
    pub fn replace(&mut self, entries: Vec<Track>) {
        self.entries = entries;
        self.current = if self.entries.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Appends a track. If the queue was empty, the new track becomes current.
    pub fn enqueue(&mut self, track: Track) {
        self.entries.push(track);
        if self.current.is_none() {
            self.current = Some(0);
        }
    }

    /// Selects an explicit queue index.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::IndexOutOfBounds`] when `index` is invalid.
    pub fn seek(&mut self, index: usize) -> Result<()> {
        if index >= self.entries.len() {
            return Err(CoreError::IndexOutOfBounds {
                index,
                len: self.entries.len(),
            });
        }
        self.current = Some(index);
        Ok(())
    }

    /// Sets the repeat mode.
    pub const fn set_repeat(&mut self, repeat: RepeatMode) {
        self.repeat = repeat;
    }

    /// Advances to the next track and returns its index.
    ///
    /// Returns `None` when playback should stop.
    pub fn advance(&mut self) -> Option<usize> {
        let current = self.current?;
        if self.entries.is_empty() {
            return None;
        }

        let next = match self.repeat {
            RepeatMode::One => current,
            RepeatMode::All => (current + 1) % self.entries.len(),
            RepeatMode::Off => current.checked_add(1).filter(|i| *i < self.entries.len())?,
        };

        self.current = Some(next);
        Some(next)
    }

    /// Moves to the previous track and returns its index.
    ///
    /// Returns `None` when already at the beginning and repeat is off.
    pub fn previous(&mut self) -> Option<usize> {
        let current = self.current?;
        if self.entries.is_empty() {
            return None;
        }

        let previous = match self.repeat {
            RepeatMode::One => current,
            RepeatMode::All => {
                if current == 0 {
                    self.entries.len() - 1
                } else {
                    current - 1
                }
            }
            RepeatMode::Off => current.checked_sub(1)?,
        };

        self.current = Some(previous);
        Some(previous)
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayQueue, RepeatMode};
    use crate::model::Track;

    fn track(id: &str) -> Track {
        Track::new(id, format!("Track {id}"), "Artist", 1_000)
    }

    #[test]
    fn empty_queue_has_no_current_track() {
        let queue = PlayQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.current(), None);
        assert_eq!(queue.current_index(), None);
    }

    #[test]
    fn advance_stops_at_end_without_repeat() {
        let mut queue = PlayQueue::with_tracks(vec![track("a"), track("b")]);

        assert_eq!(queue.advance(), Some(1));
        assert_eq!(queue.advance(), None);
        assert_eq!(queue.current().map(|entry| entry.id.as_str()), Some("b"));
    }

    #[test]
    fn repeat_all_wraps_both_directions() {
        let mut queue = PlayQueue::with_tracks(vec![track("a"), track("b")]);
        queue.set_repeat(RepeatMode::All);

        assert_eq!(queue.advance(), Some(1));
        assert_eq!(queue.advance(), Some(0));
        assert_eq!(queue.previous(), Some(1));
    }

    #[test]
    fn repeat_one_stays_on_current_track() {
        let mut queue = PlayQueue::with_tracks(vec![track("a"), track("b")]);
        queue.set_repeat(RepeatMode::One);

        assert_eq!(queue.advance(), Some(0));
        assert_eq!(queue.previous(), Some(0));
    }

    #[test]
    fn seek_rejects_out_of_bounds() {
        let mut queue = PlayQueue::with_tracks(vec![track("a")]);
        let error = queue.seek(1).expect_err("index 1 must be rejected");

        assert_eq!(
            error,
            crate::error::CoreError::IndexOutOfBounds { index: 1, len: 1 }
        );
    }

    #[test]
    fn enqueue_on_empty_queue_selects_first_track() {
        let mut queue = PlayQueue::new();
        queue.enqueue(track("a"));

        assert_eq!(queue.current_index(), Some(0));
        assert_eq!(queue.len(), 1);
    }
}
