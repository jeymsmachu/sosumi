//! The play queue.
//!
//! The queue separates two things that are easy to conflate: the **order the
//! user built** (`items`), and the **order we play in** (`order`). Shuffling
//! permutes `order` only, so the user's list survives and toggling shuffle off
//! restores it exactly. That is the behaviour every music player is judged on.

use rand::seq::SliceRandom;
use rand::thread_rng;

use sosumi_provider_api::TrackId;

/// How the queue repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    /// Stop at the end.
    #[default]
    Off,
    /// Repeat the whole queue.
    All,
    /// Repeat the current track.
    One,
}

/// An ordered list of tracks with an independent play order.
#[derive(Debug, Clone, Default)]
pub struct Queue {
    items: Vec<TrackId>,
    /// Indices into `items`, in play order.
    order: Vec<usize>,
    /// Position within `order`.
    cursor: usize,
    repeat: RepeatMode,
    shuffled: bool,
}

impl Queue {
    /// An empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a queue from tracks, starting at `start`.
    #[must_use]
    pub fn from_tracks(items: Vec<TrackId>, start: usize) -> Self {
        let len = items.len();
        let order = (0..len).collect();
        Self {
            items,
            order,
            cursor: start.min(len.saturating_sub(1)),
            repeat: RepeatMode::Off,
            shuffled: false,
        }
    }

    /// Replaces the contents.
    pub fn set_items(&mut self, items: Vec<TrackId>) {
        self.items = items;
        self.shuffled = false;
        self.order = (0..self.items.len()).collect();
        self.cursor = 0;
    }

    /// Appends a track.
    pub fn push(&mut self, track: TrackId) {
        self.items.push(track);
        if !self.shuffled {
            self.order.push(self.items.len() - 1);
        }
    }

    /// Number of tracks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The track at the current position.
    #[must_use]
    pub fn current(&self) -> Option<&TrackId> {
        self.order.get(self.cursor).and_then(|index| self.items.get(*index))
    }

    /// The index of the current track within `items`, for "now playing in this
    /// list" highlighting.
    #[must_use]
    pub fn current_index(&self) -> Option<usize> {
        self.order.get(self.cursor).copied()
    }

    /// The tracks in play order.
    pub fn iter(&self) -> impl Iterator<Item = &TrackId> {
        self.order.iter().filter_map(|index| self.items.get(*index))
    }

    /// Sets the repeat mode.
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// The repeat mode.
    #[must_use]
    pub const fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    /// True when shuffle is on.
    #[must_use]
    pub const fn is_shuffled(&self) -> bool {
        self.shuffled
    }

    /// Enables shuffle.
    ///
    /// The current track is moved to the front of the new order so shuffling
    /// mid-playback does not cut the song that is playing.
    pub fn shuffle(&mut self) {
        if self.shuffled || self.items.is_empty() {
            return;
        }
        let current = self.order.get(self.cursor).copied();
        self.order = (0..self.items.len()).collect();
        self.order.shuffle(&mut thread_rng());

        if let Some(index) = current {
            if let Some(position) = self.order.iter().position(|i| *i == index) {
                self.order.swap(0, position);
            }
            self.cursor = 0;
        }
        self.shuffled = true;
    }

    /// Disables shuffle, restoring the user's original order and keeping the
    /// same track playing.
    pub fn unshuffle(&mut self) {
        if !self.shuffled {
            return;
        }
        let current = self.order.get(self.cursor).copied();
        self.order = (0..self.items.len()).collect();
        self.cursor = current.unwrap_or(0);
        self.shuffled = false;
    }

    /// Advances to the next track.
    ///
    /// Returns `None` at the end of the queue when repeat is off, without
    /// wrapping. This keeps the caller's "did playback stop?" decision explicit.
    /// Named `advance` rather than `next` so it is never confused with
    /// `Iterator::next` at a call site.
    pub fn advance(&mut self) -> Option<&TrackId> {
        if self.is_empty() {
            return None;
        }

        if self.repeat == RepeatMode::One {
            return self.current();
        }

        if self.cursor + 1 < self.order.len() {
            self.cursor += 1;
            return self.current();
        }

        match self.repeat {
            RepeatMode::All => {
                self.cursor = 0;
                self.current()
            }
            _ => None,
        }
    }

    /// Moves to the previous track.
    pub fn retreat(&mut self) -> Option<&TrackId> {
        if self.is_empty() {
            return None;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        } else if self.repeat == RepeatMode::All {
            self.cursor = self.order.len() - 1;
        }
        self.current()
    }

    /// Jumps to a position in play order.
    pub fn jump(&mut self, position: usize) -> Option<&TrackId> {
        if position >= self.order.len() {
            return None;
        }
        self.cursor = position;
        self.current()
    }

    /// Jumps to a track by id, wherever it sits in `items`.
    pub fn jump_to_track(&mut self, track: &TrackId) -> Option<&TrackId> {
        let item_index = self.items.iter().position(|item| item == track)?;
        let order_index = self.order.iter().position(|i| *i == item_index)?;
        self.cursor = order_index;
        self.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::ProviderId;

    fn tracks(count: usize) -> Vec<TrackId> {
        (0..count)
            .map(|i| TrackId::new(&ProviderId::from(ProviderId::APPLE), i.to_string()))
            .collect()
    }

    #[test]
    fn empty_queue_yields_nothing() {
        let mut queue = Queue::new();
        assert!(queue.is_empty());
        assert!(queue.current().is_none());
        assert!(queue.advance().is_none());
        assert!(queue.retreat().is_none());
    }

    #[test]
    fn starts_at_the_requested_track() {
        let queue = Queue::from_tracks(tracks(5), 2);
        assert_eq!(queue.current_index(), Some(2));
        assert_eq!(queue.current(), Some(&tracks(5)[2]));
    }

    #[test]
    fn start_beyond_the_end_is_clamped() {
        let queue = Queue::from_tracks(tracks(3), 99);
        assert_eq!(queue.current_index(), Some(2));
    }

    #[test]
    fn next_walks_forward_in_order() {
        let mut queue = Queue::from_tracks(tracks(3), 0);
        assert_eq!(queue.current_index(), Some(0));
        queue.advance();
        assert_eq!(queue.current_index(), Some(1));
        queue.advance();
        assert_eq!(queue.current_index(), Some(2));
    }

    #[test]
    fn next_advances_then_reports_the_end() {
        let mut queue = Queue::from_tracks(tracks(3), 0);
        assert!(queue.advance().is_some());
        assert!(queue.advance().is_some());
        assert!(queue.advance().is_none(), "repeat off must not wrap");
        assert_eq!(queue.current_index(), Some(2));
    }

    #[test]
    fn repeat_all_wraps_to_the_start() {
        let mut queue = Queue::from_tracks(tracks(2), 0);
        queue.set_repeat(RepeatMode::All);
        queue.advance();
        assert!(queue.advance().is_some());
        assert_eq!(queue.current_index(), Some(0));
    }

    #[test]
    fn repeat_one_stays_on_the_same_track() {
        let mut queue = Queue::from_tracks(tracks(3), 1);
        queue.set_repeat(RepeatMode::One);
        assert_eq!(queue.current_index(), Some(1));
        queue.advance();
        assert_eq!(queue.current_index(), Some(1));
    }

    #[test]
    fn previous_stops_at_the_start_unless_repeating() {
        let mut queue = Queue::from_tracks(tracks(3), 0);
        queue.retreat();
        assert_eq!(queue.current_index(), Some(0));

        queue.set_repeat(RepeatMode::All);
        queue.retreat();
        assert_eq!(queue.current_index(), Some(2));
    }

    #[test]
    fn shuffle_keeps_all_tracks_and_holds_the_current_one() {
        let mut queue = Queue::from_tracks(tracks(50), 10);
        let was_playing = queue.current().cloned();

        queue.shuffle();
        assert!(queue.is_shuffled());
        assert_eq!(queue.len(), 50);
        assert_eq!(queue.current(), was_playing.as_ref(), "current track must not change");
        assert_eq!(queue.current_index(), Some(10), "shuffle must not desync the index");
    }

    #[test]
    fn unshuffle_restores_the_authors_order() {
        let mut queue = Queue::from_tracks(tracks(50), 7);
        queue.shuffle();
        queue.unshuffle();
        assert!(!queue.is_shuffled());
        assert_eq!(queue.current_index(), Some(7));
        assert_eq!(queue.iter().next(), Some(&tracks(50)[0]));
    }

    #[test]
    fn shuffling_twice_is_idempotent() {
        let mut queue = Queue::from_tracks(tracks(20), 3);
        queue.shuffle();
        let after_first: Vec<_> = queue.iter().cloned().collect();
        queue.shuffle();
        let after_second: Vec<_> = queue.iter().cloned().collect();
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn shuffle_on_an_empty_queue_does_nothing() {
        let mut queue = Queue::new();
        queue.shuffle();
        assert!(!queue.is_shuffled());
    }

    #[test]
    fn jump_to_track_finds_it_in_play_order() {
        let all = tracks(5);
        let mut queue = Queue::from_tracks(all.clone(), 0);
        let target = all[3].clone();
        assert_eq!(queue.jump_to_track(&target), Some(&target));
        assert_eq!(queue.current_index(), Some(3));
    }

    #[test]
    fn jump_out_of_range_is_rejected() {
        let mut queue = Queue::from_tracks(tracks(3), 0);
        assert!(queue.jump(99).is_none());
        assert_eq!(queue.current_index(), Some(0));
    }

    #[test]
    fn set_items_resets_to_the_first_track() {
        let mut queue = Queue::from_tracks(tracks(9), 5);
        queue.set_items(tracks(2));
        assert_eq!(queue.current_index(), Some(0));
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn push_appends_in_order_when_not_shuffled() {
        let mut queue = Queue::new();
        for track in tracks(3) {
            queue.push(track);
        }
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.iter().count(), 3);
    }

    #[test]
    fn a_missing_track_cannot_be_jumped_to() {
        let mut queue = Queue::from_tracks(tracks(3), 0);
        let absent = TrackId::new(&ProviderId::from(ProviderId::APPLE), "absent");
        assert!(queue.jump_to_track(&absent).is_none());
    }
}
