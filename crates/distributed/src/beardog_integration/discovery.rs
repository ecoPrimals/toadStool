// SPDX-License-Identifier: AGPL-3.0-only
//! BearDog service discovery
//!
//! **Design**: Runtime discovery via mDNS, Songbird registry, or config

use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool_common::constants::timeouts;
use toadstool_common::{ToadStoolError, ToadStoolResult};

use super::types::{BearDogCapability, BearDogEndpoint};
use super::{BearDogConfig, ServiceLocation};

/// BearDog service discovery
///
/// **Design**: Runtime discovery via mDNS, Songbird registry, or config
pub struct BearDogDiscovery {
    config: BearDogConfig,
    discovered_endpoints: Arc<RwLock<Vec<BearDogEndpoint>>>,
}

impl BearDogDiscovery {
    /// Create new discovery instance
    pub fn new(config: BearDogConfig) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create discovery with pre-populated endpoints (for testing)
    #[cfg(test)]
    pub fn with_endpoints(config: BearDogConfig, endpoints: Vec<BearDogEndpoint>) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(endpoints)),
        }
    }

    /// Discover BearDog services
    ///
    /// **Design**: Multi-strategy discovery (mDNS, Songbird, static config)
    pub async fn discover(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        let mut endpoints = Vec::new();

        // Strategy 1: mDNS discovery (local network)
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Local | ServiceLocation::Any
        ) && let Ok(local_endpoints) = self.discover_via_mdns().await
        {
            endpoints.extend(local_endpoints);
        }

        // Strategy 2: Songbird primal registry
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Network | ServiceLocation::Any
        ) && let Ok(network_endpoints) = self.discover_via_songbird().await
        {
            endpoints.extend(network_endpoints);
        }

        // Cache discovered endpoints
        *self.discovered_endpoints.write().await = endpoints.clone();

        Ok(endpoints)
    }

    /// Discover via mDNS (local network)
    ///
    /// **Design**: Look for _beardog._tcp.local service
    async fn discover_via_mdns(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let discovery_config = DiscoveryConfig {
            enable_mdns: true,
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
            ..Default::default()
        };

        match PrimalDiscovery::with_config(discovery_config) {
            Ok(discovery) => match discovery.find_capability("security").await {
                Ok(endpoint) => {
                    let beardog_endpoint = BearDogEndpoint {
                        service_id: endpoint.service_id.clone(),
                        protocol: "http".to_string(),
                        address: endpoint
                            .url()
                            .parse()
                            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 8081))),
                        api_version: "v1".to_string(),
                        capabilities: vec![BearDogCapability::Encryption {
                            algorithms: vec!["aes-256".to_string()],
                        }],
                        healthy: true,
                        latency_ms: Some(endpoint.latency_ms),
                    };
                    Ok(vec![beardog_endpoint])
                }
                Err(_) => Ok(Vec::new()),
            },
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Discover via Songbird primal registry
    ///
    /// **Design**: Query Songbird for BearDog capability
    async fn discover_via_songbird(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let mut discovery_config = DiscoveryConfig {
            enable_mdns: false,
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
            ..Default::default()
        };

        if let Ok(coordination_endpoint) =
            std::env::var("COORDINATION_ENDPOINT").or_else(|_| std::env::var("SONGBIRD_ENDPOINT"))
        {
            discovery_config
                .fallbacks
                .insert("orchestration".to_string(), coordination_endpoint);
        }

        match PrimalDiscovery::with_config(discovery_config) {
            Ok(discovery) => match discovery.find_capability("security").await {
                Ok(endpoint) => {
                    let beardog_endpoint = BearDogEndpoint {
                        service_id: endpoint.service_id.clone(),
                        protocol: "http".to_string(),
                        address: endpoint
                            .url()
                            .parse()
                            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 8081))),
                        api_version: "v1".to_string(),
                        capabilities: vec![BearDogCapability::Encryption {
                            algorithms: vec!["aes-256".to_string()],
                        }],
                        healthy: true,
                        latency_ms: Some(endpoint.latency_ms),
                    };
                    Ok(vec![beardog_endpoint])
                }
                Err(_) => Ok(Vec::new()),
            },
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Config getter for tests
    #[cfg(test)]
    pub fn config(&self) -> &BearDogConfig {
        &self.config
    }

    /// Get best endpoint based on location preference and health
    #[allow(clippy::significant_drop_tightening)] // healthy_endpoints are refs from endpoints
    pub async fn get_best_endpoint(&self) -> ToadStoolResult<BearDogEndpoint> {
        let endpoints = self.discovered_endpoints.read().await;

        if endpoints.is_empty() {
            return Err(ToadStoolError::not_found(
                "No security/crypto endpoints discovered",
            ));
        }

        let healthy_endpoints: Vec<_> = endpoints.iter().filter(|e| e.healthy).collect();

        if healthy_endpoints.is_empty() {
            return Err(ToadStoolError::not_found(
                "No healthy security/crypto endpoints available",
            ));
        }

        let mut sorted = healthy_endpoints;
        sorted.sort_by_key(|e| e.latency_ms.unwrap_or(u64::MAX));

        Ok(sorted[0].clone())
    }
}
