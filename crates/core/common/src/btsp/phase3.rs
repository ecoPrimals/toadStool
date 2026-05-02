// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP Phase 3: encrypted channel (ChaCha20-Poly1305).
//!
//! After a successful Phase 1 handshake, the client sends `btsp.negotiate`
//! to upgrade from authenticated-plaintext (NULL cipher) to an encrypted
//! channel. Both sides derive directional session keys via HKDF-SHA256 and
//! switch all subsequent framing to AEAD-encrypted length-prefixed blobs.
//!
//! # Wire format (encrypted channel)
//!
//! Each frame after negotiation:
//! ```text
//! [4 bytes: length (big-endian u32)][12 bytes: nonce][length bytes: ciphertext + Poly1305 tag]
//! ```
//!
//! # Key derivation
//!
//! ```text
//! IKM:  handshake_key (32 bytes from Phase 1 HKDF)
//! Salt: client_nonce || server_nonce (64 bytes)
//! Info: "btsp-session-v1-c2s" → encrypt_key (client→server)
//!       "btsp-session-v1-s2c" → decrypt_key (server→client)
//! ```
//!
//! Server perspective: encrypt = s2c, decrypt = c2s (mirrored from client).

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::types::HANDSHAKE_HKDF_INFO;
use super::types::HANDSHAKE_HKDF_SALT;

/// 32-byte nonce used for HKDF salt in key derivation (not the per-frame AEAD nonce).
pub const NEGOTIATE_NONCE_LEN: usize = 32;

/// 12-byte nonce prepended to each encrypted frame (ChaCha20-Poly1305 standard).
pub const AEAD_NONCE_LEN: usize = 12;

/// Poly1305 authentication tag length.
pub const AEAD_TAG_LEN: usize = 16;

/// HKDF info for client-to-server key derivation.
const HKDF_INFO_C2S: &[u8] = b"btsp-session-v1-c2s";

/// HKDF info for server-to-client key derivation.
const HKDF_INFO_S2C: &[u8] = b"btsp-session-v1-s2c";

/// Request params for `btsp.negotiate` (matches primalSpring client wire format).
#[derive(Debug, Clone, Deserialize)]
pub struct NegotiateParams {
    /// Session ID from Phase 1 handshake.
    pub session_id: String,
    /// Ordered list of supported cipher suites (primalSpring uses this field).
    #[serde(default)]
    pub ciphers: Vec<String>,
    /// Single preferred cipher (used by some primals instead of `ciphers`).
    #[serde(default)]
    pub preferred_cipher: Option<String>,
    /// 32-byte random nonce from client, base64-encoded.
    #[serde(default)]
    pub client_nonce: Option<String>,
    /// Bond type (e.g. `"Covalent"`).
    #[serde(default)]
    pub bond_type: Option<String>,
}

/// Response for `btsp.negotiate`.
#[derive(Debug, Clone, Serialize)]
pub struct NegotiateResponse {
    /// Selected cipher suite (`"chacha20-poly1305"` or `"null"`).
    pub cipher: String,
    /// 32-byte random server nonce, base64-encoded (absent for null cipher).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_nonce: Option<String>,
}

/// Directional session keys for Phase 3 encrypted framing.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct Phase3SessionKeys {
    /// Key for encrypting outgoing frames (server→client on server side).
    pub encrypt_key: [u8; 32],
    /// Key for decrypting incoming frames (client→server on server side).
    pub decrypt_key: [u8; 32],
}

impl Phase3SessionKeys {
    /// Derive directional session keys from the Phase 1 handshake key and
    /// both nonces exchanged during `btsp.negotiate`.
    ///
    /// `is_server` controls which HKDF expansion is assigned to encrypt vs decrypt.
    ///
    /// # Errors
    ///
    /// Returns error if HKDF expansion fails (should not happen with valid inputs).
    pub fn derive(
        handshake_key: &[u8; 32],
        client_nonce: &[u8],
        server_nonce: &[u8],
        is_server: bool,
    ) -> Result<Self, Phase3Error> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let mut salt = Vec::with_capacity(client_nonce.len() + server_nonce.len());
        salt.extend_from_slice(client_nonce);
        salt.extend_from_slice(server_nonce);

        let hk = Hkdf::<Sha256>::new(Some(&salt), handshake_key);

        let mut c2s = [0u8; 32];
        hk.expand(HKDF_INFO_C2S, &mut c2s)
            .map_err(|e| Phase3Error::KeyDerivation(format!("c2s expand: {e}")))?;

        let mut s2c = [0u8; 32];
        hk.expand(HKDF_INFO_S2C, &mut s2c)
            .map_err(|e| Phase3Error::KeyDerivation(format!("s2c expand: {e}")))?;

        if is_server {
            Ok(Self {
                encrypt_key: s2c,
                decrypt_key: c2s,
            })
        } else {
            Ok(Self {
                encrypt_key: c2s,
                decrypt_key: s2c,
            })
        }
    }

    /// Encrypt a plaintext payload for transmission.
    ///
    /// Returns `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
    ///
    /// # Errors
    ///
    /// Returns error if AEAD encryption fails or nonce generation fails.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, Phase3Error> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{ChaCha20Poly1305, Nonce};

        let cipher = ChaCha20Poly1305::new((&self.encrypt_key).into());

        let mut nonce_bytes = [0u8; AEAD_NONCE_LEN];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| Phase3Error::Encryption(format!("ChaCha20-Poly1305 encrypt: {e}")))?;

        let mut frame = Vec::with_capacity(AEAD_NONCE_LEN + ciphertext.len());
        frame.extend_from_slice(&nonce_bytes);
        frame.extend_from_slice(&ciphertext);
        Ok(frame)
    }

    /// Decrypt an encrypted payload received from the wire.
    ///
    /// Input format: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
    ///
    /// # Errors
    ///
    /// Returns error if the input is too short or AEAD decryption/verification fails.
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, Phase3Error> {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{ChaCha20Poly1305, Nonce};

        if encrypted.len() < AEAD_NONCE_LEN + AEAD_TAG_LEN {
            return Err(Phase3Error::Decryption(format!(
                "encrypted payload too short: {} bytes (need >= {})",
                encrypted.len(),
                AEAD_NONCE_LEN + AEAD_TAG_LEN,
            )));
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(AEAD_NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = ChaCha20Poly1305::new((&self.decrypt_key).into());
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Phase3Error::Decryption(format!("ChaCha20-Poly1305 decrypt: {e}")))
    }
}

/// Derive the BTSP handshake key from raw family seed bytes.
///
/// Same derivation as Phase 1: `HKDF-SHA256(salt="btsp-v1", ikm=family_seed, info="handshake")`.
///
/// # Errors
///
/// Returns error if HKDF expansion fails.
pub fn derive_handshake_key(family_seed: &[u8]) -> Result<[u8; 32], Phase3Error> {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hk = Hkdf::<Sha256>::new(Some(HANDSHAKE_HKDF_SALT), family_seed);
    let mut okm = [0u8; 32];
    hk.expand(HANDSHAKE_HKDF_INFO, &mut okm)
        .map_err(|e| Phase3Error::KeyDerivation(format!("handshake HKDF: {e}")))?;
    Ok(okm)
}

/// Generate a 32-byte random nonce for negotiate key derivation.
pub fn generate_negotiate_nonce() -> [u8; NEGOTIATE_NONCE_LEN] {
    let mut nonce = [0u8; NEGOTIATE_NONCE_LEN];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
    nonce
}

/// Phase 3 errors.
#[derive(Debug, thiserror::Error)]
pub enum Phase3Error {
    /// HKDF key derivation failure.
    #[error("BTSP Phase 3 key derivation: {0}")]
    KeyDerivation(String),

    /// ChaCha20-Poly1305 encryption failure.
    #[error("BTSP Phase 3 encryption: {0}")]
    Encryption(String),

    /// ChaCha20-Poly1305 decryption / authentication failure.
    #[error("BTSP Phase 3 decryption: {0}")]
    Decryption(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_keys_derive_roundtrip() {
        let handshake_key = [42u8; 32];
        let client_nonce = [1u8; 32];
        let server_nonce = [2u8; 32];

        let server_keys =
            Phase3SessionKeys::derive(&handshake_key, &client_nonce, &server_nonce, true)
                .expect("server derive");
        let client_keys =
            Phase3SessionKeys::derive(&handshake_key, &client_nonce, &server_nonce, false)
                .expect("client derive");

        assert_eq!(server_keys.encrypt_key, client_keys.decrypt_key);
        assert_eq!(server_keys.decrypt_key, client_keys.encrypt_key);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let handshake_key = [99u8; 32];
        let client_nonce = [3u8; 32];
        let server_nonce = [4u8; 32];

        let server_keys =
            Phase3SessionKeys::derive(&handshake_key, &client_nonce, &server_nonce, true)
                .expect("derive");
        let client_keys =
            Phase3SessionKeys::derive(&handshake_key, &client_nonce, &server_nonce, false)
                .expect("derive");

        let plaintext = b"hello BTSP Phase 3 encrypted channel!";
        let encrypted = server_keys.encrypt(plaintext).expect("encrypt");
        let decrypted = client_keys.decrypt(&encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);

        let encrypted2 = client_keys.encrypt(plaintext).expect("encrypt");
        let decrypted2 = server_keys.decrypt(&encrypted2).expect("decrypt");
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn decrypt_rejects_tampered_data() {
        let keys =
            Phase3SessionKeys::derive(&[1u8; 32], &[2u8; 32], &[3u8; 32], true).expect("derive");

        let mut tampered = keys.encrypt(b"secret").expect("encrypt");
        if let Some(b) = tampered.last_mut() {
            *b ^= 0xFF;
        }
        assert!(keys.decrypt(&tampered).is_err());
    }

    #[test]
    fn decrypt_rejects_short_input() {
        let keys =
            Phase3SessionKeys::derive(&[1u8; 32], &[2u8; 32], &[3u8; 32], true).expect("derive");
        assert!(keys.decrypt(&[0u8; 10]).is_err());
    }

    #[test]
    fn derive_handshake_key_deterministic() {
        let seed = b"test-family-seed";
        let k1 = derive_handshake_key(seed).expect("derive");
        let k2 = derive_handshake_key(seed).expect("derive");
        assert_eq!(k1, k2);
    }

    #[test]
    fn generate_nonce_not_zero() {
        let n = generate_negotiate_nonce();
        assert_ne!(n, [0u8; 32]);
    }
}
