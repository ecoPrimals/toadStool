// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use super::*;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capability_names() {
    // Ensure capability names are stable
    assert_eq!(capabilities::AI_PROCESSING, "ai_processing");
    assert_eq!(capabilities::AUTHENTICATION, "authentication");
    assert_eq!(capabilities::STORAGE, "persistent_storage");
}

#[test]
fn test_substrate_capabilities() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![
            SubstrateCapability::ContainerOrchestration,
            SubstrateCapability::ServiceDiscovery,
        ],
        metadata: std::collections::HashMap::new(),
    };

    assert!(substrate.has_capability(&SubstrateCapability::ContainerOrchestration));
    assert!(substrate.has_capability(&SubstrateCapability::ServiceDiscovery));
    assert!(!substrate.has_capability(&SubstrateCapability::CloudCompute));
}

#[test]
fn test_discovered_service_serialization() {
    let service = DiscoveredService {
        capability: "test_capability".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        protocols: vec!["http".to_string(), "grpc".to_string()],
        metadata: ServiceMetadata {
            version: Some("1.0.0".to_string()),
            health: ServiceHealth::Healthy,
            last_seen: std::time::SystemTime::now(),
            priority: 80,
            extra: HashMap::new(),
        },
        source: DiscoverySource::Environment,
    };

    let json = serde_json::to_string(&service).expect("Failed to serialize");
    let deserialized: DiscoveredService =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.capability, service.capability);
    assert_eq!(deserialized.endpoint, service.endpoint);
    assert_eq!(deserialized.protocols.len(), 2);
}

#[test]
fn test_service_health_variants() {
    assert_eq!(ServiceHealth::Healthy, ServiceHealth::Healthy);
    assert_ne!(ServiceHealth::Healthy, ServiceHealth::Degraded);
    assert_ne!(ServiceHealth::Degraded, ServiceHealth::Unknown);
}

#[test]
fn test_discovery_source_variants() {
    let env = DiscoverySource::Environment;
    let mdns = DiscoverySource::MDNS;
    let mesh = DiscoverySource::ServiceMesh("consul".to_string());
    let config = DiscoverySource::ConfigFile;
    let fallback = DiscoverySource::Fallback;
    let adapter = DiscoverySource::UniversalAdapter;

    assert_eq!(env, DiscoverySource::Environment);
    assert_ne!(env, mdns);
    assert_ne!(mesh, config);
    assert_ne!(fallback, adapter);
}

#[test]
fn test_discovery_source_serialization() {
    let sources = vec![
        DiscoverySource::Environment,
        DiscoverySource::MDNS,
        DiscoverySource::ServiceMesh("etcd".to_string()),
        DiscoverySource::ConfigFile,
        DiscoverySource::Fallback,
        DiscoverySource::UniversalAdapter,
    ];

    for source in sources {
        let json = serde_json::to_string(&source).expect("Failed to serialize");
        let _deserialized: DiscoverySource =
            serde_json::from_str(&json).expect("Failed to deserialize");
    }
}

#[test]
fn test_discovery_preferences_default() {
    let prefs = DiscoveryPreferences::default();

    assert!(!prefs.prefer_local);
    assert!(prefs.required_protocols.is_empty());
    assert!(prefs.timeout.is_none());
    assert_eq!(prefs.min_health, ServiceHealth::Unknown);
    assert!(prefs.preferred_sources.is_empty());
}

#[test]
fn test_discovery_preferences_with_values() {
    let prefs = DiscoveryPreferences {
        prefer_local: true,
        required_protocols: vec!["grpc".to_string()],
        timeout: Some(Duration::from_secs(5)),
        min_health: ServiceHealth::Healthy,
        preferred_sources: vec![DiscoverySource::Environment],
    };

    assert!(prefs.prefer_local);
    assert_eq!(prefs.required_protocols.len(), 1);
    assert_eq!(prefs.timeout, Some(Duration::from_secs(5)));
    assert_eq!(prefs.min_health, ServiceHealth::Healthy);
}

#[test]
fn test_substrate_capability_variants() {
    let caps = [
        SubstrateCapability::ContainerOrchestration,
        SubstrateCapability::ContainerRuntime,
        SubstrateCapability::ServiceMesh,
        SubstrateCapability::ServiceDiscovery,
        SubstrateCapability::CloudCompute,
        SubstrateCapability::BareMetal,
    ];

    assert_eq!(caps.len(), 6);
    assert_eq!(caps[0], SubstrateCapability::ContainerOrchestration);
}

#[test]
fn test_substrate_type_variants() {
    let types = [
        SubstrateType::ContainerOrchestrator,
        SubstrateType::ContainerRuntime,
        SubstrateType::Cloud,
        SubstrateType::Bare,
    ];

    assert_eq!(types.len(), 4);
    assert_eq!(types[3], SubstrateType::Bare);
}

#[test]
fn test_detected_substrate_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("version".to_string(), "1.20.0".to_string());
    metadata.insert("provider".to_string(), "k8s".to_string());

    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::ContainerOrchestrator,
        capabilities: vec![SubstrateCapability::ContainerOrchestration],
        metadata,
    };

    assert_eq!(
        substrate.get_metadata("version"),
        Some(&"1.20.0".to_string())
    );
    assert_eq!(substrate.get_metadata("provider"), Some(&"k8s".to_string()));
    assert_eq!(substrate.get_metadata("missing"), None);
}

#[test]
fn test_detected_substrate_serialization() {
    let substrate = DetectedSubstrate {
        substrate_type: SubstrateType::Cloud,
        capabilities: vec![
            SubstrateCapability::CloudCompute,
            SubstrateCapability::ServiceDiscovery,
        ],
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&substrate).expect("Failed to serialize");
    let deserialized: DetectedSubstrate =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.substrate_type, SubstrateType::Cloud);
    assert_eq!(deserialized.capabilities.len(), 2);
}

#[test]
fn test_endpoint_resolver_creation() {
    let resolver = EndpointResolver::new();
    assert_eq!(resolver.sources.len(), 0);

    let default_resolver = EndpointResolver::default();
    assert_eq!(default_resolver.sources.len(), 0);
}

#[test]
fn test_discovery_error_variants() {
    let err1 = DiscoveryError::CapabilityNotFound("test".to_string());
    let err2 = DiscoveryError::Timeout(Duration::from_secs(30));
    let err3 = DiscoveryError::NoHealthyServices("test".to_string());
    let err4 = DiscoveryError::ProtocolNotSupported("mqtt".to_string());
    let err5 = DiscoveryError::SourceFailed("mdns error".to_string());
    let err6 = DiscoveryError::ConfigError("invalid config".to_string());

    assert_eq!(err1.to_string(), "Capability 'test' not found");
    assert!(err2.to_string().contains("Discovery timeout"));
    assert!(err3.to_string().contains("No healthy services"));
    assert!(err4.to_string().contains("Protocol 'mqtt' not supported"));
    assert!(err5.to_string().contains("Discovery source failed"));
    assert!(err6.to_string().contains("Configuration error"));
}

#[test]
fn test_capability_constants() {
    // Verify all standard capabilities are defined with expected values
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

#[test]
fn test_service_metadata_priority_range() {
    let metadata = ServiceMetadata {
        version: None,
        health: ServiceHealth::Healthy,
        last_seen: std::time::SystemTime::now(),
        priority: 95,
        extra: HashMap::new(),
    };

    assert!(metadata.priority <= 100);
    // Note: priority is u8, so >= 0 check is redundant
}
