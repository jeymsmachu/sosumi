//! HLS playlist parsing.
//!
//! Apple serves audio as HLS. Two playlists matter:
//!
//! - The **master** playlist lists variants (`#EXT-X-STREAM-INF`) with their
//!   codec string, bitrate, and sample rate. Choosing a variant is how we honour
//!   a quality request.
//! - The **media** playlist lists segments and, crucially, the `#EXT-X-KEY`
//!   line that says how the stream is encrypted and under which key.
//!
//! This module is a parser only. It performs no I/O, which is why it carries the
//! project's densest tests: it is where Apple's format is interpreted.

/// A variant from a master playlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// Bandwidth in bits per second, as declared.
    pub bandwidth: u64,
    /// `CODECS` attribute, for example `alac` or `ec-3`.
    pub codecs: Vec<String>,
    /// `EXT-X-STREAM-INF` URI, resolved against the playlist.
    pub uri: String,
    /// Sample rate parsed from `CODECS`, when the codec string carries one.
    pub sample_rate: Option<u32>,
    /// Channel count parsed from `CODECS`, when present.
    pub channels: Option<u16>,
    /// True when the `CODECS` attribute names an object-based codec.
    pub spatial: bool,
}

impl Variant {
    /// True when any declared codec is lossless.
    #[must_use]
    pub fn is_lossless(&self) -> bool {
        self.codecs.iter().any(|c| {
            let lower = c.to_ascii_lowercase();
            lower.contains("alac") || lower.contains("flac")
        })
    }

    /// AAC at or below 300 kbps, the fallback tier.
    #[must_use]
    pub fn is_aac(&self) -> bool {
        self.codecs.iter().any(|c| c.to_ascii_lowercase().contains("mp4a"))
    }
}

/// How a media playlist's segments are encrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyMethod {
    /// No encryption.
    None,
    /// AES-128 with a URI to fetch the key.
    Aes128 { uri: String },
    /// A DRM key system, identified by the `URI` scheme.
    ///
    /// Apple uses `skd://` for `FairPlay`/CBCS and a `data:` URI carrying a
    /// Widevine key ID for CTR.
    Drm { system: DrmSystem, uri: String },
}

/// Which DRM system a playlist's key belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrmSystem {
    /// `FairPlay` Streaming, signalled by a `skd://` URI. Used for ALAC.
    FairPlay,
    /// Widevine, signalled by a `data:` URI. Used for AAC.
    Widevine,
    /// A system we do not recognise.
    Unknown,
}

/// A parsed media playlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPlaylist {
    /// Segment URIs, resolved against the playlist.
    pub segments: Vec<String>,
    /// The key declaration, when the playlist is encrypted.
    pub key: Option<(KeyMethod, Option<String>)>,
    /// Total duration in milliseconds, when the playlist declares it.
    pub duration_ms: Option<u64>,
}

/// Errors from parsing.
#[derive(Debug, thiserror::Error)]
pub enum HlsError {
    /// The playlist is not the kind expected.
    #[error("not an HLS playlist")]
    NotPlaylist,
    /// No variants or segments were found.
    #[error("playlist contains no {0}")]
    Empty(&'static str),
}

/// Parses a master playlist, resolving variant URIs against `base`.
///
/// # Errors
///
/// Returns [`HlsError::NotPlaylist`] when the input is not HLS.
pub fn parse_master(text: &str, base: &str) -> Result<Vec<Variant>, HlsError> {
    if !text.contains("#EXTM3U") {
        return Err(HlsError::NotPlaylist);
    }

    let mut variants = Vec::new();
    let mut pending: Option<Variant> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            let bandwidth = attribute(attrs, "BANDWIDTH").and_then(|v| v.parse().ok()).unwrap_or(0);
            let codec_string = attribute(attrs, "CODECS").unwrap_or_default();
            let codecs = parse_codec_list(&codec_string);

            pending = Some(Variant {
                bandwidth,
                sample_rate: parse_sample_rate(&codecs),
                channels: parse_channels(&codecs),
                spatial: codecs.iter().any(|c| c.to_ascii_lowercase().contains("ec-3")),
                codecs,
                uri: String::new(),
            });
        } else if !line.is_empty() && !line.starts_with('#') {
            if let Some(mut variant) = pending.take() {
                variant.uri = resolve(base, line);
                variants.push(variant);
            }
        }
    }

    if variants.is_empty() {
        return Err(HlsError::Empty("variants"));
    }
    Ok(variants)
}

/// Parses a media playlist, resolving segment URIs against `base`.
///
/// # Errors
///
/// Returns [`HlsError::NotPlaylist`] when the input is not HLS, and
/// [`HlsError::Empty`] when it has no segments.
pub fn parse_media(text: &str, base: &str) -> Result<MediaPlaylist, HlsError> {
    if !text.contains("#EXTM3U") {
        return Err(HlsError::NotPlaylist);
    }

    let mut segments = Vec::new();
    let mut key = None;
    let mut duration_ms: u64 = 0;
    let mut has_duration = false;

    for line in text.lines() {
        let line = line.trim();

        if let Some(attrs) = line.strip_prefix("#EXT-X-KEY:") {
            key = parse_key(attrs);
        } else if let Some(value) = line.strip_prefix("#EXTINF:") {
            if let Some(seconds) =
                value.split(',').next().and_then(|v| v.trim().parse::<f64>().ok())
            {
                duration_ms = duration_ms.saturating_add((seconds * 1000.0).round() as u64);
                has_duration = true;
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            segments.push(resolve(base, line));
        }
    }

    if segments.is_empty() {
        return Err(HlsError::Empty("segments"));
    }

    Ok(MediaPlaylist { segments, key, duration_ms: has_duration.then_some(duration_ms) })
}

/// Parses an `#EXT-X-KEY` attribute list.
fn parse_key(attrs: &str) -> Option<(KeyMethod, Option<String>)> {
    let method = attribute(attrs, "METHOD")?;
    let uri = attribute(attrs, "URI");

    let method = match method.as_str() {
        "NONE" => KeyMethod::None,
        "AES-128" => KeyMethod::Aes128 { uri: uri.clone().unwrap_or_default() },
        "SAMPLE-AES" | "SAMPLE-AES-CTR" => {
            let uri = uri.unwrap_or_default();
            let system = if uri.starts_with("skd://") {
                DrmSystem::FairPlay
            } else if uri.starts_with("data:") {
                DrmSystem::Widevine
            } else {
                DrmSystem::Unknown
            };
            KeyMethod::Drm { system, uri }
        }
        _ => return None,
    };

    Some((method, attribute(attrs, "KEYID")))
}

/// Extracts a top-level attribute value, handling quoted values.
fn attribute(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    while let Some(index) = rest.find(name) {
        let after = &rest[index + name.len()..];
        // Require the name to be a whole attribute, not a prefix of another.
        let boundary_ok = after.starts_with('=');
        if boundary_ok {
            let value = &after[1..];
            return Some(if let Some(stripped) = value.strip_prefix('"') {
                stripped.split('"').next().unwrap_or_default().to_owned()
            } else {
                value.split(',').next().unwrap_or_default().trim().to_owned()
            });
        }
        rest = &after[1..];
    }
    None
}

/// Splits a `CODECS` attribute into individual codec strings.
fn parse_codec_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|c| c.trim().trim_matches('"'))
        .filter(|c| !c.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Pulls the sample rate out of a codec string such as
/// `alac (44100 Hz)` or `ec-3 (48000 Hz)`.
fn parse_sample_rate(codecs: &[String]) -> Option<u32> {
    for codec in codecs {
        if let Some(open) = codec.find('(') {
            let inside = &codec[open + 1..];
            let digits: String = inside.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(rate) = digits.parse() {
                return Some(rate);
            }
        }
    }
    None
}

/// Pulls the channel count from a codec string containing a layout such as
/// `7.1` or `2`.
fn parse_channels(codecs: &[String]) -> Option<u16> {
    for codec in codecs {
        let lower = codec.to_ascii_lowercase();
        if let Some((_, layout)) = lower.split_once("ch:") {
            let digits: String = layout.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(channels) = digits.parse() {
                return Some(channels);
            }
        }
    }
    None
}

/// Resolves a possibly relative URI against a base URL.
fn resolve(base: &str, uri: &str) -> String {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        return uri.to_owned();
    }
    match url::Url::parse(base).and_then(|b| b.join(uri)) {
        Ok(joined) => joined.to_string(),
        Err(_) => uri.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-STREAM-INF:BANDWIDTH=256000,CODECS="mp4a.40.2"
https://audio.example/256/variant.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1400000,CODECS="alac (44100 Hz)",AVERAGE-BANDWIDTH=1200000
https://audio.example/alac/variant.m3u8
"#;

    const MEDIA_WIDEVINE: &str = r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-TARGETDURATION:10
#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI="data:text/plain;base64,AAAAPXBzc2g=",KEYID="deadbeef"
#EXTINF:9.000,
seg0.m4s
#EXTINF:9.000,
seg1.m4s
#EXT-X-ENDLIST
"#;

    const MEDIA_FAIRPLAY: &str = r#"#EXTM3U
#EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://itunes.apple.com/P000000000/s1/e1",KEYFORMAT="com.apple.streamingkeydelivery"
#EXTINF:8.000,
seg0.m4s
"#;

    #[test]
    fn master_parses_variants_and_resolves_uris() {
        let variants = parse_master(MASTER, "https://audio.example/master.m3u8").expect("parse");
        assert_eq!(variants.len(), 2);
        assert!(variants[0].uri.starts_with("https://"));
    }

    #[test]
    fn master_identifies_aac_and_lossless() {
        let variants = parse_master(MASTER, "https://audio.example/master.m3u8").expect("parse");
        let aac = variants.iter().find(|v| v.is_aac()).expect("aac variant");
        let alac = variants.iter().find(|v| v.is_lossless()).expect("alac variant");
        assert_eq!(aac.bandwidth, 256_000);
        assert_eq!(alac.sample_rate, Some(44_100));
        assert!(!alac.spatial);
    }

    #[test]
    fn codec_sample_rate_and_channels_are_extracted() {
        let variants = parse_master(
            r#"#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=1,CODECS="ec-3 (48000 Hz) ch:8"
a.m3u8"#,
            "https://x/y.m3u8",
        )
        .expect("parse");
        assert_eq!(variants[0].sample_rate, Some(48_000));
        assert_eq!(variants[0].channels, Some(8));
        assert!(variants[0].spatial);
    }

    #[test]
    fn widevine_key_is_recognised() {
        let media = parse_media(MEDIA_WIDEVINE, "https://audio.example/256/a.m3u8").expect("parse");
        assert_eq!(media.segments.len(), 2);
        let (method, key_id) = media.key.expect("key");
        assert!(matches!(method, KeyMethod::Drm { system: DrmSystem::Widevine, .. }));
        assert_eq!(key_id.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn fairplay_key_is_recognised() {
        let media =
            parse_media(MEDIA_FAIRPLAY, "https://audio.example/alac/a.m3u8").expect("parse");
        let (method, _) = media.key.expect("key");
        match method {
            KeyMethod::Drm { system, uri } => {
                assert_eq!(system, DrmSystem::FairPlay);
                assert!(uri.starts_with("skd://"));
            }
            other => panic!("expected a DRM key, got {other:?}"),
        }
    }

    #[test]
    fn relative_segments_resolve_against_the_media_url() {
        let media = parse_media(MEDIA_WIDEVINE, "https://audio.example/256/a.m3u8").expect("parse");
        assert_eq!(media.segments[0], "https://audio.example/256/seg0.m4s");
    }

    #[test]
    fn duration_accumulates_from_extinf() {
        let media = parse_media(MEDIA_WIDEVINE, "https://x/a.m3u8").expect("parse");
        assert_eq!(media.duration_ms, Some(18_000));
    }

    #[test]
    fn a_key_uri_with_a_comma_inside_quotes_survives() {
        let playlist = r#"#EXTM3U
#EXT-X-KEY:METHOD=AES-128,URI="https://keys.example/k?ids=1,2"
#EXTINF:1.0,
s.m4s"#;
        let media = parse_media(playlist, "https://x/a.m3u8").expect("parse");
        let (method, _) = media.key.expect("key");
        match method {
            KeyMethod::Aes128 { uri } => assert_eq!(uri, "https://keys.example/k?ids=1,2"),
            other => panic!("expected AES-128, got {other:?}"),
        }
    }

    #[test]
    fn unencrypted_playlist_has_no_key() {
        let playlist = "#EXTM3U\n#EXTINF:1.0,\ns.m4s\n";
        let media = parse_media(playlist, "https://x/a.m3u8").expect("parse");
        assert!(media.key.is_none());
    }

    #[test]
    fn non_hls_input_is_rejected() {
        assert!(matches!(parse_master("<html/>", "https://x"), Err(HlsError::NotPlaylist)));
        assert!(matches!(parse_media("<html/>", "https://x"), Err(HlsError::NotPlaylist)));
    }

    #[test]
    fn empty_playlists_are_rejected() {
        assert!(matches!(parse_master("#EXTM3U\n", "https://x"), Err(HlsError::Empty(_))));
        assert!(matches!(parse_media("#EXTM3U\n", "https://x"), Err(HlsError::Empty(_))));
    }

    #[test]
    fn unknown_method_is_tolerated() {
        let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=SOMETHING-NEW\n#EXTINF:1,\ns.m4s\n";
        let media = parse_media(playlist, "https://x/a.m3u8").expect("parse");
        assert!(media.key.is_none());
        assert_eq!(media.segments.len(), 1);
    }
}
