//! The software Widevine CDM used for the baseline AAC tier.
//!
//! Apple's web-playback HLS serves AAC encrypted with Widevine (CENC/CTR). This
//! module implements the licence-acquisition handshake in pure Rust: it builds a
//! `LicenseRequest`, signs it with a device RSA key, unwraps the licence's
//! session key, and yields the 16-byte content key.
//!
//! # Trust boundary
//!
//! [`ContentKey`] never leaves this crate. Callers receive a `Decryptor` that
//! applies and discards keys internally. Nothing here is logged at `info` or
//! above; key bytes are only ever held in [`zeroize`]-style local buffers that
//! are dropped as soon as the cipher is constructed.
//!
//! # Device identity
//!
//! A licence server accepts only a device identity it recognises. This module
//! does not ship one and does not generate one silently: [`DeviceIdentity`] is
//! loaded from the user's config directory, and a missing identity surfaces as
//! an actionable [`IdentityError::NotConfigured`] that the Settings screen
//! turns into a setup prompt.

pub mod cenc;
pub mod identity;
pub mod license;
pub mod pssh;

pub use cenc::{CencDecryptor, CencError};
pub use identity::{DeviceIdentity, IdentityError};
pub use license::{ContentKey, LicenseError};
pub use pssh::{Pssh, PsshError};
