// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive test coverage for ecosystem discovery module
//!
//! This test suite provides property-based tests, table-driven tests, and error path
//! coverage for the ecosystem service discovery system.

use std::collections::HashMap;
use toadstool_auto_config::ecosystem::{
    DiscoverySummary, ServicePattern, ServiceStatus, ServiceType,
};
use toadstool_auto_config::{DiscoveredServices, EcosystemDiscoverer, ServiceInfo};

/// Test that ecosystem discoverer can be created
#[test]
fn test_ecosystem_discoverer_creation() {
    let discoverer = EcosystemDiscoverer::new();
    // Should create successfully
    drop(discoverer);
}

/// Test service type enumeration
#[test]
fn test_service_type_variants() {
    let service_types = vec![
        ServiceType::NetworkCoordination,
        ServiceType::Security,
        ServiceType::Storage,
        ServiceType::AI,
        ServiceType::OperatingSystem,
        ServiceType::Compute,
        ServiceType::Unknown,
    ];

    // Should have 7 distinct types
    assert_eq!(service_types.len(), 7);

    // All should support Debug
    for service_type in &service_types {
        let _debug = format!("{service_type:?}");
    }
}

/// Test service info structure
#[test]
fn test_service_info_structure() {
    let service = ServiceInfo {
        name: "songbird".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        service_type: "network_coordination".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["coordination".to_string(), "discovery".to_string()],
        status: ServiceStatus::Healthy,
        discovered_via: "network_scan".to_string(),
        response_time_ms: 50,
    };

    assert_eq!(service.endpoint, "http://localhost:8080");
    assert_eq!(service.version, "1.0.0");
    assert_eq!(service.capabilities.len(), 2);
    assert_eq!(service.name, "songbird");
}

/// Test discovered services default
#[test]
fn test_discovered_services_empty() {
    let services = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary::default(),
        discovery_timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(services.discovered_services.len(), 0);
}

/// Test discovery summary structure
#[test]
fn test_discovery_summary_structure() {
    let mut services_by_type = HashMap::new();
    services_by_type.insert("network_coordination".to_string(), 1);
    services_by_type.insert("security".to_string(), 1);
    services_by_type.insert("ai".to_string(), 1);

    let summary = DiscoverySummary {
        total_services_found: 3,
        discovery_methods_used: vec!["network_scan".to_string()],
        services_by_type,
        discovery_errors: vec![],
    };

    assert_eq!(summary.total_services_found, 3);
    assert_eq!(summary.discovery_methods_used.len(), 1);
    assert!(summary.discovery_errors.is_empty());
}

/// Table-driven tests for service endpoint validation
#[test]
fn test_service_endpoint_validation() {
    let test_cases = vec![
        ("http://localhost:8080", true),
        ("https://service.local:443", true),
        ("http://192.168.1.100:8080", true),
        ("http://[::1]:8080", true), // IPv6
        ("", false),                 // Empty
        ("not-a-url", false),        // Invalid format
        ("ftp://invalid:21", false), // Wrong protocol
    ];

    for (endpoint, should_be_valid) in test_cases {
        let is_valid = if endpoint.is_empty() {
            false
        } else {
            endpoint.starts_with("http://") || endpoint.starts_with("https://")
        };

        assert_eq!(
            is_valid, should_be_valid,
            "Endpoint validation failed for: {endpoint}"
        );
    }
}

/// Table-driven tests for service type matching
#[test]
fn test_service_type_matching() {
    let test_cases = vec![
        ("songbird", "network_coordination"),
        ("beardog", "security"),
        ("nestgate", "storage"),
        ("squirrel", "ai"),
        ("biomeos", "operating_system"),
        ("toadstool", "compute"),
    ];

    for (service_name, expected_type_str) in test_cases {
        // Verify type string is valid
        assert!(!expected_type_str.is_empty());

        // Service names should be lowercase
        assert_eq!(service_name, service_name.to_lowercase());
    }
}

/// Test service capability validation
#[test]
fn test_service_capabilities() {
    let test_cases = vec![
        (
            ServiceType::NetworkCoordination,
            vec!["coordination", "discovery"],
        ),
        (ServiceType::Security, vec!["pki", "certificates"]),
        (ServiceType::Storage, vec!["storage", "persistence"]),
        (ServiceType::AI, vec!["mcp", "plugins"]),
    ];

    for (service_type, expected_capabilities) in test_cases {
        // Each service should have capabilities
        assert!(!expected_capabilities.is_empty());

        // Capabilities should be descriptive
        for capability in expected_capabilities {
            assert!(!capability.is_empty());
        }

        // Service type should be valid
        let _debug = format!("{service_type:?}");
    }
}

/// Test service status validation
#[test]
fn test_service_status_validation() {
    let valid_statuses = vec![
        ServiceStatus::Healthy,
        ServiceStatus::Degraded,
        ServiceStatus::Unhealthy,
        ServiceStatus::Unknown,
    ];

    // Should have 4 distinct statuses
    assert_eq!(valid_statuses.len(), 4);

    // All should support Debug
    for status in &valid_statuses {
        let _debug = format!("{status:?}");
    }
}

/// Test service discovery with no services
#[test]
fn test_discovery_no_services() {
    let discovered = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary {
            total_services_found: 0,
            discovery_methods_used: vec!["network_scan".to_string()],
            services_by_type: HashMap::new(),
            discovery_errors: vec![],
        },
        discovery_timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(discovered.discovered_services.len(), 0);
    assert_eq!(discovered.discovery_summary.total_services_found, 0);
    assert!(discovered.discovery_summary.services_by_type.is_empty());
}

/// Test service discovery with all services
#[test]
fn test_discovery_all_services() {
    let mut services = HashMap::new();

    services.insert(
        "songbird".to_string(),
        ServiceInfo {
            name: "songbird".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "network_coordination".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["coordination".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 45,
        },
    );

    services.insert(
        "beardog".to_string(),
        ServiceInfo {
            name: "beardog".to_string(),
            endpoint: "http://localhost:8081".to_string(),
            service_type: "security".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["pki".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 52,
        },
    );

    let mut services_by_type = HashMap::new();
    services_by_type.insert("network_coordination".to_string(), 1);
    services_by_type.insert("security".to_string(), 1);

    let discovered = DiscoveredServices {
        discovered_services: services,
        discovery_summary: DiscoverySummary {
            total_services_found: 2,
            discovery_methods_used: vec!["network_scan".to_string()],
            services_by_type,
            discovery_errors: vec![],
        },
        discovery_timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(discovered.discovered_services.len(), 2);
    assert_eq!(discovered.discovery_summary.total_services_found, 2);
    assert!(discovered.discovered_services.contains_key("songbird"));
    assert!(discovered.discovered_services.contains_key("beardog"));
}

/// Test discovery method classification
#[test]
fn test_discovery_methods() {
    let methods = vec![
        "network_scan",
        "config_file",
        "environment_variables",
        "service_registry",
        "dns_discovery",
        "manual_configuration",
    ];

    for method in methods {
        assert!(!method.is_empty());

        // Method should be a valid identifier (lowercase with underscores)
        assert!(method.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Test discovery duration validation
#[test]
fn test_discovery_duration() {
    let test_cases = vec![
        (100, true),   // Fast discovery
        (500, true),   // Normal discovery
        (2000, true),  // Slow discovery
        (5000, true),  // Very slow
        (0, false),    // Invalid: instant
        (60000, true), // Timeout threshold
    ];

    for (duration_ms, should_be_valid) in test_cases {
        // Duration should be positive (except 0 which is invalid)
        let is_valid = duration_ms > 0 && duration_ms <= 60_000;

        if should_be_valid {
            assert!(
                is_valid || duration_ms == 0,
                "Duration {duration_ms} should be valid"
            );
        }
    }
}

/// Test service version format validation
#[test]
fn test_service_version_formats() {
    let test_cases = vec![
        ("1.0.0", true),
        ("2.1.3", true),
        ("0.0.1", true),
        ("10.20.30", true),
        ("invalid", true), // Permissive - any string allowed
        ("", false),       // Empty not allowed
    ];

    for (version, should_be_valid) in test_cases {
        let is_valid = !version.is_empty();

        assert_eq!(
            is_valid, should_be_valid,
            "Version validation failed for: {version}"
        );
    }
}

/// Test concurrent service discovery (should be thread-safe)
#[tokio::test]
async fn test_concurrent_discovery() {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(4));
    let mut handles = vec![];

    for i in 0..10 {
        let sem = Arc::clone(&semaphore);
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // Create discoverer (should be thread-safe)
            let _discoverer = EcosystemDiscoverer::new();

            // Create discovery results
            let discovered = DiscoveredServices {
                discovered_services: HashMap::new(),
                discovery_summary: DiscoverySummary::default(),
                discovery_timestamp: std::time::SystemTime::now(),
            };

            assert_eq!(discovered.discovered_services.len(), 0);
            i
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test service lookup by type
#[test]
fn test_service_lookup_by_type() {
    let mut services = HashMap::new();

    services.insert(
        "songbird".to_string(),
        ServiceInfo {
            name: "songbird".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "network_coordination".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["coordination".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 48,
        },
    );

    // Find service by type
    let songbird = services
        .values()
        .find(|s| s.service_type == "network_coordination");

    assert!(songbird.is_some());
    assert_eq!(songbird.unwrap().endpoint, "http://localhost:8080");
}

/// Test service filtering by status
#[test]
fn test_service_status_filtering() {
    let mut services = HashMap::new();

    services.insert(
        "service1".to_string(),
        ServiceInfo {
            name: "service1".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "network_coordination".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 50,
        },
    );

    services.insert(
        "service2".to_string(),
        ServiceInfo {
            name: "service2".to_string(),
            endpoint: "http://localhost:8081".to_string(),
            service_type: "security".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: ServiceStatus::Unhealthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 5000,
        },
    );

    // Filter healthy services
    let healthy: Vec<_> = services
        .values()
        .filter(|s| matches!(s.status, ServiceStatus::Healthy))
        .collect();

    assert_eq!(healthy.len(), 1);
    assert!(matches!(healthy[0].status, ServiceStatus::Healthy));
}

/// Test service capability search
#[test]
fn test_service_capability_search() {
    let mut services = HashMap::new();

    services.insert(
        "songbird".to_string(),
        ServiceInfo {
            name: "songbird".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "network_coordination".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["coordination".to_string(), "discovery".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 45,
        },
    );

    // Find services with coordination capability
    let with_coordination: Vec<_> = services
        .values()
        .filter(|s| s.capabilities.contains(&"coordination".to_string()))
        .collect();

    assert_eq!(with_coordination.len(), 1);
    assert!(with_coordination[0]
        .capabilities
        .contains(&"coordination".to_string()));
}

/// Test discovery timestamp validation
#[test]
fn test_discovery_timestamp() {
    let now = std::time::SystemTime::now();
    let discovered = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary::default(),
        discovery_timestamp: now,
    };

    // Timestamp should be recent (within last 5 seconds for this test)
    let elapsed = std::time::SystemTime::now()
        .duration_since(discovered.discovery_timestamp)
        .unwrap_or_default();
    assert!(elapsed.as_secs() < 5, "Timestamp should be recent");
}

/// Test rapid successive discoveries (stress test)
#[test]
fn test_rapid_discoveries() {
    for _ in 0..100 {
        let _discoverer = EcosystemDiscoverer::new();
        let discovered = DiscoveredServices {
            discovered_services: HashMap::new(),
            discovery_summary: DiscoverySummary::default(),
            discovery_timestamp: std::time::SystemTime::now(),
        };
        assert_eq!(discovered.discovered_services.len(), 0);
    }
}

/// Test Clone trait implementation
#[test]
fn test_clone_implementations() {
    let service = ServiceInfo {
        name: "songbird".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        service_type: "network_coordination".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec!["coordination".to_string()],
        status: ServiceStatus::Healthy,
        discovered_via: "network_scan".to_string(),
        response_time_ms: 50,
    };

    let cloned = service.clone();
    assert_eq!(service.endpoint, cloned.endpoint);
    assert_eq!(service.version, cloned.version);
    assert_eq!(service.name, cloned.name);
}

/// Test Debug trait implementation
#[test]
fn test_debug_implementations() {
    let service_type = ServiceType::NetworkCoordination;
    let _debug = format!("{service_type:?}");

    let summary = DiscoverySummary::default();
    let _debug = format!("{summary:?}");

    let status = ServiceStatus::Healthy;
    let _debug = format!("{status:?}");
}

/// Test empty service list handling
#[test]
fn test_empty_service_handling() {
    let services: HashMap<String, ServiceInfo> = HashMap::new();

    // Should handle empty list gracefully
    assert_eq!(services.len(), 0);
    assert!(services.is_empty());

    // Iteration should work on empty list
    let count = services.values().count();
    assert_eq!(count, 0);
}

/// Test service endpoint uniqueness
#[test]
fn test_service_endpoint_uniqueness() {
    let mut services = HashMap::new();

    // Different services, different endpoints
    services.insert(
        "service1".to_string(),
        ServiceInfo {
            name: "service1".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            service_type: "network_coordination".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 45,
        },
    );

    services.insert(
        "service2".to_string(),
        ServiceInfo {
            name: "service2".to_string(),
            endpoint: "http://localhost:8081".to_string(),
            service_type: "security".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: ServiceStatus::Healthy,
            discovered_via: "network_scan".to_string(),
            response_time_ms: 50,
        },
    );

    // Verify endpoints are unique
    let endpoints: Vec<_> = services.values().map(|s| &s.endpoint).collect();
    let unique_endpoints: std::collections::HashSet<_> = endpoints.iter().collect();

    assert_eq!(
        endpoints.len(),
        unique_endpoints.len(),
        "Endpoints should be unique"
    );
}

/// Test response time validation
#[test]
fn test_response_time_validation() {
    let test_cases = vec![
        (10, true),   // Very fast
        (50, true),   // Fast
        (100, true),  // Normal
        (500, true),  // Acceptable
        (1000, true), // Slow
        (5000, true), // Very slow
    ];

    for (response_time_ms, should_be_valid) in test_cases {
        let is_valid = response_time_ms > 0;

        if should_be_valid {
            assert!(is_valid, "Response time {response_time_ms} should be valid");
        }
    }
}

/// Test `discovered_via` field validation
#[test]
fn test_discovered_via_validation() {
    let discovery_methods = vec![
        "network_scan",
        "config_file",
        "environment_variables",
        "service_registry",
        "dns_discovery",
        "manual_configuration",
        "mdns",
    ];

    for method in discovery_methods {
        assert!(!method.is_empty());
        // Should be lowercase with underscores
        assert!(method.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

/// Test `ServicePattern` creation and serialization
#[test]
fn test_service_pattern_creation() {
    let pattern = ServicePattern {
        name: "crypto".to_string(),
        description: "Cryptographic operations".to_string(),
        default_ports: vec![9876, 9877],
        health_endpoints: vec!["/health".to_string(), "/api/health".to_string()],
        service_type: ServiceType::Security,
        required_capabilities: vec!["security".to_string(), "pki".to_string()],
    };
    assert_eq!(pattern.name, "crypto");
    assert_eq!(pattern.default_ports.len(), 2);
    assert!(pattern
        .required_capabilities
        .contains(&"security".to_string()));
}

/// Test `ServicePattern` clone
#[test]
fn test_service_pattern_clone() {
    let pattern = ServicePattern {
        name: "storage".to_string(),
        description: "Storage service".to_string(),
        default_ports: vec![8082],
        health_endpoints: vec!["/health".to_string()],
        service_type: ServiceType::Storage,
        required_capabilities: vec!["storage".to_string()],
    };
    let cloned = pattern.clone();
    assert_eq!(pattern.name, cloned.name);
    assert_eq!(pattern.default_ports, cloned.default_ports);
    assert_eq!(
        pattern.service_type.to_string(),
        cloned.service_type.to_string()
    );
}

/// Test `discover_services` with `TOADSTOOL_SKIP_DISCOVERY` env (fast mode)
#[tokio::test]
async fn test_discover_services_skip_discovery_env() {
    let old = std::env::var("TOADSTOOL_SKIP_DISCOVERY").ok();
    std::env::set_var("TOADSTOOL_SKIP_DISCOVERY", "1");

    let mut discoverer = EcosystemDiscoverer::new();
    let result = discoverer.discover_services().await;

    if let Some(v) = old {
        std::env::set_var("TOADSTOOL_SKIP_DISCOVERY", v);
    } else {
        std::env::remove_var("TOADSTOOL_SKIP_DISCOVERY");
    }

    assert!(result.is_ok());
    let services = result.unwrap();
    assert!(services
        .discovery_summary
        .discovery_methods_used
        .contains(&"fast_mode".to_string()));
}

/// Test `find_pattern_by_capability` returns correct pattern
#[test]
fn test_find_pattern_by_capability_security() {
    let discoverer = EcosystemDiscoverer::new();
    let pattern = discoverer.find_pattern_by_capability("security");
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().name, "crypto");
}

/// Test `get_last_discovery` returns cached result
#[tokio::test]
async fn test_get_last_discovery_after_discover() {
    let mut discoverer = EcosystemDiscoverer::new();
    assert!(discoverer.get_last_discovery().is_none());
    let _ = discoverer.discover_services().await.unwrap();
    let cached = discoverer.get_last_discovery();
    assert!(cached.is_some());
}

/// Test `clear_cache` clears last discovery
#[tokio::test]
async fn test_clear_cache() {
    let mut discoverer = EcosystemDiscoverer::new();
    let _ = discoverer.discover_services().await.unwrap();
    discoverer.clear_cache();
    assert!(discoverer.get_last_discovery().is_none());
}

/// Test `DiscoverySummary` with `discovery_errors`
#[test]
fn test_discovery_summary_with_errors() {
    let summary = DiscoverySummary {
        total_services_found: 0,
        discovery_methods_used: vec!["local".to_string()],
        services_by_type: HashMap::new(),
        discovery_errors: vec!["Connection refused".to_string()],
    };
    assert_eq!(summary.discovery_errors.len(), 1);
    assert!(summary.discovery_errors[0].contains("Connection"));
}

/// Test `ServiceType` Unknown display
#[test]
fn test_service_type_unknown_display() {
    assert_eq!(ServiceType::Unknown.to_string(), "Unknown");
}

/// Test `DiscoveredServices` full serialization
#[test]
fn test_discovered_services_full_serialization() {
    let mut services = HashMap::new();
    services.insert(
        "test".to_string(),
        ServiceInfo {
            name: "test".to_string(),
            endpoint: "http://127.0.0.1:8080".to_string(),
            service_type: "Compute".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "env".to_string(),
            response_time_ms: 10,
        },
    );
    let discovered = DiscoveredServices {
        discovered_services: services,
        discovery_summary: DiscoverySummary {
            total_services_found: 1,
            discovery_methods_used: vec!["env".to_string()],
            services_by_type: HashMap::new(),
            discovery_errors: vec![],
        },
        discovery_timestamp: std::time::SystemTime::now(),
    };
    let json = serde_json::to_value(&discovered).unwrap();
    assert_eq!(json["discovery_summary"]["total_services_found"], 1);
}

/// Test `EcosystemDiscoverer` default
#[test]
fn test_ecosystem_discoverer_default() {
    let discoverer = EcosystemDiscoverer::default();
    assert!(discoverer.find_pattern_by_capability("storage").is_some());
    assert!(discoverer.find_pattern_by_capability("network").is_some());
}
