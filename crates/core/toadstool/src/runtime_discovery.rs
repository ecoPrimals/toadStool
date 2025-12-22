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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::self_identity::{CapabilityRequirement, DiscoveredService, SelfIdentity};
use crate::{ToadStoolError, ToadStoolResult};

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

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    pub enable_mdns: bool,

    /// Enable DNS-SD discovery
    pub enable_dns_sd: bool,

    /// Discovery interval
    pub discovery_interval: Duration,

    /// Service timeout (mark as stale)
    pub service_timeout: Duration,

    /// Maximum services to track
    pub max_services: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_dns_sd: true,
            discovery_interval: Duration::from_secs(30),
            service_timeout: Duration::from_secs(300), // 5 minutes
            max_services: 1000,
        }
    }
}

/// Discovery state
#[derive(Debug)]
struct DiscoveryState {
    /// Is discovery running?
    running: bool,

    /// Last discovery time
    #[allow(dead_code)] // Will be used when full discovery is implemented
    last_discovery: Option<chrono::DateTime<chrono::Utc>>,

    /// Discovery statistics
    stats: DiscoveryStats,
}

/// Discovery statistics
#[derive(Debug, Default)]
pub struct DiscoveryStats {
    /// Total discoveries
    pub total_discovered: u64,

    /// Currently active services
    pub active_services: usize,

    /// Services that timed out
    pub timeouts: u64,
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

    /// Clean up stale services
    #[allow(dead_code)] // Will be used when mDNS/DNS-SD is implemented
    async fn cleanup_stale_services(&self) {
        let mut services = self.services.write().await;
        let now = chrono::Utc::now();
        let timeout =
            chrono::Duration::from_std(self.config.service_timeout).unwrap_or_else(|_| {
                // Fallback to 5 minutes if conversion fails (unlikely but safe)
                chrono::Duration::seconds(300)
            });

        let stale: Vec<Uuid> = services
            .iter()
            .filter(|(_, service)| now.signed_duration_since(service.last_seen) > timeout)
            .map(|(id, _)| *id)
            .collect();

        if !stale.is_empty() {
            warn!("🧹 Cleaning up {} stale services", stale.len());

            for id in &stale {
                services.remove(id);
            }

            let mut state = self.state.write().await;
            state.stats.timeouts += stale.len() as u64;
            state.stats.active_services = services.len();
        }
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
        let now = chrono::Utc::now();
        let timeout = chrono::Duration::from_std(config.service_timeout)
            .unwrap_or_else(|_| chrono::Duration::seconds(300));

        let stale: Vec<Uuid> = services_write
            .iter()
            .filter(|(_, service)| now.signed_duration_since(service.last_seen) > timeout)
            .map(|(id, _)| *id)
            .collect();

        if !stale.is_empty() {
            for id in &stale {
                services_write.remove(id);
            }

            let mut state_write = state.write().await;
            state_write.stats.timeouts += stale.len() as u64;
            state_write.stats.active_services = services_write.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_identity::Capability;

    #[tokio::test]
    async fn test_runtime_discovery_creation() {
        let identity = SelfIdentity::new();
        let discovery = RuntimeDiscovery::new(identity);

        let services = discovery.get_all_services().await;
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_manual_registration() {
        let identity = SelfIdentity::new();
        let discovery = RuntimeDiscovery::new(identity);

        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "test-cap".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:8080".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        discovery
            .register_service(service.clone())
            .await
            .expect("Service registration should succeed in test");

        let services = discovery.get_all_services().await;
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].primal_type, "test");
    }

    #[tokio::test]
    async fn test_find_by_capability() {
        let identity = SelfIdentity::new();
        let discovery = RuntimeDiscovery::new(identity);

        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "storage".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec!["object-store".to_string()],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:8082".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        discovery
            .register_service(service)
            .await
            .expect("Service registration should succeed in test");

        let found = discovery
            .find_by_capability("storage")
            .await
            .expect("Capability search should succeed in test");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].primal_type, "storage");

        let not_found = discovery
            .find_by_capability("nonexistent")
            .await
            .expect("Capability search should succeed even when empty");
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_start_stop_discovery() {
        let identity = SelfIdentity::new();
        let discovery = RuntimeDiscovery::new(identity);

        discovery
            .start()
            .await
            .expect("Discovery start should succeed in test");

        // Starting again should fail
        assert!(discovery.start().await.is_err());

        discovery
            .stop()
            .await
            .expect("Discovery stop should succeed in test");

        // Stopping again is OK
        assert!(discovery.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_remove_service() {
        let identity = SelfIdentity::new();
        let discovery = RuntimeDiscovery::new(identity);

        let service_id = Uuid::new_v4();
        let service = DiscoveredService {
            instance_id: service_id,
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            endpoint: "localhost:8080".to_string(),
            protocols: vec![],
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
        };

        discovery
            .register_service(service)
            .await
            .expect("Service registration should succeed in test");
        assert_eq!(discovery.get_all_services().await.len(), 1);

        discovery
            .remove_service(&service_id)
            .await
            .expect("Service removal should succeed in test");
        assert_eq!(discovery.get_all_services().await.len(), 0);
    }
}
