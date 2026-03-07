// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Discovery Integration Tests
//!
//! Comprehensive tests for mDNS/DNS-SD discovery system and capability-based service discovery.
//!
//! ## Test Coverage
//!
//! - mDNS service creation and configuration
//! - Capability advertisement and discovery
//! - Service caching and expiration
//! - Multi-service discovery scenarios
//! - Error handling and edge cases
#![allow(clippy::single_match_else)]

use std::time::{Duration, SystemTime};
use toadstool::discovery::{DiscoveredService, MdnsDiscoveryService};
use toadstool::self_identity::{Capability, SelfIdentity};

#[tokio::test]
async fn test_mdns_service_lifecycle() {
    // Test creation
    let Ok(mdns) = MdnsDiscoveryService::new() else {
        eprintln!("⚠️  Skipping: mDNS not available in test environment");
        return;
    };

    // Test with identity that has network
    let identity = SelfIdentity::new().with_network(
        "test-node".to_string(),
        Some(8084),
        vec!["http".to_string()],
    );

    // Test advertisement
    let result = mdns.advertise(&identity);
    if result.is_err() {
        eprintln!("⚠️  mDNS advertise not available in test environment");
        return;
    }

    // Test shutdown
    let result = mdns.shutdown();
    assert!(result.is_ok(), "Shutdown should succeed");
}

#[tokio::test]
async fn test_capability_based_discovery() {
    let Ok(mdns) = MdnsDiscoveryService::new() else {
        eprintln!("⚠️  Skipping: mDNS not available");
        return;
    };

    // Create identity with network (capabilities are auto-detected)
    let identity = SelfIdentity::new().with_network(
        "multi-cap-node".to_string(),
        Some(9000),
        vec!["http".to_string()],
    );

    if mdns.advertise(&identity).is_err() {
        eprintln!("⚠️  mDNS advertise not available");
        return;
    }

    // Retry discovery until mDNS advertisement propagates (bounded loop instead of fixed sleep)
    let mut result = None;
    for _attempt in 0..5 {
        if let Ok(Ok(found)) = tokio::time::timeout(
            Duration::from_millis(100),
            mdns.discover_by_capability("storage", Duration::from_millis(50)),
        )
        .await
        {
            result = Some(Ok(found));
            break;
        }
    }
    let result = match result {
        Some(r) => r,
        None => {
            // Fallback: final attempt with full timeout if retries exhausted
            mdns.discover_by_capability("storage", Duration::from_secs(2))
                .await
        }
    };

    // Clean up
    let _ = mdns.shutdown();

    // Verify discovery attempt completed (may or may not find services in test env)
    assert!(result.is_ok(), "Discovery should complete without error");
}

#[tokio::test]
async fn test_discovered_service_has_capability() {
    // Create a mock discovered service
    let service = DiscoveredService {
        instance_id: uuid::Uuid::new_v4(),
        primal_type: "test".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![
            Capability {
                name: "compute".to_string(),
                version: "1.0.0".to_string(),
                features: vec!["gpu".to_string()],
                characteristics: std::collections::HashMap::new(),
            },
            Capability {
                name: "storage".to_string(),
                version: "1.0.0".to_string(),
                features: vec![],
                characteristics: std::collections::HashMap::new(),
            },
        ],
        endpoint: "localhost:8084".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        metadata: std::collections::HashMap::new(),
    };

    // Test capability checking
    assert!(service.has_capability("compute"));
    assert!(service.has_capability("storage"));
    assert!(!service.has_capability("networking"));
}

#[tokio::test]
async fn test_mdns_without_network_identity() {
    let Ok(mdns) = MdnsDiscoveryService::new() else {
        eprintln!("⚠️  Skipping: mDNS not available");
        return;
    };

    // Create identity without network (just default capabilities)
    let identity = SelfIdentity::new();

    // Should fail gracefully
    let result = mdns.advertise(&identity);
    assert!(result.is_err(), "Should require network identity");
    assert!(
        result.unwrap_err().to_string().contains("Network"),
        "Error should mention network"
    );

    let _ = mdns.shutdown();
}

#[tokio::test]
async fn test_discover_all_services() {
    let Ok(mdns) = MdnsDiscoveryService::new() else {
        eprintln!("⚠️  Skipping: mDNS not available");
        return;
    };

    // Discover all services (may find none in test environment)
    let result = mdns.discover_all(Duration::from_secs(1)).await;

    let _ = mdns.shutdown();

    assert!(result.is_ok(), "discover_all should complete");
    let services = result.unwrap();
    // In test environment, we might find 0 or more services
    eprintln!("  Discovered {} services", services.len());
}

#[tokio::test]
async fn test_cached_services() {
    let Ok(mdns) = MdnsDiscoveryService::new() else {
        eprintln!("⚠️  Skipping: mDNS not available");
        return;
    };

    // Get cached services (should be empty initially)
    let cached = mdns.get_cached_services().await;
    assert_eq!(cached.len(), 0, "Cache should start empty");

    // Try discovery to populate cache
    let _ = mdns.discover_all(Duration::from_millis(500)).await;

    // Check cache again
    let cached_after = mdns.get_cached_services().await;
    eprintln!("  Cached services after discovery: {}", cached_after.len());

    let _ = mdns.shutdown();
}

#[test]
fn test_self_identity_builder() {
    let identity = SelfIdentity::new().with_network(
        "test-host".to_string(),
        Some(8080),
        vec!["http".to_string()],
    );

    // Identity has auto-detected capabilities
    assert!(!identity.capabilities.is_empty());
    assert!(identity.network.is_some());

    let network = identity.network.as_ref().expect("Network should be set");
    assert_eq!(network.hostname, "test-host");
    assert_eq!(network.port, Some(8080));
}

#[test]
fn test_capability_features() {
    let mut cap = Capability {
        name: "compute".to_string(),
        version: "2.0.0".to_string(),
        features: vec!["gpu".to_string(), "cpu".to_string()],
        characteristics: std::collections::HashMap::new(),
    };

    assert_eq!(cap.features.len(), 2);
    assert!(cap.features.contains(&"gpu".to_string()));

    // Test adding characteristics
    cap.characteristics
        .insert("max_threads".to_string(), "64".to_string());
    assert_eq!(cap.characteristics.get("max_threads").unwrap(), "64");
}

#[test]
fn test_discovered_service_clone() {
    let service = DiscoveredService {
        instance_id: uuid::Uuid::new_v4(),
        primal_type: "test".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        endpoint: "localhost:8084".to_string(),
        protocols: vec!["http".to_string()],
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        metadata: std::collections::HashMap::new(),
    };

    // Test clone
    let cloned = service.clone();
    assert_eq!(cloned.instance_id, service.instance_id);
    assert_eq!(cloned.endpoint, service.endpoint);
}

#[tokio::test]
async fn test_capability_based_filtering() {
    // Create multiple mock services
    let services = vec![
        DiscoveredService {
            instance_id: uuid::Uuid::new_v4(),
            primal_type: "compute".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "compute".to_string(),
                version: "1.0.0".to_string(),
                features: vec![],
                characteristics: std::collections::HashMap::new(),
            }],
            endpoint: "node1:8084".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: std::collections::HashMap::new(),
        },
        DiscoveredService {
            instance_id: uuid::Uuid::new_v4(),
            primal_type: "storage".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability {
                name: "storage".to_string(),
                version: "1.0.0".to_string(),
                features: vec![],
                characteristics: std::collections::HashMap::new(),
            }],
            endpoint: "node2:8084".to_string(),
            protocols: vec!["http".to_string()],
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            metadata: std::collections::HashMap::new(),
        },
    ];

    // Filter by capability
    let compute_services: Vec<_> = services
        .iter()
        .filter(|s| s.has_capability("compute"))
        .collect();

    assert_eq!(compute_services.len(), 1);
    assert_eq!(compute_services[0].primal_type, "compute");

    let storage_services: Vec<_> = services
        .iter()
        .filter(|s| s.has_capability("storage"))
        .collect();

    assert_eq!(storage_services.len(), 1);
    assert_eq!(storage_services[0].primal_type, "storage");
}
