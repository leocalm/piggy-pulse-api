//! Encryption-at-rest primitives for PiggyPulse.
//!
//! See `.kiro/specs/encryption-at-rest/design.md` for the full design. Short
//! version: every user has a DEK (Data Encryption Key). The DEK is derived at
//! signup, wrapped by a KEK derived from the user's password via Argon2id,
//! stored wrapped on the server, and uploaded plaintext to a Redis session
//! store on login via `POST /v2/auth/unlock`. The service layer pulls the DEK
//! from the session store per request and uses it to encrypt on write and
//! decrypt on read.
//!
//! This module hosts the primitive types and pure functions. Higher-level
//! wiring (session store, request guard, unlock endpoint) lives in
//! `auth.rs` and `routes/v2/auth/unlock.rs`.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::Rng;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The per-user Data Encryption Key.
///
/// 32 bytes, zeroed on drop. Intentionally missing `Debug`, `Display`,
/// `Serialize`, `Deserialize`, and `Clone` derives: any code path that needs
/// to expose, log, or duplicate the key must be explicit about it at the call
/// site. `Dek::clone_for_request` is the only sanctioned way to obtain a
/// second owned copy, and its name is deliberately awkward.
///
/// When adding new fields that contain a `Dek`, do NOT derive `Debug` on
/// the enclosing struct — the custom clippy lint in CI will reject it, but
/// defense in depth means we don't rely on the lint alone.
#[derive(ZeroizeOnDrop)]
pub struct Dek([u8; 32]);

impl Dek {
    /// Construct from 32 raw bytes. Caller is responsible for ensuring the
    /// bytes came from a CSPRNG or a verified unwrap operation.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Dek(bytes)
    }

    /// Generate a fresh random DEK from the OS CSPRNG. Used at user signup
    /// and during an explicit DEK rotation.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Dek(bytes)
    }

    /// Copy the DEK into a new owned `Dek`. Named awkwardly on purpose: every
    /// call site should be a conscious decision to hold two copies.
    pub fn clone_for_request(&self) -> Self {
        Dek(self.0)
    }

    /// Expose the raw bytes. Only intended for wrapping (encrypting the DEK
    /// with a KEK) and for writing into a session store. Never log the result.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new((&self.0).into())
    }

    /// Encrypt an arbitrary byte slice. Returns the envelope
    /// `nonce (12B) || ciphertext || tag (16B)`.
    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher()
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptFailed)?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt an envelope produced by `encrypt_bytes`. Returns the plaintext
    /// as a fresh `Vec<u8>`.
    pub fn decrypt_bytes(&self, envelope: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if envelope.len() < 12 + 16 {
            return Err(CryptoError::EnvelopeTooShort);
        }
        let (nonce_bytes, ciphertext) = envelope.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher()
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptFailed)
    }

    /// Encrypt an i64 (little-endian).
    pub fn encrypt_i64(&self, value: i64) -> Result<Vec<u8>, CryptoError> {
        self.encrypt_bytes(&value.to_le_bytes())
    }

    /// Decrypt an i64 envelope. Zeroes the intermediate plaintext buffer
    /// before returning.
    pub fn decrypt_i64(&self, envelope: &[u8]) -> Result<i64, CryptoError> {
        let mut plaintext = self.decrypt_bytes(envelope)?;
        if plaintext.len() != 8 {
            plaintext.zeroize();
            return Err(CryptoError::PlaintextLengthMismatch);
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&plaintext);
        plaintext.zeroize();
        Ok(i64::from_le_bytes(arr))
    }

    /// Encrypt a UTF-8 string.
    pub fn encrypt_string(&self, value: &str) -> Result<Vec<u8>, CryptoError> {
        self.encrypt_bytes(value.as_bytes())
    }

    /// Decrypt an envelope that was produced from a UTF-8 string. Zeroes the
    /// intermediate buffer before returning.
    pub fn decrypt_string(&self, envelope: &[u8]) -> Result<String, CryptoError> {
        let mut plaintext = self.decrypt_bytes(envelope)?;
        let result = String::from_utf8(plaintext.clone());
        plaintext.zeroize();
        result.map_err(|_| CryptoError::InvalidUtf8)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptFailed,
    #[error("decryption failed (tag mismatch or corrupted ciphertext)")]
    DecryptFailed,
    #[error("envelope is too short to contain a nonce + tag")]
    EnvelopeTooShort,
    #[error("decrypted plaintext is not the expected length for the target type")]
    PlaintextLengthMismatch,
    #[error("decrypted plaintext is not valid UTF-8")]
    InvalidUtf8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_roundtrip_zero_and_extremes() {
        let dek = Dek::generate();
        for v in [0i64, 1, -1, 1_234_567_890, -987_654_321, i64::MAX, i64::MIN] {
            let env = dek.encrypt_i64(v).unwrap();
            assert_eq!(dek.decrypt_i64(&env).unwrap(), v);
        }
    }

    #[test]
    fn i64_envelope_is_stable_36_bytes() {
        // 12 nonce + 8 ct + 16 tag
        let dek = Dek::generate();
        let env = dek.encrypt_i64(42).unwrap();
        assert_eq!(env.len(), 36);
    }

    #[test]
    fn string_roundtrip_ascii_and_unicode() {
        let dek = Dek::generate();
        for s in ["", "hello", "café au lait", "🍕 pizza", "こんにちは"] {
            let env = dek.encrypt_string(s).unwrap();
            assert_eq!(dek.decrypt_string(&env).unwrap(), s);
        }
    }

    #[test]
    fn bytes_roundtrip_empty_and_long() {
        let dek = Dek::generate();
        let empty = dek.encrypt_bytes(&[]).unwrap();
        assert_eq!(dek.decrypt_bytes(&empty).unwrap(), Vec::<u8>::new());

        let long: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let env = dek.encrypt_bytes(&long).unwrap();
        assert_eq!(dek.decrypt_bytes(&env).unwrap(), long);
    }

    #[test]
    fn fresh_nonces_produce_different_envelopes_for_same_plaintext() {
        let dek = Dek::generate();
        let a = dek.encrypt_i64(42).unwrap();
        let b = dek.encrypt_i64(42).unwrap();
        assert_ne!(a, b, "fresh-nonce property violated");
        assert_eq!(dek.decrypt_i64(&a).unwrap(), 42);
        assert_eq!(dek.decrypt_i64(&b).unwrap(), 42);
    }

    #[test]
    fn wrong_dek_rejects_i64() {
        let a = Dek::generate();
        let b = Dek::generate();
        let env = a.encrypt_i64(100).unwrap();
        assert!(b.decrypt_i64(&env).is_err());
    }

    #[test]
    fn wrong_dek_rejects_string() {
        let a = Dek::generate();
        let b = Dek::generate();
        let env = a.encrypt_string("secret").unwrap();
        assert!(b.decrypt_string(&env).is_err());
    }

    #[test]
    fn corrupted_ciphertext_rejects() {
        let dek = Dek::generate();
        let mut env = dek.encrypt_i64(42).unwrap();
        env[20] ^= 0xFF; // flip a byte in the ciphertext region
        assert!(dek.decrypt_i64(&env).is_err());
    }

    #[test]
    fn corrupted_nonce_rejects() {
        let dek = Dek::generate();
        let mut env = dek.encrypt_string("hello").unwrap();
        env[0] ^= 0xFF; // flip a byte in the nonce region
        assert!(dek.decrypt_string(&env).is_err());
    }

    #[test]
    fn envelope_too_short_rejects() {
        let dek = Dek::generate();
        assert!(dek.decrypt_i64(&[0u8; 20]).is_err());
        assert!(dek.decrypt_i64(&[]).is_err());
    }

    #[test]
    fn invalid_utf8_returns_utf8_error() {
        let dek = Dek::generate();
        // Encrypt raw bytes that are not valid UTF-8, then try to decrypt as string.
        let env = dek.encrypt_bytes(&[0xFF, 0xFE, 0xFD]).unwrap();
        assert!(matches!(dek.decrypt_string(&env), Err(CryptoError::InvalidUtf8)));
    }

    #[test]
    fn clone_for_request_produces_independent_dek_with_same_key() {
        let a = Dek::generate();
        let b = a.clone_for_request();
        let env = a.encrypt_i64(42).unwrap();
        // The clone can decrypt what the original encrypted.
        assert_eq!(b.decrypt_i64(&env).unwrap(), 42);
    }

    #[test]
    fn clone_for_request_bytes_match() {
        let a = Dek::generate();
        let b = a.clone_for_request();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }
}
