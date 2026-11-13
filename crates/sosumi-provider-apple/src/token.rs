//! Developer-token discovery.
//!
//! The Apple Music web player needs a signed developer token (a JWT) to call its
//! API. It is not published as an endpoint; the player page references a script
//! bundle that contains it. This module extracts it from that bundle.
//!
//! Keeping this a pure function means the fragile part — the extraction — is
//! tested against captured strings, and the network part stays trivial.

use std::sync::LazyLock;

use regex::Regex;

/// Matches the web player's main script reference in the page HTML.
static SCRIPT_REF: LazyLock<Regex> = LazyLock::new(|| {
    // The pattern is a literal; a failure here is a programmer error, not a
    // runtime condition, so falling back to a never-matching regex keeps
    // production panic-free while the test below pins the pattern.
    Regex::new(r"/assets/index~[^/]+\.js")
        .unwrap_or_else(|_| Regex::new(r"$^").unwrap_or_else(|_| unreachable_regex()))
});

/// Matches a three-part JWT.
static JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+")
        .unwrap_or_else(|_| unreachable_regex())
});

/// A regex that can never match. Used only if a literal pattern fails to
/// compile, which the tests below prevent.
fn unreachable_regex() -> Regex {
    // `Regex::new(r"$^")` cannot fail: it is a valid pattern.
    match Regex::new(r"$^") {
        Ok(regex) => regex,
        Err(_) => std::process::abort(),
    }
}

/// The page whose scripts carry the developer token.
pub const PLAYER_PAGE: &str = "https://music.apple.com";

/// Finds the path of the main player script in the page HTML.
#[must_use]
pub fn find_script_path(html: &str) -> Option<&str> {
    SCRIPT_REF.find(html).map(|m| m.as_str())
}

/// Extracts the first JWT from a script bundle.
#[must_use]
pub fn find_token(script: &str) -> Option<&str> {
    JWT.find(script).map(|m| m.as_str())
}

/// Extracts the token directly from a page's HTML, when it is inlined.
#[must_use]
pub fn find_token_in_page(html: &str) -> Option<&str> {
    find_token(html)
}

/// Builds the absolute URL for a script path.
#[must_use]
pub fn script_url(path: &str) -> String {
    if path.starts_with("http") {
        path.to_owned()
    } else {
        format!("{PLAYER_PAGE}{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<!DOCTYPE html><html><head>
      <script type="module" crossorigin src="/assets/index~a1b2c3d4.js"></script>
      <link rel="stylesheet" href="/assets/index~f9e8d7c6.css">
    </head></html>"#;

    const SCRIPT: &str = r#"const t="eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCIsImtpZCI6IlUzVTlXIn0.eyJpc3MiOiJhcHAubXVzaWMuYXBwbGUuY29tIiwiaWF0IjoxNzAwMDAwMDAwfQ.abc-DEF_123";
      export{t as developerToken};"#;

    #[test]
    fn literal_patterns_compile() {
        // Guards the `unwrap_or_else` fallbacks above: if a pattern ever fails
        // to compile, this test fails rather than the fallback silently hiding.
        assert!(Regex::new(r"/assets/index~[^/]+\.js").is_ok());
        assert!(Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").is_ok());
    }

    #[test]
    fn script_path_is_found_in_the_page() {
        assert_eq!(find_script_path(PAGE), Some("/assets/index~a1b2c3d4.js"));
    }

    #[test]
    fn token_is_found_in_the_script() {
        let token = find_token(SCRIPT).expect("token");
        assert!(token.starts_with("eyJ"));
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn absolute_script_urls_are_built() {
        assert_eq!(script_url("/assets/index~x.js"), "https://music.apple.com/assets/index~x.js");
        assert_eq!(script_url("https://cdn/x.js"), "https://cdn/x.js");
    }

    #[test]
    fn missing_pieces_report_none_rather_than_panicking() {
        assert_eq!(find_script_path("<html></html>"), None);
        assert_eq!(find_token("no token here"), None);
    }

    #[test]
    fn the_first_token_wins_when_several_are_present() {
        let two = "a eyJhbGciOiJFUzI1NiJ9.eyJhIjoxfQ.sig b eyJhbGciOiJFUzI1NiJ9.eyJiIjoyfQ.sig2";
        let found = find_token(two).expect("token");
        assert_eq!(found.split('.').count(), 3);
        assert!(found.contains("eyJhIjoxfQ"), "the first token must be returned");
    }
}
