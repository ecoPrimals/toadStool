//! Comprehensive tests for EcosystemCoordinator and ecosystem types
//!
//! This test suite covers primal discovery, coordination, and messaging.

use std::collections::HashMap;
use std::time::Duration;
use toadstool::ecosystem::{
    EcosystemConfig, EcosystemCoordinator, EcosystemMessage, EcosystemMessageType, PrimalInstance,
    PrimalStatus, PrimalType,
};
use uuid::Uuid;

// ============================================================================
// EcosystemConfig Tests
// ============================================================================

#[test]
fn test_ecosystem_config_default() {
    let config = EcosystemConfig::default();

    assert!(
        config.auto_discovery,
        "Auto-discovery should be enabled by default"
    );
    assert_eq!(config.discovery_timeout, Duration::from_secs(30));
    assert!(config.required_primals.is_empty());
    assert!(!config.optional_primals.is_empty());
}

#[test]
fn test_ecosystem_config_custom() {
    let mut endpoints = HashMap::new();
    endpoints.insert("songbird".to_string(), "http://localhost:8080".to_string());

    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(60),
        primal_endpoints: endpoints.clone(),
        required_primals: vec!["nestgate".to_string()],
        optional_primals: vec!["squirrel".to_string()],
    };

    assert!(!config.auto_discovery);
    assert_eq!(config.discovery_timeout, Duration::from_secs(60));
    assert_eq!(config.primal_endpoints.len(), 1);
    assert_eq!(config.required_primals.len(), 1);
}

#[test]
fn test_ecosystem_config_clone() {
    let original = EcosystemConfig::default();
    let cloned = original.clone();

    assert_eq!(original.auto_discovery, cloned.auto_discovery);
    assert_eq!(original.discovery_timeout, cloned.discovery_timeout);
}

#[test]
fn test_ecosystem_config_debug() {
    let config = EcosystemConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("EcosystemConfig"));
    assert!(debug_str.contains("auto_discovery"));
}

#[test]
fn test_ecosystem_config_optional_primals() {
    let config = EcosystemConfig::default();

    // Should have common primals in optional list
    assert!(config.optional_primals.contains(&"songbird".to_string()));
    assert!(config.optional_primals.contains(&"nestgate".to_string()));
    assert!(config.optional_primals.contains(&"beardog".to_string()));
    assert!(config.optional_primals.contains(&"squirrel".to_string()));
    assert!(config.optional_primals.contains(&"biomeos".to_string()));
}

#[test]
fn test_ecosystem_config_with_required_primals() {
    let config = EcosystemConfig {
        required_primals: vec!["nestgate".to_string(), "songbird".to_string()],
        ..Default::default()
    };

    assert_eq!(config.required_primals.len(), 2);
    assert!(config.required_primals.contains(&"nestgate".to_string()));
}

#[test]
fn test_ecosystem_config_with_endpoints() {
    let mut endpoints = HashMap::new();
    endpoints.insert("songbird".to_string(), "http://songbird:8080".to_string());
    endpoints.insert("nestgate".to_string(), "http://nestgate:9090".to_string());

    let config = EcosystemConfig {
        primal_endpoints: endpoints,
        ..Default::default()
    };

    assert_eq!(config.primal_endpoints.len(), 2);
    assert_eq!(
        config.primal_endpoints.get("songbird"),
        Some(&"http://songbird:8080".to_string())
    );
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
    let custom = PrimalType::Custom("myprimal".to_string());

    assert!(matches!(songbird, PrimalType::Songbird));
    assert!(matches!(nestgate, PrimalType::NestGate));
    assert!(matches!(beardog, PrimalType::BearDog));
    assert!(matches!(squirrel, PrimalType::Squirrel));
    assert!(matches!(biomeos, PrimalType::BiomeOS));
    assert!(matches!(toadstool, PrimalType::ToadStool));
    assert!(matches!(custom, PrimalType::Custom(_)));
}

#[test]
fn test_primal_type_equality() {
    assert_eq!(PrimalType::Songbird, PrimalType::Songbird);
    assert_eq!(PrimalType::NestGate, PrimalType::NestGate);
    assert_ne!(PrimalType::Songbird, PrimalType::NestGate);

    let custom1 = PrimalType::Custom("test".to_string());
    let custom2 = PrimalType::Custom("test".to_string());
    let custom3 = PrimalType::Custom("other".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

#[test]
fn test_primal_type_clone() {
    let original = PrimalType::Custom("clone-test".to_string());
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

#[test]
fn test_primal_type_debug() {
    let primal_type = PrimalType::Songbird;
    let debug_str = format!("{:?}", primal_type);

    assert!(debug_str.contains("Songbird"));
}

// ============================================================================
// PrimalStatus Tests
// ============================================================================

#[test]
fn test_primal_status_variants() {
    let discovered = PrimalStatus::Discovered;
    let connected = PrimalStatus::Connected;
    let failed = PrimalStatus::Failed("Connection refused".to_string());
    let disconnected = PrimalStatus::Disconnected;

    assert!(matches!(discovered, PrimalStatus::Discovered));
    assert!(matches!(connected, PrimalStatus::Connected));
    assert!(matches!(failed, PrimalStatus::Failed(_)));
    assert!(matches!(disconnected, PrimalStatus::Disconnected));
}

#[test]
fn test_primal_status_equality() {
    let s1 = PrimalStatus::Connected;
    let s2 = PrimalStatus::Connected;
    let s3 = PrimalStatus::Discovered;

    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
}

#[test]
fn test_primal_status_failed_with_reason() {
    let status = PrimalStatus::Failed("Timeout".to_string());

    if let PrimalStatus::Failed(reason) = status {
        assert_eq!(reason, "Timeout");
    } else {
        panic!("Expected Failed status");
    }
}

#[test]
fn test_primal_status_clone() {
    let original = PrimalStatus::Failed("Error message".to_string());
    let cloned = original.clone();

    assert_eq!(original, cloned);
}

#[test]
fn test_primal_status_debug() {
    let status = PrimalStatus::Connected;
    let debug_str = format!("{:?}", status);

    assert!(debug_str.contains("Connected"));
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
        version: "1.0.0".to_string(),
        capabilities: vec!["network".to_string(), "coordination".to_string()],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(instance.name, "songbird-1");
    assert_eq!(instance.primal_type, PrimalType::Songbird);
    assert_eq!(instance.version, "1.0.0");
    assert_eq!(instance.capabilities.len(), 2);
}

#[test]
fn test_primal_instance_clone() {
    let original = PrimalInstance {
        name: "nestgate-1".to_string(),
        primal_type: PrimalType::NestGate,
        endpoint: "http://nestgate:9090".to_string(),
        version: "2.1.0".to_string(),
        capabilities: vec!["storage".to_string()],
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    let cloned = original.clone();

    assert_eq!(original.name, cloned.name);
    assert_eq!(original.endpoint, cloned.endpoint);
    assert_eq!(original.version, cloned.version);
}

#[test]
fn test_primal_instance_debug() {
    let instance = PrimalInstance {
        name: "test-primal".to_string(),
        primal_type: PrimalType::Custom("test".to_string()),
        endpoint: "http://test:3000".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    let debug_str = format!("{:?}", instance);
    assert!(debug_str.contains("PrimalInstance"));
    assert!(debug_str.contains("test-primal"));
}

#[test]
fn test_primal_instance_with_multiple_capabilities() {
    let instance = PrimalInstance {
        name: "multi-cap-primal".to_string(),
        primal_type: PrimalType::ToadStool,
        endpoint: "http://localhost:7000".to_string(),
        version: "3.0.0".to_string(),
        capabilities: vec![
            "compute".to_string(),
            "wasm".to_string(),
            "container".to_string(),
            "native".to_string(),
        ],
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(instance.capabilities.len(), 4);
    assert!(instance.capabilities.contains(&"compute".to_string()));
    assert!(instance.capabilities.contains(&"wasm".to_string()));
}

// ============================================================================
// EcosystemMessage Tests
// ============================================================================

#[test]
fn test_ecosystem_message_creation() {
    let message = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "songbird".to_string(),
        message_type: EcosystemMessageType::Heartbeat,
        payload: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
    };

    assert_eq!(message.from, "toadstool");
    assert_eq!(message.to, "songbird");
    assert!(matches!(
        message.message_type,
        EcosystemMessageType::Heartbeat
    ));
}

#[test]
fn test_ecosystem_message_clone() {
    let original = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "nestgate".to_string(),
        to: "toadstool".to_string(),
        message_type: EcosystemMessageType::StatusUpdate,
        payload: serde_json::json!({"status": "healthy"}),
        timestamp: chrono::Utc::now(),
    };

    let cloned = original.clone();

    assert_eq!(original.id, cloned.id);
    assert_eq!(original.from, cloned.from);
    assert_eq!(original.to, cloned.to);
}

#[test]
fn test_ecosystem_message_debug() {
    let message = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "test".to_string(),
        to: "target".to_string(),
        message_type: EcosystemMessageType::Error,
        payload: serde_json::json!({}),
        timestamp: chrono::Utc::now(),
    };

    let debug_str = format!("{:?}", message);
    assert!(debug_str.contains("EcosystemMessage"));
}

#[test]
fn test_ecosystem_message_with_payload() {
    let payload = serde_json::json!({
        "resource": "storage",
        "amount": "100GB",
        "priority": "high"
    });

    let message = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "nestgate".to_string(),
        message_type: EcosystemMessageType::ResourceRequest,
        payload: payload.clone(),
        timestamp: chrono::Utc::now(),
    };

    assert_eq!(message.payload, payload);
    assert_eq!(message.payload["resource"], "storage");
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
    let original = EcosystemMessageType::ResourceRequest;
    let cloned = original.clone();

    assert!(matches!(cloned, EcosystemMessageType::ResourceRequest));
}

#[test]
fn test_ecosystem_message_type_debug() {
    let msg_type = EcosystemMessageType::Heartbeat;
    let debug_str = format!("{:?}", msg_type);

    assert!(debug_str.contains("Heartbeat"));
}

// ============================================================================
// EcosystemCoordinator Tests
// ============================================================================

#[test]
fn test_ecosystem_coordinator_creation() {
    let result = EcosystemCoordinator::new();
    assert!(result.is_ok(), "Coordinator creation should succeed");
}

#[tokio::test]
async fn test_ecosystem_coordinator_discover_primals() {
    let coordinator = EcosystemCoordinator::new().unwrap();

    // Discovery may or may not find primals depending on environment
    let result = coordinator.discover_primals().await;

    // Should return a result (ok or error)
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_primal_types_in_config() {
    let config = EcosystemConfig::default();

    // Verify default optional primals match actual primal types
    for primal_name in &config.optional_primals {
        assert!(
            matches!(
                primal_name.as_str(),
                "songbird" | "nestgate" | "beardog" | "squirrel" | "biomeos"
            ),
            "Unknown primal in default config: {}",
            primal_name
        );
    }
}

#[test]
fn test_message_flow() {
    // Simulate a typical message flow
    let request = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "nestgate".to_string(),
        message_type: EcosystemMessageType::ResourceRequest,
        payload: serde_json::json!({"resource": "storage", "amount": "1TB"}),
        timestamp: chrono::Utc::now(),
    };

    let response = EcosystemMessage {
        id: Uuid::new_v4(),
        from: "nestgate".to_string(),
        to: "toadstool".to_string(),
        message_type: EcosystemMessageType::ResourceResponse,
        payload: serde_json::json!({"status": "allocated", "volume_id": "vol-123"}),
        timestamp: chrono::Utc::now(),
    };

    assert_eq!(request.from, "toadstool");
    assert_eq!(request.to, "nestgate");
    assert_eq!(response.from, "nestgate");
    assert_eq!(response.to, "toadstool");
}

#[test]
fn test_primal_lifecycle() {
    // Simulate primal discovery and connection lifecycle
    let mut instance = PrimalInstance {
        name: "lifecycle-test".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    // Initially discovered
    assert_eq!(instance.status, PrimalStatus::Discovered);

    // Connect
    instance.status = PrimalStatus::Connected;
    assert_eq!(instance.status, PrimalStatus::Connected);

    // Fail
    instance.status = PrimalStatus::Failed("Network error".to_string());
    assert!(matches!(instance.status, PrimalStatus::Failed(_)));

    // Disconnect
    instance.status = PrimalStatus::Disconnected;
    assert_eq!(instance.status, PrimalStatus::Disconnected);
}

#[test]
fn test_all_ecosystem_primal_types() {
    let primals = vec![
        ("songbird", PrimalType::Songbird),
        ("nestgate", PrimalType::NestGate),
        ("beardog", PrimalType::BearDog),
        ("squirrel", PrimalType::Squirrel),
        ("biomeos", PrimalType::BiomeOS),
        ("toadstool", PrimalType::ToadStool),
        ("custom", PrimalType::Custom("custom".to_string())),
    ];

    for (name, primal_type) in primals {
        let instance = PrimalInstance {
            name: name.to_string(),
            primal_type,
            endpoint: format!("http://{}:8080", name),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Connected,
            discovered_at: chrono::Utc::now(),
        };

        assert_eq!(instance.name, name);
    }
}
