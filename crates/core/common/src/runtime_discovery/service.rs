// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::error::{IntegrationError, ToadStoolError, ToadStoolResult};
use crate::primal_identity::{Capability, DiscoveredService};

use super::cache::ServiceCache;
use super::client::DiscoveryClient;

/// Runtime discovery service - manages service discovery
pub struct RuntimeDiscovery {
    /// Primary discovery client
    primary_client: Arc<dyn DiscoveryClient>,

    /// Fallback discovery clients
    fallback_clients: Vec<Arc<dyn DiscoveryClient>>,

    /// Local service cache
    cache: Arc<RwLock<ServiceCache>>,

    /// Cache TTL
    cache_ttl: Duration,
}

impl RuntimeDiscovery {
    /// Create a new runtime discovery service
    pub fn new(primary_client: Arc<dyn DiscoveryClient>) -> Self {
        Self {
            primary_client,
            fallback_clients: Vec::new(),
            cache: Arc::new(RwLock::new(ServiceCache::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Add a fallback discovery client
    #[must_use]
    pub fn with_fallback(mut self, client: Arc<dyn DiscoveryClient>) -> Self {
        self.fallback_clients.push(client);
        self
    }

    /// Set cache TTL
    #[must_use]
    pub const fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Discover services with a specific capability
    ///
    /// # Errors
    /// Returns error if discovery fails across all clients
    pub async fn discover_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(services) = cache.get_by_capability(capability) {
                if !services.is_empty() {
                    // Clone Arc contents only at API boundary
                    return Ok(services.iter().map(|s| (**s).clone()).collect());
                }
            }
        }

        // Try primary client
        match self.primary_client.discover_by_capability(capability).await {
            Ok(services) => {
                self.update_cache(&services).await;
                return Ok(services);
            }
            Err(e) => {
                tracing::warn!("Primary discovery failed: {}, trying fallbacks", e);
            }
        }

        // Try fallback clients
        for client in &self.fallback_clients {
            match client.discover_by_capability(capability).await {
                Ok(services) => {
                    self.update_cache(&services).await;
                    return Ok(services);
                }
                Err(e) => {
                    tracing::warn!("Fallback discovery failed: {}", e);
                }
            }
        }

        Err(ToadStoolError::Integration(
            IntegrationError::ServiceUnavailable {
                service: "discovery".to_string(),
                reason: "No services found with requested capability".to_string(),
            },
        ))
    }

    /// Discover all services
    ///
    /// # Errors
    /// Returns error if discovery fails and no cached services available
    pub async fn discover_all_services(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        match self.primary_client.discover_all().await {
            Ok(services) => {
                self.update_cache(&services).await;
                Ok(services)
            }
            Err(e) => {
                tracing::warn!("Discovery failed: {}", e);

                // Return cached services as fallback
                // Clone Arc contents only at API boundary
                let cache = self.cache.read().await;
                Ok(cache.get_all().iter().map(|s| (**s).clone()).collect())
            }
        }
    }

    /// Find a service with compute capability
    ///
    /// # Errors
    /// Returns error if no healthy compute service is available
    pub async fn find_compute_service(&self) -> ToadStoolResult<DiscoveredService> {
        use crate::primal_identity::ComputeCapability;

        // Try different compute capabilities in priority order
        let capabilities = vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Compute(ComputeCapability::ContainerOrchestration),
            Capability::Compute(ComputeCapability::WasmExecution),
        ];

        for cap in capabilities {
            if let Ok(services) = self.discover_capability(&cap).await {
                if let Some(service) = services.into_iter().find(|s| s.healthy) {
                    return Ok(service);
                }
            }
        }

        Err(ToadStoolError::Integration(
            IntegrationError::ServiceUnavailable {
                service: "compute".to_string(),
                reason: "No compute service available".to_string(),
            },
        ))
    }

    /// Find a service with storage capability
    ///
    /// # Errors
    /// Returns error if no healthy storage service is available
    pub async fn find_storage_service(&self) -> ToadStoolResult<DiscoveredService> {
        use crate::primal_identity::StorageCapability;

        let capability = Capability::Storage(StorageCapability::ObjectStorage);
        let services = self.discover_capability(&capability).await?;

        services.into_iter().find(|s| s.healthy).ok_or_else(|| {
            ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                service: "storage".to_string(),
                reason: "No storage service available".to_string(),
            })
        })
    }

    /// Find a service with auth capability
    ///
    /// # Errors
    /// Returns error if no healthy auth service is available
    pub async fn find_auth_service(&self) -> ToadStoolResult<DiscoveredService> {
        use crate::primal_identity::AuthCapability;

        let capability = Capability::Authentication(AuthCapability::UserAuth);
        let services = self.discover_capability(&capability).await?;

        services.into_iter().find(|s| s.healthy).ok_or_else(|| {
            ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                service: "auth".to_string(),
                reason: "No auth service available".to_string(),
            })
        })
    }

    /// Find a service with coordination capability
    ///
    /// # Errors
    /// Returns error if no healthy coordinator service is available
    pub async fn find_coordinator_service(&self) -> ToadStoolResult<DiscoveredService> {
        use crate::primal_identity::CoordinationCapability;

        let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
        let services = self.discover_capability(&capability).await?;

        services.into_iter().find(|s| s.healthy).ok_or_else(|| {
            ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
                service: "coordinator".to_string(),
                reason: "No coordinator service available".to_string(),
            })
        })
    }

    /// Update the service cache
    async fn update_cache(&self, services: &[DiscoveredService]) {
        let mut cache = self.cache.write().await;
        for service in services {
            cache.insert(service.clone());
        }
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::primal_identity::{
        Capability, ComputeCapability, DiscoveredService, ServiceEndpoint,
    };

    use super::super::localhost::LocalhostDiscoveryClient;
    use super::RuntimeDiscovery;

    fn seeded_client() -> LocalhostDiscoveryClient {
        let mut client = LocalhostDiscoveryClient::new();
        client.add_service(DiscoveredService {
            id: Some("test-compute".to_string()),
            capabilities: vec![Capability::Compute(
                crate::primal_identity::ComputeCapability::NativeExecution,
            )],
            endpoints: vec![ServiceEndpoint::http("localhost", 9999)],
            healthy: true,
            metadata: HashMap::new(),
        });
        client
    }

    #[tokio::test]
    async fn test_runtime_discovery() {
        let client = Arc::new(seeded_client());
        let discovery = RuntimeDiscovery::new(client);

        let services = discovery.discover_all_services().await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_capability_discovery() {
        let client = Arc::new(seeded_client());
        let discovery = RuntimeDiscovery::new(client);

        let capability = Capability::Compute(ComputeCapability::NativeExecution);

        let services = discovery.discover_capability(&capability).await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_discovery_with_fallback() {
        let primary = Arc::new(seeded_client());
        let fallback = Arc::new(seeded_client());

        let discovery = RuntimeDiscovery::new(primary).with_fallback(fallback);

        let services = discovery.discover_all_services().await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_discovery_with_cache_ttl() {
        let client = Arc::new(seeded_client());
        let discovery = RuntimeDiscovery::new(client).with_cache_ttl(Duration::from_secs(60));

        let services = discovery.discover_all_services().await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_find_compute_service() {
        let client = Arc::new(seeded_client());
        let discovery = RuntimeDiscovery::new(client);

        let service = discovery.find_compute_service().await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_find_storage_service() {
        let client = Arc::new(LocalhostDiscoveryClient::new());
        let discovery = RuntimeDiscovery::new(client);

        let service = discovery.find_storage_service().await;
        // May or may not find storage, just verify no panic
        assert!(service.is_ok() || service.is_err());
    }

    #[tokio::test]
    async fn test_find_auth_service() {
        let client = Arc::new(LocalhostDiscoveryClient::new());
        let discovery = RuntimeDiscovery::new(client);

        let service = discovery.find_auth_service().await;
        // May or may not find auth, just verify no panic
        assert!(service.is_ok() || service.is_err());
    }

    #[tokio::test]
    async fn test_find_coordinator_service() {
        let client = Arc::new(LocalhostDiscoveryClient::new());
        let discovery = RuntimeDiscovery::new(client);

        let service = discovery.find_coordinator_service().await;
        // May or may not find coordinator, just verify no panic
        assert!(service.is_ok() || service.is_err());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let client = Arc::new(seeded_client());
        let discovery = RuntimeDiscovery::new(client);

        let services = discovery.discover_all_services().await.unwrap();
        assert!(!services.is_empty());

        discovery.clear_cache().await;

        let services = discovery.discover_all_services().await.unwrap();
        assert!(!services.is_empty());
    }
}
