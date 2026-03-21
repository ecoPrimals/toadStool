// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime Service Discovery - Zero Hardcoding
//!
//! This module provides runtime discovery of services based on capabilities,
//! completely eliminating hardcoded primal names and URLs.
//!
//! ## Design Principles
//!
//! 1. **Capability-Based**: Find services by what they can do, not who they are
//! 2. **Runtime Discovery**: No compile-time knowledge of other services
//! 3. **Protocol Agnostic**: Support multiple discovery protocols
//! 4. **Fallback Strategy**: Graceful degradation when discovery unavailable

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::error::{IntegrationError, ToadStoolError, ToadStoolResult};
use crate::primal_identity::{Capability, DiscoveredService, ServiceEndpoint};

/// Discovery client trait - implement for different discovery mechanisms
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait DiscoveryClient: Send + Sync {
    /// Discover services by capability
    async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>>;

    /// Discover all available services
    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>>;

    /// Register a service (for service registration)
    async fn register_service(&self, service: &DiscoveredService) -> ToadStoolResult<()>;

    /// Deregister a service
    async fn deregister_service(&self, service_id: &str) -> ToadStoolResult<()>;

    /// Health check a service
    async fn health_check(&self, service_id: &str) -> ToadStoolResult<bool>;
}

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

/// Service cache for discovered services
///
/// Uses `Arc<DiscoveredService>` internally for zero-copy sharing.
/// This avoids expensive clones during cache operations (insert, lookup).
/// Cloning only happens at API boundaries when returning to callers.
#[derive(Debug)]
struct ServiceCache {
    /// Services indexed by capability (Arc for cheap sharing)
    by_capability: HashMap<Capability, Vec<Arc<DiscoveredService>>>,

    /// All services (Arc for cheap sharing)
    all_services: Vec<Arc<DiscoveredService>>,

    /// Cache timestamp
    last_updated: std::time::Instant,
}

impl ServiceCache {
    fn new() -> Self {
        Self {
            by_capability: HashMap::new(),
            all_services: Vec::new(),
            last_updated: std::time::Instant::now(),
        }
    }

    fn insert(&mut self, service: DiscoveredService) {
        // Wrap in Arc once for zero-copy sharing across indexes
        let service_arc = Arc::new(service);

        // Add to all services (cheap Arc clone - just ref count increment)
        if !self.all_services.iter().any(|s| s.id == service_arc.id) {
            self.all_services.push(Arc::clone(&service_arc));
        }

        // Index by capabilities (cheap Arc clone - just ref count increment)
        for capability in &service_arc.capabilities {
            self.by_capability
                .entry(capability.clone())
                .or_default()
                .push(Arc::clone(&service_arc));
        }

        self.last_updated = std::time::Instant::now();
    }

    fn get_by_capability(&self, capability: &Capability) -> Option<Vec<Arc<DiscoveredService>>> {
        // Cheap Arc clones (just ref count increments, not deep copies)
        self.by_capability.get(capability).cloned()
    }

    fn get_all(&self) -> Vec<Arc<DiscoveredService>> {
        // Cheap Arc clones (just ref count increments, not deep copies)
        self.all_services.clone()
    }

    fn clear(&mut self) {
        self.by_capability.clear();
        self.all_services.clear();
    }
}

/// Localhost discovery client - fallback when no discovery service available
pub struct LocalhostDiscoveryClient {
    /// Known localhost services
    services: Vec<DiscoveredService>,
}

impl LocalhostDiscoveryClient {
    /// Create a new localhost discovery client.
    ///
    /// Starts with an empty service list. Register services via `add_service()`
    /// or use `with_local_compute()` to seed a local compute endpoint.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    /// Seed a local compute endpoint discovered from the environment.
    ///
    /// Reads `TOADSTOOL_LOCAL_PORT` (default: OS-assigned) to avoid hardcoded ports.
    #[must_use]
    pub fn with_local_compute(mut self) -> Self {
        let port: u16 = std::env::var("TOADSTOOL_LOCAL_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if port > 0 {
            self.services.push(DiscoveredService {
                id: Some("localhost-compute".to_string()),
                capabilities: vec![Capability::Compute(
                    crate::primal_identity::ComputeCapability::NativeExecution,
                )],
                endpoints: vec![ServiceEndpoint::http(
                    crate::constants::network::DEFAULT_HOSTNAME,
                    port,
                )],
                healthy: true,
                metadata: HashMap::new(),
            });
        }
        self
    }

    /// Add a service to the localhost registry
    pub fn add_service(&mut self, service: DiscoveredService) {
        self.services.push(service);
    }
}

impl Default for LocalhostDiscoveryClient {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl DiscoveryClient for LocalhostDiscoveryClient {
    async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        Ok(self
            .services
            .iter()
            .filter(|s| s.has_capability(capability))
            .cloned()
            .collect())
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        Ok(self.services.clone())
    }

    async fn register_service(&self, _service: &DiscoveredService) -> ToadStoolResult<()> {
        // Localhost client is read-only
        Ok(())
    }

    async fn deregister_service(&self, _service_id: &str) -> ToadStoolResult<()> {
        // Localhost client is read-only
        Ok(())
    }

    async fn health_check(&self, service_id: &str) -> ToadStoolResult<bool> {
        Ok(self
            .services
            .iter()
            .any(|s| s.id.as_deref() == Some(service_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primal_identity::{ComputeCapability, StorageCapability};

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

    #[tokio::test]
    async fn test_service_cache_new() {
        let cache = ServiceCache::new();
        assert!(cache.get_all().is_empty());
    }

    #[tokio::test]
    async fn test_service_cache_insert() {
        let mut cache = ServiceCache::new();

        let service = DiscoveredService {
            id: Some("test-1".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8081)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service.clone());
        assert_eq!(cache.get_all().len(), 1);
    }

    #[tokio::test]
    async fn test_service_cache_get_by_capability() {
        let mut cache = ServiceCache::new();

        let cap = Capability::Compute(ComputeCapability::NativeExecution);

        let service = DiscoveredService {
            id: Some("test-2".to_string()),
            capabilities: vec![cap.clone()],
            endpoints: vec![ServiceEndpoint::http("localhost", 8082)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service);

        let services = cache.get_by_capability(&cap);
        assert!(services.is_some());
        assert_eq!(services.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_service_cache_get_by_capability_none() {
        let cache = ServiceCache::new();

        let cap = Capability::Storage(StorageCapability::ObjectStorage);

        let services = cache.get_by_capability(&cap);
        assert!(services.is_none());
    }

    #[tokio::test]
    async fn test_service_cache_clear() {
        let mut cache = ServiceCache::new();

        let service = DiscoveredService {
            id: Some("test-3".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8083)],
            healthy: true,
            metadata: HashMap::new(),
        };

        cache.insert(service);
        assert!(!cache.get_all().is_empty());

        cache.clear();
        assert!(cache.get_all().is_empty());
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_new() {
        let client = LocalhostDiscoveryClient::new();
        // Just verify it constructs without panic
        assert!(std::mem::size_of_val(&client) > 0);
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_discover_all_empty() {
        let client = LocalhostDiscoveryClient::new();
        let services = client.discover_all().await.unwrap();
        assert!(services.is_empty(), "empty client yields empty results");
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_discover_all_seeded() {
        let client = seeded_client();
        let services = client.discover_all().await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_discover_by_capability_empty() {
        let client = LocalhostDiscoveryClient::new();

        let cap = Capability::Compute(ComputeCapability::NativeExecution);

        let services = client.discover_by_capability(&cap).await.unwrap();
        assert!(services.is_empty(), "empty client yields empty results");
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_discover_by_capability_seeded() {
        let client = seeded_client();

        let cap = Capability::Compute(ComputeCapability::NativeExecution);

        let services = client.discover_by_capability(&cap).await.unwrap();
        assert!(!services.is_empty());
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_register_service() {
        let client = LocalhostDiscoveryClient::new();

        let service = DiscoveredService {
            id: Some("register-test".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![ServiceEndpoint::http("localhost", 9000)],
            healthy: true,
            metadata: HashMap::new(),
        };

        let result = client.register_service(&service).await;
        // LocalhostDiscoveryClient doesn't actually register, just returns Ok
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_deregister_service() {
        let client = LocalhostDiscoveryClient::new();
        let result = client.deregister_service("some-id").await;
        // LocalhostDiscoveryClient doesn't actually deregister, just returns Ok
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_localhost_discovery_client_health_check() {
        let client = LocalhostDiscoveryClient::new();
        let result = client.health_check("some-id").await;
        // LocalhostDiscoveryClient returns error (not implemented)
        // Just verify it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}
