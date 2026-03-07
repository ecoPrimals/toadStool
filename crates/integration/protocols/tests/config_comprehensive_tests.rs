// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Protocol Configuration
//!
//! This test suite provides extensive coverage of protocol configuration structures
//! including default values, validation, and configuration patterns.

use std::time::Duration;
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_integration_protocols::config::*;
use toadstool_integration_protocols::types::{MessageFormat, TransportType};

// ============================================================================
// ProtocolConfig Tests
// ============================================================================

#[test]
fn test_protocol_config_default() {
    let config = ProtocolConfig::default();

    assert!(!config.service_id.is_empty());
    assert!(config.service_id.starts_with("toadstool-"));
    assert!(matches!(config.default_format, MessageFormat::Json));
    assert_eq!(config.request_timeout, Duration::from_secs(30));
}

#[test]
fn test_protocol_config_default_transports() {
    let config = ProtocolConfig::default();

    assert_eq!(config.supported_transports.len(), 2);
    assert!(config.supported_transports.contains(&TransportType::Http));
    assert!(config.supported_transports.contains(&TransportType::TRpc));
}

#[test]
fn test_protocol_config_no_auth_by_default() {
    let config = ProtocolConfig::default();
    assert!(config.auth_config.is_none());
}

#[test]
fn test_protocol_config_no_discovery_by_default() {
    let config = ProtocolConfig::default();
    assert!(config.discovery_config.is_none());
}

#[test]
fn test_protocol_config_has_connection_pool() {
    let config = ProtocolConfig::default();
    assert!(config.connection_pool.max_connections_per_service > 0);
}

#[test]
fn test_protocol_config_has_routing_config() {
    let config = ProtocolConfig::default();
    // RoutingConfig should exist
    let _ = &config.routing_config;
}

#[test]
fn test_protocol_config_has_health_config() {
    let config = ProtocolConfig::default();
    // HealthConfig should exist
    let _ = &config.health_config;
}

#[test]
fn test_protocol_config_clone() {
    let config1 = ProtocolConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.service_id, config2.service_id);
    assert_eq!(config1.request_timeout, config2.request_timeout);
}

// ============================================================================
// ConnectionPoolConfig Tests
// ============================================================================

#[test]
fn test_connection_pool_config_default() {
    let config = ConnectionPoolConfig::default();

    assert_eq!(config.max_connections_per_service, 10);
    assert_eq!(config.idle_timeout, Duration::from_secs(300));
    assert_eq!(config.keep_alive_interval, Duration::from_secs(30));
    assert_eq!(config.max_concurrent_requests, 100);
}

#[test]
fn test_connection_pool_config_max_connections_positive() {
    let config = ConnectionPoolConfig::default();
    assert!(config.max_connections_per_service > 0);
}

#[test]
fn test_connection_pool_config_idle_timeout_reasonable() {
    let config = ConnectionPoolConfig::default();
    // Idle timeout should be at least 1 second
    assert!(config.idle_timeout >= Duration::from_secs(1));
}

#[test]
fn test_connection_pool_config_keep_alive_less_than_idle() {
    let config = ConnectionPoolConfig::default();
    // Keep alive should be less than idle timeout
    assert!(config.keep_alive_interval < config.idle_timeout);
}

#[test]
fn test_connection_pool_config_max_concurrent_reasonable() {
    let config = ConnectionPoolConfig::default();
    assert!(config.max_concurrent_requests >= 10);
}

#[test]
fn test_connection_pool_config_clone() {
    let config1 = ConnectionPoolConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.max_connections_per_service,
        config2.max_connections_per_service
    );
    assert_eq!(config1.idle_timeout, config2.idle_timeout);
}

// ============================================================================
// ServiceAuthConfig Tests
// ============================================================================

#[test]
fn test_auth_config_bearer_token() {
    let config = ServiceAuthConfig::bearer("test-token");

    assert!(matches!(
        config.auth_type,
        toadstool_common::AuthType::Bearer
    ));
    assert_eq!(config.credentials.token, Some("test-token".to_string()));
}

#[test]
fn test_auth_config_api_key() {
    let config = ServiceAuthConfig::api_key("api-key-123");

    assert!(matches!(
        config.auth_type,
        toadstool_common::AuthType::ApiKey
    ));
    assert!(config.credentials.api_key.is_some());
}

#[test]
fn test_auth_config_mtls_paths() {
    let config = ServiceAuthConfig::mtls(
        "/path/to/cert.pem",
        "/path/to/key.pem",
        Some("/path/to/ca.pem".to_string()),
    );

    assert!(matches!(
        config.auth_type,
        toadstool_common::AuthType::MutualTLS
    ));
    assert!(config.credentials.cert_path.is_some());
    assert!(config.credentials.key_path.is_some());
    assert!(config.credentials.ca_path.is_some());
}

#[test]
fn test_auth_config_none() {
    let config = ServiceAuthConfig::none();

    assert!(matches!(config.auth_type, toadstool_common::AuthType::None));
    assert!(config.credentials.token.is_none());
}

#[test]
fn test_auth_config_clone() {
    let config1 = ServiceAuthConfig::bearer("token");

    let config2 = config1.clone();
    assert_eq!(config1.credentials.token, config2.credentials.token);
}

// ============================================================================
// Configuration Pattern Tests
// ============================================================================

#[test]
fn test_protocol_config_with_auth() {
    let config = ProtocolConfig {
        auth_config: Some(ServiceAuthConfig::bearer("test")),
        ..Default::default()
    };

    assert!(config.auth_config.is_some());
}

#[test]
fn test_protocol_config_timeout_customization() {
    let config = ProtocolConfig {
        request_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    assert_eq!(config.request_timeout, Duration::from_secs(60));
}

#[test]
fn test_protocol_config_format_customization() {
    let config = ProtocolConfig {
        default_format: MessageFormat::MessagePack,
        ..Default::default()
    };

    assert!(matches!(config.default_format, MessageFormat::MessagePack));
}

#[test]
fn test_protocol_config_transport_customization() {
    let config = ProtocolConfig {
        supported_transports: vec![TransportType::TRpc],
        ..Default::default()
    };

    assert_eq!(config.supported_transports.len(), 1);
    assert!(config.supported_transports.contains(&TransportType::TRpc));
}

#[test]
fn test_connection_pool_config_customization() {
    let config = ConnectionPoolConfig {
        max_connections_per_service: 20,
        max_concurrent_requests: 200,
        ..Default::default()
    };

    assert_eq!(config.max_connections_per_service, 20);
    assert_eq!(config.max_concurrent_requests, 200);
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_config_coverage_summary() {
    println!("============================================");
    println!("Protocol Config Tests Summary:");
    println!("============================================");
    println!("ProtocolConfig:          8 tests");
    println!("ConnectionPoolConfig:    6 tests");
    println!("ServiceAuthConfig:              5 tests");
    println!("Pattern Tests:           5 tests");
    println!("============================================");
    println!("Total Config Tests:     24 tests");
    println!("============================================");
}
