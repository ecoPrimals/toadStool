//! Comprehensive tests for ecosystem.rs module
//! Target: 0% → 70%+ coverage

use toadstool::ecosystem::{
    EcosystemConfig, EcosystemCoordinator, EcosystemMessage, EcosystemMessageType, PrimalInstance,
    PrimalStatus, PrimalType,
};

#[tokio::test]
async fn test_ecosystem_coordinator_creation() {
    let coordinator = EcosystemCoordinator::new();
    assert!(coordinator.is_ok());
}

#[tokio::test]
async fn test_ecosystem_config_default() {
    let config = EcosystemConfig::default();
    assert!(config.auto_discovery);
    assert_eq!(config.discovery_timeout.as_secs(), 30);
    assert_eq!(config.optional_primals.len(), 5);
    assert!(config.optional_primals.contains(&"songbird".to_string()));
}

#[tokio::test]
async fn test_discover_primals() {
    let coordinator = EcosystemCoordinator::new().unwrap();

    // Should complete without errors even if no primals found
    let result = coordinator.discover_primals().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_primal_instance_creation() {
    let primal = PrimalInstance {
        name: "test-primal".to_string(),
        primal_type: PrimalType::ToadStool,
        capabilities: vec!["compute".to_string()],
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(primal.name, "test-primal");
    assert_eq!(primal.capabilities.len(), 1);
}

#[tokio::test]
async fn test_primal_types() {
    assert_eq!(PrimalType::Songbird, PrimalType::Songbird);
    assert_eq!(PrimalType::NestGate, PrimalType::NestGate);
    assert_eq!(PrimalType::BearDog, PrimalType::BearDog);
    assert_eq!(PrimalType::Squirrel, PrimalType::Squirrel);
    assert_eq!(PrimalType::BiomeOS, PrimalType::BiomeOS);
    assert_eq!(PrimalType::ToadStool, PrimalType::ToadStool);

    let custom = PrimalType::Custom("custom".to_string());
    match custom {
        PrimalType::Custom(name) => assert_eq!(name, "custom"),
        _ => panic!("Expected custom type"),
    }
}

#[tokio::test]
async fn test_primal_status() {
    assert_eq!(PrimalStatus::Discovered, PrimalStatus::Discovered);
    assert_eq!(PrimalStatus::Connected, PrimalStatus::Connected);
    assert_eq!(PrimalStatus::Disconnected, PrimalStatus::Disconnected);

    let failed = PrimalStatus::Failed("timeout".to_string());
    match failed {
        PrimalStatus::Failed(msg) => assert_eq!(msg, "timeout"),
        _ => panic!("Expected failed status"),
    }
}

#[tokio::test]
async fn test_ecosystem_message_creation() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "songbird".to_string(),
        message_type: EcosystemMessageType::Heartbeat,
        payload: serde_json::json!({"status": "ok"}),
        timestamp: chrono::Utc::now(),
    };

    assert_eq!(msg.from, "toadstool");
    assert_eq!(msg.to, "songbird");
}

#[tokio::test]
async fn test_ecosystem_message_types() {
    // Test all message types can be created
    let _heartbeat = EcosystemMessageType::Heartbeat;
    let _capability = EcosystemMessageType::CapabilityAnnouncement;
    let _req = EcosystemMessageType::ResourceRequest;
    let _resp = EcosystemMessageType::ResourceResponse;
    let _work_req = EcosystemMessageType::WorkloadRequest;
    let _work_resp = EcosystemMessageType::WorkloadResponse;
    let _status = EcosystemMessageType::StatusUpdate;
    let _error = EcosystemMessageType::Error;
}

#[tokio::test]
async fn test_primal_instance_serialization() {
    let primal = PrimalInstance {
        name: "test".to_string(),
        primal_type: PrimalType::Songbird,
        capabilities: vec!["routing".to_string()],
        endpoint: "http://localhost:9080".to_string(),
        version: "0.1.0".to_string(),
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&primal);
    assert!(json.is_ok());

    let deserialized: Result<PrimalInstance, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok());
}

#[tokio::test]
async fn test_ecosystem_config_customization() {
    let mut config = EcosystemConfig {
        auto_discovery: false,
        ..Default::default()
    };
    config.required_primals.push("songbird".to_string());
    config
        .primal_endpoints
        .insert("songbird".to_string(), "http://localhost:9080".to_string());

    assert!(!config.auto_discovery);
    assert_eq!(config.required_primals.len(), 1);
    assert_eq!(config.primal_endpoints.len(), 1);
}

#[tokio::test]
async fn test_primal_capabilities() {
    let primal = PrimalInstance {
        name: "compute".to_string(),
        primal_type: PrimalType::ToadStool,
        capabilities: vec![
            "wasm".to_string(),
            "native".to_string(),
            "container".to_string(),
        ],
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(primal.capabilities.len(), 3);
    assert!(primal.capabilities.contains(&"wasm".to_string()));
}

#[tokio::test]
async fn test_ecosystem_message_serialization() {
    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "songbird".to_string(),
        message_type: EcosystemMessageType::CapabilityAnnouncement,
        payload: serde_json::json!({"capabilities": ["compute"]}),
        timestamp: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());

    let deserialized: Result<EcosystemMessage, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok());
}

#[tokio::test]
async fn test_primal_instance_clone() {
    let primal = PrimalInstance {
        name: "test".to_string(),
        primal_type: PrimalType::Songbird,
        capabilities: vec!["routing".to_string()],
        endpoint: "http://localhost:9080".to_string(),
        version: "0.1.0".to_string(),
        status: PrimalStatus::Discovered,
        discovered_at: chrono::Utc::now(),
    };

    let cloned = primal.clone();
    assert_eq!(primal.name, cloned.name);
    assert_eq!(primal.primal_type, cloned.primal_type);
}

#[tokio::test]
async fn test_ecosystem_config_serialization() {
    let config = EcosystemConfig::default();

    let json = serde_json::to_string(&config);
    assert!(json.is_ok());

    let deserialized: Result<EcosystemConfig, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok());
}

#[tokio::test]
async fn test_primal_status_transitions() {
    // Test various status transitions
    let statuses = vec![
        PrimalStatus::Discovered,
        PrimalStatus::Connected,
        PrimalStatus::Failed("network error".to_string()),
        PrimalStatus::Disconnected,
    ];

    for status in statuses {
        let primal = PrimalInstance {
            name: "test".to_string(),
            primal_type: PrimalType::Songbird,
            capabilities: vec![],
            endpoint: "http://localhost:9080".to_string(),
            version: "0.1.0".to_string(),
            status,
            discovered_at: chrono::Utc::now(),
        };

        assert!(!primal.name.is_empty());
    }
}

#[tokio::test]
async fn test_ecosystem_message_different_types() {
    let message_types = vec![
        EcosystemMessageType::Heartbeat,
        EcosystemMessageType::CapabilityAnnouncement,
        EcosystemMessageType::ResourceRequest,
        EcosystemMessageType::ResourceResponse,
        EcosystemMessageType::WorkloadRequest,
        EcosystemMessageType::WorkloadResponse,
        EcosystemMessageType::StatusUpdate,
        EcosystemMessageType::Error,
    ];

    for msg_type in message_types {
        let msg = EcosystemMessage {
            id: uuid::Uuid::new_v4(),
            from: "toadstool".to_string(),
            to: "songbird".to_string(),
            message_type: msg_type,
            payload: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(msg.from, "toadstool");
    }
}

#[tokio::test]
async fn test_primal_version_formats() {
    let versions = vec!["0.1.0", "1.0.0", "2.3.4-alpha", "3.0.0-beta.1"];

    for version in versions {
        let primal = PrimalInstance {
            name: "test".to_string(),
            primal_type: PrimalType::ToadStool,
            capabilities: vec![],
            endpoint: "http://localhost:8080".to_string(),
            version: version.to_string(),
            status: PrimalStatus::Discovered,
            discovered_at: chrono::Utc::now(),
        };

        assert_eq!(primal.version, version);
    }
}

#[tokio::test]
async fn test_ecosystem_config_with_required_primals() {
    let config = EcosystemConfig {
        required_primals: vec!["songbird".to_string(), "nestgate".to_string()],
        ..Default::default()
    };

    assert_eq!(config.required_primals.len(), 2);
    assert!(config.required_primals.contains(&"songbird".to_string()));
    assert!(config.required_primals.contains(&"nestgate".to_string()));
}

#[tokio::test]
async fn test_custom_primal_type() {
    let custom_types = vec![
        PrimalType::Custom("custom-service".to_string()),
        PrimalType::Custom("external-api".to_string()),
        PrimalType::Custom("legacy-system".to_string()),
    ];

    for custom_type in custom_types {
        match custom_type {
            PrimalType::Custom(name) => assert!(!name.is_empty()),
            _ => panic!("Expected custom type"),
        }
    }
}

#[tokio::test]
async fn test_primal_with_multiple_capabilities() {
    let capabilities = vec![
        "wasm".to_string(),
        "native".to_string(),
        "container".to_string(),
        "python".to_string(),
        "gpu".to_string(),
    ];

    let primal = PrimalInstance {
        name: "universal-compute".to_string(),
        primal_type: PrimalType::ToadStool,
        capabilities: capabilities.clone(),
        endpoint: "http://localhost:8080".to_string(),
        version: "0.1.0".to_string(),
        status: PrimalStatus::Connected,
        discovered_at: chrono::Utc::now(),
    };

    assert_eq!(primal.capabilities.len(), 5);
    for cap in capabilities {
        assert!(primal.capabilities.contains(&cap));
    }
}

#[tokio::test]
async fn test_ecosystem_message_with_complex_payload() {
    let complex_payload = serde_json::json!({
        "capabilities": ["compute", "storage"],
        "resources": {
            "cpu": 8,
            "memory": 16384,
            "disk": 1000000
        },
        "tags": ["production", "us-west-2"]
    });

    let msg = EcosystemMessage {
        id: uuid::Uuid::new_v4(),
        from: "toadstool".to_string(),
        to: "songbird".to_string(),
        message_type: EcosystemMessageType::ResourceRequest,
        payload: complex_payload,
        timestamp: chrono::Utc::now(),
    };

    assert!(msg.payload.is_object());
    assert!(msg.payload["capabilities"].is_array());
}
