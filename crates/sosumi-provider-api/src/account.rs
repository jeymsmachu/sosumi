//! Account credentials, as a provider sees them.
//!
//! Providers must not depend on the core's session type: the core knows about
//! persistence and events, and a provider should not. This is the narrow view a
//! provider needs, so a new provider can be written without touching session
//! handling.

/// The credentials a provider needs to serve a signed-in user.
///
/// Every field is optional because "signed out" is a legitimate state: a
/// provider reports [`crate::ProviderError::NotAuthenticated`] when it needs one
/// that is absent, which the UI turns into a sign-in prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// The subscriber's token, which identifies the person.
    pub user_token: Option<String>,
    /// The application's token, which identifies the client.
    pub developer_token: Option<String>,
    /// The storefront to serve from, for example `us`.
    pub storefront: String,
}

impl Account {
    /// An account with no credentials.
    #[must_use]
    pub fn signed_out() -> Self {
        Self { storefront: "us".to_owned(), ..Self::default() }
    }

    /// True when the subscriber token is present.
    #[must_use]
    pub fn is_signed_in(&self) -> bool {
        self.user_token.as_ref().is_some_and(|token| !token.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_out_account_has_no_token() {
        let account = Account::signed_out();
        assert!(!account.is_signed_in());
        assert_eq!(account.storefront, "us");
    }

    #[test]
    fn an_empty_token_does_not_count_as_signed_in() {
        let account = Account { user_token: Some(String::new()), ..Account::default() };
        assert!(!account.is_signed_in());
    }

    #[test]
    fn a_present_token_counts_as_signed_in() {
        let account = Account { user_token: Some("token".into()), ..Account::default() };
        assert!(account.is_signed_in());
    }
}
