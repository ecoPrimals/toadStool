//! Comprehensive tests for ecosystem integration

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use toadstool::ecosystem::*;

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
    assert!(config.optional_primals.contains(&"songbird".to_string()));
}

#[test]
fn test_ecosystem_config_custom() {
    let mut endpoints = HashMap::new();
    endpoints.insert("songbird".to_string(), "http://localhost:8080".to_string());

    let config = EcosystemConfig {
        auto_discovery: false,
        discovery_timeout: Duration::from_secs(60),
        primal_endpoints: endpoints,
        ..Default::default()
    };

    assert!(!config.auto_discovery);
    assert_eq!(config.discovery_timeout, Duration::from_secs(60));
    assert_eq!(config.primal_endpoints.len(), 1);
}

#[test]
fn test_ecosystem_config_with_required_primals() {
    let config = EcosystemConfig {
        required_primals: vec!["songbird".to_string(), "nestgate".to_string()],
        ..Default::default()
    };

    assert_eq!(config.required_primals.len(), 2);
}

#[test]
fn test_ecosystem_config_timeout_variations() {
    let config = EcosystemConfig {
        discovery_timeout: Duration::from_millis(500),
        ..Default::default()
    };

    assert!(config.discovery_timeout.as_millis() < 1000);
}

#[test]
fn test_ecosystem_config_with_all_optional_primals() {
    let config = EcosystemConfig::default();
    assert!(config.optional_primals.contains(&"nestgate".to_string()));
    assert!(config.optional_primals.contains(&"beardog".to_string()));
    assert!(config.optional_primals.contains(&"squirrel".to_string()));
    assert!(config.optional_primals.contains(&"biomeos".to_string()));
}

#[test]
fn test_ecosystem_config_with_all_required_primals() {
    let config = EcosystemConfig {
        required_primals: vec![
            "songbird".to_string(),
            "nestgate".to_string(),
            "beardog".to_string(),
            "squirrel".to_string(),
            "biomeos".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(config.required_primals.len(), 5);
}

#[test]
fn test_ecosystem_config_with_custom_endpoints() {
    let mut endpoints = HashMap::new();
    endpoints.insert("songbird".to_string(), "http://songbird:8080".to_string());
    endpoints.insert("nestgate".to_string(), "http://nestgate:8081".to_string());
    endpoints.insert("beardog".to_string(), "http://beardog:8082".to_string());

    let config = EcosystemConfig {
        primal_endpoints: endpoints,
        ..Default::default()
    };

    assert_eq!(config.primal_endpoints.len(), 3);
    assert_eq!(
        config.primal_endpoints.get("songbird").unwrap(),
        "http://songbird:8080"
    );
}

// ============================================================================
// PrimalType Tests
// ============================================================================

#[test]
fn test_primal_type_songbird() {
    let primal = PrimalType::Songbird;
    assert!(matches!(primal, PrimalType::Songbird));
}

#[test]
fn test_primal_type_nestgate() {
    let primal = PrimalType::NestGate;
    assert!(matches!(primal, PrimalType::NestGate));
}

#[test]
fn test_primal_type_beardog() {
    let primal = PrimalType::BearDog;
    assert!(matches!(primal, PrimalType::BearDog));
}

#[test]
fn test_primal_type_squirrel() {
    let primal = PrimalType::Squirrel;
    assert!(matches!(primal, PrimalType::Squirrel));
}

#[test]
fn test_primal_type_biomeos() {
    let primal = PrimalType::BiomeOS;
    assert!(matches!(primal, PrimalType::BiomeOS));
}

#[test]
fn test_primal_type_toadstool() {
    let primal = PrimalType::ToadStool;
    assert!(matches!(primal, PrimalType::ToadStool));
}

#[test]
fn test_primal_type_custom() {
    let primal = PrimalType::Custom("my-primal".to_string());

    if let PrimalType::Custom(name) = primal {
        assert_eq!(name, "my-primal");
    } else {
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_primal_type_clone() {
    let primal = PrimalType::Songbird;
    let cloned = primal.clone();
    assert!(matches!(cloned, PrimalType::Songbird));
}

#[test]
fn test_primal_type_debug() {
    let primal = PrimalType::Songbird;
    let debug_str = format!("{:?}", primal);
    assert!(debug_str.contains("Songbird"));
}

#[test]
fn test_primal_type_equality() {
    let primal1 = PrimalType::Songbird;
    let primal2 = PrimalType::Songbird;
    assert_eq!(primal1, primal2);
}

#[test]
fn test_primal_type_all_variants() {
    let types = [
        PrimalType::Songbird,
        PrimalType::NestGate,
        PrimalType::BearDog,
        PrimalType::Squirrel,
        PrimalType::BiomeOS,
        PrimalType::ToadStool,
        PrimalType::Custom("test".to_string()),
    ];

    assert_eq!(types.len(), 7);
}

// ============================================================================
// PrimalStatus Tests
// ============================================================================

#[test]
fn test_primal_status_discovered() {
    let status = PrimalStatus::Discovered;
    assert!(matches!(status, PrimalStatus::Discovered));
}

#[test]
fn test_primal_status_connected() {
    let status = PrimalStatus::Connected;
    assert!(matches!(status, PrimalStatus::Connected));
}

#[test]
fn test_primal_status_disconnected() {
    let status = PrimalStatus::Disconnected;
    assert!(matches!(status, PrimalStatus::Disconnected));
}

#[test]
fn test_primal_status_failed() {
    let status = PrimalStatus::Failed("Connection timeout".to_string());

    if let PrimalStatus::Failed(msg) = status {
        assert_eq!(msg, "Connection timeout");
    } else {
        panic!("Expected Failed variant");
    }
}

#[test]
fn test_primal_status_transitions() {
    let statuses = [
        PrimalStatus::Discovered,
        PrimalStatus::Connected,
        PrimalStatus::Disconnected,
        PrimalStatus::Failed("error".to_string()),
    ];

    assert_eq!(statuses.len(), 4);
}

#[test]
fn test_primal_status_equality() {
    let status1 = PrimalStatus::Connected;
    let status2 = PrimalStatus::Connected;
    assert_eq!(status1, status2);
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
        capabilities: vec!["messaging".to_string()],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert_eq!(instance.name, "songbird-1");
    assert_eq!(instance.capabilities.len(), 1);
    assert!(matches!(instance.status, PrimalStatus::Connected));
}

#[test]
fn test_primal_instance_multiple_capabilities() {
    let instance = PrimalInstance {
        name: "beardog-1".to_string(),
        primal_type: PrimalType::BearDog,
        endpoint: "http://localhost:8082".to_string(),
        version: "1.5.0".to_string(),
        capabilities: vec![
            "monitoring".to_string(),
            "alerting".to_string(),
            "metrics".to_string(),
        ],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert_eq!(instance.capabilities.len(), 3);
    assert!(instance.capabilities.contains(&"monitoring".to_string()));
}

#[test]
fn test_primal_instance_clone() {
    let instance = PrimalInstance {
        name: "squirrel-1".to_string(),
        primal_type: PrimalType::Squirrel,
        endpoint: "http://localhost:8083".to_string(),
        version: "3.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    let cloned = instance.clone();
    assert_eq!(instance.name, cloned.name);
    assert_eq!(instance.version, cloned.version);
}

#[test]
fn test_primal_instance_version_comparison() {
    let instance1 = PrimalInstance {
        name: "songbird-v1".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    let instance2 = PrimalInstance {
        name: "songbird-v2".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080".to_string(),
        version: "2.0.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert_ne!(instance1.version, instance2.version);
}

#[test]
fn test_primal_instance_different_types() {
    let instances = vec![
        PrimalInstance {
            name: "songbird-1".to_string(),
            primal_type: PrimalType::Songbird,
            endpoint: "http://localhost:8080".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Connected,
            discovered_at: Utc::now(),
        },
        PrimalInstance {
            name: "nestgate-1".to_string(),
            primal_type: PrimalType::NestGate,
            endpoint: "http://localhost:8081".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Connected,
            discovered_at: Utc::now(),
        },
        PrimalInstance {
            name: "beardog-1".to_string(),
            primal_type: PrimalType::BearDog,
            endpoint: "http://localhost:8082".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![],
            status: PrimalStatus::Connected,
            discovered_at: Utc::now(),
        },
    ];

    assert_eq!(instances.len(), 3);
}

// ============================================================================
// EcosystemCoordinator Tests
// ============================================================================

#[test]
fn test_ecosystem_coordinator_creation() {
    let coordinator = EcosystemCoordinator::new();
    assert!(coordinator.is_ok());
}

#[test]
fn test_ecosystem_coordinator_multiple_instances() {
    let coordinator1 = EcosystemCoordinator::new();
    let coordinator2 = EcosystemCoordinator::new();

    assert!(coordinator1.is_ok());
    assert!(coordinator2.is_ok());
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn test_primal_instance_empty_capabilities() {
    let instance = PrimalInstance {
        name: "minimal-primal".to_string(),
        primal_type: PrimalType::Custom("minimal".to_string()),
        endpoint: "http://localhost:9000".to_string(),
        version: "0.1.0".to_string(),
        capabilities: vec![],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert_eq!(instance.capabilities.len(), 0);
    assert!(instance.capabilities.is_empty());
}

#[test]
fn test_ecosystem_config_empty_endpoints() {
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: Duration::from_secs(10),
        primal_endpoints: HashMap::new(),
        required_primals: vec![],
        optional_primals: vec![],
    };

    assert!(config.primal_endpoints.is_empty());
    assert!(config.required_primals.is_empty());
    assert!(config.optional_primals.is_empty());
}

#[test]
fn test_primal_status_failed_with_long_message() {
    let long_error =
        "Connection timeout after multiple retry attempts with exponential backoff".to_string();
    let status = PrimalStatus::Failed(long_error.clone());

    if let PrimalStatus::Failed(msg) = status {
        assert_eq!(msg.len(), long_error.len());
        assert!(msg.contains("timeout"));
    }
}

#[test]
fn test_primal_instance_with_versioned_endpoint() {
    let instance = PrimalInstance {
        name: "versioned-primal".to_string(),
        primal_type: PrimalType::Songbird,
        endpoint: "http://localhost:8080/v2/api".to_string(),
        version: "2.1.3".to_string(),
        capabilities: vec!["v2".to_string()],
        status: PrimalStatus::Connected,
        discovered_at: Utc::now(),
    };

    assert!(instance.endpoint.contains("/v2/"));
    assert!(instance.version.starts_with("2."));
}

#[test]
fn test_ecosystem_config_long_timeout() {
    let config = EcosystemConfig {
        discovery_timeout: Duration::from_secs(300),
        ..Default::default()
    }; // 5 minutes

    assert!(config.discovery_timeout.as_secs() >= 300);
}

#[test]
fn test_primal_type_custom_with_special_chars() {
    let primal = PrimalType::Custom("my-custom-primal-123".to_string());

    if let PrimalType::Custom(name) = primal {
        assert!(name.contains("-"));
        assert!(name.contains("123"));
    }
}
