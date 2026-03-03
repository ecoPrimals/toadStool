// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for capability-based discovery system

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool_common::infant_discovery::capabilities::*;

// ============================================================================
// DiscoveredService Tests
// ============================================================================

#[test]
fn test_discovered_service_creation() {
    let service = DiscoveredService {
        capability: "test".to_string(),
        endpoint: "http://localhost:8080".to_string(),
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

    assert_eq!(service.capability, "test");
    assert_eq!(service.endpoint, "http://localhost:8080");
    assert_eq!(service.protocols.len(), 1);
}

#[test]
fn test_discovered_service_multiple_protocols() {
    let service = DiscoveredService {
        capability: "multi_protocol".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec![
            "http".to_string(),
            "grpc".to_string(),
            "websocket".to_string(),
        ],
        metadata: ServiceMetadata {
            version: None,
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 50,
            extra: HashMap::new(),
        },
        source: DiscoverySource::MDNS,
    };

    assert_eq!(service.protocols.len(), 3);
    assert!(service.protocols.contains(&"http".to_string()));
    assert!(service.protocols.contains(&"grpc".to_string()));
    assert!(service.protocols.contains(&"websocket".to_string()));
}

#[test]
fn test_discovered_service_with_metadata() {
    let mut extra = HashMap::new();
    extra.insert("region".to_string(), "us-west-2".to_string());
    extra.insert("az".to_string(), "us-west-2a".to_string());

    let service = DiscoveredService {
        capability: "storage".to_string(),
        endpoint: "https://s3.amazonaws.com".to_string(),
        protocols: vec!["https".to_string()],
        metadata: ServiceMetadata {
            version: Some("2.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: SystemTime::now(),
            priority: 100,
            extra,
        },
        source: DiscoverySource::ServiceMesh("consul".to_string()),
    };

    assert_eq!(service.metadata.extra.len(), 2);
    assert_eq!(
        service.metadata.extra.get("region"),
        Some(&"us-west-2".to_string())
    );
    assert_eq!(service.metadata.priority, 100);
}

// ============================================================================
// ServiceMetadata Tests
// ============================================================================

#[test]
fn test_service_metadata_with_version() {
    let metadata = ServiceMetadata {
        version: Some("1.2.3".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 75,
        extra: HashMap::new(),
    };

    assert_eq!(metadata.version, Some("1.2.3".to_string()));
}

#[test]
fn test_service_metadata_without_version() {
    let metadata = ServiceMetadata {
        version: None,
        health: ServiceHealth::Unknown,
        last_seen: SystemTime::now(),
        priority: 0,
        extra: HashMap::new(),
    };

    assert!(metadata.version.is_none());
}

#[test]
fn test_service_metadata_priority_boundaries() {
    let min_priority = ServiceMetadata {
        version: None,
        health: ServiceHealth::Degraded,
        last_seen: SystemTime::now(),
        priority: 0,
        extra: HashMap::new(),
    };

    let max_priority = ServiceMetadata {
        version: None,
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 100,
        extra: HashMap::new(),
    };

    assert_eq!(min_priority.priority, 0);
    assert_eq!(max_priority.priority, 100);
}

#[test]
fn test_service_metadata_extra_fields() {
    let mut extra = HashMap::new();
    extra.insert("datacenter".to_string(), "DC1".to_string());
    extra.insert("rack".to_string(), "A23".to_string());
    extra.insert("custom".to_string(), "value".to_string());

    let metadata = ServiceMetadata {
        version: Some("1.0.0".to_string()),
        health: ServiceHealth::Healthy,
        last_seen: SystemTime::now(),
        priority: 90,
        extra,
    };

    assert_eq!(metadata.extra.len(), 3);
    assert_eq!(metadata.extra.get("datacenter"), Some(&"DC1".to_string()));
}

// ============================================================================
// ServiceHealth Tests
// ============================================================================

#[test]
fn test_service_health_ordering() {
    assert!(ServiceHealth::Healthy > ServiceHealth::Degraded);
    assert!(ServiceHealth::Degraded > ServiceHealth::Unknown);
    assert!(ServiceHealth::Healthy > ServiceHealth::Unknown);
}

#[test]
fn test_service_health_equality() {
    assert_eq!(ServiceHealth::Healthy, ServiceHealth::Healthy);
    assert_eq!(ServiceHealth::Degraded, ServiceHealth::Degraded);
    assert_eq!(ServiceHealth::Unknown, ServiceHealth::Unknown);
}

#[test]
fn test_service_health_copy() {
    let health1 = ServiceHealth::Healthy;
    let health2 = health1; // Copy
    assert_eq!(health1, health2);
}

// ============================================================================
// DiscoverySource Tests
// ============================================================================

#[test]
fn test_discovery_source_environment() {
    let source = DiscoverySource::Environment;
    assert_eq!(source, DiscoverySource::Environment);
}

#[test]
fn test_discovery_source_mdns() {
    let source = DiscoverySource::MDNS;
    assert_eq!(source, DiscoverySource::MDNS);
}

#[test]
fn test_discovery_source_service_mesh_consul() {
    let source = DiscoverySource::ServiceMesh("consul".to_string());
    match source {
        DiscoverySource::ServiceMesh(name) => assert_eq!(name, "consul"),
        _ => panic!("Expected ServiceMesh variant"),
    }
}

#[test]
fn test_discovery_source_service_mesh_etcd() {
    let source = DiscoverySource::ServiceMesh("etcd".to_string());
    match source {
        DiscoverySource::ServiceMesh(name) => assert_eq!(name, "etcd"),
        _ => panic!("Expected ServiceMesh variant"),
    }
}

#[test]
fn test_discovery_source_config_file() {
    let source = DiscoverySource::ConfigFile;
    assert_eq!(source, DiscoverySource::ConfigFile);
}

#[test]
fn test_discovery_source_fallback() {
    let source = DiscoverySource::Fallback;
    assert_eq!(source, DiscoverySource::Fallback);
}

#[test]
fn test_discovery_source_universal_adapter() {
    let source = DiscoverySource::UniversalAdapter;
    assert_eq!(source, DiscoverySource::UniversalAdapter);
}

#[test]
fn test_discovery_source_equality() {
    assert_eq!(DiscoverySource::Environment, DiscoverySource::Environment);
    assert_ne!(DiscoverySource::Environment, DiscoverySource::MDNS);
    assert_ne!(DiscoverySource::MDNS, DiscoverySource::ConfigFile);
}

// ============================================================================
// DiscoveryPreferences Tests
// ============================================================================

#[test]
fn test_discovery_preferences_prefer_local() {
    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec![],
        timeout: None,
        min_health: ServiceHealth::Healthy,
        preferred_sources: vec![],
    };

    assert!(prefs.prefer_local);
}

#[test]
fn test_discovery_preferences_required_protocols() {
    let prefs = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec!["grpc".to_string(), "http2".to_string()],
        timeout: None,
        min_health: ServiceHealth::Unknown,
        preferred_sources: vec![],
    };

    assert_eq!(prefs.required_protocols.len(), 2);
    assert!(prefs.required_protocols.contains(&"grpc".to_string()));
}

#[test]
fn test_discovery_preferences_timeout() {
    let prefs = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: Some(Duration::from_secs(10)),
        min_health: ServiceHealth::Unknown,
        preferred_sources: vec![],
    };

    assert_eq!(prefs.timeout, Some(Duration::from_secs(10)));
}

#[test]
fn test_discovery_preferences_min_health_levels() {
    let prefs_unknown = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: None,
        min_health: ServiceHealth::Unknown,
        preferred_sources: vec![],
    };

    let prefs_degraded = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: None,
        min_health: ServiceHealth::Degraded,
        preferred_sources: vec![],
    };

    let prefs_healthy = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: None,
        min_health: ServiceHealth::Healthy,
        preferred_sources: vec![],
    };

    assert_eq!(prefs_unknown.min_health, ServiceHealth::Unknown);
    assert_eq!(prefs_degraded.min_health, ServiceHealth::Degraded);
    assert_eq!(prefs_healthy.min_health, ServiceHealth::Healthy);
}

#[test]
fn test_discovery_preferences_preferred_sources() {
    let prefs = DiscoveryPreferences {
        prefer_local: false,
        required_protocols: vec![],
        timeout: None,
        min_health: ServiceHealth::Unknown,
        preferred_sources: vec![
            DiscoverySource::Environment,
            DiscoverySource::ConfigFile,
            DiscoverySource::MDNS,
        ],
    };

    assert_eq!(prefs.preferred_sources.len(), 3);
    assert_eq!(prefs.preferred_sources[0], DiscoverySource::Environment);
}

// ============================================================================
// SubstrateCapability Tests
// ============================================================================

#[test]
fn test_substrate_capability_container_orchestration() {
    let cap = SubstrateCapability::ContainerOrchestration;
    assert_eq!(cap, SubstrateCapability::ContainerOrchestration);
}

#[test]
fn test_substrate_capability_container_runtime() {
    let cap = SubstrateCapability::ContainerRuntime;
    assert_eq!(cap, SubstrateCapability::ContainerRuntime);
}

#[test]
fn test_substrate_capability_service_mesh() {
    let cap = SubstrateCapability::ServiceMesh;
    assert_eq!(cap, SubstrateCapability::ServiceMesh);
}

#[test]
fn test_substrate_capability_service_discovery() {
    let cap = SubstrateCapability::ServiceDiscovery;
    assert_eq!(cap, SubstrateCapability::ServiceDiscovery);
}

#[test]
fn test_substrate_capability_cloud_compute() {
    let cap = SubstrateCapability::CloudCompute;
    assert_eq!(cap, SubstrateCapability::CloudCompute);
}

#[test]
fn test_substrate_capability_bare_metal() {
    let cap = SubstrateCapability::BareMetal;
    assert_eq!(cap, SubstrateCapability::BareMetal);
}

// ============================================================================
// SubstrateType Tests
// ============================================================================

#[test]
fn test_substrate_type_container_orchestrator() {
    let substrate = SubstrateType::ContainerOrchestrator;
    assert_eq!(substrate, SubstrateType::ContainerOrchestrator);
}

#[test]
fn test_substrate_type_container_runtime() {
    let substrate = SubstrateType::ContainerRuntime;
    assert_eq!(substrate, SubstrateType::ContainerRuntime);
}

#[test]
fn test_substrate_type_cloud() {
    let substrate = SubstrateType::Cloud;
    assert_eq!(substrate, SubstrateType::Cloud);
}

#[test]
fn test_substrate_type_bare() {
    let substrate = SubstrateType::Bare;
    assert_eq!(substrate, SubstrateType::Bare);
}

// ============================================================================
// DetectedSubstrate Tests
// ============================================================================

#[test]
fn test_detected_substrate_has_capability_true() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![
            SubstrateCapability::ContainerOrchestration,
            SubstrateCapability::ServiceDiscovery,
        ],
        metadata: HashMap::new(),
    };

    assert!(substrate.has_capability(&SubstrateCapability::ContainerOrchestration));
    assert!(substrate.has_capability(&SubstrateCapability::ServiceDiscovery));
}

#[test]
fn test_detected_substrate_has_capability_false() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Bare,
        capabilities: vec![SubstrateCapability::BareMetal],
        metadata: HashMap::new(),
    };

    assert!(!substrate.has_capability(&SubstrateCapability::ContainerOrchestration));
    assert!(!substrate.has_capability(&SubstrateCapability::CloudCompute));
}

#[test]
fn test_detected_substrate_get_metadata_found() {
    let mut metadata = HashMap::new();
    metadata.insert("version".to_string(), "1.28.0".to_string());
    metadata.insert("provider".to_string(), "kubernetes".to_string());

    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![SubstrateCapability::ContainerOrchestration],
        metadata,
    };

    assert_eq!(
        substrate.get_metadata("version"),
        Some(&"1.28.0".to_string())
    );
    assert_eq!(
        substrate.get_metadata("provider"),
        Some(&"kubernetes".to_string())
    );
}

#[test]
fn test_detected_substrate_get_metadata_not_found() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Bare,
        capabilities: vec![],
        metadata: HashMap::new(),
    };

    assert_eq!(substrate.get_metadata("nonexistent"), None);
}

#[test]
fn test_detected_substrate_empty_capabilities() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Bare,
        capabilities: vec![],
        metadata: HashMap::new(),
    };

    assert_eq!(substrate.capabilities.len(), 0);
    assert!(!substrate.has_capability(&SubstrateCapability::BareMetal));
}

#[test]
fn test_detected_substrate_multiple_capabilities() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Cloud,
        capabilities: vec![
            SubstrateCapability::CloudCompute,
            SubstrateCapability::ServiceMesh,
            SubstrateCapability::ServiceDiscovery,
            SubstrateCapability::ContainerOrchestration,
        ],
        metadata: HashMap::new(),
    };

    assert_eq!(substrate.capabilities.len(), 4);
    assert!(substrate.has_capability(&SubstrateCapability::CloudCompute));
    assert!(substrate.has_capability(&SubstrateCapability::ContainerOrchestration));
}

// ============================================================================
// EndpointResolver Tests
// ============================================================================

#[test]
fn test_endpoint_resolver_new() {
    let _resolver = EndpointResolver::new();
    // Resolver created successfully
}

#[test]
fn test_endpoint_resolver_default() {
    let _resolver = EndpointResolver::default();
    // Default resolver created successfully
}

// ============================================================================
// DiscoveryError Tests
// ============================================================================

#[test]
fn test_discovery_error_capability_not_found() {
    let err = DiscoveryError::CapabilityNotFound("authentication".to_string());
    assert_eq!(err.to_string(), "Capability 'authentication' not found");
}

#[test]
fn test_discovery_error_timeout() {
    let err = DiscoveryError::Timeout(Duration::from_secs(30));
    let msg = err.to_string();
    assert!(msg.contains("Discovery timeout"));
    assert!(msg.contains("30"));
}

#[test]
fn test_discovery_error_no_healthy_services() {
    let err = DiscoveryError::NoHealthyServices("storage".to_string());
    assert_eq!(
        err.to_string(),
        "No healthy services found for capability 'storage'"
    );
}

#[test]
fn test_discovery_error_protocol_not_supported() {
    let err = DiscoveryError::ProtocolNotSupported("mqtt".to_string());
    assert_eq!(
        err.to_string(),
        "Protocol 'mqtt' not supported by any discovered service"
    );
}

#[test]
fn test_discovery_error_source_failed() {
    let err = DiscoveryError::SourceFailed("mdns: network unreachable".to_string());
    assert_eq!(
        err.to_string(),
        "Discovery source failed: mdns: network unreachable"
    );
}

#[test]
fn test_discovery_error_config_error() {
    let err = DiscoveryError::ConfigError("missing required field 'endpoint'".to_string());
    assert_eq!(
        err.to_string(),
        "Configuration error: missing required field 'endpoint'"
    );
}

// ============================================================================
// Capability Constants Tests
// ============================================================================

#[test]
fn test_all_capability_constants() {
    assert_eq!(capabilities::AI_PROCESSING, "ai_processing");
    assert_eq!(capabilities::NLP, "natural_language_processing");
    assert_eq!(capabilities::AUTHENTICATION, "authentication");
    assert_eq!(capabilities::AUTHORIZATION, "authorization");
    assert_eq!(capabilities::STORAGE, "persistent_storage");
    assert_eq!(capabilities::KEY_VALUE_STORE, "key_value_storage");
    assert_eq!(capabilities::ORCHESTRATION, "service_orchestration");
    assert_eq!(capabilities::LOAD_BALANCING, "load_balancing");
    assert_eq!(capabilities::SERVICE_MESH, "service_mesh");
    assert_eq!(capabilities::MONITORING, "monitoring");
    assert_eq!(capabilities::TRACING, "distributed_tracing");
    assert_eq!(capabilities::SECRETS, "secret_management");
    assert_eq!(capabilities::PKI, "public_key_infrastructure");
    assert_eq!(capabilities::MESSAGE_QUEUE, "message_queue");
    assert_eq!(capabilities::EVENT_STREAM, "event_streaming");
    assert_eq!(capabilities::CACHE, "caching");
    assert_eq!(capabilities::SEARCH, "search_indexing");
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_service_health_serialization() {
    let healths = vec![
        ServiceHealth::Unknown,
        ServiceHealth::Degraded,
        ServiceHealth::Healthy,
    ];

    for health in healths {
        let json = serde_json::to_string(&health).expect("Failed to serialize");
        let deserialized: ServiceHealth =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(health, deserialized);
    }
}

#[test]
fn test_substrate_capability_serialization() {
    let caps = vec![
        SubstrateCapability::ContainerOrchestration,
        SubstrateCapability::ContainerRuntime,
        SubstrateCapability::ServiceMesh,
        SubstrateCapability::ServiceDiscovery,
        SubstrateCapability::CloudCompute,
        SubstrateCapability::BareMetal,
    ];

    for cap in caps {
        let json = serde_json::to_string(&cap).expect("Failed to serialize");
        let deserialized: SubstrateCapability =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(cap, deserialized);
    }
}

#[test]
fn test_substrate_type_serialization() {
    let types = vec![
        SubstrateType::ContainerOrchestrator,
        SubstrateType::ContainerRuntime,
        SubstrateType::Cloud,
        SubstrateType::Bare,
    ];

    for substrate_type in types {
        let json = serde_json::to_string(&substrate_type).expect("Failed to serialize");
        let deserialized: SubstrateType =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(substrate_type, deserialized);
    }
}

#[test]
fn test_complete_discovered_service_serialization_roundtrip() {
    let mut extra = HashMap::new();
    extra.insert("key1".to_string(), "value1".to_string());
    extra.insert("key2".to_string(), "value2".to_string());

    let service = DiscoveredService {
        capability: "full_test".to_string(),
        endpoint: "https://api.example.com:9090".to_string(),
        protocols: vec!["https".to_string(), "grpc".to_string(), "http2".to_string()],
        metadata: ServiceMetadata {
            version: Some("2.1.0".to_string()),
            health: ServiceHealth::Degraded,
            last_seen: SystemTime::now(),
            priority: 42,
            extra,
        },
        source: DiscoverySource::ServiceMesh("istio".to_string()),
    };

    let json = serde_json::to_string(&service).expect("Failed to serialize");
    let deserialized: DiscoveredService =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.capability, "full_test");
    assert_eq!(deserialized.endpoint, "https://api.example.com:9090");
    assert_eq!(deserialized.protocols.len(), 3);
    assert_eq!(deserialized.metadata.version, Some("2.1.0".to_string()));
    assert_eq!(deserialized.metadata.health, ServiceHealth::Degraded);
    assert_eq!(deserialized.metadata.priority, 42);
    assert_eq!(deserialized.metadata.extra.len(), 2);
}
