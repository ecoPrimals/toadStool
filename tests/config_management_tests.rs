//! Configuration management tests
//!
//! Tests configuration loading, validation, and overrides.

use toadstool_config::{ToadStoolConfig, NetworkConfig};

#[test]
fn test_config_default() {
    let config = ToadStoolConfig::default();
    assert!(config.network.is_some());
}

#[test]
fn test_network_config_creation() {
    let network = NetworkConfig {
        host: "localhost".to_string(),
        port: 8084,
        tls_enabled: false,
        timeout_seconds: 30,
    };
    
    assert_eq!(network.host, "localhost");
    assert_eq!(network.port, 8084);
}

#[test]
fn test_config_validation() {
    let config = ToadStoolConfig::default();
    let validation = config.validate();
    assert!(validation.is_ok(), "Default config should be valid");
}

#[test]
fn test_config_with_custom_network() {
    let mut config = ToadStoolConfig::default();
    config.network = Some(NetworkConfig {
        host: "0.0.0.0".to_string(),
        port: 9000,
        tls_enabled: true,
        timeout_seconds: 60,
    });
    
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_serialization() {
    let config = ToadStoolConfig::default();
    let json = serde_json::to_string(&config);
    assert!(json.is_ok());
}

#[test]
fn test_config_deserialization() {
    let json = r#"{
        "network": {
            "host": "127.0.0.1",
            "port": 8084,
            "tls_enabled": false,
            "timeout_seconds": 30
        }
    }"#;
    
    let config: Result<ToadStoolConfig, _> = serde_json::from_str(json);
    assert!(config.is_ok());
}

#[test]
fn test_config_cloning() {
    let config = ToadStoolConfig::default();
    let cloned = config.clone();
    assert_eq!(config.network, cloned.network);
}

#[test]
fn test_network_config_validation() {
    let network = NetworkConfig {
        host: "localhost".to_string(),
        port: 8084,
        tls_enabled: false,
        timeout_seconds: 30,
    };
    
    assert!(!network.host.is_empty());
    assert!(network.port > 0);
    assert!(network.timeout_seconds > 0);
}

