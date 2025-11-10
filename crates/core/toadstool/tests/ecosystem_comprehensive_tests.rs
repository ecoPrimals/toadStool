//! Comprehensive tests for ecosystem module
//!
//! Sprint 17: ecosystem.rs coverage 26.20% → 60%+
//! Target: 329-645 lines, ~35-45 comprehensive tests

use std::collections::HashMap;
use std::time::Duration;
use toadstool::ecosystem::*;
use toadstool::*;

// ============================================================================
// EcosystemConfig Tests
// ============================================================================

#[test]
fn test_ecosystem_config_default() {
    let config = EcosystemConfig::default();

    assert!(config.auto_discovery);
    assert_eq!(config.discovery_timeout, Duration::from_secs(30));
    assert!(config.primal_endpoints.is_empty());
    assert!(config.required_primals.is_empty());
    assert_eq!(config.optional_primals.len(), 5);
}

#[test]
fn test_ecosystem_config_optional_primals() {
    let config = EcosystemConfig::default();

    let expected_primals = vec!["songbird", "nestgate", "beardog", "squirrel", "biomeos"];
    for primal in expected_primals {
        assert!(config.optional_primals.contains(&primal.to_string()));
    }
}

#[test]
fn test_ecosystem_config_clone() {
    let config = EcosystemConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.auto_discovery, config.auto_discovery);
    assert_eq!(cloned.discovery_timeout, config.discovery_timeout);
}

#[test]
fn test_ecosystem_config_debug() {
    let config = EcosystemConfig::default();
    let debug = format!("{:?}", config);

    assert!(!debug.is_empty());
    assert!(debug.contains("EcosystemConfig"));
}

#[test]
fn test_ecosystem_config_custom() {
    let mut endpoints = HashMap::new();
    endpoints.insert("songbird".to_string(), "http://localhost:8080".to_string());

    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(60),
        primal_endpoints: endpoints.clone(),
        required_primals: vec!["songbird".to_string()],
        optional_primals: vec![],
    };

    assert!(!config.auto_discovery);
    assert_eq!(config.discovery_timeout, Duration::from_secs(60));
    assert_eq!(config.primal_endpoints.len(), 1);
    assert_eq!(config.required_primals.len(), 1);
}

#[test]
fn test_ecosystem_config_serialization() {
    let config = EcosystemConfig::default();
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

#[test]
fn test_ecosystem_config_deserialization() {
    let json = r#"{
        "auto_discovery": true,
        "discovery_timeout": {"secs": 30, "nanos": 0},
        "primal_endpoints": {},
        "required_primals": [],
        "optional_primals": ["songbird"]
    }"#;

    let config: Result<EcosystemConfig, _> = serde_json::from_str(json);
    assert!(config.is_ok());
}

// ============================================================================
// PrimalType Tests
// ============================================================================

#[test]
fn test_primal_type_variants() {
    let songbird = PrimalType::Songbird;
    let nestgate = PrimalType::NestGate;
    let beardog = PrimalType::BearDog;
    let squirrel = PrimalType::Squirrel;
    let biomeos = PrimalType::BiomeOS;
    let toadstool = PrimalType::ToadStool;
    let custom = PrimalType::Custom("CustomPrimal".to_string());

    assert_eq!(songbird, PrimalType::Songbird);
    assert_eq!(nestgate, PrimalType::NestGate);
    assert_eq!(beardog, PrimalType::BearDog);
    assert_eq!(squirrel, PrimalType::Squirrel);
    assert_eq!(biomeos, PrimalType::BiomeOS);
    assert_eq!(toadstool, PrimalType::ToadStool);
    assert_eq!(custom, PrimalType::Custom("CustomPrimal".to_string()));
}

#[test]
fn test_primal_type_equality() {
    assert_eq!(PrimalType::Songbird, PrimalType::Songbird);
    assert_ne!(PrimalType::Songbird, PrimalType::NestGate);
}

#[test]
fn test_primal_type_clone() {
    let primal = PrimalType::Songbird;
    let cloned = primal.clone();

    assert_eq!(cloned, primal);
}

#[test]
fn test_primal_type_debug() {
    let primal = PrimalType::Songbird;
    let debug = format!("{:?}", primal);

    assert!(!debug.is_empty());
}

#[test]
fn test_primal_type_serialization() {
    let primal = PrimalType::Songbird;
    let json = serde_json::to_string(&primal);

    assert!(json.is_ok());
}

#[test]
fn test_primal_type_custom_variant() {
    let custom1 = PrimalType::Custom("MyPrimal".to_string());
    let custom2 = PrimalType::Custom("MyPrimal".to_string());
    let custom3 = PrimalType::Custom("OtherPrimal".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

// ============================================================================
// PrimalStatus Tests
// ============================================================================

#[test]
fn test_primal_status_variants() {
    let discovered = PrimalStatus::Discovered;
    let connected = PrimalStatus::Connected;
    let failed = PrimalStatus::Failed("Connection timeout".to_string());
    let disconnected = PrimalStatus::Disconnected;

    assert_eq!(discovered, PrimalStatus::Discovered);
    assert_eq!(connected, PrimalStatus::Connected);
    assert_eq!(disconnected, PrimalStatus::Disconnected);

    match failed {
        PrimalStatus::Failed(msg) => assert_eq!(msg, "Connection timeout"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_primal_status_equality() {
    assert_eq!(PrimalStatus::Discovered, PrimalStatus::Discovered);
    assert_ne!(PrimalStatus::Discovered, PrimalStatus::Connected);
}

#[test]
fn test_primal_status_clone() {
    let status = PrimalStatus::Connected;
    let cloned = status.clone();

    assert_eq!(cloned, status);
}

#[test]
fn test_primal_status_debug() {
    let status = PrimalStatus::Connected;
    let debug = format!("{:?}", status);

    assert!(!debug.is_empty());
}

#[test]
fn test_primal_status_serialization() {
    let status = PrimalStatus::Connected;
    let json = serde_json::to_string(&status);

    assert!(json.is_ok());
}

#[test]
fn test_primal_status_failed_with_message() {
    let status = PrimalStatus::Failed("Network unreachable".to_string());

    match status {
        PrimalStatus::Failed(msg) => assert_eq!(msg, "Network unreachable"),
        _ => panic!("Expected Failed status"),
    }
}

// ============================================================================
// PrimalInstance Tests
// ============================================================================

#[test]
fn test_primal_instance_creation() {
    let instance = PrimalInstance {
        name: "songbird-1".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec!["networking".to_string(), "coordination".to_string()],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(instance.name, "songbird-1");
    assert_eq!(instance.primal_type, PrimalType::Songbird);
    assert_eq!(instance.endpoint, "http://localhost:8080");
    assert_eq!(instance.version, "0.1.0");
    assert_eq!(instance.capabilities.len(), 2);
    assert_eq!(instance.status, PrimalStatus::Connected);
}

#[test]
fn test_primal_instance_clone() {
    let instance = PrimalInstance {
        name: "nestgate-1".to_string(),
        primal_type: PrimalType::NestGate,
        endpoint: "http://localhost:9090".to_string(),
        version: "0.2.0".to_string(),
        capabilities: vec!["storage".to_string()],
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    let cloned = instance.clone();
    assert_eq!(cloned.name, instance.name);
    assert_eq!(cloned.primal_type, instance.primal_type);
}

#[test]
fn test_primal_instance_debug() {
    let instance = PrimalInstance {
        name: "test".to_string(),
        primal_type: PrimalType::BearDog,
        endpoint: "http://localhost:7070".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    let debug = format!("{:?}", instance);
    assert!(!debug.is_empty());
}

#[test]
fn test_primal_instance_serialization() {
    let instance = PrimalInstance {
        name: "test".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&instance);
    assert!(json.is_ok());
}

#[test]
fn test_primal_instance_with_multiple_capabilities() {
    let instance = PrimalInstance {
        name: "biomeos-1".to_string(),
        primal_type: PrimalType::BiomeOS,
        endpoint: "http://localhost:6060".to_string(),
        version: "2.0.0".to_string(),
        capabilities: vec![
            "os_layer".to_string(),
            "process_management".to_string(),
            "resource_isolation".to_string(),
        ],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(instance.capabilities.len(), 3);
    assert!(instance.capabilities.contains(&"os_layer".to_string()));
}

// ============================================================================
// EcosystemMessageType Tests
// ============================================================================

#[test]
fn test_ecosystem_message_type_variants() {
    let heartbeat = EcosystemMessageType::Heartbeat;
    let capability = EcosystemMessageType::CapabilityAnnouncement;
    let resource_req = EcosystemMessageType::ResourceRequest;
    let resource_res = EcosystemMessageType::ResourceResponse;
    let workload_req = EcosystemMessageType::WorkloadRequest;
    let workload_res = EcosystemMessageType::WorkloadResponse;
    let status = EcosystemMessageType::StatusUpdate;
    let error = EcosystemMessageType::Error;

    // Just ensure they can be created
    assert!(matches!(heartbeat, EcosystemMessageType::Heartbeat));
    assert!(matches!(
        capability,
        EcosystemMessageType::CapabilityAnnouncement
    ));
    assert!(matches!(
        resource_req,
        EcosystemMessageType::ResourceRequest
    ));
    assert!(matches!(
        resource_res,
        EcosystemMessageType::ResourceResponse
    ));
    assert!(matches!(
        workload_req,
        EcosystemMessageType::WorkloadRequest
    ));
    assert!(matches!(
        workload_res,
        EcosystemMessageType::WorkloadResponse
    ));
    assert!(matches!(status, EcosystemMessageType::StatusUpdate));
    assert!(matches!(error, EcosystemMessageType::Error));
}

#[test]
fn test_ecosystem_message_type_clone() {
    let msg_type = EcosystemMessageType::Heartbeat;
    let cloned = msg_type.clone();

    assert!(matches!(cloned, EcosystemMessageType::Heartbeat));
}

#[test]
fn test_ecosystem_message_type_debug() {
    let msg_type = EcosystemMessageType::ResourceRequest;
    let debug = format!("{:?}", msg_type);

    assert!(!debug.is_empty());
}

#[test]
fn test_ecosystem_message_type_serialization() {
    let msg_type = EcosystemMessageType::Heartbeat;
    let json = serde_json::to_string(&msg_type);

    assert!(json.is_ok());
}

// ============================================================================
// EcosystemMessage Tests
// ============================================================================

#[test]
fn test_ecosystem_message_creation() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "toadstool-1".to_string(),
        to: "songbird-1".to_string(),
        message_type: EcosystemMessageType::Heartbeat,
        payload: serde_json::json!({"status": "alive"}),
        timestamp: chrono::Utc::now(),
    };

    assert_eq!(msg.from, "toadstool-1");
    assert_eq!(msg.to, "songbird-1");
    assert!(matches!(msg.message_type, EcosystemMessageType::Heartbeat));
}

#[test]
fn test_ecosystem_message_clone() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "sender".to_string(),
        to: "receiver".to_string(),
        message_type: EcosystemMessageType::StatusUpdate,
        payload: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
    };

    let cloned = msg.clone();
    assert_eq!(cloned.id, msg.id);
    assert_eq!(cloned.from, msg.from);
}

#[test]
fn test_ecosystem_message_debug() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "a".to_string(),
        to: "b".to_string(),
        message_type: EcosystemMessageType::Error,
        payload: serde_json::json!({"error": "test"}),
        timestamp: chrono::Utc::now(),
    };

    let debug = format!("{:?}", msg);
    assert!(!debug.is_empty());
}

#[test]
fn test_ecosystem_message_serialization() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "sender".to_string(),
        to: "receiver".to_string(),
        message_type: EcosystemMessageType::ResourceRequest,
        payload: serde_json::json!({"cpu": 2, "memory": "4GB"}),
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());
}

#[test]
fn test_ecosystem_message_with_complex_payload() {
    let payload = serde_json::json!({
        "workload_id": "abc-123",
        "runtime": "wasm",
        "resources": {
            "cpu": 4,
            "memory": "8GB",
            "gpu": false
        }
    });

    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "songbird".to_string(),
        message_type: EcosystemMessageType::WorkloadRequest,
        payload,
        timestamp: chrono::Utc::now(),
    };

    assert!(msg.payload.get("workload_id").is_some());
    assert!(msg.payload.get("resources").is_some());
}

// ============================================================================
// EcosystemCoordinator Tests
// ============================================================================

#[test]
fn test_ecosystem_coordinator_new() {
    let result = EcosystemCoordinator::new();
    assert!(result.is_ok());
}

#[test]
fn test_ecosystem_coordinator_creation() {
    let coordinator = EcosystemCoordinator::new().unwrap();

    // Coordinator should be created successfully
    // Internal state is private, so we just verify construction works
    drop(coordinator);
}

// ============================================================================
// Serialization Round-trip Tests
// ============================================================================

#[test]
fn test_ecosystem_config_round_trip() {
    let original = EcosystemConfig::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: EcosystemConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.auto_discovery, original.auto_discovery);
    assert_eq!(deserialized.discovery_timeout, original.discovery_timeout);
}

#[test]
fn test_primal_type_round_trip() {
    let original = PrimalType::Songbird;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PrimalType = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_primal_type_custom_round_trip() {
    let original = PrimalType::Custom("TestPrimal".to_string());

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PrimalType = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_primal_status_round_trip() {
    let original = PrimalStatus::Connected;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PrimalStatus = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_primal_instance_round_trip() {
    let original = PrimalInstance {
        name: "test".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec!["test".to_string()],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: PrimalInstance = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, original.name);
    assert_eq!(deserialized.primal_type, original.primal_type);
}

#[test]
fn test_ecosystem_message_round_trip() {
    let original = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "a".to_string(),
        to: "b".to_string(),
        message_type: EcosystemMessageType::Heartbeat,
        payload: serde_json::json!({"test": "data"}),
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: EcosystemMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.from, original.from);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_ecosystem_config_with_empty_endpoints() {
    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(10),
        primal_endpoints: HashMap::new(),
        required_primals: vec![],
        optional_primals: vec![],
    };

    assert!(config.primal_endpoints.is_empty());
}

#[test]
fn test_primal_instance_with_no_capabilities() {
    let instance = PrimalInstance {
        name: "minimal".to_string(),
        primal_type: PrimalType::Custom("Minimal".to_string()),
        endpoint: "http://localhost:1111".to_string(),
        version: "0.0.1".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    assert!(instance.capabilities.is_empty());
}

#[test]
fn test_ecosystem_message_with_empty_payload() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "sender".to_string(),
        to: "receiver".to_string(),
        message_type: EcosystemMessageType::Heartbeat,
        payload: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
    };

    assert!(msg.payload.is_object());
}

#[test]
fn test_very_long_primal_name() {
    let long_name = "a".repeat(1000);
    let instance = PrimalInstance {
        name: long_name.clone(),
        primal_type: PrimalType::Custom("Test".to_string()),
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(instance.name.len(), 1000);
}
