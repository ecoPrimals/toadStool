// SPDX-License-Identifier: AGPL-3.0-or-later
//! mDNS discovery client: cache, registration, and [`DiscoveryClient`] trait.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info};

use toadstool_common::ToadStoolResult;
use toadstool_common::primal_identity::{Capability, DiscoveredService};
use toadstool_common::runtime_discovery::DiscoveryClient;

use super::MDNS_SERVICE_TYPE;

// Note: The mdns crate (3.0) provides basic hostname resolution.
// For full mDNS-SD (Service Discovery) with TXT records, a more complete
// implementation would use mdns-sd or libmdns. This is a pragmatic
// implementation that provides the interface for future enhancement.

/// Cache entry for discovered services
#[derive(Debug, Clone)]
pub(crate) struct CachedService {
    pub(crate) service: DiscoveredService,
    pub(crate) discovered_at: SystemTime,
    pub(crate) last_seen: SystemTime,
}

impl CachedService {
    /// Returns how long this entry has been in the cache.
    pub(crate) fn age(&self) -> std::time::Duration {
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
    pub(crate) cache: Arc<RwLock<HashMap<String, CachedService>>>,

    /// How long before cached services are considered stale
    pub(super) cache_ttl: Duration,

    /// Advertised services (for tracking and deregistration)
    pub(super) advertised_services: Arc<RwLock<HashMap<String, String>>>,
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
    pub(super) async fn cleanup_stale_entries(&self) {
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
    pub(crate) async fn cache_service(&self, service: DiscoveredService) {
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
    pub(super) async fn touch_service(&self, service_id: &str) {
        if let Some(entry) = self.cache.write().await.get_mut(service_id) {
            entry.last_seen = SystemTime::now();
        }
    }
}

#[cfg(test)]
impl MdnsDiscoveryClient {
    /// Parse mDNS TXT records to extract capabilities (test and future mdns-sd helpers).
    pub(crate) fn parse_capabilities(txt_records: &[String]) -> Vec<Capability> {
        super::parser::parse_capabilities(txt_records)
    }

    /// Convert mDNS service record to `DiscoveredService` (used by unit tests; future mdns-sd path).
    pub(crate) fn mdns_to_discovered_service(
        id: String,
        address: std::net::IpAddr,
        port: u16,
        txt_records: &[String],
    ) -> DiscoveredService {
        super::parser::mdns_to_discovered_service(id, address, port, txt_records)
    }
}

impl DiscoveryClient for MdnsDiscoveryClient {
    fn discover_by_capability<'a>(
        &'a self,
        capability: &'a Capability,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }

    fn discover_all<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }

    fn register_service<'a>(
        &'a self,
        service: &'a DiscoveredService,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }

    fn deregister_service<'a>(
        &'a self,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }

    fn health_check<'a>(
        &'a self,
        service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>> {
        Box::pin(async move {
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
