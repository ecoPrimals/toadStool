// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request and response types for security RPC calls.

use serde::{Deserialize, Serialize};

/// Encryption request
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionRequest {
    /// Plaintext or payload bytes to encrypt.
    pub data: Vec<u8>,
    /// Algorithm identifier (e.g. AES-256-GCM).
    pub algorithm: String,
    /// Optional key id when not using the default key.
    pub key_id: Option<String>,
}

/// Encryption response
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    /// Ciphertext returned by the provider.
    pub encrypted_data: Vec<u8>,
    /// Key id used for this ciphertext.
    pub key_id: String,
    /// Algorithm used for encryption.
    pub algorithm: String,
}

/// Decryption request
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionRequest {
    /// Ciphertext to decrypt.
    pub encrypted_data: Vec<u8>,
    /// Key id for decryption.
    pub key_id: String,
}

/// Decryption response
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    /// Recovered plaintext.
    pub data: Vec<u8>,
}

/// Signature request
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureRequest {
    /// Data to sign.
    pub data: Vec<u8>,
    /// Signing algorithm identifier.
    pub algorithm: String,
    /// Optional key id when not using the default key.
    pub key_id: Option<String>,
}

/// Signature response
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureResponse {
    /// Raw signature bytes.
    pub signature: Vec<u8>,
    /// Key id used for signing.
    pub key_id: String,
    /// Algorithm used for the signature.
    pub algorithm: String,
}

/// Verification request
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationRequest {
    /// Original signed data.
    pub data: Vec<u8>,
    /// Signature to verify.
    pub signature: Vec<u8>,
    /// Public key or key id for verification.
    pub key_id: String,
}

/// Verification response
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResponse {
    /// Whether the signature is valid.
    pub valid: bool,
    /// Optional human-readable failure reason.
    pub reason: Option<String>,
}

/// Token validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenValidationRequest {
    /// Opaque token string (e.g. JWT).
    pub token: String,
}

/// Token validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenValidationResponse {
    /// Whether the token is valid.
    pub valid: bool,
    /// Authenticated subject id when valid.
    pub user_id: Option<String>,
    /// Granted OAuth-style scopes.
    pub scopes: Vec<String>,
    /// Expiry as Unix epoch seconds when known.
    pub expires_at: Option<i64>,
}
