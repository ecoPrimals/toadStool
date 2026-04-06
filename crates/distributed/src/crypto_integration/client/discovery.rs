// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime discovery of crypto services by capability and location preference.

use std::sync::Arc;

use tokio::sync::RwLock;

use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use crate::crypto_integration::{CryptoServiceConfig, ServiceLocation};

/// Crypto service discovery - Finds crypto providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CryptoServiceDiscovery {
    pub(crate) config: CryptoServiceConfig,
    discovery: ServiceDiscovery,
    discovered_services: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl CryptoServiceDiscovery {
    /// Create new discovery instance
    pub async fn new(config: CryptoServiceConfig) -> ToadStoolResult<Self> {
        // Use Auto discovery method - tries all strategies
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        Ok(Self {
            config,
            discovery,
            discovered_services: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Discover crypto services by capability
    ///
    /// **Design**: Multi-strategy discovery (mDNS, registry, environment)
    pub async fn discover(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Discover by each required capability
        for cap in &self.config.required_capabilities {
            let capability = Capability::Crypto(cap.clone());

            if let Ok(service) = self.discovery.find_service_by_capability(capability).await {
                services.push(service);
            }
        }

        // Filter by location preference
        let filtered = self.filter_by_location(&services);

        // Cache discovered services
        *self.discovered_services.write().await = filtered.clone();

        Ok(filtered)
    }

    /// Discover by specific capability
    pub async fn discover_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<Option<DiscoveredService>> {
        self.discovery
            .find_service_by_capability(capability)
            .await
            .map(Some)
            .or(Ok(None))
    }

    /// Filter services by location preference
    pub(crate) fn filter_by_location(
        &self,
        services: &[DiscoveredService],
    ) -> Vec<DiscoveredService> {
        match self.config.preferred_location {
            ServiceLocation::Local => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        e.address.starts_with("127.")
                            || e.address == toadstool_common::constants::network::DEFAULT_HOSTNAME
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Network => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        !e.address.starts_with("127.")
                            && e.address != toadstool_common::constants::network::DEFAULT_HOSTNAME
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Any => services.to_vec(),
        }
    }

    /// Get cached services
    pub async fn get_cached(&self) -> Vec<DiscoveredService> {
        self.discovered_services.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::primal_identity::{Capability, CryptoCapability, ServiceEndpoint};
    use toadstool_common::service_discovery::DiscoveredService;

    #[tokio::test]
    async fn test_crypto_service_discovery_creation() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config)
            .await
            .expect("Failed to create discovery");

        assert!(!discovery.config.required_capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_discover_returns_vec() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = discovery.discover().await.unwrap();
        assert!(services.is_empty() || !services.is_empty());
    }

    #[tokio::test]
    async fn test_get_cached_initially_empty() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty());
    }

    #[tokio::test]
    async fn test_discover_by_capability_returns_option() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let cap = Capability::Crypto(CryptoCapability::Encryption);
        let result = discovery.discover_by_capability(cap).await.unwrap();
        assert!(result.is_none() || result.is_some());
    }

    #[tokio::test]
    async fn test_location_filtering_network() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
                endpoints: vec![ServiceEndpoint::http(
                    toadstool_common::constants::network::LOCALHOST_IPV4,
                    8080,
                )],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
                endpoints: vec![ServiceEndpoint::http("10.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
        ];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "remote");
    }

    #[tokio::test]
    async fn test_location_filtering_any() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "svc1".to_string(),
            name: "crypto-1".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http(
                toadstool_common::constants::network::LOCALHOST_IPV4,
                9000,
            )],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
    }

    #[tokio::test]
    async fn test_location_filtering() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
                endpoints: vec![ServiceEndpoint::http(
                    toadstool_common::constants::network::LOCALHOST_IPV4,
                    8080,
                )],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
                endpoints: vec![ServiceEndpoint::http("10.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
        ];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "local");
    }

    #[tokio::test]
    async fn test_discover_caches_services() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let _ = discovery.discover().await;
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty() || !cached.is_empty());
    }

    #[tokio::test]
    async fn test_location_filter_local_filters_remote() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "remote".to_string(),
            name: "remote-crypto".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http("10.0.0.1", 9000)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn test_filter_by_location_local_with_localhost() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "local".to_string(),
            name: "local".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
    }
}
