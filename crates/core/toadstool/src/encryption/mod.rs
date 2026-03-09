// SPDX-License-Identifier: AGPL-3.0-only
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
        let key = if let Some(k) = &self.active_key {
            k.clone()
        } else {
            let new_key = provider
                .generate_key(self.config.min_security_level)
                .await?;
            self.active_key = Some(new_key.clone());
            new_key
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

    #[test]
    fn test_builder_key_id() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .key_id("my-key-123")
            .build();
        // Key ID is stored in config; context doesn't expose it directly, but build succeeds
        assert!(!ctx.is_available());
    }

    #[test]
    fn test_builder_algorithms() {
        let algorithms = vec!["aes-256-gcm".to_string(), "xsalsa20".to_string()];
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .algorithms(algorithms)
            .build();
        assert!(!ctx.is_available());
    }

    #[test]
    fn test_builder_all_options() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .required(true)
            .encrypt_results(true)
            .security_level(SecurityLevel::HardwareSecured)
            .key_id("full-config-key")
            .algorithms(vec!["aes-256-gcm".to_string()])
            .build();
        assert!(ctx.is_required());
        assert!(!ctx.is_available());
    }

    #[test]
    fn test_encryption_context_new() {
        let config = EncryptionConfig {
            required: true,
            preferred_algorithms: vec!["test-alg".to_string()],
            key_id: Some("new-key".to_string()),
            encrypt_results: true,
            min_security_level: SecurityLevel::Enhanced,
        };
        let ctx = EncryptionContext::new(Uuid::new_v4(), config);
        assert!(ctx.is_required());
        assert!(!ctx.is_available());
    }

    #[test]
    fn test_context_not_available_without_provider() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4()).build();
        assert!(!ctx.is_available());
    }

    #[test]
    fn test_context_required_reflects_config() {
        let ctx_required = EncryptionContextBuilder::new(Uuid::new_v4())
            .required(true)
            .build();
        let ctx_optional = EncryptionContextBuilder::new(Uuid::new_v4())
            .required(false)
            .build();
        assert!(ctx_required.is_required());
        assert!(!ctx_optional.is_required());
    }

    #[test]
    fn test_security_level_equality() {
        assert_eq!(SecurityLevel::Standard, SecurityLevel::Standard);
        assert_eq!(SecurityLevel::Enhanced, SecurityLevel::Enhanced);
        assert_eq!(
            SecurityLevel::HardwareSecured,
            SecurityLevel::HardwareSecured
        );
    }

    #[test]
    fn test_security_level_all_orderings() {
        use std::cmp::Ordering;
        assert_eq!(
            SecurityLevel::Standard.cmp(&SecurityLevel::Enhanced),
            Ordering::Less
        );
        assert_eq!(
            SecurityLevel::Standard.cmp(&SecurityLevel::HardwareSecured),
            Ordering::Less
        );
        assert_eq!(
            SecurityLevel::Enhanced.cmp(&SecurityLevel::HardwareSecured),
            Ordering::Less
        );
        assert_eq!(
            SecurityLevel::Enhanced.cmp(&SecurityLevel::Standard),
            Ordering::Greater
        );
        assert_eq!(
            SecurityLevel::HardwareSecured.cmp(&SecurityLevel::Standard),
            Ordering::Greater
        );
        assert_eq!(
            SecurityLevel::HardwareSecured.cmp(&SecurityLevel::Enhanced),
            Ordering::Greater
        );
    }

    #[test]
    fn test_default_config_algorithms() {
        let config = EncryptionConfig::default();
        assert_eq!(
            config.preferred_algorithms,
            vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()]
        );
    }

    #[test]
    fn test_default_config_key_id_is_none() {
        let config = EncryptionConfig::default();
        assert!(config.key_id.is_none());
    }

    #[test]
    fn test_encryption_context_debug() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4()).build();
        let _ = format!("{ctx:?}");
    }

    #[test]
    fn test_encrypted_input_serialization() {
        let input = EncryptedInput {
            payload: EncryptedPayload::new(vec![1, 2, 3, 4, 5]),
            key_id: "test-key".to_string(),
            metadata: EncryptionMetadata {
                algorithm: "chacha20poly1305".to_string(),
                nonce: vec![10, 20, 30],
                aad: None,
                kdf_info: None,
                encrypted_at: 1234567890,
            },
            security_level: SecurityLevel::Standard,
        };
        let json = serde_json::to_string(&input).unwrap();
        let deserialized: EncryptedInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input.payload.ciphertext, deserialized.payload.ciphertext);
        assert_eq!(input.key_id, deserialized.key_id);
        assert_eq!(input.metadata.algorithm, deserialized.metadata.algorithm);
        assert_eq!(input.security_level, deserialized.security_level);
    }

    #[test]
    fn test_encrypted_output_serialization() {
        let output = EncryptedOutput {
            payload: EncryptedPayload::new(vec![6, 7, 8, 9, 10]),
            key_id: "output-key".to_string(),
            metadata: EncryptionMetadata {
                algorithm: "aes-256-gcm".to_string(),
                nonce: vec![1, 2, 3],
                aad: Some(vec![4, 5, 6]),
                kdf_info: None,
                encrypted_at: 9876543210,
            },
            security_level: SecurityLevel::Enhanced,
        };
        let json = serde_json::to_string(&output).unwrap();
        let deserialized: EncryptedOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(output.payload.ciphertext, deserialized.payload.ciphertext);
        assert_eq!(output.key_id, deserialized.key_id);
        assert_eq!(output.metadata.algorithm, deserialized.metadata.algorithm);
        assert_eq!(output.security_level, deserialized.security_level);
    }

    #[test]
    fn test_ecosystem_config_serialization() {
        let config = EncryptionConfig {
            required: true,
            preferred_algorithms: vec!["aes-256-gcm".to_string()],
            key_id: Some("serial-key".to_string()),
            encrypt_results: true,
            min_security_level: SecurityLevel::HardwareSecured,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EncryptionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.required, deserialized.required);
        assert_eq!(
            config.preferred_algorithms,
            deserialized.preferred_algorithms
        );
        assert_eq!(config.key_id, deserialized.key_id);
        assert_eq!(config.encrypt_results, deserialized.encrypt_results);
        assert_eq!(config.min_security_level, deserialized.min_security_level);
    }

    #[test]
    fn test_builder_default_values() {
        let ctx = EncryptionContextBuilder::new(Uuid::new_v4()).build();
        assert!(!ctx.is_required());
        assert!(!ctx.is_available());
    }

    #[tokio::test]
    async fn test_discover_provider_empty_registry_sets_none() {
        use super::provider::CryptoProviderRegistry;

        let mut ctx = EncryptionContext::new(Uuid::new_v4(), EncryptionConfig::default());
        let registry = CryptoProviderRegistry::new();

        let result = ctx.discover_provider(&registry).await;
        assert!(result.is_ok());
        assert!(!ctx.is_available());
    }

    #[tokio::test]
    async fn test_discover_provider_with_registered_provider() {
        use super::capability::CryptoCapability;
        use super::provider::{CryptoProvider, CryptoProviderRegistry, ProviderHealth};
        use async_trait::async_trait;
        use std::sync::Arc;

        struct TestProvider;
        #[async_trait]
        impl CryptoProvider for TestProvider {
            fn provider_id(&self) -> &'static str {
                "test-crypto"
            }
            fn capabilities(&self) -> &CryptoCapability {
                static CAP: std::sync::OnceLock<CryptoCapability> = std::sync::OnceLock::new();
                CAP.get_or_init(|| CryptoCapability {
                    algorithms: vec!["chacha20poly1305".to_string()],
                    security_level: SecurityLevel::Standard,
                    hardware_backed: false,
                })
            }
            async fn encrypt(
                &self,
                data: &[u8],
                _key: &super::types::EncryptionKey,
            ) -> crate::ToadStoolResult<(
                super::types::EncryptedPayload,
                super::types::EncryptionMetadata,
            )> {
                Ok((
                    super::types::EncryptedPayload::new(data.to_vec()),
                    super::types::EncryptionMetadata::default(),
                ))
            }
            async fn decrypt(
                &self,
                encrypted: &super::types::EncryptedPayload,
                _key: &super::types::EncryptionKey,
                _metadata: &super::types::EncryptionMetadata,
            ) -> crate::ToadStoolResult<Vec<u8>> {
                Ok(encrypted.ciphertext.clone())
            }
            async fn generate_key(
                &self,
                level: SecurityLevel,
            ) -> crate::ToadStoolResult<super::types::EncryptionKey> {
                Ok(super::types::EncryptionKey::new(
                    "gen-key".to_string(),
                    vec![1u8; 32],
                    "chacha20poly1305".to_string(),
                    level,
                ))
            }
            async fn get_key(
                &self,
                key_id: &str,
            ) -> crate::ToadStoolResult<super::types::EncryptionKey> {
                Ok(super::types::EncryptionKey::new(
                    key_id.to_string(),
                    vec![1u8; 32],
                    "chacha20poly1305".to_string(),
                    SecurityLevel::Standard,
                ))
            }
            async fn health_check(&self) -> crate::ToadStoolResult<ProviderHealth> {
                Ok(ProviderHealth::healthy(1))
            }
        }

        let mut ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .encrypt_results(true)
            .build();
        let registry = CryptoProviderRegistry::new();
        registry
            .register(Arc::new(TestProvider))
            .await
            .expect("register");

        let result = ctx.discover_provider(&registry).await;
        assert!(result.is_ok());
        assert!(ctx.is_available());
    }

    #[tokio::test]
    async fn test_decrypt_input_without_provider_returns_error() {
        let mut ctx = EncryptionContext::new(Uuid::new_v4(), EncryptionConfig::default());
        let encrypted = EncryptedInput {
            payload: EncryptedPayload::new(vec![1, 2, 3]),
            key_id: "key-1".to_string(),
            metadata: EncryptionMetadata::default(),
            security_level: SecurityLevel::Standard,
        };

        let result = ctx.decrypt_input(&encrypted).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No crypto provider"));
    }

    #[tokio::test]
    async fn test_decrypt_input_security_level_below_minimum_returns_error() {
        let mut ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .security_level(SecurityLevel::HardwareSecured)
            .build();
        let encrypted = EncryptedInput {
            payload: EncryptedPayload::new(vec![1, 2, 3]),
            key_id: "key-1".to_string(),
            metadata: EncryptionMetadata::default(),
            security_level: SecurityLevel::Standard,
        };

        let result = ctx.decrypt_input(&encrypted).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("security level"));
    }

    #[tokio::test]
    async fn test_encrypt_output_without_encrypt_results_returns_error() {
        let mut ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .encrypt_results(false)
            .build();

        let result = ctx.encrypt_output(b"hello").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not enabled"));
    }

    #[tokio::test]
    async fn test_encrypt_output_without_provider_returns_error() {
        let mut ctx = EncryptionContextBuilder::new(Uuid::new_v4())
            .encrypt_results(true)
            .build();

        let result = ctx.encrypt_output(b"hello").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No crypto provider"));
    }
}
