//! Filename templating for downloads.
//!
//! Users configure a pattern like `{artist}/{album}/{track:02} {title}`. This
//! module expands it and sanitises the result so it is always a valid path.

use std::collections::BTreeMap;

/// A filename pattern with `{placeholder}` substitutions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilenameTemplate {
    pattern: String,
}

/// Values available to a [`FilenameTemplate`].
#[derive(Debug, Clone, Default)]
pub struct NamingValues {
    values: BTreeMap<String, String>,
}

impl NamingValues {
    /// Creates an empty value set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts one placeholder value.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Builder-style insert.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.set(key, value);
        self
    }
}

impl FilenameTemplate {
    /// The default pattern: `Artist/Album/NN Title`.
    pub const DEFAULT: &'static str = "{artist}/{album}/{track:02} {title}";

    /// Wraps a pattern. Unknown placeholders are left untouched so a typo is
    /// visible rather than silently dropped.
    #[must_use]
    pub fn new(pattern: impl Into<String>) -> Self {
        Self { pattern: pattern.into() }
    }

    /// The raw pattern.
    #[must_use]
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Expands the pattern, sanitising each substituted value.
    ///
    /// Padding is supported with `{track:02}` (zero-pad to two digits).
    #[must_use]
    pub fn expand(&self, values: &NamingValues) -> String {
        let mut out = String::with_capacity(self.pattern.len() + 32);
        let mut rest = self.pattern.as_str();

        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close_rel) = rest[open..].find('}') else {
                out.push_str(&rest[open..]);
                return sanitize_path(&out);
            };
            let close = open + close_rel;
            let token = &rest[open + 1..close];
            out.push_str(&resolve(token, values));
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        sanitize_path(&out)
    }
}

/// Resolves one `{name}` or `{name:0N}` token.
fn resolve(token: &str, values: &NamingValues) -> String {
    let (name, pad) = match token.split_once(':') {
        Some((name, spec)) => (name, spec.strip_prefix('0').and_then(|s| s.parse::<usize>().ok())),
        None => (token, None),
    };

    let Some(raw) = values.values.get(name) else {
        return format!("{{{token}}}");
    };

    let sanitized = sanitize_component(raw);
    match pad {
        Some(width) => {
            let digits = sanitized.trim_start_matches('0');
            let padded = if digits.is_empty() { "0" } else { digits };
            format!("{padded:0>width$}")
        }
        None => sanitized,
    }
}

/// Replaces characters that are illegal in a path component.
#[must_use]
pub fn sanitize_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0') {
                '_'
            } else {
                c
            }
        })
        .collect();
    cleaned.trim().trim_matches('.').to_owned()
}

/// Sanitises each path segment, dropping empty ones.
fn sanitize_path(value: &str) -> String {
    value
        .split('/')
        .map(sanitize_component)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NamingValues {
        NamingValues::new()
            .with("artist", "Radiohead")
            .with("album", "In Rainbows")
            .with("title", "15 Step")
            .with("track", "1")
    }

    #[test]
    fn default_pattern_expands_with_padding() {
        let template = FilenameTemplate::new(FilenameTemplate::DEFAULT);
        assert_eq!(template.expand(&sample()), "Radiohead/In Rainbows/01 15 Step");
    }

    #[test]
    fn slash_in_a_value_does_not_create_a_directory() {
        let values = NamingValues::new().with("title", "AC/DC");
        assert_eq!(FilenameTemplate::new("{title}").expand(&values), "AC_DC");
    }

    #[test]
    fn unknown_placeholders_survive() {
        let values = NamingValues::new().with("title", "Song");
        assert_eq!(FilenameTemplate::new("{title} {bogus}").expand(&values), "Song {bogus}");
    }

    #[test]
    fn empty_segments_are_dropped() {
        let values = NamingValues::new().with("title", "Song");
        assert_eq!(FilenameTemplate::new("/{title}/").expand(&values), "Song");
    }

    #[test]
    fn padding_applies_to_numeric_values() {
        let values = NamingValues::new().with("track", "7");
        assert_eq!(FilenameTemplate::new("{track:02}").expand(&values), "07");
        assert_eq!(FilenameTemplate::new("{track:03}").expand(&values), "007");
    }
}
