// SPDX-License-Identifier: AGPL-3.0-or-later
//! security service discovery
//!
//! **Design**: Runtime discovery via mDNS, Coordination registry, or config

use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool_common::constants::timeouts;
use toadstool_common::{ToadStoolError, ToadStoolResult};

use super::types::{SecurityCapability, SecurityEndpoint};
use super::{SecurityConfig, ServiceLocation};

/// security service discovery
///
/// **Design**: Runtime discovery via mDNS, Coordination registry, or config
pub struct SecurityDiscovery {
    config: SecurityConfig,
    discovered_endpoints: Arc<RwLock<Vec<SecurityEndpoint>>>,
}

impl SecurityDiscovery {
    /// Create new discovery instance
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create discovery with pre-populated endpoints (for testing)
    #[cfg(test)]
    pub fn with_endpoints(config: SecurityConfig, endpoints: Vec<SecurityEndpoint>) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(endpoints)),
        }
    }

    /// Discover security services
    ///
    /// **Design**: Multi-strategy discovery (mDNS, Coordination, static config)
    pub async fn discover(&self) -> ToadStoolResult<Vec<SecurityEndpoint>> {
        let mut endpoints = Vec::new();

        // Strategy 1: mDNS discovery (local network)
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Local | ServiceLocation::Any
        ) && let Ok(local_endpoints) = self.discover_via_mdns().await
        {
            endpoints.extend(local_endpoints);
        }

        // Strategy 2: Coordination primal registry
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Network | ServiceLocation::Any
        ) && let Ok(network_endpoints) = self.discover_via_coordination().await
        {
            endpoints.extend(network_endpoints);
        }

        // Cache discovered endpoints
        *self.discovered_endpoints.write().await = endpoints.clone();

        Ok(endpoints)
    }

    /// Discover via mDNS (local network)
    ///
    /// **Design**: Look for _security._tcp.local service
    async fn discover_via_mdns(&self) -> ToadStoolResult<Vec<SecurityEndpoint>> {
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let discovery_config = DiscoveryConfig {
            enable_mdns: true,
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
            ..Default::default()
        };

        match PrimalDiscovery::with_config(discovery_config) {
            Ok(discovery) => match discovery.find_capability("security").await {
                Ok(endpoint) => {
                    let security_endpoint = SecurityEndpoint {
                        service_id: endpoint.service_id.clone(),
                        protocol: "http".to_string(),
                        address: endpoint
                            .url()
                            .parse()
                            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 8081))),
                        api_version: "v1".to_string(),
                        capabilities: vec![SecurityCapability::Encryption {
                            algorithms: vec!["aes-256".to_string()],
                        }],
                        healthy: true,
                        latency_ms: Some(endpoint.latency_ms),
                    };
                    Ok(vec![security_endpoint])
                }
                Err(_) => Ok(Vec::new()),
            },
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Discover via Coordination primal registry
    ///
    /// **Design**: Query Coordination for Security capability
    async fn discover_via_coordination(&self) -> ToadStoolResult<Vec<SecurityEndpoint>> {
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let mut discovery_config = DiscoveryConfig {
            enable_mdns: false,
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
            ..Default::default()
        };

        let socket_env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
        if let Some(coordination_endpoint) = socket_env.coordination_connection_hint.clone() {
            discovery_config
                .fallbacks
                .insert("orchestration".to_string(), coordination_endpoint);
        }

        match PrimalDiscovery::with_config(discovery_config) {
            Ok(discovery) => match discovery.find_capability("security").await {
                Ok(endpoint) => {
                    let security_endpoint = SecurityEndpoint {
                        service_id: endpoint.service_id.clone(),
                        protocol: "http".to_string(),
                        address: endpoint
                            .url()
                            .parse()
                            .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 8081))),
                        api_version: "v1".to_string(),
                        capabilities: vec![SecurityCapability::Encryption {
                            algorithms: vec!["aes-256".to_string()],
                        }],
                        healthy: true,
                        latency_ms: Some(endpoint.latency_ms),
                    };
                    Ok(vec![security_endpoint])
                }
                Err(_) => Ok(Vec::new()),
            },
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Config getter for tests
    #[cfg(test)]
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Get best endpoint based on location preference and health
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // healthy_endpoints are refs from endpoints
    pub async fn get_best_endpoint(&self) -> ToadStoolResult<SecurityEndpoint> {
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
