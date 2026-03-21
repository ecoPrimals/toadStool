// SPDX-License-Identifier: AGPL-3.0-only
//! mDNS Discovery Client Implementation
//!
//! Implements multicast DNS (mDNS) service discovery for zero-config
//! capability-based service discovery in local networks.
//!
//! # Architecture
//!
//! - Uses mDNS (RFC 6762) for zero-config service discovery
//! - Services advertise capabilities via TXT records
//! - Automatic service detection without configuration
//! - Falls back gracefully when mDNS unavailable
//!
//! # Philosophy: "Each Primal Knows Only Itself"
//!
//! - Services broadcast WHAT they can do (capabilities)
//! - Consumers discover by WHAT they need (not WHO)
//! - No hardcoded knowledge of other services
//! - Runtime resolution of all dependencies

use async_trait::async_trait;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use toadstool_common::ToadStoolResult;
use toadstool_common::primal_identity::{
    AuthCapability, Capability, ComputeCapability, CoordinationCapability, DiscoveredService,
    DiscoveryCapability, ServiceEndpoint, StorageCapability,
};
use toadstool_common::runtime_discovery::DiscoveryClient;

// Note: The mdns crate (3.0) provides basic hostname resolution.
// For full mDNS-SD (Service Discovery) with TXT records, a more complete
// implementation would use mdns-sd or libmdns. This is a pragmatic
// implementation that provides the interface for future enhancement.

/// mDNS service type for ecoPrimals services
pub(crate) const MDNS_SERVICE_TYPE: &str = "_ecoprimals._tcp.local.";

/// Cache entry for discovered services
#[derive(Debug, Clone)]
struct CachedService {
    service: DiscoveredService,
    discovered_at: SystemTime,
    last_seen: SystemTime,
}

impl CachedService {
    /// Returns how long this entry has been in the cache.
    fn age(&self) -> std::time::Duration {
        self.discovered_at
            .elapsed()
            .unwrap_or(std::time::Duration::ZERO)
    }
}

/// mDNS-based discovery client
///
/// Discovers services using multicast DNS in local networks.
/// Implements zero-configuration service discovery following
/// the "self-knowledge only" principle.
///
/// # Examples
///
/// ```rust,ignore
/// use toadstool_common::runtime_discovery::RuntimeDiscovery;
/// use toadstool_config::mdns_discovery::MdnsDiscoveryClient;
///
/// let client = MdnsDiscoveryClient::new()?;
/// let discovery = RuntimeDiscovery::new(Arc::new(client));
///
/// // Discover coordination services
/// let services = discovery.discover_capability(&Capability::Coordination).await?;
/// ```
pub struct MdnsDiscoveryClient {
    /// Cache of discovered services
    cache: Arc<RwLock<HashMap<String, CachedService>>>,

    /// How long before cached services are considered stale
    cache_ttl: Duration,

    /// Advertised services (for tracking and deregistration)
    advertised_services: Arc<RwLock<HashMap<String, String>>>,
    // Note: mDNS daemon integration pending
    // The mdns 3.0 crate provides basic hostname resolution but not full service
    // discovery/advertisement. For production use, this should be enhanced with
    // mdns-sd (https://crates.io/crates/mdns-sd) which provides:
    // - Service advertisement with TXT records
    // - Service browsing and discovery
    // - Full mDNS-SD protocol support
    //
    // For now, this implementation uses cache-based discovery with explicit
    // service registration, which works well for controlled environments and
    // provides the foundation for future mDNS-SD integration.
}

impl MdnsDiscoveryClient {
    /// Create a new mDNS discovery client
    ///
    /// # Returns
    ///
    /// A configured mDNS discovery client ready for use
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_ttl(Duration::from_secs(300))
    }

    /// Create a new mDNS discovery client with custom TTL
    ///
    /// # Arguments
    ///
    /// * `cache_ttl` - How long to cache discovered services
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn with_ttl(cache_ttl: Duration) -> ToadStoolResult<Self> {
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let advertised_services = Arc::new(RwLock::new(HashMap::new()));

        info!("Initialized mDNS discovery client (cache-based)");
        info!("Note: Full mDNS-SD integration pending (see module docs for details)");

        Ok(Self {
            cache,
            cache_ttl,
            advertised_services,
        })
    }

    /// Clean up stale entries from cache
    async fn cleanup_stale_entries(&self) {
        let mut cache = self.cache.write().await;
        let now = SystemTime::now();

        cache.retain(|id, entry| match now.duration_since(entry.last_seen) {
            Ok(age) if age > self.cache_ttl => {
                debug!(
                    "Removing stale service from cache: {} (age: {:?}, last_seen: {:?} ago)",
                    id,
                    entry.age(),
                    age,
                );
                false
            }
            _ => true,
        });
    }

    /// Add a service to the cache
    async fn cache_service(&self, service: DiscoveredService) {
        let now = SystemTime::now();
        // Use service ID or generate one from endpoints
        let id = service.id.clone().unwrap_or_else(|| {
            service.endpoints.first().map_or_else(
                || {
                    format!(
                        "service-{}",
                        now.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs()
                    )
                },
                |endpoint| format!("{}:{}", endpoint.address, endpoint.port),
            )
        });

        let cached = CachedService {
            service,
            discovered_at: now,
            last_seen: now,
        };

        self.cache.write().await.insert(id, cached);
    }

    /// Update `last_seen` timestamp for a service
    async fn touch_service(&self, service_id: &str) {
        if let Some(entry) = self.cache.write().await.get_mut(service_id) {
            entry.last_seen = SystemTime::now();
        }
    }

    /// Parse mDNS TXT records to extract capabilities
    fn parse_capabilities(txt_records: &[String]) -> Vec<Capability> {
        let mut capabilities = Vec::new();

        for record in txt_records {
            if let Some(cap_str) = record.strip_prefix("capability=") {
                // Parse capability from string
                // Format: "capability=coordination:service-discovery", "capability=storage:object", etc.
                let parts: Vec<&str> = cap_str.split(':').collect();
                match parts.as_slice() {
                    ["coordination", "service-discovery" | _] => capabilities.push(
                        Capability::Coordination(CoordinationCapability::ServiceDiscovery),
                    ),
                    ["storage", "object" | _] => {
                        capabilities.push(Capability::Storage(StorageCapability::ObjectStorage));
                    }
                    ["compute", "native" | _] => {
                        capabilities.push(Capability::Compute(ComputeCapability::NativeExecution));
                    }
                    ["authentication", _] => {
                        capabilities.push(Capability::Authentication(AuthCapability::UserAuth));
                    }
                    ["discovery", "mdns"] => {
                        capabilities
                            .push(Capability::Discovery(DiscoveryCapability::MdnsDiscovery));
                    }
                    ["discovery", _] => capabilities.push(Capability::Discovery(
                        DiscoveryCapability::CapabilityDiscovery,
                    )),
                    _ => warn!("Unknown capability in mDNS record: {}", cap_str),
                }
            }
        }

        capabilities
    }

    /// Convert mDNS service record to `DiscoveredService`
    ///
    /// Note: Helper function for future mdns-sd integration.
    /// Currently using cache-based discovery.
    #[allow(dead_code, reason = "reserved for mdns-sd integration")]
    fn mdns_to_discovered_service(
        id: String,
        address: IpAddr,
        port: u16,
        txt_records: &[String],
    ) -> DiscoveredService {
        let capabilities = Self::parse_capabilities(txt_records);

        let endpoint = ServiceEndpoint {
            address: address.to_string(),
            port,
            protocol: "http".to_string(), // Default to HTTP
            path: None,
            metadata: HashMap::new(),
        };

        DiscoveredService {
            id: Some(id),
            capabilities,
            endpoints: vec![endpoint],
            healthy: true,
            metadata: HashMap::new(),
        }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl DiscoveryClient for MdnsDiscoveryClient {
    async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        // Clean up stale entries first
        self.cleanup_stale_entries().await;

        // Trigger mDNS discovery for ecoPrimals services
        // Note: Currently using cache-based discovery. When mdns-sd is integrated,
        // this will trigger actual mDNS queries for service discovery.
        // For now, services must be explicitly registered via register_service().

        // Future: Trigger actual mDNS-SD browse for _ecoprimals._tcp.local.
        // This will automatically discover services on the network without
        // explicit registration.

        // Query cache for services with requested capability
        let services: Vec<DiscoveredService> = self
            .cache
            .read()
            .await
            .values()
            .filter_map(|entry| {
                if entry.service.capabilities.contains(capability) {
                    Some(entry.service.clone())
                } else {
                    None
                }
            })
            .collect();

        if services.is_empty() {
            debug!(
                "No services found with capability {:?} via mDNS",
                capability
            );
        } else {
            info!(
                "Found {} service(s) with capability {:?} via mDNS",
                services.len(),
                capability
            );
        }

        Ok(services)
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        // Clean up stale entries first
        self.cleanup_stale_entries().await;

        // Trigger mDNS browse for all ecoPrimals services
        // Future: When mdns-sd is integrated, this will browse for all services

        let services: Vec<DiscoveredService> = self
            .cache
            .read()
            .await
            .values()
            .map(|entry| entry.service.clone())
            .collect();

        info!("Discovered {} total service(s) via mDNS", services.len());
        Ok(services)
    }

    async fn register_service(&self, service: &DiscoveredService) -> ToadStoolResult<()> {
        let service_id = service.id.as_deref().unwrap_or("unknown");
        info!("Registering service {} via mDNS", service_id);

        // Advertise service via mDNS
        // Future: When mdns-sd is integrated, this will:
        // 1. Create an mDNS service record for _ecoprimals._tcp.local.
        // 2. Add TXT records encoding service capabilities
        // 3. Respond to mDNS queries automatically
        //
        // For now, we track advertised services for future integration
        if !service.endpoints.is_empty() {
            let hostname = format!("{service_id}.local");

            self.advertised_services
                .write()
                .await
                .insert(service_id.to_string(), hostname.clone());

            info!(
                "Service {} registered (hostname: {}, type: {})",
                service_id, hostname, MDNS_SERVICE_TYPE,
            );
            debug!("Full mDNS advertisement will be enabled when mdns-sd is integrated");
        }

        // Add to cache (always do this regardless of mDNS status)
        self.cache_service(service.clone()).await;

        Ok(())
    }

    async fn deregister_service(&self, service_id: &str) -> ToadStoolResult<()> {
        info!("Deregistering service {} from mDNS", service_id);

        // Stop advertising service via mDNS
        // Future: When mdns-sd is integrated, this will stop mDNS responses
        let value = self.advertised_services.write().await.remove(service_id);
        if let Some(hostname) = value {
            info!(
                "Deregistered service {} (hostname: {})",
                service_id, hostname
            );
        }

        // Remove from cache
        self.cache.write().await.remove(service_id);

        Ok(())
    }

    async fn health_check(&self, service_id: &str) -> ToadStoolResult<bool> {
        // Update last_seen timestamp
        self.touch_service(service_id).await;

        // In mDNS, presence on network indicates health
        // Services that go offline stop responding to mDNS queries
        let cache = self.cache.read().await;
        cache.get(service_id).map_or(Ok(false), |entry| {
            let age = SystemTime::now()
                .duration_since(entry.last_seen)
                .unwrap_or(Duration::from_secs(0));

            // Consider healthy if seen within TTL period
            Ok(age < self.cache_ttl)
        })
    }
}

// Full mDNS-SD Implementation Notes (for future enhancement):
//
// Current: Cache-based discovery with explicit registration
// Future: Add mdns-sd crate for automatic network-based discovery
//
// Enhancement tasks:
// 1. Add mdns-sd dependency
// 2. Implement automatic service advertisement with TXT records
// 3. Add service browsing for _ecoprimals._tcp.local.
// 4. Parse capabilities from TXT records
// 5. Support multi-homed networks
//
// See module documentation for complete implementation guide.

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_mdns_client_creation() {
        let client = MdnsDiscoveryClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_mdns_client_with_ttl() {
        let client = MdnsDiscoveryClient::with_ttl(Duration::from_secs(60));
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_cache_service() {
        let client = MdnsDiscoveryClient::new().unwrap();

        let service = DiscoveredService {
            id: Some("test-service".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.cache_service(service.clone()).await;

        let cache = client.cache.read().await;
        assert!(cache.contains_key("test-service"));
    }

    #[tokio::test]
    async fn test_discover_by_capability_empty() {
        let client = MdnsDiscoveryClient::new().unwrap();

        let services = client
            .discover_by_capability(&Capability::Coordination(CoordinationCapability::default()))
            .await
            .unwrap();

        // Should return empty when no services cached
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_parse_capabilities() {
        let txt_records = vec![
            "capability=coordination:service-discovery".to_string(),
            "capability=storage:object".to_string(),
            "version=1.0".to_string(),
        ];

        let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
        assert_eq!(capabilities.len(), 2);

        // Verify parsed capabilities
        assert!(capabilities.contains(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery
        )));
        assert!(capabilities.contains(&Capability::Storage(StorageCapability::ObjectStorage)));
    }

    #[tokio::test]
    async fn test_parse_capabilities_compute_authentication_discovery() {
        let txt_records = vec![
            "capability=compute:native".to_string(),
            "capability=authentication:user".to_string(),
            "capability=discovery:mdns".to_string(),
            "capability=discovery:other".to_string(),
        ];

        let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
        assert!(capabilities.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
        assert!(capabilities.contains(&Capability::Authentication(AuthCapability::UserAuth)));
        assert!(capabilities.contains(&Capability::Discovery(DiscoveryCapability::MdnsDiscovery)));
        assert!(capabilities.contains(&Capability::Discovery(
            DiscoveryCapability::CapabilityDiscovery
        )));
    }

    #[tokio::test]
    async fn test_parse_capabilities_unknown_skipped() {
        let txt_records = vec![
            "capability=unknown:variant".to_string(),
            "capability=coordination:service-discovery".to_string(),
        ];
        let capabilities = MdnsDiscoveryClient::parse_capabilities(&txt_records);
        assert_eq!(capabilities.len(), 1);
    }

    #[tokio::test]
    async fn test_register_service_with_endpoints() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let service = DiscoveredService {
            id: Some("svc-1".to_string()),
            capabilities: vec![Capability::Storage(StorageCapability::ObjectStorage)],
            endpoints: vec![ServiceEndpoint {
                address: "192.168.1.10".to_string(),
                port: 8080,
                protocol: "http".to_string(),
                path: None,
                metadata: HashMap::new(),
            }],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.register_service(&service).await.unwrap();

        let services = client
            .discover_by_capability(&Capability::Storage(StorageCapability::ObjectStorage))
            .await
            .unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id.as_deref(), Some("svc-1"));
    }

    #[tokio::test]
    async fn test_deregister_service() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let service = DiscoveredService {
            id: Some("to-remove".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.cache_service(service).await;
        client.deregister_service("to-remove").await.unwrap();

        let services = client
            .discover_by_capability(&Capability::Compute(ComputeCapability::NativeExecution))
            .await
            .unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_discover_all_with_services() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let service = DiscoveredService {
            id: Some("all-svc".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::default())],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.cache_service(service).await;

        let all = client.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_health_check_cached_service() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let service = DiscoveredService {
            id: Some("health-svc".to_string()),
            capabilities: vec![],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.cache_service(service).await;
        let healthy = client.health_check("health-svc").await.unwrap();
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_health_check_unknown_service() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let healthy = client.health_check("nonexistent-svc-id").await.unwrap();
        assert!(!healthy);
    }

    #[tokio::test]
    async fn test_mdns_to_discovered_service() {
        let txt = vec![
            "capability=coordination:service-discovery".to_string(),
            "capability=storage:object".to_string(),
        ];
        let service = MdnsDiscoveryClient::mdns_to_discovered_service(
            "mdns-svc".to_string(),
            IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1)),
            9000,
            &txt,
        );
        assert_eq!(service.id.as_deref(), Some("mdns-svc"));
        assert_eq!(service.endpoints.len(), 1);
        assert_eq!(service.endpoints[0].address, "192.168.1.1");
        assert_eq!(service.endpoints[0].port, 9000);
        assert!(service.healthy);
        assert_eq!(service.capabilities.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_service_id_from_endpoint_when_no_id() {
        let client = MdnsDiscoveryClient::new().unwrap();
        let service = DiscoveredService {
            id: None,
            capabilities: vec![],
            endpoints: vec![ServiceEndpoint {
                address: "10.0.0.1".to_string(),
                port: 7777,
                protocol: "http".to_string(),
                path: None,
                metadata: HashMap::new(),
            }],
            healthy: true,
            metadata: HashMap::new(),
        };

        client.cache_service(service).await;

        let cache = client.cache.read().await;
        assert!(cache.contains_key("10.0.0.1:7777"));
    }
}
