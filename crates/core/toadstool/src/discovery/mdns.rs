//! # mDNS Discovery Service
//!
//! Implements automatic service discovery using mDNS/DNS-SD (Multicast DNS).
//!
//! ## Evolution (Feb 15, 2026) - Songbird Delegation
//!
//! ToadStool **exposes** mDNS capability to Songbird (comms primal).
//! This follows the ecoPrimal separation of concerns:
//!
//! - **ToadStool**: Owns hardware discovery (GPU, NPU, CPU), exposes mDNS capability
//! - **Songbird**: Owns network discovery, coordinates service mesh
//!
//! ToadStool advertises its capabilities via mDNS. Songbird discovers
//! and coordinates primals across the network. ToadStool does NOT
//! implement vendor-specific service meshes (K8s, Consul, etc.) -
//! that's Songbird's domain.
//!
//! ## Key Concepts
//!
//! - **Service Type**: `_toadstool._tcp.local.`
//! - **Advertise by Capability**: Services advertise WHAT they can do
//! - **Discover by Capability**: Find services by WHAT you need
//! - **No Hardcoding**: Zero hardcoded addresses
//! - **Songbird Integration**: mDNS exposed for comms primal
//!
//! ## Example
//!
//! ```rust,no_run
//! use toadstool::discovery::MdnsDiscoveryService;
//! use toadstool::self_identity::SelfIdentity;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let identity = SelfIdentity::new()
//!     .with_network("toadstool-01".to_string(), Some(8084), vec!["http".to_string()]);
//!
//! let mdns = MdnsDiscoveryService::new()?;
//! mdns.advertise(&identity)?;
//!
//! // Discover services by capability
//! let storage_services = mdns.discover_by_capability("storage", std::time::Duration::from_secs(5)).await?;
//! # Ok(())
//! # }
//! ```

use super::{DiscoveredService, DiscoveryConfig};
use crate::error::{ToadStoolError, ToadStoolResult};
use crate::self_identity::{Capability, SelfIdentity};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// mDNS service type for Toadstool ecosystem
pub const TOADSTOOL_SERVICE_TYPE: &str = "_toadstool._tcp.local.";

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
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create mDNS daemon: {}", e)))?;

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
        .map_err(|e| ToadStoolError::runtime(format!("Failed to create service info: {}", e)))?;

        self.daemon.register(service_info).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to register mDNS service: {}", e))
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
        let receiver = self.daemon.browse(&self.service_type).map_err(|e| {
            ToadStoolError::runtime(format!("Failed to browse mDNS services: {}", e))
        })?;

        let mut discovered = Vec::new();
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    if let Ok(service) = Self::parse_service_info(&info) {
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

    /// Parse ServiceInfo into DiscoveredService
    fn parse_service_info(info: &ServiceInfo) -> ToadStoolResult<DiscoveredService> {
        parse_service_info_impl(info)
    }
}

/// Parse ServiceInfo into DiscoveredService (extracted for testability)
fn parse_service_info_impl(info: &ServiceInfo) -> ToadStoolResult<DiscoveredService> {
    // Extract instance ID using mdns-sd 0.10 API
    let instance_id = info
        .get_property_val_str("instance_id")
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| ToadStoolError::runtime("Missing or invalid instance_id"))?;

    // Extract primal type
    let primal_type = info
        .get_property_val_str("primal_type")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract version
    let version = info
        .get_property_val_str("version")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract capabilities
    let mut capabilities = Vec::new();
    let mut processed_caps = std::collections::HashSet::new();

    // Iterate through all properties to find capabilities
    for prop in info.get_properties().iter() {
        let key = prop.key();
        if let Some(cap_name) = key.strip_prefix("cap_") {
            if !cap_name.ends_with("_features") && processed_caps.insert(cap_name.to_string()) {
                let cap_version = info
                    .get_property_val_str(key)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let features_key = format!("cap_{}_features", cap_name);
                let features = info
                    .get_property_val_str(&features_key)
                    .map(|f| f.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default();

                capabilities.push(Capability {
                    name: cap_name.to_string(),
                    version: cap_version,
                    features,
                    characteristics: HashMap::new(),
                });
            }
        }
    }

    // Build endpoint
    let addresses = info.get_addresses();
    let endpoint = if let Some(addr) = addresses.iter().next() {
        format!("{}:{}", addr, info.get_port())
    } else {
        format!("{}:{}", info.get_hostname(), info.get_port())
    };

    // Build metadata map from properties
    let mut metadata = HashMap::new();
    for prop in info.get_properties().iter() {
        let key = prop.key();
        if let Some(value) = info.get_property_val_str(key) {
            metadata.insert(key.to_string(), value.to_string());
        }
    }

    let now = chrono::Utc::now();

    Ok(DiscoveredService {
        instance_id,
        primal_type,
        version,
        capabilities,
        endpoint,
        protocols: vec!["http".to_string()],
        discovered_at: now,
        last_seen: now,
        metadata,
    })
}

impl MdnsDiscoveryService {
    /// Shutdown the mDNS service
    pub fn shutdown(self) -> ToadStoolResult<()> {
        // mdns-sd 0.10 shutdown() returns Receiver, we just drop it
        let _receiver = self.daemon.shutdown().map_err(|e| {
            ToadStoolError::runtime(format!("Failed to shutdown mDNS daemon: {}", e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_identity::SelfIdentity;
    use mdns_sd::ServiceInfo;
    use std::collections::HashMap;

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
            let msg = format!("{}", e);
            assert!(msg.contains("Network identity") || msg.contains("mDNS"));
        }
    }

    #[test]
    fn test_parse_service_info_full_properties() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("primal_type".to_string(), "songbird".to_string());
        properties.insert("version".to_string(), "2.0.0".to_string());
        properties.insert("cap_storage".to_string(), "1.0".to_string());
        properties.insert(
            "cap_storage_features".to_string(),
            "object,metadata".to_string(),
        );
        properties.insert("cap_compute".to_string(), "1.2".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test-instance",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info_impl(&info).expect("parse should succeed");

        assert_eq!(service.instance_id, instance_id);
        assert_eq!(service.primal_type, "songbird");
        assert_eq!(service.version, "2.0.0");
        assert_eq!(service.endpoint, "127.0.0.1:8080");
        assert_eq!(service.protocols, vec!["http"]);
        assert!(service.has_capability("storage"));
        assert!(service.has_capability("compute"));
        assert_eq!(service.capability_version("storage"), Some("1.0"));
        assert!(service.has_capability_features("storage", &["object".to_string()]));
        assert!(service
            .has_capability_features("storage", &["object".to_string(), "metadata".to_string()]));
    }

    #[test]
    fn test_parse_service_info_missing_instance_id() {
        let mut properties = HashMap::new();
        properties.insert("primal_type".to_string(), "test".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let result = parse_service_info_impl(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_service_info_invalid_instance_id() {
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), "not-a-valid-uuid".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let result = parse_service_info_impl(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_service_info_defaults_for_missing() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.example",
            "127.0.0.1",
            9000u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info_impl(&info).expect("parse should succeed");

        assert_eq!(service.primal_type, "unknown");
        assert_eq!(service.version, "unknown");
        assert_eq!(service.endpoint, "127.0.0.1:9000");
        assert!(service.capabilities.is_empty());
    }

    #[test]
    fn test_parse_service_info_endpoint_fallback_to_hostname() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "myhost.local",
            "",
            7070u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info_impl(&info).expect("parse should succeed");

        let addresses = info.get_addresses();
        if addresses.is_empty() {
            assert_eq!(service.endpoint, "myhost.local:7070");
        } else {
            assert!(service.endpoint.contains(":7070"));
        }
    }

    #[test]
    fn test_parse_service_info_metadata_populated() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("custom_key".to_string(), "custom_value".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties.clone()),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info_impl(&info).expect("parse should succeed");

        assert!(service.metadata.contains_key("instance_id"));
        assert!(service.metadata.contains_key("custom_key"));
        assert_eq!(
            service.metadata.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_parse_service_info_skips_features_key_as_capability() {
        let instance_id = Uuid::new_v4();
        let mut properties = HashMap::new();
        properties.insert("instance_id".to_string(), instance_id.to_string());
        properties.insert("cap_storage".to_string(), "1.0".to_string());
        properties.insert("cap_storage_features".to_string(), "a,b".to_string());

        let info = ServiceInfo::new(
            TOADSTOOL_SERVICE_TYPE,
            "test",
            "host.local",
            "127.0.0.1",
            8080u16,
            Some(properties),
        )
        .expect("ServiceInfo creation");

        let service = parse_service_info_impl(&info).expect("parse should succeed");

        assert!(!service
            .capabilities
            .iter()
            .any(|c| c.name == "storage_features"));
        assert!(service.has_capability("storage"));
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
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
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
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
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
        let mdns = match MdnsDiscoveryService::new() {
            Ok(m) => m,
            Err(_) => {
                eprintln!("Skipping test - mDNS not available");
                return;
            }
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
}
