//! Core encryption types
//!
//! **Design Philosophy**:
//! - Zero-copy where possible
//! - Modern Rust: Strong types, no stringly-typed
//! - Security: Explicit zeroization of sensitive data

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::SecurityLevel;

/// Encrypted payload container
///
/// **Design**: Binary data, no assumptions about format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EncryptedPayload {
    /// The encrypted data
    #[serde(default)]
    pub ciphertext: Vec<u8>,

    /// Optional authentication tag (for AEAD ciphers)
    #[serde(default)]
    pub auth_tag: Option<Vec<u8>>,
}

impl EncryptedPayload {
    /// Create new encrypted payload
    pub fn new(ciphertext: Vec<u8>) -> Self {
        Self {
            ciphertext,
            auth_tag: None,
        }
    }

    /// Create with authentication tag
    pub fn with_auth_tag(mut self, tag: Vec<u8>) -> Self {
        self.auth_tag = Some(tag);
        self
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.ciphertext.len() + self.auth_tag.as_ref().map_or(0, |t| t.len())
    }
}

/// Encryption metadata
///
/// **Design**: Algorithm-agnostic metadata, extensible
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptionMetadata {
    /// Algorithm used for encryption
    pub algorithm: String,

    /// Initialization vector / nonce
    pub nonce: Vec<u8>,

    /// Additional authenticated data (if any)
    pub aad: Option<Vec<u8>>,

    /// Key derivation info (if applicable)
    pub kdf_info: Option<KeyDerivationInfo>,

    /// Timestamp when encrypted
    pub encrypted_at: i64,
}

impl Default for EncryptionMetadata {
    fn default() -> Self {
        Self {
            algorithm: "chacha20poly1305".to_string(),
            nonce: Vec::new(),
            aad: None,
            kdf_info: None,
            encrypted_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Key derivation information
///
/// **Design**: Captures KDF parameters for key reproduction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyDerivationInfo {
    /// KDF algorithm (e.g., "HKDF-SHA256", "Argon2id")
    pub algorithm: String,

    /// Salt used for derivation
    pub salt: Vec<u8>,

    /// Iteration count / work factor
    pub iterations: Option<u32>,

    /// Memory cost (for memory-hard KDFs)
    pub memory_kb: Option<u32>,

    /// Parallelism factor
    pub parallelism: Option<u8>,
}

/// Encryption key
///
/// **Design**: Secure by default, zeroizes on drop
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    /// Key identifier
    #[zeroize(skip)]
    pub id: String,

    /// The actual key material (zeroized on drop)
    pub key_material: Vec<u8>,

    /// Security level of this key
    #[zeroize(skip)]
    pub security_level: SecurityLevel,

    /// Algorithm this key is for
    #[zeroize(skip)]
    pub algorithm: String,

    /// When this key was created
    #[zeroize(skip)]
    pub created_at: i64,

    /// Optional expiration timestamp
    #[zeroize(skip)]
    pub expires_at: Option<i64>,
}

impl Default for EncryptionKey {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key_material: Vec::new(),
            security_level: SecurityLevel::Standard,
            algorithm: "chacha20poly1305".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        }
    }
}

impl EncryptionKey {
    /// Create new key
    pub fn new(
        id: String,
        key_material: Vec<u8>,
        algorithm: String,
        security_level: SecurityLevel,
    ) -> Self {
        Self {
            id,
            key_material,
            security_level,
            algorithm,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        }
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }

    /// Check if key is valid
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.key_material.is_empty()
    }

    /// Get key size in bytes
    pub fn size(&self) -> usize {
        self.key_material.len()
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptionKey")
            .field("id", &self.id)
            .field("key_material", &"[REDACTED]")
            .field("security_level", &self.security_level)
            .field("algorithm", &self.algorithm)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Key rotation policy
///
/// **Design**: Automate key rotation for enhanced security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    /// Rotate keys after this many uses
    pub max_uses: Option<u64>,

    /// Rotate keys after this duration (seconds)
    pub max_age_seconds: Option<u64>,

    /// Rotate keys after encrypting this much data (bytes)
    pub max_data_bytes: Option<u64>,

    /// Automatically retire old keys
    pub auto_retire: bool,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            max_uses: Some(100_000),
            max_age_seconds: Some(86400 * 30), // 30 days
            max_data_bytes: Some(1024 * 1024 * 1024 * 100), // 100 GB
            auto_retire: true,
        }
    }
}

impl KeyRotationPolicy {
    /// Check if key should be rotated
    pub fn should_rotate(&self, uses: u64, age_seconds: u64, data_bytes: u64) -> bool {
        if let Some(max_uses) = self.max_uses {
            if uses >= max_uses {
                return true;
            }
        }

        if let Some(max_age) = self.max_age_seconds {
            if age_seconds >= max_age {
                return true;
            }
        }

        if let Some(max_data) = self.max_data_bytes {
            if data_bytes >= max_data {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_payload_size() {
        let payload = EncryptedPayload {
            ciphertext: vec![0u8; 100],
            auth_tag: Some(vec![0u8; 16]),
        };

        assert_eq!(payload.size(), 116);
    }

    #[test]
    fn test_key_expiration() {
        let mut key = EncryptionKey::new(
            "test-key".to_string(),
            vec![1, 2, 3, 4, 5],
            "aes-256-gcm".to_string(),
            SecurityLevel::Standard,
        );
        assert!(!key.is_expired());
        assert!(key.is_valid());

        // Set expiration to past
        key.expires_at = Some(chrono::Utc::now().timestamp() - 1000);
        assert!(key.is_expired());
        assert!(!key.is_valid());
    }

    #[test]
    fn test_key_rotation_policy() {
        let policy = KeyRotationPolicy::default();

        // Should rotate when exceeding max uses
        assert!(policy.should_rotate(100_001, 0, 0));

        // Should rotate when exceeding max age
        assert!(policy.should_rotate(0, 86400 * 31, 0));

        // Should not rotate when within limits
        assert!(!policy.should_rotate(1000, 86400, 1024 * 1024));
    }

    #[test]
    fn test_key_debug_redacts_material() {
        let key = EncryptionKey::new(
            "test".to_string(),
            vec![1, 2, 3, 4, 5],
            "test-alg".to_string(),
            SecurityLevel::Standard,
        );

        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("1, 2, 3, 4, 5"));
    }
}
