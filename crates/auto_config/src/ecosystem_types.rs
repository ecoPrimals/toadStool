// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Ecosystem discovery types
//!
//! Type definitions for service discovery, patterns, and discovery summaries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pattern for discovering a specific service type
#[derive(Debug, Clone)]
pub struct ServicePattern {
    /// Service name (e.g., "discovery", "crypto")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Default ports to probe
    pub default_ports: Vec<u16>,
    /// Health check endpoints
    pub health_endpoints: Vec<String>,
    /// Classification of the service
    pub service_type: ServiceType,
    /// Capabilities this service provides
    pub required_capabilities: Vec<String>,
}

/// Type of ecosystem service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    /// Network coordination
    NetworkCoordination,
    /// Security and crypto
    Security,
    /// Storage
    Storage,
    /// AI/ML compute
    AI,
    /// OS orchestration (e.g., BiomeOS)
    OperatingSystem,
    /// Universal compute (e.g., ToadStool)
    Compute,
    /// Unknown service type
    Unknown,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkCoordination => write!(f, "Network Coordination"),
            Self::Security => write!(f, "Security"),
            Self::Storage => write!(f, "Storage"),
            Self::AI => write!(f, "AI"),
            Self::OperatingSystem => write!(f, "Operating System"),
            Self::Compute => write!(f, "Compute"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Discovered services container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServices {
    /// Map of service identifier to service information
    pub discovered_services: HashMap<String, ServiceInfo>,
    /// Summary of the discovery process
    pub discovery_summary: DiscoverySummary,
    /// When the discovery was performed
    #[serde(with = "toadstool_common::system_time_serde")]
    pub discovery_timestamp: std::time::SystemTime,
}

/// Information about a discovered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name (e.g., "coordination", "security")
    pub name: String,
    /// Full endpoint URL
    pub endpoint: String,
    /// Type of service
    pub service_type: String,
    /// Service version
    pub version: String,
    /// Service capabilities
    pub capabilities: Vec<String>,
    /// Current service status
    pub status: ServiceStatus,
    /// How the service was discovered
    pub discovered_via: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is healthy and responding
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is not responding
    Unhealthy,
    /// Status could not be determined
    Unknown,
}

/// Summary of the discovery process
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoverySummary {
    /// Total number of services found
    pub total_services_found: usize,
    /// Discovery methods that were used
    pub discovery_methods_used: Vec<String>,
    /// Services found by type
    pub services_by_type: HashMap<String, usize>,
    /// Any errors encountered during discovery
    pub discovery_errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_local_coordination_endpoint() -> String {
        format!(
            "{}{}:{}",
            toadstool_common::constants::network::HTTP_PROTOCOL,
            toadstool_common::constants::network::DEFAULT_HOSTNAME,
            toadstool_config::ports::capability_fallback::COORDINATION,
        )
    }

    #[test]
    fn test_service_info_serialization() {
        let endpoint = sample_local_coordination_endpoint();
        let service_info = ServiceInfo {
            name: "test_service".to_string(),
            endpoint,
            service_type: "Test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["test".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "test".to_string(),
            response_time_ms: 100,
        };

        let json = serde_json::to_string(&service_info).unwrap();
        assert!(json.contains("test_service"));
        assert!(json.contains(":808") || json.contains("127.0.0.1"));
    }

    #[test]
    fn test_discovery_summary_default() {
        let summary = DiscoverySummary::default();
        assert_eq!(summary.total_services_found, 0);
        assert!(summary.discovery_methods_used.is_empty());
        assert!(summary.services_by_type.is_empty());
        assert!(summary.discovery_errors.is_empty());
    }

    #[test]
    fn test_service_status_variants() {
        let statuses = vec![
            ServiceStatus::Healthy,
            ServiceStatus::Degraded,
            ServiceStatus::Unhealthy,
            ServiceStatus::Unknown,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_service_type_display() {
        assert_eq!(
            ServiceType::NetworkCoordination.to_string(),
            "Network Coordination"
        );
        assert_eq!(ServiceType::Security.to_string(), "Security");
        assert_eq!(ServiceType::Storage.to_string(), "Storage");
        assert_eq!(ServiceType::AI.to_string(), "AI");
        assert_eq!(ServiceType::Compute.to_string(), "Compute");
    }

    #[test]
    fn test_service_type_operating_system_display() {
        assert_eq!(ServiceType::OperatingSystem.to_string(), "Operating System");
    }

    #[test]
    fn test_service_type_unknown_display() {
        assert_eq!(ServiceType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_discovered_services_has_timestamp() {
        let services = DiscoveredServices {
            discovered_services: std::collections::HashMap::new(),
            discovery_summary: DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };
        let _ = services.discovery_timestamp;
    }

    #[test]
    fn test_discovery_summary_serialization() {
        let mut summary = DiscoverySummary {
            total_services_found: 3,
            discovery_methods_used: vec!["local".to_string()],
            ..Default::default()
        };
        summary.services_by_type.insert("compute".to_string(), 1);
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["total_services_found"], 3);
    }

    #[test]
    fn test_service_info_deserialization() {
        let expected_endpoint = sample_local_coordination_endpoint();
        let json = serde_json::json!({
            "name": "test",
            "endpoint": expected_endpoint,
            "service_type": "Test",
            "version": "1.0",
            "capabilities": ["test"],
            "status": "Healthy",
            "discovered_via": "test",
            "response_time_ms": 10
        });
        let info: ServiceInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.name, "test");
        assert_eq!(info.endpoint, expected_endpoint);
        assert_eq!(info.response_time_ms, 10);
    }

    #[test]
    fn test_service_pattern_clone() {
        let pattern = ServicePattern {
            name: "test".to_string(),
            description: "desc".to_string(),
            default_ports: vec![8080],
            health_endpoints: vec!["/health".to_string()],
            service_type: ServiceType::Compute,
            required_capabilities: vec!["compute".to_string()],
        };
        let cloned = pattern.clone();
        assert_eq!(cloned.name, pattern.name);
        assert_eq!(cloned.default_ports, pattern.default_ports);
    }

    #[test]
    fn test_discovered_services_serialization() {
        let services = DiscoveredServices {
            discovered_services: HashMap::new(),
            discovery_summary: DiscoverySummary {
                total_services_found: 0,
                discovery_methods_used: vec!["fast_mode".to_string()],
                services_by_type: HashMap::new(),
                discovery_errors: Vec::new(),
            },
            discovery_timestamp: std::time::SystemTime::now(),
        };
        let json = serde_json::to_value(&services).unwrap();
        assert_eq!(json["discovery_summary"]["total_services_found"], 0);
    }
}
