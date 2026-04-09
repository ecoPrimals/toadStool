// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP protocol types per `BTSP_PROTOCOL_STANDARD.md` v1.0.0.

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// BTSP protocol version.
pub const BTSP_VERSION: u32 = 1;

/// HKDF salt for deriving the handshake key from the family seed.
pub const HANDSHAKE_HKDF_SALT: &[u8] = b"btsp-v1";

/// HKDF info for deriving the handshake key.
pub const HANDSHAKE_HKDF_INFO: &[u8] = b"handshake";

/// HKDF info for deriving session keys after ECDH.
pub const SESSION_HKDF_INFO: &[u8] = b"btsp-session-v1";

/// Maximum BTSP frame size: 16 MiB.
pub const MAX_FRAME_SIZE: u32 = 0x0100_0000;

/// Step 1: Client → Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// Protocol version (must be `BTSP_VERSION`).
    pub version: u32,
    /// X25519 ephemeral public key (32 bytes, base64-encoded on wire).
    pub client_ephemeral_pub: [u8; 32],
}

/// Step 2: Server → Client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    /// Protocol version.
    pub version: u32,
    /// X25519 ephemeral public key (32 bytes).
    pub server_ephemeral_pub: [u8; 32],
    /// 32-byte random challenge.
    pub challenge: [u8; 32],
}

/// Step 3: Client → Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    /// HMAC-SHA256 over `challenge ‖ client_pub ‖ server_pub` with handshake_key.
    pub response: [u8; 32],
    /// Client's preferred cipher suite.
    pub preferred_cipher: BtspCipher,
}

/// Step 4: Server → Client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeComplete {
    /// Negotiated cipher suite.
    pub cipher: BtspCipher,
    /// 16-byte session identifier.
    pub session_id: [u8; 16],
}

/// Negotiable cipher suites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtspCipher {
    /// ChaCha20-Poly1305 AEAD — confidentiality + integrity + authentication.
    Chacha20Poly1305,
    /// HMAC-SHA256 per frame — integrity + authentication, no confidentiality.
    HmacPlain,
    /// No per-frame protection — Covalent bonds only.
    Null,
}

impl BtspCipher {
    /// Wire name for cipher negotiation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Chacha20Poly1305 => "chacha20_poly1305",
            Self::HmacPlain => "hmac_plain",
            Self::Null => "null",
        }
    }
}

/// Directional session keys derived from the ECDH shared secret.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SessionKeys {
    /// Key for encrypting outgoing frames.
    pub encrypt_key: [u8; 32],
    /// Key for decrypting incoming frames.
    pub decrypt_key: [u8; 32],
}

/// BTSP handshake errors.
#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    /// I/O error during handshake.
    #[error("BTSP I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol version mismatch.
    #[error("BTSP version mismatch: expected {expected}, got {got}")]
    VersionMismatch {
        /// Expected version.
        expected: u32,
        /// Received version.
        got: u32,
    },

    /// Server rejected the challenge response (wrong family seed).
    #[error("BTSP handshake failed: family verification")]
    FamilyVerification,

    /// JSON serialization/deserialization error.
    #[error("BTSP serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Key derivation failure.
    #[error("BTSP key derivation error: {0}")]
    KeyDerivation(String),
}
