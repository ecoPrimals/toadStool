// SPDX-License-Identifier: AGPL-3.0-only
//! # Runtime Discovery Module
//!
//! "Discover Others at Runtime"
//!
//! This module implements runtime discovery of other primals using:
//! - Multicast DNS (mDNS) for local network discovery
//! - DNS-SD for service discovery
//! - Manual registration for explicit endpoints
//!
//! NO hardcoded endpoints. NO assumptions about peer locations.

mod config;
mod state;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::self_identity::{CapabilityRequirement, DiscoveredService, SelfIdentity};
use crate::{ToadStoolError, ToadStoolResult};

pub use config::DiscoveryConfig;
pub use state::DiscoveryStats;

use state::DiscoveryState;

/// Runtime discovery engine
///
/// This engine discovers other primals at runtime without any hardcoded knowledge.
pub struct RuntimeDiscovery {
    /// Our self-identity
    identity: SelfIdentity,

    /// Discovered services
    services: Arc<RwLock<HashMap<Uuid, DiscoveredService>>>,

    /// Discovery configuration
    config: DiscoveryConfig,

    /// Discovery state
    state: Arc<RwLock<DiscoveryState>>,
}

impl RuntimeDiscovery {
    /// Create a new runtime discovery engine
    pub fn new(identity: SelfIdentity) -> Self {
        Self {
            identity,
            services: Arc::new(RwLock::new(HashMap::new())),
            config: DiscoveryConfig::default(),
            state: Arc::new(RwLock::new(DiscoveryState {
                running: false,
                last_discovery: None,
                stats: DiscoveryStats::default(),
            })),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(identity: SelfIdentity, config: DiscoveryConfig) -> Self {
        Self {
            identity,
            services: Arc::new(RwLock::new(HashMap::new())),
            config,
            state: Arc::new(RwLock::new(DiscoveryState {
                running: false,
                last_discovery: None,
                stats: DiscoveryStats::default(),
            })),
        }
    }

    /// Start discovery
    pub async fn start(&self) -> ToadStoolResult<()> {
        let mut state = self.state.write().await;
        if state.running {
            return Err(ToadStoolError::runtime("Discovery already running"));
        }

        state.running = true;
        info!("🔍 Runtime discovery started");

        // Start background discovery tasks
        self.spawn_discovery_tasks();

        Ok(())
    }

    /// Stop discovery
    pub async fn stop(&self) -> ToadStoolResult<()> {
        let mut state = self.state.write().await;
        if !state.running {
            return Ok(());
        }

        state.running = false;
        info!("🛑 Runtime discovery stopped");

        Ok(())
    }

    /// Find services by capability
    ///
    /// This is the key method: we find services by WHAT THEY CAN DO,
    /// not by WHO THEY ARE or WHERE THEY ARE.
    pub async fn find_by_capability(
        &self,
        capability: &str,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        let services = self.services.read().await;

        let matching: Vec<DiscoveredService> = services
            .values()
            .filter(|service| {
                service
                    .capabilities
                    .iter()
                    .any(|cap| cap.name == capability)
            })
            .cloned()
            .collect();

        debug!(
            "🔍 Found {} services with capability '{}'",
            matching.len(),
            capability
        );

        Ok(matching)
    }

    /// Find services matching a requirement
    pub async fn find_by_requirement(
        &self,
        requirement: &CapabilityRequirement,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        let services = self.services.read().await;

        let matching: Vec<DiscoveredService> = services
            .values()
            .filter(|service| self.identity.matches_requirement(requirement, service))
            .cloned()
            .collect();

        debug!("🔍 Found {} services matching requirement", matching.len());

        Ok(matching)
    }

    /// Get all discovered services
    pub async fn get_all_services(&self) -> Vec<DiscoveredService> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    /// Get discovery statistics
    pub async fn get_stats(&self) -> DiscoveryStats {
        let state = self.state.read().await;
        DiscoveryStats {
            total_discovered: state.stats.total_discovered,
            active_services: state.stats.active_services,
            timeouts: state.stats.timeouts,
        }
    }

    /// Manually register a service (for explicit endpoints)
    ///
    /// This allows for manual registration while still maintaining
    /// the capability-based discovery model.
    pub async fn register_service(&self, service: DiscoveredService) -> ToadStoolResult<()> {
        let mut services = self.services.write().await;

        // Check max services limit
        if services.len() >= self.config.max_services {
            return Err(ToadStoolError::runtime(format!(
                "Maximum services limit reached: {}",
                self.config.max_services
            )));
        }

        info!(
            "📝 Manually registered service: {} ({})",
            service.primal_type, service.instance_id
        );
        services.insert(service.instance_id, service);

        let mut state = self.state.write().await;
        state.stats.total_discovered += 1;
        state.stats.active_services = services.len();

        Ok(())
    }

    /// Remove a service
    pub async fn remove_service(&self, instance_id: &Uuid) -> ToadStoolResult<()> {
        let mut services = self.services.write().await;

        if services.remove(instance_id).is_some() {
            info!("🗑️ Removed service: {}", instance_id);

            let mut state = self.state.write().await;
            state.stats.active_services = services.len();
        }

        Ok(())
    }

    /// Spawn background discovery tasks
    fn spawn_discovery_tasks(&self) {
        // NOTE: Full mDNS/DNS-SD discovery is implemented in discovery::mdns module
        // This is a lightweight runtime registry for locally registered runtimes
        // Use MdnsDiscoveryService for full distributed discovery

        let services = Arc::clone(&self.services);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.discovery_interval);

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let state_read = state.read().await;
                    if !state_read.running {
                        break;
                    }
                }

                // Clean up stale services
                Self::cleanup_stale_internal(&services, &state, &config).await;
            }
        });
    }

    /// Internal cleanup helper
    async fn cleanup_stale_internal(
        services: &Arc<RwLock<HashMap<Uuid, DiscoveredService>>>,
        state: &Arc<RwLock<DiscoveryState>>,
        config: &DiscoveryConfig,
    ) {
        let mut services_write = services.write().await;
        let now = std::time::SystemTime::now();
        let timeout = config.service_timeout;

        let stale_ids: Vec<Uuid> = services_write
            .iter()
            .filter(|(_, service)| {
                now.duration_since(service.last_seen)
                    .map(|elapsed| elapsed > timeout)
                    .unwrap_or(true)
            })
            .map(|(id, _)| *id)
            .collect();

        if !stale_ids.is_empty() {
            for id in &stale_ids {
                services_write.remove(id);
            }

            let mut state_write = state.write().await;
            let stale_count = stale_ids.len();
            #[allow(clippy::cast_possible_truncation)]
            let timeout_delta = stale_count as u64;
            state_write.stats.timeouts += timeout_delta;
            state_write.stats.active_services = services_write.len();
        }
    }
}

#[cfg(test)]
mod tests;
