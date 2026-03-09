// SPDX-License-Identifier: AGPL-3.0-only
//! Crypto integration types - Protocol-level, vendor-agnostic
//!
//! **Design Philosophy**:
//! - Protocol-agnostic: Works with any crypto provider's API
//! - Version-agnostic: Handle API evolution gracefully
//! - Type-safe: Strong types, no stringly-typed data
//! - Zero vendor lock-in: These types work with ANY crypto service

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Crypto request (encryption, decryption, signing, etc.)
///
/// **Design**: Vendor-agnostic protocol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Operation type
    pub operation: CryptoOperation,

    /// Data to process
    pub data: Vec<u8>,

    /// Key identifier (if using existing key)
    pub key_id: Option<String>,

    /// Algorithm preference
    pub algorithm: Option<EncryptionAlgorithm>,

    /// Security level required
    pub security_level: SecurityLevel,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Crypto response from service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Result data
    pub data: Vec<u8>,

    /// Key used
    pub key_id: String,

    /// Algorithm used
    pub algorithm: String,

    /// Metadata (nonce, tag, IV, etc.)
    pub metadata: serde_json::Value,
}

/// Crypto operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CryptoOperation {
    /// Encrypt data
    Encrypt,

    /// Decrypt data
    Decrypt,

    /// Sign data
    Sign,

    /// Verify signature
    Verify,

    /// Hash data
    Hash,

    /// Generate key
    GenerateKey { key_type: KeyType },

    /// Rotate key
    RotateKey { old_key_id: String },

    /// Export key (for backup/migration)
    ExportKey { key_id: String },

    /// Import key
    ImportKey { key_data: Vec<u8> },
}

/// Encryption algorithm (extensible list)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM (recommended)
    Aes256Gcm,

    /// AES-128-GCM
    Aes128Gcm,

    /// ChaCha20-Poly1305
    ChaCha20Poly1305,

    /// RSA with OAEP
    RsaOaep { bits: u16 },

    /// Elliptic curve (ECDSA, ECDH)
    EllipticCurve { curve: String },

    /// Custom/provider-specific
    Custom(String),
}

/// Key type for generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyType {
    /// Symmetric key (AES, ChaCha)
    Symmetric { bits: u16 },

    /// Asymmetric keypair (RSA, EC)
    Asymmetric { algorithm: String, bits: u16 },

    /// Signing key
    Signing { algorithm: String },

    /// Key encryption key
    Kek,
}

/// Security level requirement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Standard security (software-based)
    Standard,

    /// High security (HSM recommended)
    High,

    /// Maximum security (HSM required, FIPS 140-2 Level 3+)
    Maximum,

    /// Quantum-resistant algorithms required
    QuantumResistant,
}

/// Key management request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Key operation
    pub operation: KeyOperation,

    /// Key metadata
    pub metadata: serde_json::Value,
}

/// Key management response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Key identifier
    pub key_id: String,

    /// Operation result
    pub success: bool,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Key operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyOperation {
    /// Generate new key
    Generate { key_type: KeyType },

    /// Rotate existing key
    Rotate { key_id: String },

    /// Delete key
    Delete { key_id: String },

    /// Export key for backup
    Export { key_id: String },

    /// Import key from backup
    Import { key_data: Vec<u8> },

    /// List keys
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_request_creation() {
        let request = CryptoRequest {
            request_id: Uuid::new_v4(),
            operation: CryptoOperation::Encrypt,
            data: vec![1, 2, 3],
            key_id: None,
            algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
            security_level: SecurityLevel::High,
            metadata: serde_json::Value::Null,
        };

        assert_eq!(request.operation, CryptoOperation::Encrypt);
        assert_eq!(request.security_level, SecurityLevel::High);
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Standard < SecurityLevel::High);
        assert!(SecurityLevel::High < SecurityLevel::Maximum);
        assert!(SecurityLevel::Maximum < SecurityLevel::QuantumResistant);
    }

    #[test]
    fn test_encryption_algorithm_custom() {
        let algo = EncryptionAlgorithm::Custom("XSalsa20".to_string());
        assert!(matches!(algo, EncryptionAlgorithm::Custom(_)));
    }
}
