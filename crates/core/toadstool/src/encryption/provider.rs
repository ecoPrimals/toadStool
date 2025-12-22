//! Crypto provider interface and registry
//!
//! **Design Philosophy**:
//! - Trait-based: Any primal can provide crypto services
//! - Discovery: Runtime registration and lookup
//! - No hardcoding: Providers announce capabilities, consumers discover

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{ToadStoolError, ToadStoolResult};

use super::{CryptoCapability, EncryptedPayload, EncryptionKey, EncryptionMetadata, SecurityLevel};

/// Trait for crypto service providers
///
/// **Design**: Any primal implementing this can provide crypto
/// (BearDog, external HSM, cloud KMS, etc.)
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    /// Get provider identifier (discovered at runtime)
    fn provider_id(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> &CryptoCapability;

    /// Encrypt data
    async fn encrypt(
        &self,
        data: &[u8],
        key: &EncryptionKey,
    ) -> ToadStoolResult<(EncryptedPayload, EncryptionMetadata)>;

    /// Decrypt data
    async fn decrypt(
        &self,
        encrypted: &EncryptedPayload,
        key: &EncryptionKey,
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<Vec<u8>>;

    /// Generate new encryption key
    async fn generate_key(&self, security_level: SecurityLevel) -> ToadStoolResult<EncryptionKey>;

    /// Get existing key by ID
    async fn get_key(&self, key_id: &str) -> ToadStoolResult<EncryptionKey>;

    /// Check if provider is healthy and reachable
    async fn health_check(&self) -> ToadStoolResult<ProviderHealth>;
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Is provider reachable?
    pub available: bool,

    /// Response latency (milliseconds)
    pub latency_ms: u64,

    /// Current load (0.0 - 1.0)
    pub load: f32,

    /// Error message if unavailable
    pub error: Option<String>,
}

impl ProviderHealth {
    pub const fn healthy(latency_ms: u64) -> Self {
        Self {
            available: true,
            latency_ms,
            load: 0.0,
            error: None,
        }
    }

    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            available: false,
            latency_ms: 0,
            load: 0.0,
            error: Some(error.into()),
        }
    }
}

/// Registry of available crypto providers
///
/// **Design**: Central discovery point, no hardcoded providers
pub struct CryptoProviderRegistry {
    providers: Arc<RwLock<HashMap<String, Arc<dyn CryptoProvider>>>>,
}

impl CryptoProviderRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a crypto provider
    ///
    /// **Design**: Providers announce themselves, no pre-configuration
    pub async fn register(&self, provider: Arc<dyn CryptoProvider>) -> ToadStoolResult<()> {
        let provider_id = provider.provider_id().to_string();
        let mut providers = self.providers.write().await;

        // Check if already registered
        if providers.contains_key(&provider_id) {
            return Err(ToadStoolError::configuration(format!(
                "Provider {} already registered",
                provider_id
            )));
        }

        providers.insert(provider_id, provider);
        Ok(())
    }

    /// Unregister a provider
    pub async fn unregister(&self, provider_id: &str) -> ToadStoolResult<()> {
        let mut providers = self.providers.write().await;
        providers.remove(provider_id).ok_or_else(|| {
            ToadStoolError::not_found(format!("Provider {} not found", provider_id))
        })?;
        Ok(())
    }

    /// Find provider matching capability
    ///
    /// **Design**: Capability-based lookup, returns best match
    pub async fn find_provider(
        &self,
        capability: &CryptoCapability,
    ) -> ToadStoolResult<Option<Arc<dyn CryptoProvider>>> {
        let providers = self.providers.read().await;

        // Find all matching providers
        let mut matches: Vec<(u32, Arc<dyn CryptoProvider>)> = providers
            .values()
            .filter(|p| p.capabilities().matches(capability))
            .map(|p| {
                let score = p.capabilities().match_score(capability);
                (score, Arc::clone(p))
            })
            .collect();

        if matches.is_empty() {
            return Ok(None);
        }

        // Sort by score (best first)
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        // Return best match
        Ok(Some(matches[0].1.clone()))
    }

    /// Find all providers matching capability
    pub async fn find_all_providers(
        &self,
        capability: &CryptoCapability,
    ) -> ToadStoolResult<Vec<Arc<dyn CryptoProvider>>> {
        let providers = self.providers.read().await;

        let matches: Vec<Arc<dyn CryptoProvider>> = providers
            .values()
            .filter(|p| p.capabilities().matches(capability))
            .map(Arc::clone)
            .collect();

        Ok(matches)
    }

    /// Get provider by ID
    pub async fn get_provider(
        &self,
        provider_id: &str,
    ) -> ToadStoolResult<Arc<dyn CryptoProvider>> {
        let providers = self.providers.read().await;
        providers
            .get(provider_id)
            .map(Arc::clone)
            .ok_or_else(|| ToadStoolError::not_found(format!("Provider {} not found", provider_id)))
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> HashMap<String, ProviderHealth> {
        let providers = self.providers.read().await;
        let mut health_map = HashMap::new();

        for (id, provider) in providers.iter() {
            let health = provider
                .health_check()
                .await
                .unwrap_or_else(|e| ProviderHealth::unhealthy(e.to_string()));
            health_map.insert(id.clone(), health);
        }

        health_map
    }
}

impl Default for CryptoProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock provider for testing
    struct MockProvider {
        id: String,
        capability: CryptoCapability,
    }

    #[async_trait]
    impl CryptoProvider for MockProvider {
        fn provider_id(&self) -> &str {
            &self.id
        }

        fn capabilities(&self) -> &CryptoCapability {
            &self.capability
        }

        async fn encrypt(
            &self,
            _data: &[u8],
            _key: &EncryptionKey,
        ) -> ToadStoolResult<(EncryptedPayload, EncryptionMetadata)> {
            Ok((EncryptedPayload::default(), EncryptionMetadata::default()))
        }

        async fn decrypt(
            &self,
            _encrypted: &EncryptedPayload,
            _key: &EncryptionKey,
            _metadata: &EncryptionMetadata,
        ) -> ToadStoolResult<Vec<u8>> {
            Ok(vec![])
        }

        async fn generate_key(
            &self,
            _security_level: SecurityLevel,
        ) -> ToadStoolResult<EncryptionKey> {
            Ok(EncryptionKey::default())
        }

        async fn get_key(&self, _key_id: &str) -> ToadStoolResult<EncryptionKey> {
            Ok(EncryptionKey::default())
        }

        async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
            Ok(ProviderHealth::healthy(10))
        }
    }

    #[tokio::test]
    async fn test_registry_register() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });

        assert!(registry.register(provider).await.is_ok());

        let providers = registry.list_providers().await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_registry_find_provider() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Enhanced,
                hardware_backed: false,
            },
        });

        registry.register(provider).await.unwrap();

        let required = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        };

        let found = registry.find_provider(&required).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().provider_id(), "test");
    }
}
