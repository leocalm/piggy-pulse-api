use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Test DEK — must match the one sent in unlock_session (32 zero bytes).
const TEST_DEK_BYTES: [u8; 32] = [0u8; 32];

/// Decrypt an AES-GCM envelope (base64-encoded nonce+ciphertext) with the test DEK.
#[allow(dead_code)]
pub fn decrypt_envelope(envelope_b64: &str) -> Vec<u8> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    let envelope = BASE64.decode(envelope_b64.as_bytes()).expect("valid base64");
    if envelope.len() < 12 + 16 {
        panic!("envelope too short");
    }
    let (nonce_bytes, ciphertext) = envelope.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(&TEST_DEK_BYTES).expect("valid key");
    let nonce = Nonce::try_from(nonce_bytes).expect("nonce is always 12 bytes");
    cipher.decrypt(&nonce, ciphertext).expect("decrypt with test DEK")
}

/// Decrypt an encrypted i64 field from an AES-GCM envelope.
#[allow(dead_code)]
pub fn decrypt_i64(envelope_b64: &str) -> i64 {
    let plaintext = decrypt_envelope(envelope_b64);
    let bytes: [u8; 8] = plaintext.try_into().expect("8 bytes for i64");
    i64::from_le_bytes(bytes)
}

/// Decrypt an encrypted string field from an AES-GCM envelope.
#[allow(dead_code)]
pub fn decrypt_string(envelope_b64: &str) -> String {
    let plaintext = decrypt_envelope(envelope_b64);
    String::from_utf8(plaintext).expect("valid UTF-8")
}

/// Decrypt an optional encrypted i64 field. Returns None if the value is null/missing.
#[allow(dead_code)]
pub fn decrypt_i64_opt(val: &serde_json::Value) -> Option<i64> {
    val.as_str().map(decrypt_i64)
}

/// Decrypt an optional encrypted string field. Returns None if the value is null/missing.
#[allow(dead_code)]
pub fn decrypt_string_opt(val: &serde_json::Value) -> Option<String> {
    val.as_str().map(decrypt_string)
}
