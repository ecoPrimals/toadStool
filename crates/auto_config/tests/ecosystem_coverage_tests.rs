// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for ecosystem configuration, service discovery, capability mapping

#![allow(clippy::pedantic)]

use std::collections::HashMap;

use toadstool_auto_config::ecosystem::{DiscoverySummary, ServiceStatus, ServiceType};
use toadstool_auto_config::{DiscoveredServices, EcosystemDiscoverer, ServiceInfo};

#[test]
fn test_ecosystem_discoverer_has_all_capability_patterns() {
    let discoverer = EcosystemDiscoverer::new();
    let capabilities = [
        "network",
        "coordination",
        "security",
        "authentication",
        "storage",
        "data_management",
        "ai",
        "machine_learning",
        "os_management",
        "environment",
        "compute",
        "universal_execution",
    ];
    let mut found = std::collections::HashSet::new();
    for cap in &capabilities {
        if let Some(p) = discoverer.find_pattern_by_capability(cap) {
            found.insert(p.name.clone());
        }
    }
    assert!(
        found.len() >= 5,
        "Should find patterns for multiple capabilities"
    );
}

#[test]
fn test_ecosystem_capability_mapping_discovery() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("discovery");
    assert!(pattern.is_none()); // "discovery" is capability key, not required_capability
    let pattern = discoverer.find_pattern_by_capability("network");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "discovery");
}

#[test]
fn test_ecosystem_capability_mapping_crypto() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("security");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "crypto");
}

#[test]
fn test_ecosystem_capability_mapping_storage() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("storage");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "storage");
}

#[test]
fn test_ecosystem_capability_mapping_compute() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("ai");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "compute");
}

#[test]
fn test_ecosystem_capability_mapping_orchestration() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("os_management");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "orchestration");
}

#[test]
fn test_ecosystem_service_pattern_structure_via_find() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer
        .find_pattern_by_capability("storage")
        .expect("storage pattern");
    assert!(!pattern.name.is_empty());
    assert!(!pattern.description.is_empty());
    assert!(!pattern.default_ports.is_empty());
    assert!(!pattern.health_endpoints.is_empty());
    assert!(!pattern.required_capabilities.is_empty());
}

#[test]
fn test_ecosystem_discovery_summary_default() {
    let summary = DiscoverySummary::default();
    assert_eq!(summary.total_services_found, 0);
    assert!(summary.discovery_methods_used.is_empty());
    assert!(summary.services_by_type.is_empty());
    assert!(summary.discovery_errors.is_empty());
}

#[test]
fn test_ecosystem_service_info_construction() {
    let info = ServiceInfo {
        name: "test-svc".to_string(),
        endpoint: "http://127.0.0.1:9000".to_string(),
        service_type: "Compute".to_string(),
        version: "2.0.0".to_string(),
        capabilities: vec!["compute".to_string(), "gpu".to_string()],
        status: ServiceStatus::Healthy,
        discovered_via: "env".to_string(),
        response_time_ms: 5,
    };
    assert_eq!(info.name, "test-svc");
    assert_eq!(info.endpoint, "http://127.0.0.1:9000");
    assert_eq!(info.capabilities.len(), 2);
}

#[test]
fn test_ecosystem_service_status_variants() {
    let _ = ServiceStatus::Healthy;
    let _ = ServiceStatus::Degraded;
    let _ = ServiceStatus::Unhealthy;
    let _ = ServiceStatus::Unknown;
}

#[test]
fn test_ecosystem_service_type_all_variants() {
    let types = [
        ServiceType::NetworkCoordination,
        ServiceType::Security,
        ServiceType::Storage,
        ServiceType::AI,
        ServiceType::OperatingSystem,
        ServiceType::Compute,
        ServiceType::Unknown,
    ];
    for t in &types {
        let s = format!("{t:?}");
        assert!(!s.is_empty());
    }
}

#[test]
fn test_ecosystem_discovered_services_with_summary() {
    let mut services_by_type = HashMap::new();
    services_by_type.insert("compute".to_string(), 2);
    let services = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary {
            total_services_found: 2,
            discovery_methods_used: vec!["local".to_string(), "network".to_string()],
            services_by_type,
            discovery_errors: vec!["timeout".to_string()],
        },
        discovery_timestamp: std::time::SystemTime::now(),
    };
    assert_eq!(services.discovery_summary.total_services_found, 2);
    assert_eq!(services.discovery_summary.discovery_methods_used.len(), 2);
    assert_eq!(services.discovery_summary.discovery_errors.len(), 1);
}

#[tokio::test]
async fn test_ecosystem_discover_services_fast_mode_in_ci() {
    // cfg!(test) triggers fast mode - skips network I/O
    let mut discoverer = EcosystemDiscoverer::new();
    let result = discoverer.discover_services().await;
    assert!(result.is_ok());
    let services = result.unwrap();
    assert!(services
        .discovery_summary
        .discovery_methods_used
        .contains(&"fast_mode".to_string()));
}

#[tokio::test]
async fn test_ecosystem_get_last_discovery_after_discover() {
    let mut discoverer = EcosystemDiscoverer::new();
    assert!(discoverer.get_last_discovery().is_none());
    let _ = discoverer.discover_services().await.unwrap();
    let cached = discoverer.get_last_discovery();
    assert!(cached.is_some());
}
