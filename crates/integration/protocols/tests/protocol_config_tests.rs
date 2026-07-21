// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for protocol configuration types

use std::sync::Arc;
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
        service_id: Arc::from("my-service"),
        default_format: MessageFormat::MessagePack,
        supported_transports: vec![TransportType::TRpc],
        auth_config: None,
        request_timeout: Duration::from_mins(1),
        connection_pool: ConnectionPoolConfig::default(),
        discovery_config: None,
        routing_config: RoutingConfig::default(),
        health_config: HealthConfig::default(),
    };

    assert_eq!(&*config.service_id, "my-service");
    assert_eq!(config.default_format, MessageFormat::MessagePack);
}

#[test]
fn test_protocol_config_with_auth() {
    let auth_config = ServiceAuthConfig::bearer("token123");

    let config = ProtocolConfig {
        service_id: Arc::from("secure-service"),
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
        service_id: Arc::from("multi-transport"),
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

    assert_eq!(&*config.service_id, &*cloned.service_id);
    assert_eq!(config.default_format, cloned.default_format);
}

#[test]
fn test_protocol_config_default_service_id_format() {
    let config = ProtocolConfig::default();
    assert!(config.service_id.starts_with("toadstool-"));
    assert!(config.service_id.len() > 10);
}

#[test]
fn test_protocol_config_default_request_timeout() {
    let config = ProtocolConfig::default();
    assert_eq!(config.request_timeout, Duration::from_secs(30));
}

// ============================================================================
// ConnectionPoolConfig Tests
// ============================================================================

#[test]
fn test_connection_pool_config_default() {
    let pool = ConnectionPoolConfig::default();
    assert_eq!(pool.max_connections_per_service, 10);
    assert_eq!(pool.idle_timeout, Duration::from_mins(5));
    assert_eq!(pool.keep_alive_interval, Duration::from_secs(30));
    assert_eq!(pool.max_concurrent_requests, 100);
}

#[test]
fn test_connection_pool_config_custom() {
    let pool = ConnectionPoolConfig {
        max_connections_per_service: 50,
        idle_timeout: Duration::from_mins(1),
        keep_alive_interval: Duration::from_secs(10),
        max_concurrent_requests: 200,
    };
    assert_eq!(pool.max_connections_per_service, 50);
    assert_eq!(pool.idle_timeout, Duration::from_mins(1));
}
