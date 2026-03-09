// SPDX-License-Identifier: AGPL-3.0-only
//! Capability registry - tracks which services provide which capabilities

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::taxonomy::CapabilityId;
use toadstool_common::infant_discovery::{DiscoveredService, ServiceHealth, ServiceMetadata};

/// Service provider information
#[derive(Debug, Clone)]
pub struct ServiceProvider {
    /// Endpoint URL or address
    pub endpoint: String,

    /// Supported protocols
    pub protocols: Vec<String>,

    /// Service health status
    pub health: ServiceHealth,

    /// Additional metadata
    pub metadata: ServiceMetadata,

    /// When this provider was last seen/updated
    pub last_seen: Instant,

    /// Priority (0-100, higher = preferred)
    pub priority: u8,
}

impl From<DiscoveredService> for ServiceProvider {
    fn from(discovered: DiscoveredService) -> Self {
        Self {
            endpoint: discovered.endpoint,
            protocols: discovered.protocols,
            health: discovered.metadata.health,
            metadata: discovered.metadata,
            last_seen: Instant::now(),
            priority: 50, // Default priority
        }
    }
}

/// Capability registry - maintains mapping of capabilities to providers
pub struct CapabilityRegistry {
    /// Map of capability ID to list of providers
    providers: Arc<RwLock<HashMap<CapabilityId, Vec<ServiceProvider>>>>,

    /// TTL for provider entries
    provider_ttl: Duration,

    /// Enable automatic cleanup of stale providers
    auto_cleanup: bool,
}

impl CapabilityRegistry {
    /// Create a new capability registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            provider_ttl: Duration::from_secs(300), // 5 minutes
            auto_cleanup: true,
        }
    }

    /// Configure provider TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.provider_ttl = ttl;
        self
    }

    /// Configure automatic cleanup
    pub fn with_auto_cleanup(mut self, enabled: bool) -> Self {
        self.auto_cleanup = enabled;
        self
    }

    /// Register a service provider for a capability
    pub async fn register(&self, capability: impl Into<CapabilityId>, provider: ServiceProvider) {
        let capability = capability.into();
        let mut providers = self.providers.write().await;

        let provider_list = providers.entry(capability).or_insert_with(Vec::new);

        // Check if provider already exists (update instead of duplicate)
        if let Some(existing) = provider_list
            .iter_mut()
            .find(|p| p.endpoint == provider.endpoint)
        {
            *existing = provider;
        } else {
            provider_list.push(provider);
        }

        // Sort by priority (highest first)
        provider_list.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Get providers for a capability
    pub async fn get_providers(&self, capability: &CapabilityId) -> Vec<ServiceProvider> {
        if self.auto_cleanup {
            self.cleanup_stale_providers().await;
        }

        let providers = self.providers.read().await;
        providers.get(capability).cloned().unwrap_or_default()
    }

    /// Get the best provider for a capability (highest priority, healthy)
    pub async fn get_best_provider(&self, capability: &CapabilityId) -> Option<ServiceProvider> {
        let providers = self.get_providers(capability).await;

        // Find first healthy provider (already sorted by priority)
        let healthy = providers
            .iter()
            .find(|p| matches!(p.health, ServiceHealth::Healthy))
            .cloned();

        healthy.or_else(|| {
            // If no healthy, try unknown
            providers
                .into_iter()
                .find(|p| matches!(p.health, ServiceHealth::Unknown))
        })
    }

    /// Get all registered capabilities
    pub async fn list_capabilities(&self) -> Vec<CapabilityId> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Remove a provider
    pub async fn unregister(&self, capability: &CapabilityId, endpoint: &str) {
        let mut providers = self.providers.write().await;

        if let Some(provider_list) = providers.get_mut(capability) {
            provider_list.retain(|p| p.endpoint != endpoint);

            // Remove capability if no providers left
            if provider_list.is_empty() {
                providers.remove(capability);
            }
        }
    }

    /// Clean up stale providers (older than TTL)
    pub async fn cleanup_stale_providers(&self) {
        let mut providers = self.providers.write().await;
        let now = Instant::now();

        providers.retain(|_, provider_list| {
            provider_list.retain(|p| now.duration_since(p.last_seen) < self.provider_ttl);
            !provider_list.is_empty()
        });
    }

    /// Clear all providers
    pub async fn clear(&self) {
        let mut providers = self.providers.write().await;
        providers.clear();
    }

    /// Get statistics
    pub async fn stats(&self) -> RegistryStats {
        let providers = self.providers.read().await;

        let total_capabilities = providers.len();
        let total_providers: usize = providers.values().map(|v| v.len()).sum();

        let healthy_providers = providers
            .values()
            .flatten()
            .filter(|p| matches!(p.health, ServiceHealth::Healthy))
            .count();

        RegistryStats {
            total_capabilities,
            total_providers,
            healthy_providers,
        }
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_capabilities: usize,
    pub total_providers: usize,
    pub healthy_providers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::capabilities::taxonomy::StandardCapability;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_register_and_get_providers() {
        let registry = CapabilityRegistry::new();

        let capability = StandardCapability::CryptoSignatureEd25519.id();
        let provider = ServiceProvider {
            endpoint: toadstool_common::constants::http_url(
                toadstool_common::constants::DEFAULT_HOSTNAME,
                toadstool_common::constants::DEFAULT_WS_PORT,
            ),
            protocols: vec!["http".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: Instant::now(),
            priority: 80,
        };

        registry.register(capability.clone(), provider).await;

        let providers = registry.get_providers(&capability).await;
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].endpoint, "http://localhost:8081");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_best_provider_selection() {
        let registry = CapabilityRegistry::new();

        let capability = StandardCapability::StorageObjectS3.id();

        // Register low priority provider
        let provider1 = ServiceProvider {
            endpoint: "http://slow-storage:8082".to_string(),
            protocols: vec!["http".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: Instant::now(),
            priority: 30,
        };

        // Register high priority provider
        let provider2 = ServiceProvider {
            endpoint: "http://fast-storage:8083".to_string(),
            protocols: vec!["http".to_string()],
            health: ServiceHealth::Healthy,
            metadata: ServiceMetadata::default(),
            last_seen: Instant::now(),
            priority: 90,
        };

        registry.register(capability.clone(), provider1).await;
        registry.register(capability.clone(), provider2).await;

        let best = registry.get_best_provider(&capability).await.unwrap();
        assert_eq!(best.endpoint, "http://fast-storage:8083");
        assert_eq!(best.priority, 90);
    }
}
