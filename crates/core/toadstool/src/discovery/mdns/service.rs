// SPDX-License-Identifier: AGPL-3.0-or-later
//! mDNS-based discovery service.
//!
//! Advertises and discovers ToadStool primals via mDNS/DNS-SD.

use super::constants::TOADSTOOL_SERVICE_TYPE;
use super::parse;
use crate::discovery::{DiscoveredService, DiscoveryConfig};
use crate::error::{ToadStoolError, ToadStoolResult};
use crate::self_identity::SelfIdentity;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// mDNS-based discovery service
pub struct MdnsDiscoveryService {
    /// mDNS daemon
    daemon: ServiceDaemon,
    /// Service type
    service_type: String,
    /// Discovered services cache
    services: Arc<RwLock<HashMap<Uuid, DiscoveredService>>>,
    /// Configuration
    #[allow(dead_code)] // Retained for reconfiguration (timeout/interval changes)
    config: DiscoveryConfig,
}

impl MdnsDiscoveryService {
    /// Create a new mDNS discovery service
    pub fn new() -> ToadStoolResult<Self> {
        Self::with_config(DiscoveryConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: DiscoveryConfig) -> ToadStoolResult<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create mDNS daemon: {e}")))?;

        Ok(Self {
            daemon,
            service_type: TOADSTOOL_SERVICE_TYPE.to_string(),
            services: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Advertise our capabilities via mDNS
    ///
    /// This broadcasts WHAT we can do, not WHO we are
    pub fn advertise(&self, identity: &SelfIdentity) -> ToadStoolResult<()> {
        let network = identity
            .network
            .as_ref()
            .ok_or_else(|| ToadStoolError::configuration("Network identity required for mDNS"))?;

        let mut properties = HashMap::new();

        // Advertise capabilities (not identity!)
        for cap in &identity.capabilities {
            properties.insert(format!("cap_{}", cap.name), cap.version.clone());

            // Include features
            if !cap.features.is_empty() {
                properties.insert(format!("cap_{}_features", cap.name), cap.features.join(","));
            }
        }

        // Add metadata
        properties.insert("primal_type".to_string(), identity.primal_type.to_string());
        properties.insert("version".to_string(), identity.version.clone());
        properties.insert("instance_id".to_string(), identity.instance_id.to_string());

        let service_info = ServiceInfo::new(
            &self.service_type,
            &identity.instance_id.to_string(),
            &network.hostname,
            "",
            network.port.unwrap_or(0),
            Some(properties),
        )
        .map_err(|e| ToadStoolError::runtime(format!("Failed to create service info: {e}")))?;

        self.daemon.register(service_info).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to register mDNS service: {e}"))
        })?;

        info!(
            "🔊 Advertised capabilities via mDNS: {} on {}:{}",
            identity
                .capabilities
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            network.hostname,
            network.port.unwrap_or(0)
        );

        Ok(())
    }

    /// Discover all services
    pub async fn discover_all(&self, timeout: Duration) -> ToadStoolResult<Vec<DiscoveredService>> {
        let receiver = self
            .daemon
            .browse(&self.service_type)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to browse mDNS services: {e}")))?;

        let mut discovered = Vec::new();
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    if let Ok(service) = parse::parse_service_info(&info) {
                        debug!(
                            "🔍 Discovered service: {} ({})",
                            service.primal_type, service.instance_id
                        );

                        // Update cache
                        let mut services = self.services.write().await;
                        services.insert(service.instance_id, service.clone());

                        discovered.push(service);
                    }
                }
                Ok(ServiceEvent::ServiceRemoved(_, full_name)) => {
                    debug!("👋 Service removed: {}", full_name);
                }
                Ok(_) => {}      // Ignore other events
                Err(_) => break, // Timeout
            }
        }

        Ok(discovered)
    }

    /// Discover services by capability
    ///
    /// This is the key method: find by WHAT services can do
    pub async fn discover_by_capability(
        &self,
        capability: &str,
        timeout: Duration,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        let all_services = self.discover_all(timeout).await?;

        let matching: Vec<DiscoveredService> = all_services
            .into_iter()
            .filter(|service| service.has_capability(capability))
            .collect();

        info!(
            "🎯 Found {} services with capability '{}'",
            matching.len(),
            capability
        );

        Ok(matching)
    }

    /// Get cached services
    pub async fn get_cached_services(&self) -> Vec<DiscoveredService> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }
}

impl MdnsDiscoveryService {
    /// Shutdown the mDNS service
    pub fn shutdown(self) -> ToadStoolResult<()> {
        // mdns-sd 0.10 shutdown() returns Receiver, we just drop it
        let _receiver = self
            .daemon
            .shutdown()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to shutdown mDNS daemon: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_identity::Capability;
    use std::collections::HashMap;
    use std::time::SystemTime;

    #[test]
    fn test_toadstool_service_type_constant() {
        assert_eq!(TOADSTOOL_SERVICE_TYPE, "_toadstool._tcp.local.");
        assert!(TOADSTOOL_SERVICE_TYPE.ends_with(".local."));
    }

    #[test]
    fn test_mdns_with_config_stores_config() {
        let config = DiscoveryConfig {
            discovery_interval: Duration::from_secs(60),
            service_timeout: Duration::from_secs(600),
            max_services: 50,
            ..DiscoveryConfig::default()
        };

        let result = MdnsDiscoveryService::with_config(config.clone());
        if let Ok(mdns) = result {
            assert_eq!(mdns.config.discovery_interval, Duration::from_secs(60));
            assert_eq!(mdns.config.service_timeout, Duration::from_secs(600));
            assert_eq!(mdns.config.max_services, 50);
        }
    }

    #[test]
    fn test_advertise_requires_network_identity() {
        let result = MdnsDiscoveryService::new();
        let mdns = match result {
            Ok(m) => m,
            Err(_) => return,
        };

        let identity = SelfIdentity::new();
        assert!(identity.network.is_none());

        let result = mdns.advertise(&identity);
        assert!(result.is_err());
        if let Err(e) = result {
            let msg = format!("{e}");
            assert!(msg.contains("Network identity") || msg.contains("mDNS"));
        }
    }

    #[test]
    fn test_discovered_service_discover_by_capability_filter_logic() {
        let storage_service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "storage".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:9000".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let compute_service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "compute".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability {
                name: "compute".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:9001".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let all = vec![storage_service.clone(), compute_service];
        let matching: Vec<_> = all
            .into_iter()
            .filter(|s| s.has_capability("storage"))
            .collect();

        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].instance_id, storage_service.instance_id);
    }

    #[tokio::test]
    async fn test_get_cached_services_initially_empty() {
        let result = MdnsDiscoveryService::new();
        let mdns = match result {
            Ok(m) => m,
            Err(_) => return,
        };

        let cached = mdns.get_cached_services().await;
        assert!(cached.is_empty());
    }

    #[test]
    fn test_mdns_service_creation() {
        let result = MdnsDiscoveryService::new();
        if result.is_err() {
            eprintln!("mDNS not available in test environment");
        }
    }

    #[tokio::test]
    async fn test_mdns_advertise() {
        let mdns = if let Ok(m) = MdnsDiscoveryService::new() {
            m
        } else {
            eprintln!("Skipping test - mDNS not available");
            return;
        };

        let identity = SelfIdentity::new().with_network(
            "test-host".to_string(),
            Some(8084),
            vec!["http".to_string()],
        );

        let result = mdns.advertise(&identity);
        if result.is_err() {
            eprintln!("mDNS advertise not available in test environment");
        }
    }

    #[test]
    fn test_discovered_service_has_capability_filter_no_match() {
        let storage_service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "storage".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:9000".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let all = vec![storage_service];
        let matching: Vec<_> = all
            .into_iter()
            .filter(|s| s.has_capability("compute"))
            .collect();

        assert!(matching.is_empty());
    }
}
