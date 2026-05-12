// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Discovery Module
//!
//! Implements capability-based runtime discovery for primals.
//!
//! ## `WateringHole` Sovereignty Principle
//!
//! **Discover by CAPABILITY, not by hardcoded name.** Code should scan for what a
//! service CAN DO, not what it IS CALLED. Use `service.has_capability("crypto.encrypt")`
//! style checks, never `if name == "beardog"` pattern matching.
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

#[cfg(feature = "mdns")]
pub mod mdns;
pub mod orchestration;

#[cfg(feature = "mdns")]
pub use mdns::{MdnsDiscoveryService, TOADSTOOL_SERVICE_TYPE};
pub use orchestration::{OrchestrationClient, discover_orchestration};

use crate::error::{ToadStoolError, ToadStoolResult};
use crate::self_identity::Capability;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// A service discovered at runtime
#[derive(Debug, Clone)]
pub struct DiscoveredService {
    /// Unique instance ID
    pub instance_id: Uuid,
    /// Primal type (e.g., "coordination", "storage")
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
    pub discovered_at: SystemTime,
    /// Last seen (for timeout detection)
    pub last_seen: SystemTime,
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
        self.capabilities
            .iter()
            .find(|c| c.name == capability_name)
            .is_some_and(|cap| {
                required_features
                    .iter()
                    .all(|feat| cap.features.contains(feat))
            })
    }

    /// Parse socket address from endpoint
    ///
    /// # Errors
    ///
    /// Returns error if `endpoint` is not a valid socket address string.
    pub fn socket_addr(&self) -> ToadStoolResult<SocketAddr> {
        self.endpoint
            .parse()
            .map_err(|e| ToadStoolError::configuration(format!("Invalid endpoint: {e}")))
    }
}

/// Discovery method configuration
#[derive(Debug, Clone, Default)]
pub enum DiscoveryMethod {
    /// Use mDNS/DNS-SD for automatic discovery
    #[default]
    Mdns,
    /// Use explicit configuration
    Explicit(HashMap<String, String>),
    /// Hybrid: Try mDNS first, fallback to explicit
    Hybrid {
        /// Explicit endpoint overrides when mDNS fails.
        explicit_fallback: HashMap<String, String>,
        /// Timeout before falling back to explicit config.
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
    /// Enable `IPv6`
    pub enable_ipv6: bool,
}

const DEFAULT_DISCOVERY_INTERVAL_SECS: u64 = 30;
const DEFAULT_SERVICE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_SERVICES: usize = 100;

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            method: DiscoveryMethod::default(),
            discovery_interval: Duration::from_secs(DEFAULT_DISCOVERY_INTERVAL_SECS),
            service_timeout: Duration::from_secs(DEFAULT_SERVICE_TIMEOUT_SECS),
            max_services: DEFAULT_MAX_SERVICES,
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
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
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
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
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

    #[test]
    fn test_discovered_service_socket_addr_valid() {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            endpoint: "127.0.0.1:8080".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let addr = service.socket_addr().unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_discovered_service_socket_addr_invalid() {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: "test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            endpoint: "not-a-valid-address".to_string(),
            protocols: vec![],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: HashMap::new(),
        };

        assert!(service.socket_addr().is_err());
    }

    #[test]
    fn test_discovery_method_default() {
        let method = DiscoveryMethod::default();
        assert!(matches!(method, DiscoveryMethod::Mdns));
    }
}
