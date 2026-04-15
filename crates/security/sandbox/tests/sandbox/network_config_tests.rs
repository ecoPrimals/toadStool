// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;

#[test]
fn test_network_config_isolated() {
    let config = NetworkConfig {
        enabled: false,
        isolation_mode: NetworkIsolationMode::Isolated,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec![],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Isolated
    ));
    assert!(config.allowed_hosts.is_empty());
}

#[test]
fn test_network_config_firewall() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![
            "api.example.com".to_string(),
            "data.example.com".to_string(),
        ],
        allowed_ports: vec![443, 8443],
        dns_servers: vec!["8.8.8.8".to_string()],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Firewall
    ));
    assert_eq!(config.allowed_hosts.len(), 2);
    assert_eq!(config.allowed_ports.len(), 2);
}

#[test]
fn test_network_config_namespace() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Namespace,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Namespace
    ));
    assert_eq!(config.dns_servers.len(), 2);
}

#[test]
fn test_network_config_explicitly_enabled() {
    let config = NetworkConfig {
        enabled: true,
        ..Default::default()
    };

    assert!(config.enabled);
}

#[test]
fn test_network_config_custom_dns() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec![
            "192.168.1.1".to_string(),
            "192.168.1.2".to_string(),
            "192.168.1.3".to_string(),
        ],
        bandwidth_limits: None,
    };

    assert_eq!(config.dns_servers.len(), 3);
}

#[test]
fn test_network_config_allowed_ports_range() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![],
        allowed_ports: vec![80, 443, 8000, 8080, 8443],
        dns_servers: vec![],
        bandwidth_limits: None,
    };

    assert_eq!(config.allowed_ports.len(), 5);
    assert!(config.allowed_ports.contains(&443));
    assert!(config.allowed_ports.contains(&8443));
}

#[test]
fn test_network_isolation_mode_variants() {
    let isolated = NetworkIsolationMode::Isolated;
    let firewall = NetworkIsolationMode::Firewall;
    let namespace = NetworkIsolationMode::Namespace;
    let none = NetworkIsolationMode::None;

    assert!(matches!(isolated, NetworkIsolationMode::Isolated));
    assert!(matches!(firewall, NetworkIsolationMode::Firewall));
    assert!(matches!(namespace, NetworkIsolationMode::Namespace));
    assert!(matches!(none, NetworkIsolationMode::None));
}
