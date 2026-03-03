// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for runtime discovery module

use std::collections::HashMap;
use std::time::Duration;

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
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
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
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
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
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
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

#[tokio::test]
async fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();
    assert!(config.enable_mdns);
    assert!(config.enable_dns_sd);
    assert_eq!(config.discovery_interval, Duration::from_secs(30));
    assert_eq!(config.service_timeout, Duration::from_secs(300));
    assert_eq!(config.max_services, 1000);
}

#[tokio::test]
async fn test_with_config() {
    let identity = SelfIdentity::new();
    let config = DiscoveryConfig {
        enable_mdns: false,
        enable_dns_sd: false,
        discovery_interval: Duration::from_secs(60),
        service_timeout: Duration::from_secs(120),
        max_services: 50,
    };
    let discovery = RuntimeDiscovery::with_config(identity, config);
    let services = discovery.get_all_services().await;
    assert!(services.is_empty());
}

#[tokio::test]
async fn test_register_service_max_limit() {
    let identity = SelfIdentity::new();
    let config = DiscoveryConfig {
        max_services: 2,
        ..DiscoveryConfig::default()
    };
    let discovery = RuntimeDiscovery::with_config(identity, config);

    for i in 0..2 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("svc{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: "localhost:8080".to_string(),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };
        discovery.register_service(service).await.unwrap();
    }

    let third = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "svc3".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    let result = discovery.register_service(third).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Maximum services limit"));
}

#[tokio::test]
async fn test_get_stats_after_registration() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "stats-test".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();
    let stats = discovery.get_stats().await;
    assert_eq!(stats.total_discovered, 1);
    assert_eq!(stats.active_services, 1);
    assert_eq!(stats.timeouts, 0);
}

#[tokio::test]
async fn test_remove_nonexistent_service_ok() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);
    let result = discovery.remove_service(&Uuid::new_v4()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_find_by_capability_multiple_matches() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let cap = Capability {
        name: "compute".to_string(),
        version: "1.0".to_string(),
        features: vec![],
        characteristics: HashMap::new(),
    };

    for i in 0..3 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("compute{i}"),
            version: "1.0".to_string(),
            capabilities: vec![cap.clone()],
            endpoint: format!("localhost:808{i}"),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };
        discovery.register_service(service).await.unwrap();
    }

    let found = discovery.find_by_capability("compute").await.unwrap();
    assert_eq!(found.len(), 3);
}

#[tokio::test]
async fn test_find_by_requirement() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "storage".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "1.0".to_string(),
            features: vec!["object".to_string()],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8082".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };
    discovery.register_service(service).await.unwrap();

    let req = CapabilityRequirement {
        capability: "storage".to_string(),
        min_version: Some("1.0".to_string()),
        required: true,
        features: vec![],
        purpose: "test".to_string(),
    };

    let found = discovery.find_by_requirement(&req).await.unwrap();
    assert!(!found.is_empty());
}

#[tokio::test]
async fn test_discovery_stats_default() {
    let stats = DiscoveryStats::default();
    assert_eq!(stats.total_discovered, 0);
    assert_eq!(stats.active_services, 0);
    assert_eq!(stats.timeouts, 0);
}

#[tokio::test]
async fn test_get_all_services_after_remove() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let id = Uuid::new_v4();
    let service = DiscoveredService {
        instance_id: id,
        primal_type: "ephemeral".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();
    assert_eq!(discovery.get_all_services().await.len(), 1);

    discovery.remove_service(&id).await.unwrap();
    assert_eq!(discovery.get_all_services().await.len(), 0);
}

#[tokio::test]
async fn test_start_then_stop_then_start_again() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    discovery.start().await.unwrap();
    discovery.stop().await.unwrap();
    let result = discovery.start().await;
    assert!(result.is_ok());
}

// ── Additional coverage: capability matching, health tracking, stats ──

#[tokio::test]
async fn test_find_by_requirement_no_match() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "storage".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability {
            name: "storage".to_string(),
            version: "0.5".to_string(),
            features: vec![],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8082".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };
    discovery.register_service(service).await.unwrap();

    // matches_requirement checks capability name; require "compute" which service lacks
    let req = CapabilityRequirement {
        capability: "compute".to_string(),
        min_version: None,
        required: true,
        features: vec![],
        purpose: "test".to_string(),
    };

    let found = discovery.find_by_requirement(&req).await.unwrap();
    assert!(found.is_empty());
}

#[tokio::test]
async fn test_find_by_capability_service_with_multiple_caps() {
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
        endpoint: "localhost:8090".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };
    discovery.register_service(service).await.unwrap();

    let compute = discovery.find_by_capability("compute").await.unwrap();
    let storage = discovery.find_by_capability("storage").await.unwrap();
    assert_eq!(compute.len(), 1);
    assert_eq!(storage.len(), 1);
    assert_eq!(compute[0].instance_id, storage[0].instance_id);
}

#[tokio::test]
async fn test_discovery_stats_after_remove() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let id = Uuid::new_v4();
    let service = DiscoveredService {
        instance_id: id,
        primal_type: "ephemeral".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8080".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };

    discovery.register_service(service).await.unwrap();
    let stats = discovery.get_stats().await;
    assert_eq!(stats.active_services, 1);
    assert_eq!(stats.total_discovered, 1);

    discovery.remove_service(&id).await.unwrap();
    let stats_after = discovery.get_stats().await;
    assert_eq!(stats_after.active_services, 0);
    assert_eq!(stats_after.total_discovered, 1);
}

#[tokio::test]
async fn test_discovery_config_custom_values() {
    let config = DiscoveryConfig {
        enable_mdns: false,
        enable_dns_sd: false,
        discovery_interval: Duration::from_secs(10),
        service_timeout: Duration::from_secs(60),
        max_services: 5,
    };
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::with_config(identity, config);
    let services = discovery.get_all_services().await;
    assert!(services.is_empty());
}

#[tokio::test]
async fn test_get_all_services_multiple() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    for i in 0..5 {
        let service = DiscoveredService {
            instance_id: Uuid::new_v4(),
            primal_type: format!("svc{i}"),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoint: format!("localhost:808{i}"),
            protocols: vec![],
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
        };
        discovery.register_service(service).await.unwrap();
    }

    let all = discovery.get_all_services().await;
    assert_eq!(all.len(), 5);
}

#[tokio::test]
async fn test_discovery_stats_default_values() {
    let stats = DiscoveryStats::default();
    assert_eq!(stats.total_discovered, 0);
    assert_eq!(stats.active_services, 0);
    assert_eq!(stats.timeouts, 0);
}

#[tokio::test]
async fn test_find_by_requirement_optional_version() {
    let identity = SelfIdentity::new();
    let discovery = RuntimeDiscovery::new(identity);

    let service = DiscoveredService {
        instance_id: Uuid::new_v4(),
        primal_type: "optional".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![Capability {
            name: "auth".to_string(),
            version: "1.0".to_string(),
            features: vec![],
            characteristics: HashMap::new(),
        }],
        endpoint: "localhost:8085".to_string(),
        protocols: vec![],
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };
    discovery.register_service(service).await.unwrap();

    let req = CapabilityRequirement {
        capability: "auth".to_string(),
        min_version: None,
        required: true,
        features: vec![],
        purpose: "test".to_string(),
    };
    let found = discovery.find_by_requirement(&req).await.unwrap();
    assert_eq!(found.len(), 1);
}
