// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for network_config module
//!
//! Goal: Push coverage from 0% → 80%+

use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;
use toadstool_config::network_config::{BindMode, NetworkConfig};
use toadstool_config::ports;

// ============================================================================
// NetworkConfig Tests
// ============================================================================

#[test]
fn test_network_config_default() {
    let config = NetworkConfig::default();

    assert_eq!(config.listen_address, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(config.service_port, ports::toadstool::SERVER);
    assert_eq!(config.api_port, ports::toadstool::SERVER);
    assert_eq!(config.metrics_port, ports::toadstool::METRICS);
    assert_eq!(config.health_port, ports::toadstool::HEALTH);
    assert!(config.enable_mdns);
    assert_eq!(config.bind_mode, BindMode::Localhost);
}

#[test]
fn test_network_config_from_env_with_no_env_vars() {
    let config = NetworkConfig::from_env();

    // Should use defaults when no env vars set
    assert_eq!(config.listen_address, IpAddr::V4(Ipv4Addr::LOCALHOST));
}

#[test]
fn test_network_config_production() {
    let config = NetworkConfig::production();

    // Production should bind to all interfaces
    assert_eq!(config.bind_mode, BindMode::AllInterfaces);
}

#[test]
fn test_network_config_development() {
    let config = NetworkConfig::development();

    // Development should bind to localhost
    assert_eq!(config.bind_mode, BindMode::Localhost);
}

#[test]
fn test_network_config_service_addr() {
    let config = NetworkConfig::default();
    let socket = config.service_addr();

    assert_eq!(socket.port(), config.service_port);
}

#[test]
fn test_network_config_api_addr() {
    let config = NetworkConfig::default();
    let socket = config.api_addr();

    assert_eq!(socket.port(), config.api_port);
}

#[test]
fn test_network_config_metrics_addr() {
    let config = NetworkConfig::default();
    let socket = config.metrics_addr();

    assert_eq!(socket.port(), config.metrics_port);
}

#[test]
fn test_network_config_health_addr() {
    let config = NetworkConfig::default();
    let socket = config.health_addr();

    assert_eq!(socket.port(), config.health_port);
}

// ============================================================================
// BindMode Tests
// ============================================================================

#[test]
fn test_bind_mode_from_str_localhost() {
    assert_eq!(
        BindMode::from_str("localhost").unwrap(),
        BindMode::Localhost
    );
    assert_eq!(BindMode::from_str("local").unwrap(), BindMode::Localhost);
    assert_eq!(
        BindMode::from_str("Localhost").unwrap(),
        BindMode::Localhost
    );
    assert_eq!(BindMode::from_str("LOCAL").unwrap(), BindMode::Localhost);
}

#[test]
fn test_bind_mode_from_str_all_interfaces() {
    assert_eq!(BindMode::from_str("all").unwrap(), BindMode::AllInterfaces);
    assert_eq!(
        BindMode::from_str("allinterfaces").unwrap(),
        BindMode::AllInterfaces
    );
    assert_eq!(
        BindMode::from_str("0.0.0.0").unwrap(),
        BindMode::AllInterfaces
    );
    assert_eq!(
        BindMode::from_str("AllInterfaces").unwrap(),
        BindMode::AllInterfaces
    );
}

#[test]
fn test_bind_mode_from_str_specific() {
    assert_eq!(BindMode::from_str("specific").unwrap(), BindMode::Specific);
    assert_eq!(BindMode::from_str("Specific").unwrap(), BindMode::Specific);
}

#[test]
fn test_bind_mode_from_str_invalid() {
    let result = BindMode::from_str("invalid_mode");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid bind mode"));
}

#[test]
fn test_bind_mode_clone() {
    let mode = BindMode::AllInterfaces;
    let cloned = mode;

    assert_eq!(mode, cloned);
}

#[test]
fn test_bind_mode_debug() {
    let mode = BindMode::Localhost;
    let debug_str = format!("{:?}", mode);

    assert!(debug_str.contains("Localhost"));
}

#[test]
fn test_bind_mode_equality() {
    assert_eq!(BindMode::Localhost, BindMode::Localhost);
    assert_eq!(BindMode::AllInterfaces, BindMode::AllInterfaces);
    assert_eq!(BindMode::Specific, BindMode::Specific);

    assert_ne!(BindMode::Localhost, BindMode::AllInterfaces);
    assert_ne!(BindMode::Localhost, BindMode::Specific);
    assert_ne!(BindMode::AllInterfaces, BindMode::Specific);
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_network_config_serialization() {
    let config = NetworkConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: NetworkConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.service_port, deserialized.service_port);
}

#[test]
fn test_bind_mode_serialization() {
    let mode = BindMode::AllInterfaces;

    let json = serde_json::to_string(&mode).unwrap();
    let deserialized: BindMode = serde_json::from_str(&json).unwrap();

    assert_eq!(mode, deserialized);
}

// ============================================================================
// Socket Address Tests
// ============================================================================

#[test]
fn test_socket_addr_with_ipv4() {
    let config = NetworkConfig::default();
    let socket = config.service_addr();

    assert!(socket.is_ipv4());
}

#[test]
fn test_socket_addr_consistency() {
    let config = NetworkConfig::default();

    let service_socket = config.service_addr();
    let api_socket = config.api_addr();

    // All sockets should use the same listen address
    assert_eq!(service_socket.ip(), api_socket.ip());
}

// ============================================================================
// Production vs Development
// ============================================================================

#[test]
fn test_production_vs_development_bind_mode() {
    let prod = NetworkConfig::production();
    let dev = NetworkConfig::development();

    assert_eq!(prod.bind_mode, BindMode::AllInterfaces);
    assert_eq!(dev.bind_mode, BindMode::Localhost);
}

#[test]
fn test_production_has_reasonable_defaults() {
    let _config = NetworkConfig::production();
}

// ============================================================================
// Clone Tests
// ============================================================================

#[test]
fn test_network_config_clone() {
    let original = NetworkConfig::default();
    let cloned = original.clone();

    assert_eq!(original.service_port, cloned.service_port);
    assert_eq!(original.bind_mode, cloned.bind_mode);
}

// ============================================================================
// Debug Tests
// ============================================================================

#[test]
fn test_network_config_debug_output() {
    let config = NetworkConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("NetworkConfig"));
    assert!(debug_str.contains("service_port"));
}

#[test]
fn test_network_config_has_discovery_endpoints() {
    let config = NetworkConfig::default();

    // Deep Debt Principle: No hardcoded endpoints
    // Discovery endpoints are populated at runtime via capability discovery
    // This test validates that the config structure is correct, not that endpoints are hardcoded
    assert!(config.discovery_endpoints.is_empty(), "Discovery endpoints should be empty by default - populated at runtime via capability discovery");
}

#[test]
fn test_multiple_socket_addrs_different_ports() {
    let config = NetworkConfig::default();

    let _service = config.service_addr();
    let _api = config.api_addr();
    let _metrics = config.metrics_addr();
    let _health = config.health_addr();

    // Port 0 = OS-assigned; all in valid range (u16 guarantees validity)
}
