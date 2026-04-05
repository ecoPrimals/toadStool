// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for [`crate::ecosystem::EcosystemDiscoverer`] discovery paths and patterns.

use crate::ecosystem::{EcosystemDiscoverer, ServicePattern, ServiceType};

#[tokio::test]
async fn test_discover_local_services_smoke() {
    let discoverer = EcosystemDiscoverer::new();
    let result = discoverer.discover_local_services().await;
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn test_discover_local_services_invalid_env_endpoint_skips_insert() {
    temp_env::async_with_vars(
        [("DISCOVERY_ENDPOINT", Some("not-a-valid-url!!!"))],
        async {
            let discoverer = EcosystemDiscoverer::new();
            let result = discoverer.discover_local_services().await;
            assert!(result.is_ok());
            let map = result.expect("ok");
            assert!(!map.contains_key("discovery"));
        },
    )
    .await;
}

#[tokio::test]
async fn test_discover_local_services_unknown_capability_uses_empty_legacy_list() {
    let mut discoverer = EcosystemDiscoverer::new();
    discoverer.insert_service_pattern_for_test(
        "custom_capability_key".to_string(),
        ServicePattern {
            name: "custom".to_string(),
            description: "coverage".to_string(),
            default_ports: vec![59_999],
            health_endpoints: vec![],
            service_type: ServiceType::Unknown,
            required_capabilities: vec![],
        },
    );
    let result = discoverer.discover_local_services().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_discover_local_services_respects_toadstool_discovery_bind_addr() {
    temp_env::async_with_vars(
        [("TOADSTOOL_DISCOVERY_BIND_ADDR", Some("192.0.2.1"))],
        async {
            let discoverer = EcosystemDiscoverer::new();
            let result = discoverer.discover_local_services().await;
            assert!(result.is_ok());
        },
    )
    .await;
}

#[tokio::test]
async fn test_discover_wellknown_services_smoke() {
    let discoverer = EcosystemDiscoverer::new();
    let result = discoverer.discover_wellknown_services().await;
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn test_find_pattern_by_capability_security() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("security");
    assert!(pattern.is_some());
    assert_eq!(pattern.expect("p").name, "crypto");
}

#[test]
fn test_service_patterns_cover_all_service_types() {
    let discoverer = EcosystemDiscoverer::new();
    let types: Vec<_> = discoverer
        .service_patterns()
        .values()
        .map(|p| &p.service_type)
        .collect();
    assert!(
        types
            .iter()
            .any(|t| matches!(t, ServiceType::NetworkCoordination))
    );
    assert!(types.iter().any(|t| matches!(t, ServiceType::Security)));
    assert!(types.iter().any(|t| matches!(t, ServiceType::Storage)));
    assert!(types.iter().any(|t| matches!(t, ServiceType::AI)));
    assert!(
        types
            .iter()
            .any(|t| matches!(t, ServiceType::OperatingSystem))
    );
    assert!(types.iter().any(|t| matches!(t, ServiceType::Compute)));
}

#[test]
fn test_ecosystem_discoverer_creation() {
    let discoverer = EcosystemDiscoverer::new();
    assert_eq!(discoverer.service_patterns().len(), 6);
    assert!(discoverer.service_patterns().contains_key("discovery"));
    assert!(discoverer.service_patterns().contains_key("crypto"));
    assert!(discoverer.service_patterns().contains_key("storage"));
    assert!(discoverer.service_patterns().contains_key("compute"));
    assert!(discoverer.service_patterns().contains_key("orchestration"));
    assert!(discoverer.service_patterns().contains_key("self"));
}

#[test]
fn test_service_pattern_structure() {
    let discoverer = EcosystemDiscoverer::new();
    let discovery_pattern = discoverer.service_patterns().get("discovery").unwrap();

    assert_eq!(discovery_pattern.name, "discovery");
    assert!(!discovery_pattern.default_ports.is_empty());
    assert!(!discovery_pattern.health_endpoints.is_empty());
    assert!(matches!(
        discovery_pattern.service_type,
        ServiceType::NetworkCoordination
    ));
}

#[test]
fn test_find_pattern_by_capability() {
    let discoverer = EcosystemDiscoverer::new();
    let storage = discoverer.find_pattern_by_capability("storage");
    assert!(storage.is_some());
    assert_eq!(storage.unwrap().name, "storage");

    let network = discoverer.find_pattern_by_capability("network");
    assert!(network.is_some());

    let unknown = discoverer.find_pattern_by_capability("nonexistent_capability_xyz");
    assert!(unknown.is_none());
}

#[test]
fn test_find_pattern_by_capability_machine_learning() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("machine_learning");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "compute");
}

#[test]
fn test_find_pattern_by_capability_authentication() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("authentication");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "crypto");
}

#[test]
fn test_find_pattern_by_capability_os_management() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("os_management");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "orchestration");
}

#[test]
fn test_find_pattern_by_capability_compute() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("compute");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "toadstool");
}

#[test]
fn test_ecosystem_discoverer_clear_cache() {
    let mut discoverer = EcosystemDiscoverer::new();
    discoverer.clear_cache();
    assert!(discoverer.get_last_discovery().is_none());
}

#[test]
fn test_discover_mdns_services_returns_empty() {
    let services = EcosystemDiscoverer::discover_mdns_services();
    assert!(services.is_empty());
}

#[test]
fn test_find_pattern_by_capability_data_management() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("data_management");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "storage");
}

#[test]
fn test_find_pattern_by_capability_coordination() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("coordination");
    assert!(pattern.is_some());
}

#[test]
fn test_find_pattern_by_capability_environment() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("environment");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "orchestration");
}

#[test]
fn test_find_pattern_by_capability_universal_execution() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("universal_execution");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "toadstool");
}

#[tokio::test]
async fn test_discover_services_fast_mode() {
    let mut discoverer = EcosystemDiscoverer::new();
    let result = discoverer.discover_services().await;
    assert!(result.is_ok());
    let services = result.unwrap();
    assert_eq!(services.discovered_services.len(), 0);
    assert!(
        services
            .discovery_summary
            .discovery_methods_used
            .contains(&"fast_mode".to_string())
    );
}

#[tokio::test]
async fn test_discover_services_caches_result() {
    let mut discoverer = EcosystemDiscoverer::new();
    assert!(discoverer.get_last_discovery().is_none());
    let _ = discoverer.discover_services().await.unwrap();
    let cached = discoverer.get_last_discovery();
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().discovered_services.len(), 0);
}

#[test]
fn test_ecosystem_discoverer_default() {
    let discoverer = EcosystemDiscoverer::default();
    assert_eq!(discoverer.service_patterns().len(), 6);
}

#[test]
fn test_service_pattern_required_capabilities() {
    let discoverer = EcosystemDiscoverer::new();
    let discovery = discoverer.service_patterns().get("discovery").unwrap();
    assert!(
        discovery
            .required_capabilities
            .contains(&"network".to_string())
    );
    assert!(
        discovery
            .required_capabilities
            .contains(&"coordination".to_string())
    );

    let storage = discoverer.service_patterns().get("storage").unwrap();
    assert!(
        storage
            .required_capabilities
            .contains(&"storage".to_string())
    );
}

#[test]
fn test_service_pattern_default_ports() {
    let discoverer = EcosystemDiscoverer::new();
    for pattern in discoverer.service_patterns().values() {
        assert!(
            !pattern.default_ports.is_empty(),
            "{} has no ports",
            pattern.name
        );
    }
}

#[test]
fn test_service_pattern_health_endpoints() {
    let discoverer = EcosystemDiscoverer::new();
    let discovery = discoverer.service_patterns().get("discovery").unwrap();
    assert!(
        discovery
            .health_endpoints
            .iter()
            .any(|e| e.contains("health"))
    );
}
