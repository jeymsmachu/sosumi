//! Artwork URL templating and dominant-colour helpers.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Expands provider artwork URLs to a requested size.
///
/// Apple serves artwork from a templated URL that ends in `{w}x{h}` (optionally
/// with a format suffix). Rather than special-case each provider's scheme, we
/// accept a small placeholder vocabulary and substitute it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtworkTemplate {
    /// The provider-supplied URL, possibly containing `{w}` and `{h}`.
    pub template: String,
}

impl ArtworkTemplate {
    /// Wraps a provider URL.
    #[must_use]
    pub fn new(template: impl Into<String>) -> Self {
        Self { template: template.into() }
    }

    /// True when the URL contains any placeholder we can substitute.
    #[must_use]
    pub fn is_templated(&self) -> bool {
        self.template.contains("{w}") || self.template.contains("{h}")
    }

    /// Substitutes `{w}` and `{h}` with a concrete square size.
    ///
    /// URLs without placeholders are returned unchanged, so this is always safe
    /// to call on any provider URL.
    #[must_use]
    pub fn at_size(&self, size: u32) -> String {
        self.template.replace("{w}", &size.to_string()).replace("{h}", &size.to_string())
    }

    /// A square thumbnail at `2x` `size` pixels for high-DPI displays.
    #[must_use]
    pub fn at_dpi(&self, size: u32, scale: u32) -> String {
        self.at_size(size.saturating_mul(scale.max(1)))
    }
}

/// Parses a `#rrggbb` colour, returning it normalised to lower case.
#[must_use]
pub fn normalize_color(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let is_hex = hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit());
    is_hex.then(|| format!("#{}", hex.to_ascii_lowercase()))
}

/// Picks a readable foreground (`#000000` or `#ffffff`) for a background
/// colour, using the WCAG relative-luminance test.
#[must_use]
pub fn readable_foreground(background: &str) -> &'static str {
    let Some(hex) = background.strip_prefix('#') else {
        return "#ffffff";
    };
    if hex.len() != 6 {
        return "#ffffff";
    }
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).map_or(0.0, |v| f64::from(v) / 255.0)
    };
    let linear = |c: f64| if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    let luminance = 0.2126 * linear(channel(0..2))
        + 0.7152 * linear(channel(2..4))
        + 0.0722 * linear(channel(4..6));
    if luminance > 0.5 {
        "#000000"
    } else {
        "#ffffff"
    }
}

/// Extracts the artwork id from a typical `/image/thumb/...` artwork URL.
///
/// Used to build the animated-artwork URL without re-querying the catalog.
#[must_use]
pub fn artwork_id(url: &str) -> Option<String> {
    let re = Regex::new(r"/image/thumb/([^/]+)").ok()?;
    re.captures(url).map(|caps| caps[1].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templated_urls_are_substituted() {
        let art = ArtworkTemplate::new("https://example.com/{w}x{h}bb.jpg");
        assert!(art.is_templated());
        assert_eq!(art.at_size(600), "https://example.com/600x600bb.jpg");
    }

    #[test]
    fn untemplated_urls_pass_through() {
        let art = ArtworkTemplate::new("https://example.com/cover.jpg");
        assert!(!art.is_templated());
        assert_eq!(art.at_size(600), "https://example.com/cover.jpg");
    }

    #[test]
    fn dpi_scaling_multiplies() {
        let art = ArtworkTemplate::new("https://example.com/{w}x{h}.jpg");
        assert_eq!(art.at_dpi(300, 2), "https://example.com/600x600.jpg");
    }

    #[test]
    fn colours_are_normalised() {
        assert_eq!(normalize_color("#AABBCC").as_deref(), Some("#aabbcc"));
        assert_eq!(normalize_color("aabbcc").as_deref(), Some("#aabbcc"));
        assert_eq!(normalize_color("nope"), None);
    }

    #[test]
    fn foreground_contrasts_with_background() {
        assert_eq!(readable_foreground("#ffffff"), "#000000");
        assert_eq!(readable_foreground("#000000"), "#ffffff");
    }

    #[test]
    fn artwork_id_is_extracted() {
        let url = "https://is1-ssl.mzstatic.com/image/thumb/Features123/600x600bb.jpg";
        assert_eq!(artwork_id(url).as_deref(), Some("Features123"));
    }
}
