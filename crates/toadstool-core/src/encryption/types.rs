// SPDX-License-Identifier: AGPL-3.0-or-later
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

use super::security::SecurityLevel;

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
    pub const fn new(ciphertext: Vec<u8>) -> Self {
        Self {
            ciphertext,
            auth_tag: None,
        }
    }

    /// Create with authentication tag
    #[must_use]
    pub fn with_auth_tag(mut self, tag: Vec<u8>) -> Self {
        self.auth_tag = Some(tag);
        self
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.ciphertext.len() + self.auth_tag.as_ref().map_or(0, std::vec::Vec::len)
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

#[expect(
    clippy::cast_possible_wrap,
    reason = "Unix epoch seconds fit in i64 for realistic dates"
)]
fn unix_timestamp_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Default for EncryptionMetadata {
    fn default() -> Self {
        Self {
            algorithm: "chacha20poly1305".to_string(),
            nonce: Vec::new(),
            aad: None,
            kdf_info: None,
            encrypted_at: unix_timestamp_now(),
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
            #[cfg(feature = "runtime")]
            id: Uuid::new_v4().to_string(),
            #[cfg(not(feature = "runtime"))]
            id: Uuid::nil().to_string(),
            key_material: Vec::new(),
            security_level: SecurityLevel::Standard,
            algorithm: "chacha20poly1305".to_string(),
            created_at: unix_timestamp_now(),
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
            created_at: unix_timestamp_now(),
            expires_at: None,
        }
    }

    /// Set expiration time
    #[must_use]
    pub const fn with_expiration(mut self, expires_at: i64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| unix_timestamp_now() > expires_at)
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
    pub const fn should_rotate(&self, uses: u64, age_seconds: u64, data_bytes: u64) -> bool {
        if let Some(max_uses) = self.max_uses
            && uses >= max_uses
        {
            return true;
        }

        if let Some(max_age) = self.max_age_seconds
            && age_seconds >= max_age
        {
            return true;
        }

        if let Some(max_data) = self.max_data_bytes
            && data_bytes >= max_data
        {
            return true;
        }

        false
    }
}
