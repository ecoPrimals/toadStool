// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use super::security::SecurityLevel;
use super::types::{EncryptedPayload, EncryptionMetadata};

/// Encryption configuration for execution requests
///
/// **Design**: Optional encryption, graceful fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Whether encryption is required (vs. optional)
    pub required: bool,

    /// Preferred encryption algorithms (in priority order)
    pub preferred_algorithms: Vec<String>,

    /// Key identifier (if using pre-shared key)
    pub key_id: Option<String>,

    /// Whether to encrypt results
    pub encrypt_results: bool,

    /// Minimum security level required
    pub min_security_level: SecurityLevel,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            required: false,
            preferred_algorithms: vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()],
            key_id: None,
            encrypt_results: false,
            min_security_level: SecurityLevel::Standard,
        }
    }
}

/// Encrypted execution input
///
/// **Design**: Opaque encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedInput {
    /// The encrypted payload
    pub payload: EncryptedPayload,

    /// Key identifier used for encryption
    pub key_id: String,

    /// Encryption metadata (algorithm, nonce, etc.)
    pub metadata: EncryptionMetadata,

    /// Security level of this encryption
    pub security_level: SecurityLevel,
}

/// Encrypted execution output
///
/// **Design**: Symmetric with input, same metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedOutput {
    /// The encrypted result payload
    pub payload: EncryptedPayload,

    /// Key identifier used for encryption
    pub key_id: String,

    /// Encryption metadata
    pub metadata: EncryptionMetadata,

    /// Security level of this encryption
    pub security_level: SecurityLevel,
}
