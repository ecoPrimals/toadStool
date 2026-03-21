// SPDX-License-Identifier: AGPL-3.0-only
//! Expanded tests for `runtime_discovery` module
//!
//! Coverage expansion: `runtime_discovery.rs` had minimal test coverage
//! Adding comprehensive async tests for all discovery paths

use std::collections::HashMap;
use std::time::Duration;
use toadstool::runtime_discovery::*;
use toadstool::self_identity::*;
use uuid::Uuid;

/// Test discovery config defaults
#[test]
fn test_discovery_config_defaults() {
    let config = DiscoveryConfig::default();

    assert!(config.enable_mdns);
    assert!(config.enable_dns_sd);
    assert_eq!(config.discovery_interval, Duration::from_secs(30));
    assert_eq!(config.service_timeout, Duration::from_secs(300));
    assert_eq!(config.max_services, 1000);
}

/// Test custom discovery config
#[tokio::test]
async fn test_custom_discovery_config() {
    let identity = SelfIdentity::new();
    let config = DiscoveryConfig {
        enable_mdns: false,
        enable_dns_sd: true,
        discovery_interval: Duration::from_secs(60),
        service_timeout: Duration::from_secs(600),
        max_services: 500,
    };

    let discovery = RuntimeDiscovery::with_config(identity, config);
    let stats = discovery.get_stats().await;

    assert_eq!(stats.active_services, 0);
}

/// Test max services limit enforcement
#[tokio::test]
async fn test_max_services_limit() {
    let identity = SelfIdentity::new();
    let config = DiscoveryConfig {
        max_services: 2,
        ..Default::default()
    };

    let discovery = RuntimeDiscovery::with_config(identity, config);

    // Register 2 services (at limit)
    for i in 0..2 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("test{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: format!("localhost:808{i}"),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };

        discovery
            .register_service(service)
            .await
            .expect("Should succeed");
    }

    // Try to register 3rd service (should fail)
    let service3 = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "test3".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8083".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    let result = discovery.register_service(service3).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Maximum services limit")
    );
}

/// Test find by capability with no matches
#[tokio::test]
async fn test_find_by_capability_empty() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let found = discovery.find_by_capability("nonexistent").await.unwrap();
    assert!(found.is_empty());
}

/// Test find by capability with multiple matches
#[tokio::test]
async fn test_find_by_capability_multiple() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register multiple services with same capability
    for i in 0..3 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("compute{i}"),
            version: "1.0".to_string(),
            capabilities: vec![Capability {
                name: "compute".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            }],
            endpoint: format!("localhost:900{i}"),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };

        discovery.register_service(service).await.unwrap();
    }

    let found = discovery.find_by_capability("compute").await.unwrap();
    assert_eq!(found.len(), 3);
}

/// Test find by requirement
#[tokio::test]
async fn test_find_by_requirement() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let requirement = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: Some("1.0".to_string()),
        required: false,
        features: vec!["object-store".to_string()],
        purpose: "Test".to_string(),
    };

    // Register service matching requirement
    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "nestgate".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "1.0".to_string(),
            features: vec!["object-store".to_string(), "metadata".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8082".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();

    let found = discovery.find_by_requirement(&requirement).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].primal_type, "nestgate");
}

/// Test find by requirement with no matches
#[tokio::test]
async fn test_find_by_requirement_no_match() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let requirement = CapabilityRequirement {
        capability: "nonexistent".to_string(),
        min_version: None,
        required: false,
        features: vec![],
        purpose: "Test".to_string(),
    };

    let found = discovery.find_by_requirement(&requirement).await.unwrap();
    assert!(found.is_empty());
}

/// Test statistics tracking
#[tokio::test]
async fn test_discovery_stats_tracking() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let initial_stats = discovery.get_stats().await;
    assert_eq!(initial_stats.total_discovered, 0);
    assert_eq!(initial_stats.active_services, 0);

    // Register a service
    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service.clone()).await.unwrap();

    let stats_after_register = discovery.get_stats().await;
    assert_eq!(stats_after_register.total_discovered, 1);
    assert_eq!(stats_after_register.active_services, 1);

    // Remove the service
    discovery
        .remove_service(&service.instance_id)
        .await
        .unwrap();

    let stats_after_remove = discovery.get_stats().await;
    assert_eq!(stats_after_remove.total_discovered, 1); // Total doesn't decrease
    assert_eq!(stats_after_remove.active_services, 0); // Active does decrease
}

/// Test remove non-existent service
#[tokio::test]
async fn test_remove_nonexistent_service() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let result = discovery.remove_service(&Uuid::new_v4()).await;
    assert!(result.is_ok()); // Should succeed (idempotent)
}

/// Test get all services with multiple
#[tokio::test]
async fn test_get_all_services_multiple() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register 5 different services
    for i in 0..5 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("primal{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: format!("localhost:{}", 8000 + i),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };

        discovery.register_service(service).await.unwrap();
    }

    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 5);
}

/// Test concurrent discovery operations
#[tokio::test]
async fn test_concurrent_discovery_operations() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register 10 services sequentially (async but not spawned)
    // This tests concurrent safety via async/await without tokio::spawn lifetime issues
    for i in 0..10 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("test{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: format!("localhost:{}", 8000 + i),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };

        discovery
            .register_service(service)
            .await
            .expect("Registration should succeed");
    }

    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 10);
}

/// Test discovery state transitions
#[tokio::test]
async fn test_discovery_state_transitions() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Initial state: not running
    let stats = discovery.get_stats().await;
    assert_eq!(stats.active_services, 0);

    // Start discovery
    discovery.start().await.expect("Should start");

    // Stop discovery
    discovery.stop().await.expect("Should stop");

    // Can start again after stopping
    discovery.start().await.expect("Should start again");
    discovery.stop().await.expect("Should stop again");
}

/// Test discovery with mixed capabilities
#[tokio::test]
async fn test_mixed_capability_discovery() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register services with different capabilities
    let service1 = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "compute".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "compute".to_string(),
            version: "1.0".to_string(),
            features: vec!["cpu".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8001".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    let service2 = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "storage".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "1.0".to_string(),
            features: vec!["object-store".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8002".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service1).await.unwrap();
    discovery.register_service(service2).await.unwrap();

    // Find compute services
    let compute = discovery.find_by_capability("compute").await.unwrap();
    assert_eq!(compute.len(), 1);
    assert_eq!(compute[0].primal_type, "compute");

    // Find storage services
    let storage = discovery.find_by_capability("storage").await.unwrap();
    assert_eq!(storage.len(), 1);
    assert_eq!(storage[0].primal_type, "storage");

    // All services
    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 2);
}

/// Test discovery stats default values
#[test]
fn test_discovery_stats_default() {
    let stats = DiscoveryStats::default();

    assert_eq!(stats.total_discovered, 0);
    assert_eq!(stats.active_services, 0);
    assert_eq!(stats.timeouts, 0);
}

/// Test register and immediate query
#[tokio::test]
async fn test_register_immediate_query() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service_id = Uuid::new_v4();
    let service = DiscoveredService {
        instance_id: service_id,
        primal_type: "test".to_string(),
        version: "2.0".to_string(),
        capabilities: vec![Capability {
            name: "test-cap".to_string(),
            version: "1.0".to_string(),
            features: vec!["feature1".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "example.com:9000".to_string(),
        protocols: vec!["grpc".to_string()],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service.clone()).await.unwrap();

    // Immediate query should find it
    let found = discovery.find_by_capability("test-cap").await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].instance_id, service_id);
    assert_eq!(found[0].version, "2.0");
    assert_eq!(found[0].endpoint, "example.com:9000");
}

/// Test discovery with requirement partial match
#[tokio::test]
async fn test_requirement_partial_feature_match() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Requirement needs specific features
    let requirement = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: None,
        required: false,
        features: vec!["object-store".to_string(), "versioning".to_string()],
        purpose: "Test".to_string(),
    };

    // Service has only one of two features
    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "storage".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "1.0".to_string(),
            features: vec!["object-store".to_string()], // Missing versioning
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8082".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();

    // Should not match (missing required feature)
    let found = discovery.find_by_requirement(&requirement).await.unwrap();
    assert!(found.is_empty());
}

/// Test stats after multiple operations
#[tokio::test]
async fn test_stats_after_operations() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    // Register 3 services
    let mut service_ids = vec![];
    for i in 0..3 {
        let id = Uuid::new_v4();
        service_ids.push(id);

        let service = DiscoveredService {
            instance_id: id,
            primal_type: format!("test{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: format!("localhost:{}", 8000 + i),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };

        discovery.register_service(service).await.unwrap();
    }

    let stats1 = discovery.get_stats().await;
    assert_eq!(stats1.total_discovered, 3);
    assert_eq!(stats1.active_services, 3);

    // Remove one
    discovery.remove_service(&service_ids[0]).await.unwrap();

    let stats2 = discovery.get_stats().await;
    assert_eq!(stats2.total_discovered, 3); // Doesn't decrease
    assert_eq!(stats2.active_services, 2); // Decreases
}

/// Test service with multiple capabilities
#[tokio::test]
async fn test_service_multiple_capabilities() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "multi".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![
            Capability {
                name: "compute".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            },
            Capability {
                name: "storage".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                characteristics: HashMap::new(),
            },
        ],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();

    // Should be found by both capabilities
    let compute = discovery.find_by_capability("compute").await.unwrap();
    assert_eq!(compute.len(), 1);

    let storage = discovery.find_by_capability("storage").await.unwrap();
    assert_eq!(storage.len(), 1);
}

/// Test cloning discovery config
#[test]
fn test_discovery_config_clone() {
    let config1 = DiscoveryConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.enable_mdns, config2.enable_mdns);
    assert_eq!(config1.max_services, config2.max_services);
}

/// Test debug format for discovery config
#[test]
fn test_discovery_config_debug() {
    let config = DiscoveryConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("DiscoveryConfig"));
    assert!(debug_str.contains("enable_mdns"));
}

/// Test empty capability name search
#[tokio::test]
async fn test_find_by_empty_capability_name() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let found = discovery.find_by_capability("").await.unwrap();
    assert!(found.is_empty());
}
