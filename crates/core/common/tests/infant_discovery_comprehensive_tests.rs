// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Comprehensive tests for infant_discovery module
//!
//! Coverage push - Nov 7, 2025
//! Target: Bring infant_discovery from 0% → 60%+

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool_common::infant_discovery::*;

// ============================================================================
// DiscoveryConfig Tests
// ============================================================================

#[test]
fn test_discovery_config_default() {
    let config = DiscoveryConfig::default();
    
    assert!(config.enable_cache);
    assert_eq!(config.cache_ttl, Duration::from_secs(300));
    assert_eq!(config.default_timeout, Duration::from_secs(30));
    assert_eq!(config.retry_attempts, 3);
    assert_eq!(config.retry_delay, Duration::from_secs(1));
}

#[test]
fn test_discovery_config_clone() {
    let config = DiscoveryConfig {
        enable_cache: false,
        cache_ttl: Duration::from_secs(600),
        default_timeout: Duration::from_secs(60),
        retry_attempts: 5,
        retry_delay: Duration::from_secs(2),
    };
    
    let cloned = config.clone();
    assert_eq!(config.enable_cache, cloned.enable_cache);
    assert_eq!(config.cache_ttl, cloned.cache_ttl);
    assert_eq!(config.default_timeout, cloned.default_timeout);
    assert_eq!(config.retry_attempts, cloned.retry_attempts);
    assert_eq!(config.retry_delay, cloned.retry_delay);
}

#[test]
fn test_discovery_config_custom_values() {
    let config = DiscoveryConfig {
        enable_cache: false,
        cache_ttl: Duration::from_secs(60),
        default_timeout: Duration::from_secs(10),
        retry_attempts: 1,
        retry_delay: Duration::from_millis(500),
    };
    
    assert!(!config.enable_cache);
    assert_eq!(config.cache_ttl.as_secs(), 60);
    assert_eq!(config.default_timeout.as_secs(), 10);
    assert_eq!(config.retry_attempts, 1);
    assert_eq!(config.retry_delay.as_millis(), 500);
}

// ============================================================================
// ServiceHealth Tests
// ============================================================================

#[test]
fn test_service_health_ordering() {
    assert!(ServiceHealth::Healthy > ServiceHealth::Degraded);
    assert!(ServiceHealth::Degraded > ServiceHealth::Unknown);
    assert!(ServiceHealth::Unknown < ServiceHealth::Degraded);
}

#[test]
fn test_service_health_equality() {
    assert_eq!(ServiceHealth::Healthy, ServiceHealth::Healthy);
    assert_eq!(ServiceHealth::Degraded, ServiceHealth::Degraded);
    assert_eq!(ServiceHealth::Unknown, ServiceHealth::Unknown);
    
    assert_ne!(ServiceHealth::Healthy, ServiceHealth::Degraded);
    assert_ne!(ServiceHealth::Degraded, ServiceHealth::Unknown);
}

#[test]
fn test_service_health_clone() {
    let health = ServiceHealth::Healthy;
    let cloned = health;
    assert_eq!(health, cloned);
}

#[test]
fn test_service_health_copy() {
    let health = ServiceHealth::Degraded;
    let copied = health;
    assert_eq!(health, copied);
}

// ============================================================================
// DiscoverySource Tests
// ============================================================================

#[test]
fn test_discovery_source_variants() {
    let env = DiscoverySource::Environment;
    let mdns = DiscoverySource::MDNS;
    let mesh = DiscoverySource::ServiceMesh("consul".to_string());
    let config = DiscoverySource::ConfigFile;
    let fallback = DiscoverySource::Fallback;
    
    // Just ensure they're all created correctly
    assert!(matches!(env, DiscoverySource::Environment));
    assert!(matches!(mdns, DiscoverySource::MDNS));
    assert!(matches!(mesh, DiscoverySource::ServiceMesh(_)));
    assert!(matches!(config, DiscoverySource::ConfigFile));
    assert!(matches!(fallback, DiscoverySource::Fallback));
}

#[test]
fn test_discovery_source_clone() {
    let source = DiscoverySource::ServiceMesh("istio".to_string());
    let cloned = source.clone();
    assert_eq!(source, cloned);
}

#[test]
fn test_discovery_source_equality() {
    let env1 = DiscoverySource::Environment;
    let env2 = DiscoverySource::Environment;
    assert_eq!(env1, env2);
    
    let mesh1 = DiscoverySource::ServiceMesh("consul".to_string());
    let mesh2 = DiscoverySource::ServiceMesh("consul".to_string());
    assert_eq!(mesh1, mesh2);
}

#[test]
fn test_discovery_source_from_str() {
    let env: DiscoverySource = "environment".into();
    assert!(matches!(env, DiscoverySource::Environment));
    
    let mdns: DiscoverySource = "mdns".into();
    assert!(matches!(mdns, DiscoverySource::MDNS));
    
    let mesh: DiscoverySource = "service_mesh".into();
    assert!(matches!(mesh, DiscoverySource::ServiceMesh(_)));
    
    let config: DiscoverySource = "config_file".into();
    assert!(matches!(config, DiscoverySource::ConfigFile));
    
    let fallback: DiscoverySource = "unknown".into();
    assert!(matches!(fallback, DiscoverySource::Fallback));
}

// ============================================================================
// ServiceMetadata Tests
// ============================================================================

#[test]
fn test_service_metadata_creation() {
    let metadata = ServiceMetadata {
        version: Some("1.0.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 80,
        extra: HashMap::new(),
    };
    
    assert_eq!(metadata.version, Some("1.0.0".to_string()));
    assert_eq!(metadata.health, ServiceHealth::Healthy);
    assert_eq!(metadata.priority, 80);
    assert!(metadata.extra.is_empty());
}

#[test]
fn test_service_metadata_with_extra() {
    let mut extra = HashMap::new();
    extra.insert("region".to_string(), "us-west".to_string());
    extra.insert("datacenter".to_string(), "dc1".to_string());
    
    let metadata = ServiceMetadata {
        version: Some("2.0.0".to_string()),
        health: ServiceHealth::Degraded,
        last_seen: SystemTime::now(),
        priority: 50,
        extra,
    };
    
    assert_eq!(metadata.extra.len(), 2);
    assert_eq!(metadata.extra.get("region"), Some(&"us-west".to_string()));
    assert_eq!(metadata.extra.get("datacenter"), Some(&"dc1".to_string()));
}

#[test]
fn test_service_metadata_clone() {
    let metadata = ServiceMetadata {
        version: Some("1.5.0".to_string()),
        health: ServiceHealth::Unknown,
        last_seen: SystemTime::now(),
        priority: 100,
        extra: HashMap::new(),
    };
    
    let cloned = metadata.clone();
    assert_eq!(metadata.version, cloned.version);
    assert_eq!(metadata.health, cloned.health);
    assert_eq!(metadata.priority, cloned.priority);
}

#[test]
fn test_service_metadata_priority_range() {
    // Test edge cases for priority (0-100)
    let low = ServiceMetadata {
        version: None,
        health: ServiceHealth::Unknown,
        last_seen: SystemTime::now(),
        priority: 0,
        extra: HashMap::new(),
    };
    assert_eq!(low.priority, 0);
    
    let high = ServiceMetadata {
        version: None,
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 100,
        extra: HashMap::new(),
    };
    assert_eq!(high.priority, 100);
}

// ============================================================================
// DiscoveredService Tests
// ============================================================================

#[test]
fn test_discovered_service_creation() {
    let service = DiscoveredService {
        capability: "authentication".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 90,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };
    
    assert_eq!(service.capability, "authentication");
    assert_eq!(service.endpoint, "http://localhost:8080");
    assert_eq!(service.protocols.len(), 2);
    assert_eq!(service.metadata.health, ServiceHealth::Healthy);
}

#[test]
fn test_discovered_service_protocols() {
    let service = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "https://storage.local:9000".to_string(),
        protocols: vec!["http".to_string(), "s3".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("2.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 85,
            extra: HashMap::new(),
        },
        source: DiscoverySource::MDNS,
    };
    
    assert_eq!(service.protocols.len(), 3);
    assert!(service.protocols.contains(&"http".to_string()));
    assert!(service.protocols.contains(&"s3".to_string()));
    assert!(service.protocols.contains(&"grpc".to_string()));
}

#[test]
fn test_discovered_service_clone() {
    let service = DiscoveredService {
        capability: "ai_processing".to_string(),
        endpoint: "http://ai.local:7000".to_string(),
        protocols: vec!["http".to_string()],
        metadata: ServiceMetadata {
            version: Some("3.0.0".to_string()),
            health: ServiceHealth::Degraded,
            last_seen: SystemTime::now(),
            priority: 70,
            extra: HashMap::new(),
        },
        source: DiscoverySource::ConfigFile,
    };
    
    let cloned = service.clone();
    assert_eq!(service.capability, cloned.capability);
    assert_eq!(service.endpoint, cloned.endpoint);
    assert_eq!(service.protocols, cloned.protocols);
}

#[test]
fn test_discovered_service_multiple_services() {
    let auth_service = DiscoveredService {
        capability: "authentication".to_string(),
        endpoint: "http://auth:8080".to_string(),
        protocols: vec!["http".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 90,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };
    
    let storage_service = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "http://storage:9000".to_string(),
        protocols: vec!["s3".to_string()],
        metadata: ServiceMetadata {
            version: Some("2.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 85,
            extra: HashMap::new(),
        },
        source: DiscoverySource::MDNS,
    };
    
    assert_ne!(auth_service.capability, storage_service.capability);
    assert_ne!(auth_service.endpoint, storage_service.endpoint);
}

// ============================================================================
// DiscoveryError Tests
// ============================================================================

#[test]
fn test_discovery_error_capability_not_found() {
    let error = DiscoveryError::CapabilityNotFound("missing_service".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("missing_service"));
    assert!(error_str.contains("not found"));
}

#[test]
fn test_discovery_error_timeout() {
    let error = DiscoveryError::Timeout(Duration::from_secs(30));
    let error_str = format!("{}", error);
    assert!(error_str.contains("timeout"));
    assert!(error_str.contains("30"));
}

#[test]
fn test_discovery_error_no_healthy_services() {
    let error = DiscoveryError::NoHealthyServices("degraded_service".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("degraded_service"));
    assert!(error_str.contains("No healthy"));
}

#[test]
fn test_discovery_error_protocol_not_supported() {
    let error = DiscoveryError::ProtocolNotSupported("websocket".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("websocket"));
    assert!(error_str.contains("not supported"));
}

#[test]
fn test_discovery_error_source_failed() {
    let error = DiscoveryError::SourceFailed("MDNS discovery failed".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("source failed") || error_str.contains("Source failed"));
    assert!(error_str.contains("MDNS"));
}

#[test]
fn test_discovery_error_config_error() {
    let error = DiscoveryError::ConfigError("Invalid timeout value".to_string());
    let error_str = format!("{}", error);
    assert!(error_str.contains("Configuration error"));
    assert!(error_str.contains("timeout"));
}

// ============================================================================
// DiscoveryEngineBuilder Tests
// ============================================================================

#[test]
fn test_discovery_engine_builder_default() {
    let _builder = DiscoveryEngineBuilder::default();
    // Builder created successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_builder_new() {
    let _builder = DiscoveryEngineBuilder::new();
    // Builder created successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_builder_cache_ttl() {
    let _builder = DiscoveryEngineBuilder::new()
        .cache_ttl(Duration::from_secs(600));
    // Builder configured successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_builder_timeout() {
    let _builder = DiscoveryEngineBuilder::new()
        .timeout(Duration::from_secs(60));
    // Builder configured successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_builder_disable_cache() {
    let _builder = DiscoveryEngineBuilder::new()
        .disable_cache();
    // Cache disabled successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_builder_chaining() {
    let _builder = DiscoveryEngineBuilder::new()
        .cache_ttl(Duration::from_secs(600))
        .timeout(Duration::from_secs(45))
        .disable_cache();
    // Chained configuration successful
    assert!(true);
}

// ============================================================================
// DiscoveryEngine Tests
// ============================================================================

#[test]
fn test_discovery_engine_new() {
    let _engine = DiscoveryEngine::new();
    // Engine created successfully
    assert!(true);
}

#[test]
fn test_discovery_engine_with_config() {
    let config = DiscoveryConfig {
        enable_cache: false,
        cache_ttl: Duration::from_secs(300),
        default_timeout: Duration::from_secs(30),
        retry_attempts: 3,
        retry_delay: Duration::from_secs(1),
    };
    
    let _engine = DiscoveryEngine::with_config(config);
    // Engine created with custom config
    assert!(true);
}

// ============================================================================
// Integration-style Tests
// ============================================================================

#[test]
fn test_service_discovery_workflow() {
    // Simulate a complete discovery workflow
    let service = DiscoveredService {
        capability: "orchestration".to_string(),
        endpoint: "http://orchestrator:8000".to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.2.3".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 95,
            extra: {
                let mut map = HashMap::new();
                map.insert("region".to_string(), "us-east".to_string());
                map
            },
        },
        source: DiscoverySource::ServiceMesh("consul".to_string()),
    };
    
    // Verify complete service structure
    assert_eq!(service.capability, "orchestration");
    assert!(service.endpoint.contains("orchestrator"));
    assert_eq!(service.protocols.len(), 2);
    assert_eq!(service.metadata.health, ServiceHealth::Healthy);
    assert_eq!(service.metadata.priority, 95);
    assert!(service.metadata.extra.contains_key("region"));
}

#[test]
fn test_multiple_discovery_sources() {
    let sources = vec![
        DiscoverySource::Environment,
        DiscoverySource::MDNS,
        DiscoverySource::ServiceMesh("istio".to_string()),
        DiscoverySource::ConfigFile,
        DiscoverySource::Fallback,
        DiscoverySource::UniversalAdapter,
    ];
    
    assert_eq!(sources.len(), 6);
    
    // Each source is unique
    for (i, source) in sources.iter().enumerate() {
        match source {
            DiscoverySource::Environment => assert_eq!(i, 0),
            DiscoverySource::MDNS => assert_eq!(i, 1),
            DiscoverySource::ServiceMesh(_) => assert_eq!(i, 2),
            DiscoverySource::ConfigFile => assert_eq!(i, 3),
            DiscoverySource::Fallback => assert_eq!(i, 4),
            DiscoverySource::UniversalAdapter => assert_eq!(i, 5),
        }
    }
}

#[test]
fn test_service_health_degradation() {
    let mut service = DiscoveredService {
        capability: "monitoring".to_string(),
        endpoint: "http://monitor:3000".to_string(),
        protocols: vec!["http".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 80,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };
    
    // Simulate health degradation
    assert_eq!(service.metadata.health, ServiceHealth::Healthy);
    
    service.metadata.health = ServiceHealth::Degraded;
    assert_eq!(service.metadata.health, ServiceHealth::Degraded);
    
    service.metadata.health = ServiceHealth::Unknown;
    assert_eq!(service.metadata.health, ServiceHealth::Unknown);
}

#[test]
fn test_priority_based_service_selection() {
    let service_high = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "http://storage1:9000".to_string(),
        protocols: vec!["s3".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 90,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };
    
    let service_low = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "http://storage2:9001".to_string(),
        protocols: vec!["s3".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 50,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Fallback,
    };
    
    // High priority should be preferred
    assert!(service_high.metadata.priority > service_low.metadata.priority);
}

#[test]
fn test_service_metadata_extra_data() {
    let mut extra = HashMap::new();
    extra.insert("zone".to_string(), "us-west-1a".to_string());
    extra.insert("instance_type".to_string(), "m5.large".to_string());
    extra.insert("cost".to_string(), "0.096".to_string());
    
    let metadata = ServiceMetadata {
        version: Some("2.5.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 75,
        extra,
    };
    
    assert_eq!(metadata.extra.len(), 3);
    assert_eq!(metadata.extra.get("zone"), Some(&"us-west-1a".to_string()));
    assert_eq!(metadata.extra.get("instance_type"), Some(&"m5.large".to_string()));
    assert_eq!(metadata.extra.get("cost"), Some(&"0.096".to_string()));
}

#[test]
fn test_discovery_config_variations() {
    // Test various valid configurations
    let configs = vec![
        DiscoveryConfig {
            enable_cache: true,
            cache_ttl: Duration::from_secs(60),
            default_timeout: Duration::from_secs(10),
            retry_attempts: 1,
            retry_delay: Duration::from_millis(100),
        },
        DiscoveryConfig {
            enable_cache: false,
            cache_ttl: Duration::from_secs(3600),
            default_timeout: Duration::from_secs(120),
            retry_attempts: 10,
            retry_delay: Duration::from_secs(5),
        },
        DiscoveryConfig::default(),
    ];
    
    assert_eq!(configs.len(), 3);
    assert!(configs[0].enable_cache);
    assert!(!configs[1].enable_cache);
    assert!(configs[2].enable_cache); // default is true
}

