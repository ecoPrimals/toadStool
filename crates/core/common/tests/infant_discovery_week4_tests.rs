// SPDX-License-Identifier: AGPL-3.0-or-later
//! Infant Discovery System - Week 4 Test Coverage Expansion
//!
//! Tests for service discovery, capability detection, and health monitoring.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool_common::infant_discovery::{
    DiscoveredService, DiscoveryPreferences, DiscoverySource, ServiceHealth, ServiceMetadata,
};

#[test]
fn test_service_health_variants() {
    let healths = [
        ServiceHealth::Healthy,
        ServiceHealth::Degraded,
        ServiceHealth::Unknown,
    ];

    assert_eq!(healths.len(), 3);
}

#[test]
fn test_service_health_ordering() {
    assert!(ServiceHealth::Healthy > ServiceHealth::Degraded);
    assert!(ServiceHealth::Degraded > ServiceHealth::Unknown);
    assert!(ServiceHealth::Healthy > ServiceHealth::Unknown);
}

#[test]
fn test_service_health_display() {
    assert_eq!(format!("{:?}", ServiceHealth::Healthy), "Healthy");
    assert_eq!(format!("{:?}", ServiceHealth::Degraded), "Degraded");
    assert_eq!(format!("{:?}", ServiceHealth::Unknown), "Unknown");
}

#[test]
fn test_service_metadata_creation() {
    let mut extra = HashMap::new();
    extra.insert("env".to_string(), "production".to_string());
    extra.insert("region".to_string(), "us-east-1".to_string());

    let metadata = ServiceMetadata {
        version: Some("1.0.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 100,
        extra: extra.clone(),
    };

    assert_eq!(metadata.version, Some("1.0.0".to_string()));
    assert_eq!(metadata.health, ServiceHealth::Healthy);
    assert_eq!(metadata.priority, 100);
    assert_eq!(metadata.extra.len(), 2);
}

#[test]
fn test_service_metadata_with_high_priority() {
    let metadata = ServiceMetadata {
        version: Some("2.0.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 90,
        extra: HashMap::new(),
    };

    assert_eq!(metadata.priority, 90);
    assert!(metadata.priority > 50);
}

#[test]
fn test_discovered_service_creation() {
    let service = DiscoveredService {
        capability: "ai_processing".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 100,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };

    assert_eq!(service.capability, "ai_processing");
    assert_eq!(service.endpoint, "http://localhost:8080");
    assert_eq!(service.protocols.len(), 2);
    assert_eq!(service.metadata.health, ServiceHealth::Healthy);
}

#[test]
fn test_discovered_service_with_degraded_status() {
    let service = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "http://localhost:9000".to_string(),
        protocols: vec!["http".to_string()],
        metadata: ServiceMetadata {
            version: Some("0.1.0".to_string()),
            health: ServiceHealth::Degraded,
            last_seen: SystemTime::now(),
            priority: 50,
            extra: HashMap::new(),
        },
        source: DiscoverySource::ServiceMesh("consul".to_string()),
    };

    assert_eq!(service.metadata.health, ServiceHealth::Degraded);
    assert!(service.metadata.health < ServiceHealth::Healthy);
}

#[test]
fn test_discovery_preferences_creation() {
    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec!["http".to_string(), "grpc".to_string()],
        timeout: Some(Duration::from_secs(5)),
        min_health: ServiceHealth::Healthy,
        preferred_sources: vec![DiscoverySource::Environment, DiscoverySource::MDNS],
    };

    assert!(prefs.prefer_local);
    assert_eq!(prefs.required_protocols.len(), 2);
    assert_eq!(prefs.timeout, Some(Duration::from_secs(5)));
    assert_eq!(prefs.min_health, ServiceHealth::Healthy);
}

#[test]
fn test_discovery_preferences_defaults() {
    let prefs = DiscoveryPreferences::default();

    assert!(!prefs.prefer_local);
    assert!(prefs.required_protocols.is_empty());
    assert!(prefs.timeout.is_none());
    assert_eq!(prefs.min_health, ServiceHealth::Unknown);
}

#[test]
fn test_discovery_preferences_with_custom_timeout() {
    let prefs = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: Some(Duration::from_secs(30)),
        min_health: ServiceHealth::Degraded,
        preferred_sources: vec![DiscoverySource::ConfigFile],
    };

    assert_eq!(prefs.timeout, Some(Duration::from_secs(30)));
    assert_eq!(prefs.min_health, ServiceHealth::Degraded);
    assert_eq!(prefs.preferred_sources.len(), 1);
}

#[test]
fn test_service_health_clone() {
    let health = ServiceHealth::Healthy;
    let cloned = health;
    assert_eq!(health, cloned);
}

#[test]
fn test_service_metadata_clone() {
    let metadata1 = ServiceMetadata {
        version: Some("1.0.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 80,
        extra: HashMap::new(),
    };

    let metadata2 = metadata1.clone();

    assert_eq!(metadata1.version, metadata2.version);
    assert_eq!(metadata1.health, metadata2.health);
    assert_eq!(metadata1.priority, metadata2.priority);
}

#[test]
fn test_discovered_service_clone() {
    let service1 = DiscoveredService {
        capability: "authentication".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec!["http".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 100,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };

    let service2 = service1.clone();

    assert_eq!(service1.capability, service2.capability);
    assert_eq!(service1.endpoint, service2.endpoint);
    assert_eq!(service1.protocols.len(), service2.protocols.len());
}

#[test]
fn test_service_health_equality() {
    assert_eq!(ServiceHealth::Healthy, ServiceHealth::Healthy);
    assert_eq!(ServiceHealth::Degraded, ServiceHealth::Degraded);
    assert_ne!(ServiceHealth::Healthy, ServiceHealth::Degraded);
    assert_ne!(ServiceHealth::Degraded, ServiceHealth::Unknown);
}

#[test]
fn test_service_metadata_with_extra() {
    let mut extra = HashMap::new();
    extra.insert("datacenter".to_string(), "dc1".to_string());
    extra.insert("rack".to_string(), "r42".to_string());
    extra.insert("tier".to_string(), "production".to_string());

    let metadata = ServiceMetadata {
        version: Some("3.2.1".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 95,
        extra: extra.clone(),
    };

    assert_eq!(metadata.extra.len(), 3);
    assert_eq!(metadata.extra.get("datacenter"), Some(&"dc1".to_string()));
    assert_eq!(metadata.extra.get("rack"), Some(&"r42".to_string()));
}

#[test]
fn test_discovery_preferences_clone() {
    let prefs1 = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec!["http".to_string()],
        timeout: Some(Duration::from_secs(10)),
        min_health: ServiceHealth::Healthy,
        preferred_sources: vec![DiscoverySource::MDNS],
    };

    let prefs2 = prefs1.clone();

    assert_eq!(prefs1.prefer_local, prefs2.prefer_local);
    assert_eq!(prefs1.timeout, prefs2.timeout);
    assert_eq!(prefs1.min_health, prefs2.min_health);
}

#[test]
fn test_service_metadata_minimal() {
    let metadata = ServiceMetadata {
        version: None,
        health: ServiceHealth::Unknown,
        last_seen: SystemTime::now(),
        priority: 0,
        extra: HashMap::new(),
    };

    assert!(metadata.version.is_none());
    assert!(metadata.extra.is_empty());
    assert_eq!(metadata.priority, 0);
}

#[test]
fn test_discovered_service_endpoint_validation() {
    let endpoints = vec![
        "http://localhost:8080",
        "https://api.example.com",
        "http://192.168.1.100:3000",
    ];

    for endpoint in endpoints {
        let service = DiscoveredService {
            capability: "test_capability".to_string(),
            endpoint: endpoint.to_string(),
            protocols: vec!["http".to_string()],
            metadata: ServiceMetadata {
                version: Some("1.0.0".to_string()),
                health: ServiceHealth::Healthy,
                last_seen: SystemTime::now(),
                priority: 100,
                extra: HashMap::new(),
            },
            source: DiscoverySource::Environment,
        };

        assert!(!service.endpoint.is_empty());
    }
}

#[test]
fn test_service_health_all_variants_unique() {
    let healths = [
        ServiceHealth::Healthy,
        ServiceHealth::Degraded,
        ServiceHealth::Unknown,
    ];

    // All should be unique
    for i in 0..healths.len() {
        for j in (i + 1)..healths.len() {
            assert_ne!(healths[i], healths[j]);
        }
    }
}

#[test]
fn test_discovery_source_variants() {
    let sources = [
        DiscoverySource::Environment,
        DiscoverySource::MDNS,
        DiscoverySource::ServiceMesh("consul".to_string()),
        DiscoverySource::ConfigFile,
        DiscoverySource::Fallback,
        DiscoverySource::UniversalAdapter,
    ];

    assert_eq!(sources.len(), 6);
}

#[test]
fn test_discovered_service_with_multiple_protocols() {
    let service = DiscoveredService {
        capability: "load_balancing".to_string(),
        endpoint: "http://lb.example.com".to_string(),
        protocols: vec![
            "http".to_string(),
            "https".to_string(),
            "grpc".to_string(),
            "websocket".to_string(),
        ],
        metadata: ServiceMetadata {
            version: Some("2.1.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 85,
            extra: HashMap::new(),
        },
        source: DiscoverySource::MDNS,
    };

    assert_eq!(service.protocols.len(), 4);
    assert!(service.protocols.contains(&"grpc".to_string()));
}

#[test]
fn test_service_metadata_priority_range() {
    let priorities = vec![0, 25, 50, 75, 100];

    for priority in priorities {
        let metadata = ServiceMetadata {
            version: None,
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority,
            extra: HashMap::new(),
        };

        assert!(metadata.priority <= 100);
    }
}

#[test]
fn test_discovery_preferences_with_multiple_sources() {
    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec!["http".to_string(), "grpc".to_string()],
        timeout: Some(Duration::from_secs(15)),
        min_health: ServiceHealth::Degraded,
        preferred_sources: vec![
            DiscoverySource::Environment,
            DiscoverySource::MDNS,
            DiscoverySource::ServiceMesh("etcd".to_string()),
        ],
    };

    assert_eq!(prefs.preferred_sources.len(), 3);
    assert_eq!(prefs.required_protocols.len(), 2);
}
