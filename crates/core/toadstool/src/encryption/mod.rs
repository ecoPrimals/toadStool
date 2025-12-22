//! Encryption Layer for ToadStool
//!
//! **Design Philosophy**:
//! - Capability-based: Discover crypto providers at runtime
//! - Self-knowledge: Toadstool knows it can execute, not who provides crypto
//! - Zero hardcoding: No URLs, ports, or specific primal names
//! - Modern Rust: Strong types, zero-copy where possible
//! - Graceful degradation: Works without encryption

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{ToadStoolError, ToadStoolResult};

pub mod capability;
pub mod provider;
pub mod types;

pub use capability::CryptoCapability;
pub use provider::{CryptoProvider, CryptoProviderRegistry};
pub use types::{EncryptedPayload, EncryptionKey, EncryptionMetadata};

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

/// Security level for encryption
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Basic encryption (software-based)
    Standard,
    /// Enhanced encryption (genetic keys, entropy mixing)
    Enhanced,
    /// Hardware security module required
    HardwareSecured,
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

/// Encryption context for an execution
///
/// **Design**: Holds encryption state for an execution lifecycle
pub struct EncryptionContext {
    /// Execution identifier
    execution_id: Uuid,

    /// Encryption configuration
    config: EncryptionConfig,

    /// Discovered crypto provider (runtime discovery)
    provider: Option<Arc<dyn CryptoProvider>>,

    /// Active encryption key
    active_key: Option<EncryptionKey>,
}

impl EncryptionContext {
    /// Create new encryption context
    ///
    /// **Design**: No provider passed in - discovered at runtime
    pub fn new(execution_id: Uuid, config: EncryptionConfig) -> Self {
        Self {
            execution_id,
            config,
            provider: None,
            active_key: None,
        }
    }

    /// Discover and set crypto provider
    ///
    /// **Capability-based**: Query registry for crypto capability
    pub async fn discover_provider(
        &mut self,
        registry: &CryptoProviderRegistry,
    ) -> ToadStoolResult<()> {
        let capability = CryptoCapability {
            algorithms: self.config.preferred_algorithms.clone(),
            security_level: self.config.min_security_level,
            hardware_backed: matches!(
                self.config.min_security_level,
                SecurityLevel::HardwareSecured
            ),
        };

        self.provider = registry.find_provider(&capability).await?;
        Ok(())
    }

    /// Decrypt input data
    ///
    /// **Design**: Transparent decryption, returns raw bytes
    pub async fn decrypt_input(&mut self, encrypted: &EncryptedInput) -> ToadStoolResult<Vec<u8>> {
        // Validate security level
        if encrypted.security_level < self.config.min_security_level {
            return Err(ToadStoolError::security(format!(
                "Encryption security level {:?} below minimum {:?}",
                encrypted.security_level, self.config.min_security_level
            )));
        }

        // Clone provider Arc to avoid borrow conflicts
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| {
                ToadStoolError::configuration("No crypto provider available for decryption")
            })?
            .clone();

        // Get or fetch key (can mutate self now)
        let key = self
            .get_or_fetch_key(&encrypted.key_id, provider.as_ref())
            .await?;

        // Decrypt using discovered provider
        provider
            .decrypt(&encrypted.payload, &key, &encrypted.metadata)
            .await
    }

    /// Encrypt output data
    ///
    /// **Design**: Symmetric encryption using same key
    pub async fn encrypt_output(&mut self, data: &[u8]) -> ToadStoolResult<EncryptedOutput> {
        if !self.config.encrypt_results {
            return Err(ToadStoolError::configuration(
                "Result encryption not enabled in config",
            ));
        }

        // Clone provider Arc to avoid borrow conflicts
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| {
                ToadStoolError::configuration("No crypto provider available for encryption")
            })?
            .clone();

        // Use active key or generate new one
        let key = match &self.active_key {
            Some(k) => k.clone(),
            None => {
                let new_key = provider
                    .generate_key(self.config.min_security_level)
                    .await?;
                self.active_key = Some(new_key.clone());
                new_key
            }
        };

        // Encrypt using discovered provider
        let (payload, metadata) = provider.encrypt(data, &key).await?;

        Ok(EncryptedOutput {
            payload,
            key_id: key.id.clone(),
            metadata,
            security_level: key.security_level,
        })
    }

    /// Get or fetch encryption key
    ///
    /// **Design**: Cache keys locally, fetch from provider if needed
    async fn get_or_fetch_key(
        &mut self,
        key_id: &str,
        provider: &dyn CryptoProvider,
    ) -> ToadStoolResult<EncryptionKey> {
        // Check if we already have this key
        if let Some(ref key) = self.active_key {
            if key.id == key_id {
                return Ok(key.clone());
            }
        }

        // Fetch from provider (moved out of borrow scope)
        let key = provider.get_key(key_id).await?;
        // Now we can mutate self
        self.active_key = Some(key.clone());
        Ok(key)
    }

    /// Check if encryption is available
    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// Check if encryption is required
    pub const fn is_required(&self) -> bool {
        self.config.required
    }
}

impl std::fmt::Debug for EncryptionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionContext")
            .field("execution_id", &self.execution_id)
            .field("config", &self.config)
            .field("provider", &self.provider.as_ref().map(|p| p.provider_id()))
            .field("active_key", &self.active_key.as_ref().map(|k| &k.id))
            .finish()
    }
}

/// Builder for encryption contexts
///
/// **Design**: Fluent API, modern Rust idioms
pub struct EncryptionContextBuilder {
    execution_id: Uuid,
    config: EncryptionConfig,
}

impl EncryptionContextBuilder {
    pub fn new(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            config: EncryptionConfig::default(),
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.config.required = required;
        self
    }

    pub fn encrypt_results(mut self, encrypt: bool) -> Self {
        self.config.encrypt_results = encrypt;
        self
    }

    pub fn security_level(mut self, level: SecurityLevel) -> Self {
        self.config.min_security_level = level;
        self
    }

    pub fn key_id(mut self, key_id: impl Into<String>) -> Self {
        self.config.key_id = Some(key_id.into());
        self
    }

    pub fn algorithms(mut self, algorithms: Vec<String>) -> Self {
        self.config.preferred_algorithms = algorithms;
        self
    }

    pub fn build(self) -> EncryptionContext {
        EncryptionContext::new(self.execution_id, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_context_builder() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .required(true)
            .encrypt_results(true)
            .security_level(SecurityLevel::Enhanced)
            .build();

        assert!(ctx.is_required());
        assert!(!ctx.is_available()); // No provider discovered yet
    }

    #[test]
    fn test_default_config() {
        let config = EncryptionConfig::default();
        assert!(!config.required);
        assert!(!config.encrypt_results);
        assert_eq!(config.min_security_level, SecurityLevel::Standard);
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Standard < SecurityLevel::Enhanced);
        assert!(SecurityLevel::Enhanced < SecurityLevel::HardwareSecured);
    }
}
