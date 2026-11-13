//! Web playback: turning a track id into a playable, decryptable stream.
//!
//! Apple's model is: ask `play.music.apple.com` for a track's assets, get back
//! HLS URLs per quality family, pick the best one the caller can actually play,
//! then read the media playlist to learn the segments and their encryption.
//!
//! The quality-family identifiers are opaque Apple strings. They are named here
//! as constants rather than inlined, because getting one wrong silently selects
//! the wrong codec.

use serde::Deserialize;

/// Apple's web-playback host.
pub const PLAYBACK_BASE: &str = "https://play.music.apple.com";

/// Quality-family identifiers, as they appear in a `subDownload` response.
///
/// The leading number is Apple's asset id; the suffix names the encryption and
/// codec. Two families matter to us:
///
/// - `ctrp` — CTR/Widevine, used for AAC.
/// - `cbcp` — CBCS/`FairPlay`, used for ALAC.
pub mod quality {
    /// AAC 256 kbps, Widevine.
    pub const AAC_256: &str = "28:ctrp256";
    /// AAC 64 kbps, Widevine.
    pub const AAC_64: &str = "32:ctrp64";
    /// AAC 256 kbps, `FairPlay`.
    pub const AAC_256_FAIRPLAY: &str = "30:cbcp256";
    /// AAC 64 kbps, `FairPlay`.
    pub const AAC_64_FAIRPLAY: &str = "34:cbcp64";
}

/// A `subDownload` response, one entry per requested quality.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SubDownloadResponse {
    /// The storefront Apple used. Carried for diagnostics; the caller's own
    /// storefront is authoritative.
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) storefront: Option<String>,
    /// Assets, joined with the requested qualities by the order Apple returns.
    #[serde(default)]
    pub(crate) assets: Vec<SubDownloadAsset>,
}

/// One asset in a `subDownload` response.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SubDownloadAsset {
    /// The quality family id.
    #[serde(default)]
    pub(crate) flavor: Option<String>,
    /// The HLS media playlist URL.
    #[serde(default)]
    pub(crate) url: Option<String>,
    /// The decryption key URI, when Apple reports it out of band instead of in
    /// the media playlist. Consumed by the DRM path once the optional tier lands.
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) key: Option<String>,
}

impl SubDownloadResponse {
    /// Finds an asset by quality family.
    #[must_use]
    pub(crate) fn asset(&self, flavor: &str) -> Option<&SubDownloadAsset> {
        self.assets.iter().find(|asset| asset.flavor.as_deref() == Some(flavor))
    }

    /// The best asset for the requested quality, with a fallback chain.
    ///
    /// The chain walks down from the most capable family the caller allows, so a
    /// track with no lossless master still plays at the highest tier available.
    #[must_use]
    pub(crate) fn best_for(
        &self,
        quality: sosumi_provider_api::AudioQuality,
    ) -> Option<&SubDownloadAsset> {
        use sosumi_provider_api::AudioQuality;

        let chain: &[&str] = match quality {
            AudioQuality::Spatial => &[quality::AAC_256_FAIRPLAY, quality::AAC_256],
            AudioQuality::HiResLossless | AudioQuality::Lossless => {
                &[quality::AAC_256_FAIRPLAY, quality::AAC_256]
            }
            AudioQuality::High => &[quality::AAC_256, quality::AAC_256_FAIRPLAY],
            AudioQuality::Low => &[
                quality::AAC_64,
                quality::AAC_256,
                quality::AAC_64_FAIRPLAY,
                quality::AAC_256_FAIRPLAY,
            ],
        };

        chain.iter().find_map(|flavor| self.asset(flavor))
    }
}

/// Builds the `subDownload` URL for a track and a set of qualities.
///
/// Apple expects the flavor as `storefront:family`, URL-encoded as a whole.
#[must_use]
pub fn sub_download_url(storefront: &str, adam_id: &str, qualities: &[&str]) -> String {
    let encode = |value: &str| -> String {
        url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
    };
    let flavors: Vec<String> =
        qualities.iter().map(|q| encode(&format!("{storefront}:{q}"))).collect();
    format!("{PLAYBACK_BASE}/subDownload?id={adam_id}&flavor={}", flavors.join("%2C"))
}

/// The qualities to request for a given target quality, most capable first.
#[must_use]
pub fn qualities_for(quality: sosumi_provider_api::AudioQuality) -> Vec<&'static str> {
    use sosumi_provider_api::AudioQuality;
    match quality {
        AudioQuality::Spatial | AudioQuality::HiResLossless | AudioQuality::Lossless => {
            vec![quality::AAC_256_FAIRPLAY, quality::AAC_256]
        }
        AudioQuality::High => vec![quality::AAC_256, quality::AAC_256_FAIRPLAY],
        AudioQuality::Low => vec![quality::AAC_64, quality::AAC_256],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sosumi_provider_api::AudioQuality;

    fn response() -> SubDownloadResponse {
        serde_json::from_str(
            r#"{
              "storefront": "us",
              "assets": [
                {"flavor": "28:ctrp256", "url": "https://a/256.m3u8"},
                {"flavor": "30:cbcp256", "url": "https://a/alac.m3u8"}
              ]
            }"#,
        )
        .expect("parse")
    }

    #[test]
    fn assets_are_found_by_flavor() {
        let response = response();
        assert_eq!(
            response.asset("28:ctrp256").expect("aac").url.as_deref(),
            Some("https://a/256.m3u8")
        );
        assert!(response.asset("nonexistent").is_none());
    }

    #[test]
    fn high_quality_prefers_aac() {
        let response = response();
        let asset = response.best_for(AudioQuality::High).expect("asset");
        assert_eq!(asset.flavor.as_deref(), Some(quality::AAC_256));
    }

    #[test]
    fn lossless_prefers_fairplay_when_available() {
        let response = response();
        let asset = response.best_for(AudioQuality::Lossless).expect("asset");
        assert_eq!(asset.flavor.as_deref(), Some(quality::AAC_256_FAIRPLAY));
    }

    #[test]
    fn lossless_falls_back_to_aac_when_no_lossless_asset_exists() {
        let response: SubDownloadResponse = serde_json::from_str(
            r#"{"assets":[{"flavor":"28:ctrp256","url":"https://a/256.m3u8"}]}"#,
        )
        .expect("parse");
        let asset = response.best_for(AudioQuality::Lossless).expect("asset");
        assert_eq!(asset.flavor.as_deref(), Some(quality::AAC_256));
    }

    #[test]
    fn low_quality_prefers_the_smaller_asset() {
        let response: SubDownloadResponse = serde_json::from_str(
            r#"{"assets":[
                {"flavor":"32:ctrp64","url":"https://a/64.m3u8"},
                {"flavor":"28:ctrp256","url":"https://a/256.m3u8"}]}"#,
        )
        .expect("parse");
        assert_eq!(
            response.best_for(AudioQuality::Low).expect("asset").flavor.as_deref(),
            Some(quality::AAC_64)
        );
    }

    #[test]
    fn missing_assets_report_none() {
        let response: SubDownloadResponse =
            serde_json::from_str(r#"{"assets":[]}"#).expect("parse");
        assert!(response.best_for(AudioQuality::High).is_none());
    }

    #[test]
    fn sub_download_url_carries_every_requested_flavor() {
        let url =
            sub_download_url("us", "1440833098", &[quality::AAC_256, quality::AAC_256_FAIRPLAY]);
        assert!(url.contains("id=1440833098"));
        assert!(url.contains("28%3A"));
        assert!(url.contains("%2C"));
    }

    #[test]
    fn qualities_for_lossless_requests_the_fairplay_family_first() {
        let qualities = qualities_for(AudioQuality::Lossless);
        assert_eq!(qualities[0], quality::AAC_256_FAIRPLAY);
    }
}
