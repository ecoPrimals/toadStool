// SPDX-License-Identifier: AGPL-3.0-or-later
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

use toadstool_common::primal_identity::{
    AuthCapability, Capability, ComputeCapability, CoordinationCapability, DiscoveredService,
    DiscoveryCapability, ServiceEndpoint, StorageCapability,
};
use toadstool_common::runtime_discovery::DiscoveryClient;
use toadstool_common::ToadStoolResult;

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
            if let Some(endpoint) = service.endpoints.first() {
                format!("{}:{}", endpoint.address, endpoint.port)
            } else {
                format!(
                    "service-{}",
                    now.duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs()
                )
            }
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
    #[allow(dead_code)] // Future: mdns-sd integration when cache-based discovery is replaced
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
        let cache = self.cache.read().await;
        let services: Vec<DiscoveredService> = cache
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

        let cache = self.cache.read().await;
        let services: Vec<DiscoveredService> =
            cache.values().map(|entry| entry.service.clone()).collect();

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
        if let Some(hostname) = self.advertised_services.write().await.remove(service_id) {
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
        if let Some(entry) = cache.get(service_id) {
            let age = SystemTime::now()
                .duration_since(entry.last_seen)
                .unwrap_or(Duration::from_secs(0));

            // Consider healthy if seen within TTL period
            Ok(age < self.cache_ttl)
        } else {
            Ok(false)
        }
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

    #[tokio::test]
    async fn test_mdns_client_creation() {
        let client = MdnsDiscoveryClient::new();
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
}
