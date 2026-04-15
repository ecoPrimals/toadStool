// SPDX-License-Identifier: AGPL-3.0-or-later
// ============================================================================
// Network Configuration Tests
// ============================================================================

use toadstool_security_sandbox::*;

#[test]
fn test_network_config_disabled() {
    let config = NetworkConfig {
        enabled: false,
        isolation_mode: NetworkIsolationMode::Isolated,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec![],
        bandwidth_limits: None,
    };

    assert!(!config.enabled);
    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Isolated
    ));
}

#[test]
fn test_network_config_enabled() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec!["api.example.com".to_string()],
        allowed_ports: vec![443],
        dns_servers: vec!["8.8.8.8".to_string()],
        bandwidth_limits: None,
    };

    assert!(config.enabled);
    assert_eq!(config.allowed_hosts.len(), 1);
    assert_eq!(config.allowed_ports.len(), 1);
}

#[test]
fn test_network_isolation_mode_none() {
    let mode = NetworkIsolationMode::None;
    assert!(matches!(mode, NetworkIsolationMode::None));
}

#[test]
fn test_network_isolation_mode_firewall() {
    let mode = NetworkIsolationMode::Firewall;
    assert!(matches!(mode, NetworkIsolationMode::Firewall));
}

#[test]
fn test_network_isolation_mode_namespace() {
    let mode = NetworkIsolationMode::Namespace;
    assert!(matches!(mode, NetworkIsolationMode::Namespace));
}

#[test]
fn test_network_isolation_mode_isolated() {
    let mode = NetworkIsolationMode::Isolated;
    assert!(matches!(mode, NetworkIsolationMode::Isolated));
}

