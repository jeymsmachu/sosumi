//! The signed-in session.
//!
//! A session is the pair of tokens Apple needs: a **developer token** (who the
//! app is) and a **Music-User-Token** (who the subscriber is). This type owns
//! them and the storefront, and persists them through the encrypted store so a
//! restart does not require signing in again.
//!
//! It deliberately does not know how sign-in happens; that belongs to the UI,
//! which drives the native flow. It only holds and validates what it is given.

use serde::{Deserialize, Serialize};
use sosumi_storage::SecretStore;

/// Store keys, namespaced so the store can hold other things too.
const KEY_MUSIC_USER_TOKEN: &str = "apple.music_user_token";
const KEY_DEVELOPER_TOKEN: &str = "apple.developer_token";
const KEY_STOREFRONT: &str = "apple.storefront";

/// A signed-in Apple Music session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// The subscriber's Music-User-Token.
    pub music_user_token: Option<String>,
    /// The app's developer token.
    pub developer_token: Option<String>,
    /// The storefront, for example `us`.
    pub storefront: String,
}

impl Session {
    /// A session with the default storefront and no tokens.
    #[must_use]
    pub fn new() -> Self {
        Self { storefront: "us".to_owned(), ..Self::default() }
    }

    /// True when the subscriber token is present.
    #[must_use]
    pub fn is_signed_in(&self) -> bool {
        self.music_user_token.as_ref().is_some_and(|token| !token.is_empty())
    }

    /// True when both tokens are present, which is what playback needs.
    #[must_use]
    pub fn is_playable(&self) -> bool {
        self.is_signed_in() && self.developer_token.as_ref().is_some_and(|t| !t.is_empty())
    }

    /// Records the subscriber token.
    pub fn set_music_user_token(&mut self, token: impl Into<String>) {
        self.music_user_token = Some(token.into());
    }

    /// Records the developer token.
    pub fn set_developer_token(&mut self, token: impl Into<String>) {
        self.developer_token = Some(token.into());
    }

    /// Sets the storefront, ignoring an empty value.
    pub fn set_storefront(&mut self, storefront: impl Into<String>) {
        let storefront = storefront.into();
        if !storefront.is_empty() {
            self.storefront = storefront;
        }
    }

    /// Clears the tokens, keeping the storefront.
    pub fn sign_out(&mut self) {
        self.music_user_token = None;
        self.developer_token = None;
    }

    /// Loads a session from the encrypted store.
    ///
    /// A store that cannot be read yields a fresh, signed-out session rather
    /// than an error: the user simply signs in again.
    #[must_use]
    pub fn load(store: &SecretStore) -> Self {
        let mut session = Self::new();
        if let Some(token) = store.get(KEY_MUSIC_USER_TOKEN) {
            session.music_user_token = Some(token.to_owned());
        }
        if let Some(token) = store.get(KEY_DEVELOPER_TOKEN) {
            session.developer_token = Some(token.to_owned());
        }
        if let Some(storefront) = store.get(KEY_STOREFRONT) {
            storefront.clone_into(&mut session.storefront);
        }
        session
    }

    /// Persists the session to the encrypted store.
    ///
    /// # Errors
    ///
    /// Propagates a failure to write the store.
    pub fn save(&self, store: &mut SecretStore) -> Result<(), sosumi_storage::StoreError> {
        match &self.music_user_token {
            Some(token) => store.set(KEY_MUSIC_USER_TOKEN, token.clone()),
            None => {
                store.remove(KEY_MUSIC_USER_TOKEN);
            }
        }
        match &self.developer_token {
            Some(token) => store.set(KEY_DEVELOPER_TOKEN, token.clone()),
            None => {
                store.remove(KEY_DEVELOPER_TOKEN);
            }
        }
        store.set(KEY_STOREFRONT, self.storefront.clone());
        store.save()
    }

    /// Extracts the Music-User-Token from a cookie header, which is how the web
    /// sign-in surfaces it.
    #[must_use]
    pub fn from_cookie_header(cookies: &str) -> Option<String> {
        cookies
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == "media-user-token")
            .map(|(_, value)| value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_security::Sealer;

    fn store() -> (tempfile::TempDir, SecretStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealer = Sealer::new(&[5u8; 32], "test").expect("sealer");
        let store = SecretStore::open(dir.path().join("secrets.json"), sealer).expect("open");
        (dir, store)
    }

    #[test]
    fn a_fresh_session_is_not_signed_in() {
        let session = Session::new();
        assert!(!session.is_signed_in());
        assert!(!session.is_playable());
        assert_eq!(session.storefront, "us");
    }

    #[test]
    fn playable_requires_both_tokens() {
        let mut session = Session::new();
        session.set_music_user_token("mut");
        assert!(session.is_signed_in());
        assert!(!session.is_playable());

        session.set_developer_token("dev");
        assert!(session.is_playable());
    }

    #[test]
    fn an_empty_token_does_not_count_as_signed_in() {
        let mut session = Session::new();
        session.set_music_user_token("");
        assert!(!session.is_signed_in());
    }

    #[test]
    fn sign_out_clears_tokens_but_keeps_the_storefront() {
        let mut session = Session::new();
        session.set_music_user_token("mut");
        session.set_developer_token("dev");
        session.set_storefront("gb");

        session.sign_out();
        assert!(!session.is_signed_in());
        assert_eq!(session.storefront, "gb");
    }

    #[test]
    fn an_empty_storefront_is_ignored() {
        let mut session = Session::new();
        session.set_storefront("");
        assert_eq!(session.storefront, "us");
    }

    #[test]
    fn round_trips_through_the_encrypted_store() {
        let (_dir, mut store) = store();
        let mut session = Session::new();
        session.set_music_user_token("mut-secret");
        session.set_developer_token("dev-secret");
        session.set_storefront("jp");
        session.save(&mut store).expect("save");

        let loaded = Session::load(&store);
        assert_eq!(loaded, session);
        assert!(loaded.is_playable());
    }

    #[test]
    fn signing_out_persists_the_removal() {
        let (_dir, mut store) = store();
        let mut session = Session::new();
        session.set_music_user_token("mut");
        session.save(&mut store).expect("save");

        session.sign_out();
        session.save(&mut store).expect("save");

        let loaded = Session::load(&store);
        assert!(!loaded.is_signed_in());
    }

    #[test]
    fn a_missing_store_yields_a_blank_session() {
        let (_dir, store) = store();
        let loaded = Session::load(&store);
        assert!(!loaded.is_signed_in());
    }

    #[test]
    fn music_user_token_is_read_from_cookies() {
        let cookies = "foo=bar; media-user-token=ABC123; other=baz";
        assert_eq!(Session::from_cookie_header(cookies).as_deref(), Some("ABC123"));
        assert_eq!(Session::from_cookie_header("foo=bar"), None);
    }
}
