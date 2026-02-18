//! Tests for protocol configuration types

use std::time::Duration;
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_integration_protocols::config::*;
use toadstool_integration_protocols::types::*;

// ============================================================================
// ProtocolConfig Tests
// ============================================================================

#[test]
fn test_protocol_config_default() {
    let config = ProtocolConfig::default();

    assert!(!config.service_id.is_empty());
    assert_eq!(config.default_format, MessageFormat::Json);
    assert!(config.supported_transports.contains(&TransportType::Http));
    assert!(config.supported_transports.contains(&TransportType::TRpc));
}

#[test]
fn test_protocol_config_custom_service_id() {
    let config = ProtocolConfig {
        service_id: "my-service".to_string(),
        default_format: MessageFormat::MessagePack,
        supported_transports: vec![TransportType::TRpc],
        auth_config: None,
        request_timeout: Duration::from_secs(60),
        connection_pool: ConnectionPoolConfig::default(),
        discovery_config: None,
        routing_config: RoutingConfig::default(),
        health_config: HealthConfig::default(),
    };

    assert_eq!(config.service_id, "my-service");
    assert_eq!(config.default_format, MessageFormat::MessagePack);
}

#[test]
fn test_protocol_config_with_auth() {
    let auth_config = ServiceAuthConfig::bearer("token123");

    let config = ProtocolConfig {
        service_id: "secure-service".to_string(),
        default_format: MessageFormat::Json,
        supported_transports: vec![TransportType::Http],
        auth_config: Some(auth_config),
        request_timeout: Duration::from_secs(30),
        connection_pool: ConnectionPoolConfig::default(),
        discovery_config: None,
        routing_config: RoutingConfig::default(),
        health_config: HealthConfig::default(),
    };

    assert!(config.auth_config.is_some());
}

#[test]
fn test_protocol_config_multiple_transports() {
    let config = ProtocolConfig {
        service_id: "multi-transport".to_string(),
        default_format: MessageFormat::Json,
        supported_transports: vec![
            TransportType::Http,
            TransportType::TRpc,
            TransportType::TRpc,
            TransportType::Tcp,
        ],
        auth_config: None,
        request_timeout: Duration::from_secs(45),
        connection_pool: ConnectionPoolConfig::default(),
        discovery_config: None,
        routing_config: RoutingConfig::default(),
        health_config: HealthConfig::default(),
    };

    assert_eq!(config.supported_transports.len(), 4);
}

#[test]
fn test_protocol_config_clone() {
    let config = ProtocolConfig::default();
    let cloned = config.clone();

    assert_eq!(config.service_id, cloned.service_id);
    assert_eq!(config.default_format, cloned.default_format);
}
