//! The event bus.
//!
//! One broadcast channel carries everything the UI and the integrations need to
//! react to. Subscribers are passive: they never call back into the core, so a
//! slow observer cannot stall playback. A subscriber that falls behind simply
//! misses events, which is the right trade for progress and state notifications.

use tokio::sync::broadcast;

use sosumi_provider_api::TrackId;

/// Something that happened, worth telling the UI about.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A track started playing.
    TrackChanged {
        /// The track now loaded.
        track: TrackId,
        /// Display title, so the UI does not have to re-fetch.
        title: String,
        /// Display artist.
        artist: String,
        /// Duration in milliseconds, when known.
        duration_ms: Option<u64>,
        /// Artwork URL, when known.
        artwork_url: Option<String>,
    },
    /// Playback started, or resumed.
    Playing,
    /// Playback paused.
    Paused,
    /// Playback stopped and the track unloaded.
    Stopped,
    /// Position advanced. Emitted at a low rate, not per frame.
    Position {
        /// Elapsed milliseconds.
        position_ms: u64,
    },
    /// The track reached its end.
    Ended,
    /// The queue changed.
    QueueChanged {
        /// Number of tracks in the queue.
        length: usize,
    },
    /// The active quality changed, for example after a capability was installed.
    QualityChanged {
        /// Machine-readable quality name.
        quality: String,
    },
    /// Sign-in state changed.
    AuthChanged {
        /// True when signed in.
        signed_in: bool,
    },
    /// A capability's state changed, so Settings should refresh.
    CapabilitiesChanged,
    /// Something failed, with a message fit for display.
    Error {
        /// A short, user-facing description.
        message: String,
    },
}

/// The publishing half of the bus.
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

/// The subscribing half of the bus.
#[derive(Debug)]
pub struct EventReceiver {
    inner: broadcast::Receiver<Event>,
}

impl EventBus {
    /// Creates a bus with room for `capacity` buffered events.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publishes an event.
    ///
    /// Returns the number of subscribers that received it. Zero is normal: the
    /// UI may not be listening yet.
    pub fn publish(&self, event: Event) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribes to the bus.
    #[must_use]
    pub fn subscribe(&self) -> EventReceiver {
        EventReceiver { inner: self.sender.subscribe() }
    }

    /// The number of active subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl EventReceiver {
    /// Waits for the next event.
    ///
    /// Returns `None` when the bus is gone. A lagged receiver resumes at the
    /// oldest retained event with [`EventReceiver::next_event`] returning the
    /// most recent one instead, which is the desirable behaviour for UI state.
    pub async fn next(&mut self) -> Option<Event> {
        loop {
            match self.inner.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(target: "sosumi::core", skipped, "event receiver lagged");
                    // Resume from the newest retained event rather than stopping.
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Alias for [`EventReceiver::next`], named for readability at call sites.
    pub async fn next_event(&mut self) -> Option<Event> {
        self.next().await
    }

    /// Returns the next event without waiting, or `None` when none is queued.
    ///
    /// Used where a caller already has a reason to run — a UI frame, an
    /// integration poll — and only wants to drain what is pending.
    pub fn try_next_event(&mut self) -> Option<Event> {
        loop {
            match self.inner.try_recv() {
                Ok(event) => return Some(event),
                Err(broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(
                    broadcast::error::TryRecvError::Empty | broadcast::error::TryRecvError::Closed,
                ) => {
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> Event {
        Event::Playing
    }

    #[tokio::test]
    async fn subscribers_receive_published_events() {
        let bus = EventBus::new(8);
        let mut receiver = bus.subscribe();
        assert_eq!(bus.publish(sample_event()), 1);
        assert_eq!(receiver.next().await, Some(sample_event()));
    }

    #[tokio::test]
    async fn publishing_without_subscribers_is_not_an_error() {
        let bus = EventBus::new(8);
        assert_eq!(bus.publish(sample_event()), 0);
    }

    #[tokio::test]
    async fn every_subscriber_gets_a_copy() {
        let bus = EventBus::new(8);
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();
        assert_eq!(bus.publish(Event::Paused), 2);
        assert_eq!(first.next().await, Some(Event::Paused));
        assert_eq!(second.next().await, Some(Event::Paused));
    }

    #[tokio::test]
    async fn a_dropped_bus_ends_the_stream() {
        let bus = EventBus::new(8);
        let mut receiver = bus.subscribe();
        drop(bus);
        assert_eq!(receiver.next().await, None);
    }

    #[test]
    fn default_capacity_is_generous() {
        assert_eq!(EventBus::default().subscriber_count(), 0);
    }
}
