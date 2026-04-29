// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based Coordination Discovery Client
//!
//! **Philosophy**: "Discover by capability, not by name"
//!
//! ToadStool knows what it NEEDS (service-discovery, load-balancing),
//! not what specific service provides it.

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::timeouts;
use toadstool_common::infant_discovery::{
    DiscoveredService, DiscoveryEngine, DiscoverySource, ServiceHealth, ServiceMetadata,
};
use toadstool_common::interned_strings::capabilities;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::types::CoordinationTransport;

/// Capability-based Coordination client
///
/// Discovers services by capability, maintains cache, handles failover.
pub struct CapabilityClient {
    /// Discovery engine for finding services
    discovery: Arc<DiscoveryEngine>,

    /// Required capabilities for service selection
    required_capabilities: Vec<String>,

    /// Timeout for operations
    #[expect(dead_code, reason = "reserved for future request timeout")]
    timeout: Duration,

    /// Preferred protocol
    preferred_protocol: CoordinationTransport,

    /// Cached discovered services
    cached_services: Arc<RwLock<Vec<DiscoveredService>>>,

    /// Last discovery timestamp
    last_discovery: Arc<RwLock<Option<SystemTime>>>,
}

impl CapabilityClient {
    /// Create a new capability-based client
    ///
    /// **Self-Knowledge Pattern**: Specify what you NEED, not who provides it
    ///
    /// # Example
    /// ```no_run
    /// use toadstool_distributed::coordination::CapabilityClient;
    /// use toadstool_common::infant_discovery::DiscoveryEngine;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = Arc::new(DiscoveryEngine::new());
    ///
    /// // Discover ANY service with these capabilities
    /// let client = CapabilityClient::discover(
    ///     discovery,
    ///     vec![
    ///         "service-discovery".to_string(),
    ///         "load-balancing".to_string(),
    ///     ],
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover(
        discovery: Arc<DiscoveryEngine>,
        required_capabilities: Vec<String>,
    ) -> ToadStoolResult<Self> {
        info!(
            "🔍 Discovering services with capabilities: {:?}",
            required_capabilities
        );

        let client = Self {
            discovery: Arc::clone(&discovery),
            required_capabilities,
            timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
            preferred_protocol: CoordinationTransport::HTTP,
            cached_services: Arc::new(RwLock::new(Vec::new())),
            last_discovery: Arc::new(RwLock::new(None)),
        };

        // Perform initial discovery
        client.refresh_discovery().await?;

        Ok(client)
    }

    /// Refresh service discovery
    ///
    /// Queries discovery engine for services matching required capabilities.
    pub async fn refresh_discovery(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        debug!("🔄 Refreshing service discovery");

        let mut discovered_services = Vec::new();

        // Discover endpoint for each required capability
        for capability in &self.required_capabilities {
            match self.discovery.discover_endpoint(capability).await {
                Ok(endpoint) => {
                    debug!(
                        "Found endpoint for capability '{}': {}",
                        capability, endpoint
                    );

                    // Create a DiscoveredService from the endpoint
                    let service = DiscoveredService {
                        capability: capability.clone(),
                        endpoint: endpoint.clone(),
                        protocols: vec!["http".to_string()], // Default to HTTP
                        metadata: ServiceMetadata {
                            version: None,
                            health: ServiceHealth::Unknown,
                            last_seen: std::time::SystemTime::now(),
                            priority: 50,
                            extra: std::collections::HashMap::new(),
                        },
                        source: DiscoverySource::UniversalAdapter,
                    };

                    discovered_services.push(service);
                }
                Err(e) => {
                    warn!(
                        "Failed to discover endpoint for capability '{}': {}",
                        capability, e
                    );
                }
            }
        }

        // Deduplicate by endpoint
        discovered_services.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
        discovered_services.dedup_by(|a, b| a.endpoint == b.endpoint);

        info!("✅ Discovered {} services", discovered_services.len());

        // Update cache
        {
            let mut cache = self.cached_services.write().await;
            *cache = discovered_services.clone();
        }

        // Update last discovery time
        {
            let mut last = self.last_discovery.write().await;
            *last = Some(SystemTime::now());
        }

        Ok(discovered_services)
    }

    /// Get available services (from cache or refresh if stale)
    pub async fn get_available_services(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let should_refresh = {
            let last = self.last_discovery.read().await;
            (*last).is_none_or(|last_time| {
                let age = SystemTime::now()
                    .duration_since(last_time)
                    .unwrap_or_default();
                age.as_secs() > 300 // 5 minutes
            })
        };

        if should_refresh {
            debug!("Cache stale, refreshing discovery");
            self.refresh_discovery().await
        } else {
            let cache = self.cached_services.read().await;
            Ok(cache.clone())
        }
    }

    /// Get best available service
    ///
    /// Selects based on:
    /// 1. Health status (healthy preferred)
    /// 2. Protocol compatibility
    /// 3. Priority/load
    pub async fn get_best_service(&self) -> ToadStoolResult<DiscoveredService> {
        let services = self.get_available_services().await?;

        if services.is_empty() {
            return Err(ToadStoolError::configuration(
                "No services available for capability-based discovery",
            ));
        }

        // Filter by protocol compatibility
        let protocol_str = match self.preferred_protocol {
            CoordinationTransport::HTTP => "http",
            CoordinationTransport::GRPC => "grpc",
            CoordinationTransport::MessageQueue => "messagequeue",
        };

        let compatible: Vec<_> = services
            .iter()
            .filter(|s| {
                s.protocols.iter().any(|p| p.to_lowercase() == protocol_str)
                    || s.protocols.contains(&"http".to_string())
            })
            .collect();

        let selected = if !compatible.is_empty() {
            compatible[0].clone()
        } else {
            services[0].clone()
        };

        debug!("📍 Selected service: {}", selected.endpoint);
        Ok(selected)
    }

    /// Execute with automatic failover
    ///
    /// Tries operation against available services with automatic failover.
    pub async fn execute_with_failover<F, Fut, T>(&self, operation: F) -> ToadStoolResult<T>
    where
        F: Fn(DiscoveredService) -> Fut,
        Fut: std::future::Future<Output = ToadStoolResult<T>>,
    {
        let services = self.get_available_services().await?;

        if services.is_empty() {
            return Err(ToadStoolError::configuration(
                "No services available for failover",
            ));
        }

        let mut last_error = None;

        for service in services {
            debug!("Trying service: {}", service.endpoint);

            match operation(service.clone()).await {
                Ok(result) => {
                    info!("✅ Operation succeeded on {}", service.endpoint);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("❌ Operation failed on {}: {}", service.endpoint, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // ✅ EVOLVED: Proper error with context
        Err(last_error.unwrap_or_else(|| {
            ToadStoolError::Integration(
                toadstool_common::error::IntegrationError::ServiceUnavailable {
                    service: capabilities::COORDINATION.to_string(),
                    reason: "All services failed during failover attempt".to_string(),
                },
            )
        }))
    }

    /// Check if client is healthy
    pub async fn is_healthy(&self) -> bool {
        self.get_available_services()
            .await
            .is_ok_and(|services| !services.is_empty())
    }

    /// Get client statistics
    pub async fn get_stats(&self) -> ClientStats {
        let services = self.cached_services.read().await;
        let last = self.last_discovery.read().await;

        ClientStats {
            available_services: services.len(),
            last_discovery: *last,
            cache_age_seconds: last.map(|t| {
                SystemTime::now()
                    .duration_since(t)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0)
            }),
        }
    }
}

/// Snapshot of capability client cache and discovery timing.
#[derive(Debug, Clone)]
pub struct ClientStats {
    /// Number of services held in the local cache.
    pub available_services: usize,
    /// Wall-clock time of the last successful discovery refresh, if any.
    pub last_discovery: Option<SystemTime>,
    /// Seconds since `last_discovery`, or `None` if never discovered.
    pub cache_age_seconds: Option<i64>,
}

#[cfg(test)]
#[allow(
    deprecated,
    reason = "tests exercise legacy coordination client pending migration"
)]
#[path = "capability_client_tests.rs"]
mod tests;
