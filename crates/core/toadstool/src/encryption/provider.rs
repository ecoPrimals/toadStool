// SPDX-License-Identifier: AGPL-3.0-only
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
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
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
                "Provider {provider_id} already registered"
            )));
        }

        providers.insert(provider_id, provider);
        drop(providers);
        Ok(())
    }

    /// Unregister a provider
    pub async fn unregister(&self, provider_id: &str) -> ToadStoolResult<()> {
        self.providers
            .write()
            .await
            .remove(provider_id)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Provider {provider_id} not found"))
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
        // Find all matching providers
        let mut matches: Vec<(u32, Arc<dyn CryptoProvider>)> = self
            .providers
            .read()
            .await
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
        let matches: Vec<Arc<dyn CryptoProvider>> = self
            .providers
            .read()
            .await
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
        self.providers
            .read()
            .await
            .get(provider_id)
            .map(Arc::clone)
            .ok_or_else(|| ToadStoolError::not_found(format!("Provider {provider_id} not found")))
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> HashMap<String, ProviderHealth> {
        let to_check: Vec<(String, Arc<dyn CryptoProvider>)> = self
            .providers
            .read()
            .await
            .iter()
            .map(|(id, p)| (id.clone(), Arc::clone(p)))
            .collect();

        let mut health_map = HashMap::new();
        for (id, provider) in to_check {
            let health = provider
                .health_check()
                .await
                .unwrap_or_else(|e| ProviderHealth::unhealthy(e.to_string()));
            health_map.insert(id, health);
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

    // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
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

    #[test]
    #[allow(clippy::float_cmp)] // literal just set by healthy()/unhealthy()
    fn test_provider_health_healthy() {
        let health = ProviderHealth::healthy(25);
        assert!(health.available);
        assert_eq!(health.latency_ms, 25);
        assert_eq!(health.load, 0.0);
        assert!(health.error.is_none());
    }

    #[test]
    #[allow(clippy::float_cmp)] // literal just set by unhealthy()
    fn test_provider_health_unhealthy() {
        let health = ProviderHealth::unhealthy("connection refused");
        assert!(!health.available);
        assert_eq!(health.latency_ms, 0);
        assert_eq!(health.load, 0.0);
        assert_eq!(health.error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn test_provider_health_unhealthy_from_string() {
        let msg = "timeout".to_string();
        let health = ProviderHealth::unhealthy(msg);
        assert!(!health.available);
        assert_eq!(health.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_provider_health_debug_clone() {
        let health = ProviderHealth::healthy(10);
        let cloned = health.clone();
        assert_eq!(health.available, cloned.available);
        assert_eq!(health.latency_ms, cloned.latency_ms);
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = CryptoProviderRegistry::default();
        assert!(registry.list_providers().await.is_empty());
    }

    #[tokio::test]
    async fn test_registry_register_duplicate_fails() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "dup".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["aes-256-gcm".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });

        assert!(registry.register(provider.clone()).await.is_ok());
        let result = registry.register(provider).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already registered")
        );
    }

    #[tokio::test]
    async fn test_registry_unregister_not_found() {
        let registry = CryptoProviderRegistry::new();
        let result = registry.unregister("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_registry_unregister() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "unreg".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });
        registry.register(provider).await.unwrap();
        assert_eq!(registry.list_providers().await.len(), 1);

        assert!(registry.unregister("unreg").await.is_ok());
        assert!(registry.list_providers().await.is_empty());
    }

    #[tokio::test]
    async fn test_registry_find_provider_no_match() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "test".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["aes-256-gcm".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });
        registry.register(provider).await.unwrap();

        let required = CryptoCapability {
            algorithms: vec!["nonexistent-alg".to_string()],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        };
        let found = registry.find_provider(&required).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_registry_find_all_providers() {
        let registry = CryptoProviderRegistry::new();
        let p1 = Arc::new(MockProvider {
            id: "p1".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });
        let p2 = Arc::new(MockProvider {
            id: "p2".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()],
                security_level: SecurityLevel::Enhanced,
                hardware_backed: false,
            },
        });
        registry.register(p1).await.unwrap();
        registry.register(p2).await.unwrap();

        let required = CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string()],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        };
        let found = registry.find_all_providers(&required).await.unwrap();
        assert_eq!(found.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_get_provider() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "get-me".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });
        registry.register(provider).await.unwrap();

        let got = registry.get_provider("get-me").await.unwrap();
        assert_eq!(got.provider_id(), "get-me");

        let result = registry.get_provider("missing").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_health_check_all() {
        let registry = CryptoProviderRegistry::new();
        let provider = Arc::new(MockProvider {
            id: "healthy".to_string(),
            capability: CryptoCapability {
                algorithms: vec!["chacha20poly1305".to_string()],
                security_level: SecurityLevel::Standard,
                hardware_backed: false,
            },
        });
        registry.register(provider).await.unwrap();

        let health_map = registry.health_check_all().await;
        assert_eq!(health_map.len(), 1);
        assert!(health_map.get("healthy").unwrap().available);
        assert_eq!(health_map.get("healthy").unwrap().latency_ms, 10);
    }

    #[tokio::test]
    async fn test_registry_list_providers_empty() {
        let registry = CryptoProviderRegistry::new();
        let list = registry.list_providers().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_registry_list_providers_multiple() {
        let registry = CryptoProviderRegistry::new();
        for (id, alg) in [("a", "aes"), ("b", "chacha")] {
            let provider = Arc::new(MockProvider {
                id: id.to_string(),
                capability: CryptoCapability {
                    algorithms: vec![alg.to_string()],
                    security_level: SecurityLevel::Standard,
                    hardware_backed: false,
                },
            });
            registry.register(provider).await.unwrap();
        }
        let list = registry.list_providers().await;
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"a".to_string()));
        assert!(list.contains(&"b".to_string()));
    }
}
