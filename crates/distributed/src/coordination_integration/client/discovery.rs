// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination service discovery - Finds coordination providers by capability
//!
//! **Design**: Runtime discovery, no hardcoded service names.
//! Multi-strategy discovery (mDNS, registry, environment).

use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool_common::primal_identity::Capability;
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use crate::coordination_integration::{CoordinationConfig, ServiceLocation};

/// Coordination service discovery - Finds coordination providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CoordinationDiscovery {
    config: CoordinationConfig,
    discovery: ServiceDiscovery,
    discovered_services: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl CoordinationDiscovery {
    /// Create new discovery instance
    pub async fn new(config: CoordinationConfig) -> ToadStoolResult<Self> {
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

    /// Discover coordination services by capability
    ///
    /// **Design**: Multi-strategy discovery (mDNS, registry, environment)
    pub async fn discover(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Discover by each required capability
        for cap in &self.config.required_capabilities {
            let capability = Capability::Coordination(cap.clone());

            if let Ok(service) = self.discovery.find_service_by_capability(capability).await {
                services.push(service);
            }
        }

        // Remove duplicates (same service ID)
        services.dedup_by(|a, b| a.id == b.id);

        // Filter by location preference
        let filtered = self.filter_by_location(&services);

        // Cache discovered services
        let mut cache = self.discovered_services.write().await;
        *cache = filtered.clone();

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
