//! Widevine PSSH construction.
//!
//! A PSSH ("Protection System Specific Header") box tells a CDM which content
//! and key IDs a stream uses. Apple's HLS playlists carry the key ID in the
//! `EXT-X-KEY` URI; this module turns that key ID into the PSSH the licence
//! request must carry.

use prost::Message;

/// The Widevine system id, as it appears in a PSSH box
/// (`edef8ba9-79d6-4ace-a3c8-27dcd51d21ed`).
pub const WIDEVINE_SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

/// Encryption algorithm declared in a Widevine CENC header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, prost::Enumeration)]
#[repr(i32)]
pub enum Algorithm {
    Unencrypted = 0,
    AesCtr = 1,
    AesCtrV1 = 2,
}

/// The `WidevineCencHeader` protobuf, embedded in a PSSH box.
#[derive(Clone, PartialEq, Message)]
pub struct WidevineCencHeader {
    #[prost(enumeration = "Algorithm", tag = "1")]
    pub algorithm: i32,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub key_id: Vec<Vec<u8>>,
    #[prost(string, optional, tag = "3")]
    pub provider: Option<String>,
    #[prost(bytes = "vec", optional, tag = "4")]
    pub content_id: Option<Vec<u8>>,
}

/// Errors from PSSH construction.
#[derive(Debug, thiserror::Error)]
pub enum PsshError {
    /// The key ID is not a valid length.
    #[error("key id must be 16 bytes, got {0}")]
    BadKeyId(usize),
    /// The key ID is not valid hex.
    #[error("key id is not valid hex: {0}")]
    BadHex(#[from] hex::FromHexError),
}

/// A Widevine PSSH, ready to base64-encode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pssh {
    bytes: Vec<u8>,
}

impl Pssh {
    /// Builds the PSSH for a single key ID.
    ///
    /// # Errors
    ///
    /// Fails when the key ID is not exactly 16 bytes.
    pub fn for_key_id(key_id: &[u8]) -> Result<Self, PsshError> {
        if key_id.len() != 16 {
            return Err(PsshError::BadKeyId(key_id.len()));
        }
        Self::from_key_ids(std::slice::from_ref(&key_id.to_vec()))
    }

    /// Builds the PSSH for one or more key IDs, hex-encoded.
    ///
    /// # Errors
    ///
    /// Fails when a key is not 16 bytes or is not valid hex.
    pub fn for_key_ids_hex<S: AsRef<str>>(keys: &[S]) -> Result<Self, PsshError> {
        let decoded = keys
            .iter()
            .map(|key| hex::decode(key.as_ref().trim()))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_key_ids(&decoded)
    }

    fn from_key_ids(key_ids: &[Vec<u8>]) -> Result<Self, PsshError> {
        for key in key_ids {
            if key.len() != 16 {
                return Err(PsshError::BadKeyId(key.len()));
            }
        }

        let header = WidevineCencHeader {
            algorithm: Algorithm::AesCtr as i32,
            key_id: key_ids.to_vec(),
            provider: None,
            content_id: None,
        };

        Ok(Self { bytes: box_pssh(&header.encode_to_vec()) })
    }

    /// The raw PSSH box bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Base64-encoded PSSH, as the licence request expects it.
    #[must_use]
    pub fn to_base64(&self) -> String {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode(&self.bytes)
    }
}

/// Wraps a `WidevineCencHeader` in an ISO BMFF `pssh` box.
///
/// Layout: `[size:4][type:4="pssh"][version:1][flags:3][systemId:16][dataSize:4][data]`
fn box_pssh(data: &[u8]) -> Vec<u8> {
    let size = 4 + 4 + 4 + 16 + 4 + data.len();
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&(size as u32).to_be_bytes());
    out.extend_from_slice(b"pssh");
    out.extend_from_slice(&[0u8; 4]); // version 0, flags 0
    out.extend_from_slice(&WIDEVINE_SYSTEM_ID);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];

    #[test]
    fn box_has_the_iso_bmff_shape() {
        let pssh = Pssh::for_key_id(&KEY).expect("valid key");
        let bytes = pssh.as_bytes();
        let declared = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(declared as usize, bytes.len());
        assert_eq!(&bytes[4..8], b"pssh");
        assert_eq!(&bytes[12..28], &WIDEVINE_SYSTEM_ID);
    }

    #[test]
    fn header_round_trips_through_protobuf() {
        let pssh = Pssh::for_key_id(&KEY).expect("valid key");
        let bytes = pssh.as_bytes();
        let data_size = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;
        let data = &bytes[32..32 + data_size];
        let header = WidevineCencHeader::decode(data).expect("decode");
        assert_eq!(header.algorithm, Algorithm::AesCtr as i32);
        assert_eq!(header.key_id, vec![KEY.to_vec()]);
    }

    #[test]
    fn hex_keys_are_accepted() {
        let pssh = Pssh::for_key_ids_hex(&["00112233445566778899aabbccddeeff"]).expect("valid");
        assert!(pssh.to_base64().starts_with("AAA"));
    }

    #[test]
    fn wrong_length_keys_are_rejected() {
        assert!(matches!(Pssh::for_key_id(&[0u8; 8]), Err(PsshError::BadKeyId(8))));
    }

    #[test]
    fn multiple_key_ids_are_all_encoded() {
        let pssh = Pssh::for_key_ids_hex(&[
            "00112233445566778899aabbccddeeff",
            "ffeeddccbbaa99887766554433221100",
        ])
        .expect("valid");
        let bytes = pssh.as_bytes();
        let data_size = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize;
        let header = WidevineCencHeader::decode(&bytes[32..32 + data_size]).expect("decode");
        assert_eq!(header.key_id.len(), 2);
    }
}
