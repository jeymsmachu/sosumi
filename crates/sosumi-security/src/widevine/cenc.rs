//! CENC AES-CTR sample decryption.
//!
//! Widevine-protected HLS segments use CENC (`ISO/IEC 23001-7`). In CTR mode the
//! 16-byte initialisation vector is per-sample and is *not* prepended to the
//! sample; it is carried in the track's `senc`/`tenc` boxes or, for the simple
//! scheme Apple uses, derived from the sample's sequence number. This module
//! decrypts a buffer in place given a key and IV.
//!
//! CTR is a stream cipher: encryption and decryption are the same operation, and
//! the byte range can be decrypted out of order by advancing the counter, which
//! is what makes seeking cheap.

use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use ctr::Ctr128BE;

use super::license::ContentKey;

type Aes128Ctr = Ctr128BE<aes::Aes128>;

/// Errors from CENC decryption.
#[derive(Debug, thiserror::Error)]
pub enum CencError {
    /// The initialisation vector is not a supported length.
    #[error("IV must be 8 or 16 bytes, got {0}")]
    BadIv(usize),
    /// The counter offset would run past the counter space.
    #[error("counter offset {0} exceeds the encrypted range")]
    BadOffset(u64),
}

/// Decrypts CENC samples with one content key.
///
/// Cheap to construct; hold one per track and reuse it across samples.
pub struct CencDecryptor {
    key: [u8; 16],
    iv: [u8; 16],
}

impl std::fmt::Debug for CencDecryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CencDecryptor").finish_non_exhaustive()
    }
}

impl CencDecryptor {
    /// Builds a decryptor from a content key and a per-sample IV.
    ///
    /// An 8-byte IV is zero-extended to 16 bytes, matching the CENC default
    /// where the IV is the sample's 64-bit sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`CencError::BadIv`] when the IV is neither 8 nor 16 bytes.
    pub fn new(key: &ContentKey, iv: &[u8]) -> Result<Self, CencError> {
        let mut full_iv = [0u8; 16];
        match iv.len() {
            8 => full_iv[8..].copy_from_slice(iv),
            16 => full_iv.copy_from_slice(iv),
            other => return Err(CencError::BadIv(other)),
        }
        Ok(Self { key: *key.bytes(), iv: full_iv })
    }

    /// Decrypts a whole sample in place.
    pub fn decrypt(&self, data: &mut [u8]) {
        let mut cipher = Aes128Ctr::new((&self.key).into(), (&self.iv).into());
        cipher.apply_keystream(data);
    }

    /// Decrypts `data` starting `offset` bytes into the sample's keystream.
    ///
    /// Used when resuming mid-sample, for example after a seek into the middle
    /// of a segment.
    ///
    /// # Errors
    ///
    /// Returns [`CencError::BadOffset`] when the offset cannot be represented by
    /// the block counter.
    pub fn decrypt_at(&self, offset: u64, data: &mut [u8]) -> Result<(), CencError> {
        if offset > (1u64 << 38) {
            return Err(CencError::BadOffset(offset));
        }
        let mut cipher = Aes128Ctr::new((&self.key).into(), (&self.iv).into());
        cipher.seek(offset);
        cipher.apply_keystream(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> ContentKey {
        // Built through the licence path so the invariant (16-byte key) holds.
        ContentKey::from_bytes_for_test(*b"0123456789abcdef", vec![0xaa; 16])
    }

    #[test]
    fn round_trip_is_identity() {
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let decryptor = CencDecryptor::new(&key(), &[0u8; 8]).expect("valid iv");

        let mut buffer = plaintext.to_vec();
        decryptor.decrypt(&mut buffer);
        assert_ne!(&buffer, plaintext, "ciphertext must differ from plaintext");

        let encryptor = CencDecryptor::new(&key(), &[0u8; 8]).expect("valid iv");
        encryptor.decrypt(&mut buffer);
        assert_eq!(&buffer, plaintext);
    }

    #[test]
    fn offset_decryption_matches_whole_buffer() {
        let plaintext: Vec<u8> = (0..64).collect();
        let decryptor = CencDecryptor::new(&key(), &[7u8; 16]).expect("valid iv");

        let mut whole = plaintext.clone();
        decryptor.decrypt(&mut whole);

        let mut tail = plaintext[32..].to_vec();
        decryptor.decrypt_at(32, &mut tail).expect("offset in range");

        assert_eq!(&tail, &whole[32..]);
    }

    #[test]
    fn eight_and_sixteen_byte_ivs_are_both_accepted() {
        assert!(CencDecryptor::new(&key(), &[0u8; 8]).is_ok());
        assert!(CencDecryptor::new(&key(), &[0u8; 16]).is_ok());
    }

    #[test]
    fn a_short_iv_is_rejected() {
        assert!(matches!(CencDecryptor::new(&key(), &[0u8; 4]), Err(CencError::BadIv(4))));
    }

    #[test]
    fn different_ivs_produce_different_ciphertext() {
        let data = b"same plaintext for both".to_vec();
        let a = CencDecryptor::new(&key(), &[1u8; 16]).expect("iv");
        let b = CencDecryptor::new(&key(), &[2u8; 16]).expect("iv");

        let mut ea = data.clone();
        let mut eb = data;
        a.decrypt(&mut ea);
        b.decrypt(&mut eb);
        assert_ne!(ea, eb);
    }

    #[test]
    fn debug_never_leaks_key_material() {
        let decryptor = CencDecryptor::new(&key(), &[0u8; 16]).expect("iv");
        assert_eq!(format!("{decryptor:?}"), "CencDecryptor { .. }");
    }
}
