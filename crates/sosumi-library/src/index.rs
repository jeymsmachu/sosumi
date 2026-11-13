//! The library index.
//!
//! The store keeps entries keyed by track id and remembers when it last synced.
//! It is deliberately tolerant: a corrupted or unreadable file yields an empty
//! library rather than an error, because a user would rather re-sync than be
//! locked out of the app.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sosumi_provider_api::{Track, TrackId};

/// What kind of thing an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryKind {
    /// A saved song.
    Song,
    /// A saved album.
    Album,
    /// A saved playlist.
    Playlist,
}

/// One saved item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryEntry {
    /// The id the provider uses for this item.
    pub id: String,
    pub kind: LibraryKind,
    /// A title for display, so the list renders without a fetch.
    pub title: String,
    /// The artist or curator.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subtitle: String,
    /// Artwork URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork_url: Option<String>,
    /// When it was added, in milliseconds since the epoch.
    #[serde(default)]
    pub added_at_ms: i64,
}

impl LibraryEntry {
    /// Builds a song entry from a track.
    #[must_use]
    pub fn from_track(track: &Track, added_at_ms: i64) -> Self {
        Self {
            id: track.id.as_str().to_owned(),
            kind: LibraryKind::Song,
            title: track.title.clone(),
            subtitle: track.artist.name.clone(),
            artwork_url: track.artwork.as_ref().map(|a| a.url.clone()),
            added_at_ms,
        }
    }

    /// True when a search term matches the entry.
    #[must_use]
    fn matches(&self, needle: &str) -> bool {
        self.title.to_lowercase().contains(needle) || self.subtitle.to_lowercase().contains(needle)
    }
}

/// Errors from the library store.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// The index could not be written.
    #[error("could not save the library: {0}")]
    Save(String),
}

/// The persisted shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Persisted {
    entries: BTreeMap<String, LibraryEntry>,
    synced_at_ms: Option<i64>,
}

/// The user's saved music.
#[derive(Debug, Default)]
pub struct Library {
    entries: BTreeMap<String, LibraryEntry>,
    synced_at_ms: Option<i64>,
}

impl Library {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a library from its persisted JSON.
    ///
    /// An unreadable document yields an empty library rather than an error:
    /// re-syncing is cheaper for the user than being blocked by a corrupt file.
    #[must_use]
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str::<Persisted>(json).map_or_else(
            |_| Self::new(),
            |persisted| Self { entries: persisted.entries, synced_at_ms: persisted.synced_at_ms },
        )
    }

    /// Serialises the library.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Save`] when serialisation fails, which cannot
    /// happen for the shapes stored here but is handled rather than unwrapped.
    pub fn to_json(&self) -> Result<String, LibraryError> {
        let persisted =
            Persisted { entries: self.entries.clone(), synced_at_ms: self.synced_at_ms };
        serde_json::to_string(&persisted).map_err(|e| LibraryError::Save(e.to_string()))
    }

    /// Adds or replaces an entry.
    pub fn put(&mut self, entry: LibraryEntry) {
        self.entries.insert(entry.id.clone(), entry);
    }

    /// Adds a track.
    pub fn add_track(&mut self, track: &Track, added_at_ms: i64) {
        self.put(LibraryEntry::from_track(track, added_at_ms));
    }

    /// Removes an entry, returning it when it was present.
    pub fn remove(&mut self, id: &str) -> Option<LibraryEntry> {
        self.entries.remove(id)
    }

    /// True when the id is saved.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Looks up an entry.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&LibraryEntry> {
        self.entries.get(id)
    }

    /// Replaces the whole library.
    pub fn replace(&mut self, entries: impl IntoIterator<Item = LibraryEntry>) {
        self.entries = entries.into_iter().map(|entry| (entry.id.clone(), entry)).collect();
    }

    /// The number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is saved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry, ordered by id.
    pub fn iter(&self) -> impl Iterator<Item = &LibraryEntry> {
        self.entries.values()
    }

    /// Entries of one kind, most recently added first.
    #[must_use]
    pub fn by_kind(&self, kind: LibraryKind) -> Vec<&LibraryEntry> {
        let mut entries: Vec<&LibraryEntry> =
            self.entries.values().filter(|entry| entry.kind == kind).collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.added_at_ms));
        entries
    }

    /// Entries whose title or subtitle matches a term.
    #[must_use]
    pub fn search(&self, term: &str) -> Vec<&LibraryEntry> {
        let needle = term.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut matches: Vec<&LibraryEntry> =
            self.entries.values().filter(|entry| entry.matches(&needle)).collect();
        matches.sort_by_key(|entry| entry.title.to_lowercase());
        matches
    }

    /// The track ids of every saved song, for feeding the player.
    #[must_use]
    pub fn song_ids(&self) -> Vec<TrackId> {
        let mut ids: Vec<&LibraryEntry> =
            self.entries.values().filter(|entry| entry.kind == LibraryKind::Song).collect();
        ids.sort_by_key(|entry| std::cmp::Reverse(entry.added_at_ms));
        ids.into_iter().map(|entry| TrackId(entry.id.clone())).collect()
    }

    /// Records that a sync completed now.
    pub const fn mark_synced(&mut self, at_ms: i64) {
        self.synced_at_ms = Some(at_ms);
    }

    /// When the last sync completed, in milliseconds since the epoch.
    #[must_use]
    pub const fn synced_at_ms(&self) -> Option<i64> {
        self.synced_at_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::{Artist, Artwork, ProviderId};

    fn entry(id: &str, kind: LibraryKind, title: &str, added: i64) -> LibraryEntry {
        LibraryEntry {
            id: id.to_owned(),
            kind,
            title: title.to_owned(),
            subtitle: "Artist".to_owned(),
            artwork_url: None,
            added_at_ms: added,
        }
    }

    fn track(id: &str, title: &str) -> Track {
        Track {
            id: TrackId::new(&ProviderId::from(ProviderId::APPLE), id),
            title: title.to_owned(),
            artist: Artist { id: "a".into(), name: "Radiohead".into(), artwork: None },
            album: None,
            artwork: Some(Artwork { url: "https://art/1.jpg".into(), ..Artwork::default() }),
            duration_ms: None,
            isrc: None,
            track_number: None,
            disc_number: None,
        }
    }

    #[test]
    fn a_new_library_is_empty_and_unsynced() {
        let library = Library::new();
        assert!(library.is_empty());
        assert_eq!(library.len(), 0);
        assert!(library.synced_at_ms().is_none());
    }

    #[test]
    fn putting_the_same_id_twice_replaces_rather_than_duplicates() {
        let mut library = Library::new();
        library.put(entry("1", LibraryKind::Song, "First", 0));
        library.put(entry("1", LibraryKind::Song, "Updated", 0));
        assert_eq!(library.len(), 1);
        assert_eq!(library.get("1").expect("entry").title, "Updated");
    }

    #[test]
    fn a_track_becomes_a_song_entry_with_its_artwork() {
        let mut library = Library::new();
        library.add_track(&track("42", "Weird Fishes"), 1_700_000_000_000);

        let saved = library.get("apple:42").expect("entry");
        assert_eq!(saved.kind, LibraryKind::Song);
        assert_eq!(saved.title, "Weird Fishes");
        assert_eq!(saved.subtitle, "Radiohead");
        assert_eq!(saved.artwork_url.as_deref(), Some("https://art/1.jpg"));
    }

    #[test]
    fn removal_reports_what_was_removed() {
        let mut library = Library::new();
        library.put(entry("1", LibraryKind::Song, "Song", 0));
        assert!(library.remove("1").is_some());
        assert!(library.remove("1").is_none());
        assert!(library.is_empty());
    }

    #[test]
    fn by_kind_filters_and_orders_newest_first() {
        let mut library = Library::new();
        library.put(entry("a", LibraryKind::Song, "Older", 100));
        library.put(entry("b", LibraryKind::Song, "Newer", 200));
        library.put(entry("c", LibraryKind::Album, "An Album", 300));

        let songs = library.by_kind(LibraryKind::Song);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title, "Newer");
        assert_eq!(library.by_kind(LibraryKind::Playlist).len(), 0);
    }

    #[test]
    fn search_matches_title_and_subtitle_ignoring_case() {
        let mut library = Library::new();
        library.put(entry("1", LibraryKind::Song, "Weird Fishes", 0));
        library.put(entry("2", LibraryKind::Song, "Bodysnatchers", 0));

        assert_eq!(library.search("FISHES").len(), 1);
        assert_eq!(library.search("artist").len(), 2, "the subtitle must match too");
        assert_eq!(library.search("nothing").len(), 0);
    }

    #[test]
    fn an_empty_search_term_matches_nothing() {
        let mut library = Library::new();
        library.put(entry("1", LibraryKind::Song, "Song", 0));
        assert!(library.search("   ").is_empty());
    }

    #[test]
    fn song_ids_are_newest_first() {
        let mut library = Library::new();
        library.put(entry("a", LibraryKind::Song, "Older", 100));
        library.put(entry("b", LibraryKind::Song, "Newer", 200));
        library.put(entry("c", LibraryKind::Album, "An Album", 300));

        let ids: Vec<String> = library.song_ids().iter().map(|id| id.as_str().to_owned()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn a_library_round_trips_through_json() {
        let mut library = Library::new();
        library.put(entry("1", LibraryKind::Song, "Song", 50));
        library.mark_synced(1_700_000_000_000);

        let json = library.to_json().expect("serialise");
        let restored = Library::from_json(&json);

        assert_eq!(restored.len(), 1);
        assert_eq!(restored.synced_at_ms(), Some(1_700_000_000_000));
        assert!(restored.contains("1"));
    }

    #[test]
    fn a_corrupt_document_yields_an_empty_library() {
        let restored = Library::from_json("{ this is not json");
        assert!(restored.is_empty());
        assert!(restored.synced_at_ms().is_none());
    }

    #[test]
    fn replace_discards_the_previous_contents() {
        let mut library = Library::new();
        library.put(entry("old", LibraryKind::Song, "Old", 0));
        library.replace([entry("new", LibraryKind::Song, "New", 1)]);

        assert_eq!(library.len(), 1);
        assert!(!library.contains("old"));
        assert!(library.contains("new"));
    }

    #[test]
    fn entries_iterate_in_a_stable_order() {
        let mut library = Library::new();
        library.put(entry("b", LibraryKind::Song, "B", 0));
        library.put(entry("a", LibraryKind::Song, "A", 0));
        let ids: Vec<&str> = library.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"], "a BTreeMap iterates by key");
    }
}
