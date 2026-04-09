// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP client-side handshake implementation.
//!
//! Performs the 4-step handshake as the **initiating** side:
//!
//! 1. Send `ClientHello` (ephemeral X25519 public key)
//! 2. Receive `ServerHello` (server public key + challenge)
//! 3. Compute HMAC and send `ChallengeResponse`
//! 4. Receive `HandshakeComplete` (negotiated cipher + session ID)
//!
//! After handshake, derives session keys via ECDH + HKDF and returns
//! a [`BtspSession`] ready for length-prefixed framing.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use x25519_dalek::{EphemeralSecret, PublicKey};

use super::framing;
use super::types::*;

type HmacSha256 = Hmac<Sha256>;

/// Derive the handshake key from a family seed using HKDF-SHA256.
///
/// `HKDF(ikm=family_seed, salt="btsp-v1", info="handshake") → 32 bytes`
fn derive_handshake_key(family_seed: &[u8]) -> Result<[u8; 32], HandshakeError> {
    use hkdf::Hkdf;

    let hk = Hkdf::<Sha256>::new(Some(HANDSHAKE_HKDF_SALT), family_seed);
    let mut okm = [0u8; 32];
    hk.expand(HANDSHAKE_HKDF_INFO, &mut okm)
        .map_err(|e| HandshakeError::KeyDerivation(format!("handshake HKDF: {e}")))?;
    Ok(okm)
}

/// Derive directional session keys from the ECDH shared secret and session ID.
fn derive_session_keys(
    shared_secret: &[u8; 32],
    session_id: &[u8; 16],
) -> Result<SessionKeys, HandshakeError> {
    use hkdf::Hkdf;

    let hk = Hkdf::<Sha256>::new(Some(session_id), shared_secret);
    let mut encrypt_key = [0u8; 32];
    let mut decrypt_key = [0u8; 32];

    hk.expand(b"client-encrypt", &mut encrypt_key)
        .map_err(|e| HandshakeError::KeyDerivation(format!("encrypt key: {e}")))?;
    hk.expand(b"client-decrypt", &mut decrypt_key)
        .map_err(|e| HandshakeError::KeyDerivation(format!("decrypt key: {e}")))?;

    Ok(SessionKeys {
        encrypt_key,
        decrypt_key,
    })
}

/// Completed BTSP session with negotiated parameters.
pub struct BtspSession {
    /// Negotiated cipher suite.
    pub cipher: BtspCipher,
    /// Session identifier (16 bytes).
    pub session_id: [u8; 16],
    /// Directional session keys.
    pub keys: SessionKeys,
}

/// BTSP client — performs handshake on an established connection.
pub struct BtspClient;

impl BtspClient {
    /// Perform the BTSP handshake over an async stream.
    ///
    /// `family_seed` is the shared secret derived from `.family.seed` or
    /// the `FAMILY_SEED` environment variable. Both sides must use the
    /// same seed for the handshake to succeed.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError`] if the handshake fails (version mismatch,
    /// family verification failure, I/O error, etc.).
    pub async fn handshake<S>(
        stream: &mut S,
        family_seed: &[u8],
        preferred_cipher: BtspCipher,
    ) -> Result<BtspSession, HandshakeError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        let handshake_key = derive_handshake_key(family_seed)?;

        // Generate ephemeral X25519 keypair
        let client_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let client_public = PublicKey::from(&client_secret);

        // Step 1: Send ClientHello
        let hello = ClientHello {
            version: BTSP_VERSION,
            client_ephemeral_pub: *client_public.as_bytes(),
        };
        let hello_bytes = serde_json::to_vec(&hello)?;
        framing::write_frame(stream, &hello_bytes).await?;

        // Step 2: Receive ServerHello
        let server_hello_bytes = framing::read_frame(stream).await?;
        let server_hello: ServerHello = serde_json::from_slice(&server_hello_bytes)?;

        if server_hello.version != BTSP_VERSION {
            return Err(HandshakeError::VersionMismatch {
                expected: BTSP_VERSION,
                got: server_hello.version,
            });
        }

        // Step 3: Compute HMAC(challenge ‖ client_pub ‖ server_pub) and send ChallengeResponse
        let mut mac = HmacSha256::new_from_slice(&handshake_key)
            .map_err(|e| HandshakeError::KeyDerivation(format!("HMAC init: {e}")))?;
        mac.update(&server_hello.challenge);
        mac.update(client_public.as_bytes());
        mac.update(&server_hello.server_ephemeral_pub);
        let hmac_result = mac.finalize().into_bytes();

        let mut response_bytes = [0u8; 32];
        response_bytes.copy_from_slice(&hmac_result);

        let challenge_response = ChallengeResponse {
            response: response_bytes,
            preferred_cipher,
        };
        let cr_bytes = serde_json::to_vec(&challenge_response)?;
        framing::write_frame(stream, &cr_bytes).await?;

        // Step 4: Receive HandshakeComplete (or error)
        let complete_bytes = framing::read_frame(stream).await?;

        // Check for error response
        if let Ok(err_obj) = serde_json::from_slice::<serde_json::Value>(&complete_bytes) {
            if err_obj.get("error").is_some() {
                return Err(HandshakeError::FamilyVerification);
            }
        }

        let complete: HandshakeComplete = serde_json::from_slice(&complete_bytes)?;

        // Derive session keys via ECDH
        let server_public = PublicKey::from(server_hello.server_ephemeral_pub);
        let shared_secret = client_secret.diffie_hellman(&server_public);

        let keys = derive_session_keys(shared_secret.as_bytes(), &complete.session_id)?;

        Ok(BtspSession {
            cipher: complete.cipher,
            session_id: complete.session_id,
            keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_key_derivation_is_deterministic() {
        let seed = b"test-family-seed-32-bytes-long!!";
        let key1 = derive_handshake_key(seed).expect("key1");
        let key2 = derive_handshake_key(seed).expect("key2");
        assert_eq!(key1, key2);
    }

    #[test]
    fn handshake_key_derivation_different_seeds_differ() {
        let key1 = derive_handshake_key(b"seed-alpha").expect("key1");
        let key2 = derive_handshake_key(b"seed-beta").expect("key2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn session_key_derivation_produces_different_encrypt_decrypt() {
        let shared = [42u8; 32];
        let session_id = [7u8; 16];
        let keys = derive_session_keys(&shared, &session_id).expect("keys");
        assert_ne!(keys.encrypt_key, keys.decrypt_key);
    }

    #[test]
    fn session_key_derivation_is_deterministic() {
        let shared = [99u8; 32];
        let sid = [1u8; 16];
        let k1 = derive_session_keys(&shared, &sid).expect("k1");
        let k2 = derive_session_keys(&shared, &sid).expect("k2");
        assert_eq!(k1.encrypt_key, k2.encrypt_key);
        assert_eq!(k1.decrypt_key, k2.decrypt_key);
    }

    #[tokio::test]
    async fn client_hello_round_trip_serialization() {
        let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let public = PublicKey::from(&secret);

        let hello = ClientHello {
            version: BTSP_VERSION,
            client_ephemeral_pub: *public.as_bytes(),
        };
        let bytes = serde_json::to_vec(&hello).expect("serialize");
        let decoded: ClientHello = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(decoded.version, BTSP_VERSION);
        assert_eq!(decoded.client_ephemeral_pub, *public.as_bytes());
    }

    #[test]
    fn cipher_as_str() {
        assert_eq!(BtspCipher::Chacha20Poly1305.as_str(), "chacha20_poly1305");
        assert_eq!(BtspCipher::HmacPlain.as_str(), "hmac_plain");
        assert_eq!(BtspCipher::Null.as_str(), "null");
    }
}
