//! # Discovery Module
//!
//! Implements capability-based runtime discovery for primals.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge Only**: Each primal knows WHAT it can do (capabilities)
//! - **Runtime Discovery**: Primals discover EACH OTHER at runtime
//! - **Capability-Based**: Find by WHAT services can do, not WHO they are
//! - **Zero Hardcoding**: No hardcoded IPs, ports, or primal locations
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  SelfIdentity   │ ← Knows "I can do X, Y, Z"
//! └────────┬────────┘
//!          │
//!          ├──→ MdnsDiscovery ──→ Advertise capabilities
//!          │
//!          └──→ RuntimeDiscovery ──→ Find by capability
//! ```

pub mod mdns;
pub mod orchestration;

pub use mdns::{MdnsDiscoveryService, TOADSTOOL_SERVICE_TYPE};
pub use orchestration::{discover_orchestration, OrchestrationClient};

use crate::error::{ToadStoolError, ToadStoolResult};
use crate::self_identity::Capability;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;

/// A service discovered at runtime
#[derive(Debug, Clone)]
pub struct DiscoveredService {
    /// Unique instance ID
    pub instance_id: Uuid,
    /// Primal type (e.g., "songbird", "nestgate")
    pub primal_type: String,
    /// Version
    pub version: String,
    /// Capabilities this service provides
    pub capabilities: Vec<Capability>,
    /// Network endpoint
    pub endpoint: String,
    /// Supported protocols
    pub protocols: Vec<String>,
    /// When discovered
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    /// Last seen (for timeout detection)
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl DiscoveredService {
    /// Check if this service has a specific capability
    pub fn has_capability(&self, capability_name: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.name == capability_name)
    }

    /// Get capability version if present
    pub fn capability_version(&self, capability_name: &str) -> Option<&str> {
        self.capabilities
            .iter()
            .find(|cap| cap.name == capability_name)
            .map(|cap| cap.version.as_str())
    }

    /// Check if this service has all required features for a capability
    pub fn has_capability_features(
        &self,
        capability_name: &str,
        required_features: &[String],
    ) -> bool {
        if let Some(cap) = self.capabilities.iter().find(|c| c.name == capability_name) {
            required_features
                .iter()
                .all(|feat| cap.features.contains(feat))
        } else {
            false
        }
    }

    /// Parse socket address from endpoint
    pub fn socket_addr(&self) -> ToadStoolResult<SocketAddr> {
        self.endpoint
            .parse()
            .map_err(|e| ToadStoolError::configuration(format!("Invalid endpoint: {}", e)))
    }
}

/// Discovery method configuration
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum DiscoveryMethod {
    /// Use mDNS/DNS-SD for automatic discovery
    #[default]
    Mdns,
    /// Use explicit configuration
    Explicit(HashMap<String, String>),
    /// Hybrid: Try mDNS first, fallback to explicit
    Hybrid {
        explicit_fallback: HashMap<String, String>,
        mdns_timeout: Duration,
    },
}


/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery method
    pub method: DiscoveryMethod,
    /// Discovery interval for background refresh
    pub discovery_interval: Duration,
    /// Service timeout (mark as stale)
    pub service_timeout: Duration,
    /// Maximum services to track
    pub max_services: usize,
    /// Enable IPv6
    pub enable_ipv6: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            method: DiscoveryMethod::default(),
            discovery_interval: Duration::from_secs(30),
            service_timeout: Duration::from_secs(300), // 5 minutes
            max_services: 100,
            enable_ipv6: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_service_has_capability() {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec!["object-store".to_string()],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:8080".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        assert!(service.has_capability("storage"));
        assert!(!service.has_capability("compute"));
        assert_eq!(service.capability_version("storage"), Some("1.0"));
    }

    #[test]
    fn test_discovered_service_has_capability_features() {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec!["object-store".to_string(), "metadata".to_string()],
                characteristics: HashMap::new(),
            }],
            endpoint: "localhost:8080".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        assert!(service.has_capability_features("storage", &["object-store".to_string()]));
        assert!(service.has_capability_features(
            "storage",
            &["object-store".to_string(), "metadata".to_string()]
        ));
        assert!(!service.has_capability_features("storage", &["missing-feature".to_string()]));
    }

    #[test]
    fn test_discovery_config_defaults() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.discovery_interval, Duration::from_secs(30));
        assert_eq!(config.service_timeout, Duration::from_secs(300));
        assert_eq!(config.max_services, 100);
        assert!(config.enable_ipv6);
    }
}
