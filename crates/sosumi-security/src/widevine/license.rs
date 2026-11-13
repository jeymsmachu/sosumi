//! Widevine licence acquisition and content-key unwrapping.
//!
//! The flow, in order:
//!
//! 1. Build a [`LicenseRequest`] carrying the PSSH from [`super::pssh`].
//! 2. Sign it with the device RSA key and wrap it in a signed message.
//! 3. POST it to the licence server with the provider's tokens.
//! 4. Parse the [`License`], RSA-decrypt the session key with the device key,
//!    and AES-CBC-decrypt each content key in the response.
//!
//! Everything in this file is pure: it takes bytes and returns bytes. The
//! provider crate owns the HTTP call, so this module is trivially testable.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use prost::Message;
use rand::RngCore;
use rsa::Pkcs1v15Encrypt;
use zeroize::Zeroize;

use super::identity::{DeviceIdentity, IdentityError};

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// Errors from licence acquisition.
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    /// The device identity could not be used.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// The licence server returned an error payload.
    #[error("licence server refused the request: {0}")]
    Refused(String),
    /// The licence response could not be parsed.
    #[error("malformed licence response: {0}")]
    Malformed(String),
    /// No content key was present in the licence.
    #[error("licence contained no content key")]
    NoContentKey,
    /// A cryptographic step failed.
    #[error("cryptographic failure: {0}")]
    Crypto(String),
}

/// The raw licence-request message.
#[derive(Clone, PartialEq, Message)]
pub struct LicenseRequest {
    #[prost(message, optional, tag = "1")]
    pub client_id: Option<ClientIdentification>,
    #[prost(message, optional, tag = "2")]
    pub content_id: Option<ContentIdentification>,
    #[prost(enumeration = "LicenseType", tag = "3")]
    pub r#type: i32,
    #[prost(uint64, tag = "4")]
    pub request_time: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub key_control_nonce: Vec<u8>,
    #[prost(message, optional, tag = "7")]
    pub encrypted_client_id: Option<EncryptedClientIdentification>,
}

/// Wrapper that carries a signed request.
#[derive(Clone, PartialEq, Message)]
pub struct SignedMessage {
    #[prost(enumeration = "MessageType", tag = "1")]
    pub r#type: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub msg: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub signature: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub session_key: Vec<u8>,
    #[prost(bytes = "vec", optional, tag = "5")]
    pub remote_attestation_certificate: Option<Vec<u8>>,
}

/// Client identification, echoed from the device identity.
#[derive(Clone, PartialEq, Message)]
pub struct ClientIdentification {
    #[prost(enumeration = "ClientIdentificationType", tag = "1")]
    pub r#type: i32,
    #[prost(bytes = "vec", tag = "2")]
    pub token: Vec<u8>,
    #[prost(message, repeated, tag = "3")]
    pub client_info: Vec<ClientInfo>,
    #[prost(bytes = "vec", optional, tag = "4")]
    pub provider_client_token: Option<Vec<u8>>,
    #[prost(uint32, tag = "5")]
    pub license_counter: u32,
    #[prost(message, optional, tag = "6")]
    pub client_capabilities: Option<ClientCapabilities>,
}

/// One client-info key/value pair.
#[derive(Clone, PartialEq, Message)]
pub struct ClientInfo {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// What the client tells the server it can do.
#[derive(Clone, PartialEq, Message)]
pub struct ClientCapabilities {
    #[prost(message, repeated, tag = "1")]
    pub supported_licenses: Vec<ClientCapabilities_License>,
    #[prost(message, optional, tag = "2")]
    pub client_token: Option<ClientCapabilities_ClientToken>,
}

/// A licence capability.
#[allow(non_camel_case_types)]
#[derive(Clone, PartialEq, Message)]
pub struct ClientCapabilities_License {
    #[prost(enumeration = "LicenseType", tag = "1")]
    pub r#type: i32,
}

/// Client-token capabilities.
#[allow(non_camel_case_types)]
#[derive(Clone, PartialEq, Message)]
pub struct ClientCapabilities_ClientToken {
    #[prost(bool, tag = "1")]
    pub allow_sensitive_client_data: bool,
}

/// Encrypted client identification, used when privacy mode is on.
#[derive(Clone, PartialEq, Message)]
pub struct EncryptedClientIdentification {
    #[prost(string, tag = "1")]
    pub provider_id: String,
    #[prost(bytes = "vec", tag = "2")]
    pub service_certificate_serial_number: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub encrypted_client_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub encrypted_client_id_iv: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub encrypted_privacy_key: Vec<u8>,
}

/// Identifies the requested content.
#[derive(Clone, PartialEq, Message)]
pub struct ContentIdentification {
    #[prost(message, optional, tag = "1")]
    pub widevine_pssh_data: Option<WidevinePsshData>,
    #[prost(string, optional, tag = "2")]
    pub license_release: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub request_id: Option<String>,
}

/// PSSH data the licence server needs.
#[derive(Clone, PartialEq, Message)]
pub struct WidevinePsshData {
    #[prost(enumeration = "Algorithm", tag = "1")]
    pub algorithm: i32,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub key_id: Vec<Vec<u8>>,
    #[prost(string, optional, tag = "3")]
    pub provider: Option<String>,
    #[prost(bytes = "vec", optional, tag = "4")]
    pub content_id: Option<Vec<u8>>,
    #[prost(string, optional, tag = "5")]
    pub policy: Option<String>,
    #[prost(uint32, optional, tag = "6")]
    pub crypto_period_index: Option<u32>,
    #[prost(string, optional, tag = "8")]
    pub group_id: Option<String>,
}

/// An encryption algorithm, as declared in a PSSH.
#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum Algorithm {
    Unencrypted = 0,
    AesCtr = 1,
}

/// A licence request type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum LicenseType {
    Streaming = 1,
    Offline = 2,
    Temporary = 3,
}

/// A signed-message type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    LicenseRequest = 1,
    License = 2,
    LicenseError = 3,
}

/// A client-identification type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum ClientIdentificationType {
    DeviceCertificate = 1,
}

/// The licence response.
#[derive(Clone, PartialEq, Message)]
pub struct License {
    #[prost(message, optional, tag = "1")]
    pub id: Option<LicenseIdentification>,
    #[prost(message, optional, tag = "2")]
    pub policy: Option<Policy>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    pub key: Vec<Vec<u8>>,
    #[prost(bytes = "vec", repeated, tag = "4")]
    pub key_ids: Vec<Vec<u8>>,
    #[prost(uint32, optional, tag = "5")]
    pub key_type: Option<u32>,
}

/// Identifies a licence.
#[derive(Clone, PartialEq, Message)]
pub struct LicenseIdentification {
    #[prost(bytes = "vec", optional, tag = "1")]
    pub request_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "2")]
    pub session_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "3")]
    pub purchase_id: Option<Vec<u8>>,
}

/// The licence policy block.
#[derive(Clone, PartialEq, Message)]
pub struct Policy {
    #[prost(bool, optional, tag = "1")]
    pub can_play: Option<bool>,
    #[prost(bool, optional, tag = "2")]
    pub can_persist: Option<bool>,
    #[prost(uint32, optional, tag = "3")]
    pub license_duration_seconds: Option<u32>,
}

/// A 16-byte AES content key.
///
/// Deliberately opaque: it has no `Display`, its bytes are zeroed on drop, and
/// it is never serialised. Callers pass it straight to a decryptor.
pub struct ContentKey {
    key_id: Vec<u8>,
    key: [u8; 16],
}

impl std::fmt::Debug for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentKey")
            .field("key_id", &hex::encode(&self.key_id))
            .field("key", &"<redacted>")
            .finish()
    }
}

impl Drop for ContentKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl ContentKey {
    /// The key ID this key decrypts.
    #[must_use]
    pub fn key_id(&self) -> &[u8] {
        &self.key_id
    }

    /// Borrows the raw key. Used only by the decryptor in this crate.
    #[must_use]
    pub(crate) fn bytes(&self) -> &[u8; 16] {
        &self.key
    }

    /// Builds a key directly. Test-only, so production code cannot conjure a
    /// key from nowhere.
    #[cfg(test)]
    pub(crate) fn from_bytes_for_test(key: [u8; 16], key_id: Vec<u8>) -> Self {
        Self { key_id, key }
    }
}

/// Everything needed to acquire a licence.
#[derive(Debug, Clone)]
pub struct LicenseParams {
    /// The PSSH box bytes.
    pub pssh: Vec<u8>,
    /// Content ID, when the provider has one.
    pub content_id: Option<Vec<u8>>,
}

/// Builds and signs a licence request.
///
/// # Errors
///
/// Fails when the device identity cannot be used to sign.
pub fn build_request(
    identity: &DeviceIdentity,
    params: &LicenseParams,
    now_unix: u64,
) -> Result<Vec<u8>, LicenseError> {
    let pssh = WidevinePsshData::decode(params.pssh.as_slice())
        .map_err(|_| LicenseError::Malformed("PSSH payload is not a WidevineCencHeader".into()))?;

    let mut nonce = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    let arc_request = LicenseRequest {
        client_id: Some(identity.client_identification()),
        content_id: Some(ContentIdentification {
            widevine_pssh_data: Some(pssh),
            license_release: None,
            request_id: None,
        }),
        r#type: LicenseType::Streaming as i32,
        request_time: now_unix,
        key_control_nonce: nonce,
        encrypted_client_id: None,
    };

    let msg = arc_request.encode_to_vec();
    let signature = identity.sign(&msg).map_err(|e| LicenseError::Crypto(e.to_string()))?;

    let signed = SignedMessage {
        r#type: MessageType::LicenseRequest as i32,
        msg,
        signature,
        session_key: Vec::new(),
        remote_attestation_certificate: None,
    };
    Ok(signed.encode_to_vec())
}

/// Parses a signed licence response and returns the content keys.
///
/// # Errors
///
/// Returns [`LicenseError`] when the response is a licence error, cannot be
/// parsed, or its keys cannot be decrypted.
pub fn parse_license(
    identity: &DeviceIdentity,
    response: &[u8],
) -> Result<Vec<ContentKey>, LicenseError> {
    let signed = SignedMessage::decode(response)
        .map_err(|e| LicenseError::Malformed(format!("signed message: {e}")))?;

    if signed.r#type == MessageType::LicenseError as i32 {
        return Err(LicenseError::Refused(String::from_utf8_lossy(&signed.msg).into_owned()));
    }
    if signed.r#type != MessageType::License as i32 {
        return Err(LicenseError::Malformed(format!("unexpected message type {}", signed.r#type)));
    }

    // The session key is RSA-encrypted with the device public key.
    let session_key = identity
        .private_key()
        .decrypt(Pkcs1v15Encrypt, &signed.session_key)
        .map_err(|e| LicenseError::Crypto(format!("session key: {e}")))?;

    let license = License::decode(signed.msg.as_slice())
        .map_err(|e| LicenseError::Malformed(e.to_string()))?;

    let mut out = Vec::with_capacity(license.key.len());
    for (index, encrypted) in license.key.iter().enumerate() {
        let key_id = license.key_ids.get(index).cloned().unwrap_or_default();
        let plain = decrypt_content_key(&session_key, encrypted)?;
        let key: [u8; 16] = plain
            .get(..16)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| LicenseError::Crypto("content key is not 16 bytes".into()))?;
        out.push(ContentKey { key_id, key });
    }

    if out.is_empty() {
        return Err(LicenseError::NoContentKey);
    }
    Ok(out)
}

/// AES-128-CBC-decrypts one content key, with the IV in the leading block.
fn decrypt_content_key(session_key: &[u8], encrypted: &[u8]) -> Result<Vec<u8>, LicenseError> {
    // Widevine places the 16-byte IV at the front of the encrypted key.
    if encrypted.len() < 32 {
        return Err(LicenseError::Crypto("encrypted content key too short".into()));
    }
    let (iv, body) = encrypted.split_at(16);
    let cipher = Aes128CbcDec::new_from_slices(&session_key[..16], iv)
        .map_err(|e| LicenseError::Crypto(e.to_string()))?;
    cipher
        .decrypt_padded_vec_mut::<Pkcs7>(body)
        .map_err(|_| LicenseError::Crypto("content key padding invalid".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{block_padding::Pkcs7 as Pad, BlockEncryptMut, KeyIvInit};
    use aes::Aes128;
    use rand::rngs::OsRng;
    use rsa::RsaPrivateKey;

    type Aes128CbcEnc = cbc::Encryptor<Aes128>;

    fn test_identity() -> DeviceIdentity {
        use super::super::license::{ClientIdentification, ClientIdentificationType};
        let key = RsaPrivateKey::new(&mut OsRng, 1024).expect("keygen");
        let blob = ClientIdentification {
            r#type: ClientIdentificationType::DeviceCertificate as i32,
            token: vec![1, 2, 3],
            client_info: vec![],
            provider_client_token: None,
            license_counter: 0,
            client_capabilities: None,
        }
        .encode_to_vec();
        DeviceIdentity::from_parts(key, blob).expect("identity")
    }

    #[test]
    fn request_carries_the_pssh_and_a_signature() {
        let identity = test_identity();
        let pssh_bytes =
            super::super::pssh::Pssh::for_key_id(&[0xab; 16]).expect("pssh").as_bytes().to_vec();
        // Extract the inner WidevineCencHeader bytes (skip the 32-byte box header).
        let params = LicenseParams { pssh: pssh_bytes[32..].to_vec(), content_id: None };

        let encoded = build_request(&identity, &params, 1_700_000_000).expect("build");
        let signed = SignedMessage::decode(encoded.as_slice()).expect("decode");
        assert_eq!(signed.r#type, MessageType::LicenseRequest as i32);
        assert!(!signed.signature.is_empty());

        let request = LicenseRequest::decode(signed.msg.as_slice()).expect("request");
        assert_eq!(request.r#type, LicenseType::Streaming as i32);
        assert_eq!(request.key_control_nonce.len(), 32);
        assert!(request.client_id.is_some());
    }

    #[test]
    fn licence_error_is_surfaced() {
        let identity = test_identity();
        let signed = SignedMessage {
            r#type: MessageType::LicenseError as i32,
            msg: b"denied".to_vec(),
            signature: vec![],
            session_key: vec![],
            remote_attestation_certificate: None,
        };
        let err = parse_license(&identity, &signed.encode_to_vec()).expect_err("must fail");
        assert!(matches!(err, LicenseError::Refused(msg) if msg == "denied"));
    }

    #[test]
    fn licence_session_key_is_unwrapped_and_content_key_decrypted() {
        let identity = test_identity();

        // Simulate the licence server: random session key, encrypt it with the
        // device public key, then encrypt a content key under it.
        let session_key = [0x42u8; 16];
        let wrapped = identity
            .public_key()
            .encrypt(&mut OsRng, Pkcs1v15Encrypt, &session_key)
            .expect("wrap session key");

        let content_key = [0x24u8; 16];
        let iv = [0x11u8; 16];
        let cipher = Aes128CbcEnc::new_from_slices(&session_key, &iv).expect("cipher");
        let mut encrypted_key = cipher.encrypt_padded_vec_mut::<Pad>(&content_key);
        let mut payload = iv.to_vec();
        payload.append(&mut encrypted_key);

        let license = License {
            id: None,
            policy: None,
            key: vec![payload],
            key_ids: vec![vec![0xcd; 16]],
            key_type: None,
        };
        let signed = SignedMessage {
            r#type: MessageType::License as i32,
            msg: license.encode_to_vec(),
            signature: vec![],
            session_key: wrapped,
            remote_attestation_certificate: None,
        };

        let keys = parse_license(&identity, &signed.encode_to_vec()).expect("parse");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].bytes(), &content_key);
        assert_eq!(keys[0].key_id(), &[0xcd; 16]);
    }

    #[test]
    fn empty_licence_reports_no_content_key() {
        let identity = test_identity();
        let session_key = [0x01u8; 16];
        let wrapped =
            identity.public_key().encrypt(&mut OsRng, Pkcs1v15Encrypt, &session_key).expect("wrap");
        let signed = SignedMessage {
            r#type: MessageType::License as i32,
            msg: License { id: None, policy: None, key: vec![], key_ids: vec![], key_type: None }
                .encode_to_vec(),
            signature: vec![],
            session_key: wrapped,
            remote_attestation_certificate: None,
        };
        assert!(matches!(
            parse_license(&identity, &signed.encode_to_vec()),
            Err(LicenseError::NoContentKey)
        ));
    }

    #[test]
    fn content_key_debug_never_leaks_the_key() {
        let key = ContentKey { key_id: vec![0xab; 16], key: [0x55; 16] };
        let debug = format!("{key:?}");
        assert!(!debug.contains("55"));
        assert!(debug.contains("abab"));
    }
}
