// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI `network_config` types

use std::collections::HashMap;
use std::time::Duration;
use toadstool_cli::network_config::*;
use toadstool_common::config_bases::TelemetryConfig;

// ============================================================================
// SidecarResources Tests
// ============================================================================

#[test]
fn test_sidecar_resources_creation() {
    let resources = SidecarResources {
        cpu_limit: "2000m".to_string(),
        memory_limit: "2Gi".to_string(),
        cpu_request: "500m".to_string(),
        memory_request: "512Mi".to_string(),
    };

    assert_eq!(resources.cpu_limit, "2000m");
    assert_eq!(resources.memory_limit, "2Gi");
}

#[test]
fn test_sidecar_resources_minimal() {
    let resources = SidecarResources {
        cpu_limit: "100m".to_string(),
        memory_limit: "128Mi".to_string(),
        cpu_request: "50m".to_string(),
        memory_request: "64Mi".to_string(),
    };

    assert_eq!(resources.cpu_request, "50m");
    assert_eq!(resources.memory_request, "64Mi");
}

#[test]
fn test_sidecar_resources_high_performance() {
    let resources = SidecarResources {
        cpu_limit: "8000m".to_string(),
        memory_limit: "16Gi".to_string(),
        cpu_request: "4000m".to_string(),
        memory_request: "8Gi".to_string(),
    };

    assert_eq!(resources.cpu_limit, "8000m");
}

// ============================================================================
// ProxyConfig Tests
// ============================================================================

#[test]
fn test_proxy_config_envoy() {
    use toadstool_common::config_bases::TimeoutConfig;
    let config = ProxyConfig {
        proxy_type: "envoy".to_string(),
        listen_port: 15001,
        admin_port: 15000,
        concurrency: 4,
        timeouts: TimeoutConfig {
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_mins(1),
            read_timeout: Duration::from_secs(90),
            write_timeout: Duration::from_secs(90),
        },
    };

    assert_eq!(config.proxy_type, "envoy");
    assert_eq!(config.listen_port, 15001);
    assert_eq!(config.admin_port, 15000);
}

#[test]
fn test_proxy_config_nginx() {
    use toadstool_common::config_bases::TimeoutConfig;
    let config = ProxyConfig {
        proxy_type: "nginx".to_string(),
        listen_port: 8080,
        admin_port: 8081,
        concurrency: 8,
        timeouts: TimeoutConfig {
            connection_timeout: Duration::from_secs(15),
            request_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(90),
            write_timeout: Duration::from_secs(90),
        },
    };

    assert_eq!(config.proxy_type, "nginx");
    assert_eq!(config.concurrency, 8);
    assert_eq!(config.timeouts.connection_timeout, Duration::from_secs(15));
}

#[test]
fn test_proxy_config_haproxy() {
    use toadstool_common::config_bases::TimeoutConfig;
    let config = ProxyConfig {
        proxy_type: "haproxy".to_string(),
        listen_port: 9000,
        admin_port: 9001,
        concurrency: 16,
        timeouts: TimeoutConfig {
            connection_timeout: Duration::from_mins(1),
            request_timeout: Duration::from_mins(2),
            read_timeout: Duration::from_secs(180),
            write_timeout: Duration::from_secs(180),
        },
    };

    assert_eq!(config.proxy_type, "haproxy");
    assert_eq!(config.timeouts.request_timeout, Duration::from_mins(2));
}

// ============================================================================
// TelemetryConfig Tests
// ============================================================================

#[test]
fn test_telemetry_config_full() {
    let config = TelemetryConfig {
        metrics_enabled: true,
        tracing_enabled: true,
        access_logs: true,
        metrics_port: 9090,
        tracing_endpoint: Some("http://jaeger:14268/api/traces".to_string()),
    };

    assert!(config.metrics_enabled);
    assert!(config.tracing_enabled);
    assert!(config.access_logs);
    assert_eq!(config.metrics_port, 9090);
}

#[test]
fn test_telemetry_config_minimal() {
    let config = TelemetryConfig {
        metrics_enabled: true,
        tracing_enabled: false,
        access_logs: false,
        metrics_port: 8080,
        tracing_endpoint: None,
    };

    assert!(config.metrics_enabled);
    assert!(!config.tracing_enabled);
    assert!(config.tracing_endpoint.is_none());
}

#[test]
fn test_telemetry_config_disabled() {
    let config = TelemetryConfig {
        metrics_enabled: false,
        tracing_enabled: false,
        access_logs: false,
        metrics_port: 0,
        tracing_endpoint: None,
    };

    assert!(!config.metrics_enabled);
    assert!(!config.tracing_enabled);
    assert!(!config.access_logs);
}

// ============================================================================
// MutualTLSConfig Tests
// ============================================================================

#[test]
fn test_mtls_config_strict() {
    let config = MutualTLSConfig {
        enabled: true,
        ca_cert: "/etc/certs/ca.crt".to_string(),
        service_cert: "/etc/certs/service.crt".to_string(),
        private_key: "/etc/certs/service.key".to_string(),
        rotation_interval: Duration::from_hours(24),
        verification_mode: "strict".to_string(),
    };

    assert!(config.enabled);
    assert_eq!(config.verification_mode, "strict");
}

#[test]
fn test_mtls_config_permissive() {
    let config = MutualTLSConfig {
        enabled: true,
        ca_cert: "/certs/ca.pem".to_string(),
        service_cert: "/certs/cert.pem".to_string(),
        private_key: "/certs/key.pem".to_string(),
        rotation_interval: Duration::from_hours(1),
        verification_mode: "permissive".to_string(),
    };

    assert_eq!(config.verification_mode, "permissive");
}

#[test]
fn test_mtls_config_disabled() {
    let config = MutualTLSConfig {
        enabled: false,
        ca_cert: String::new(),
        service_cert: String::new(),
        private_key: String::new(),
        rotation_interval: Duration::from_secs(0),
        verification_mode: "disabled".to_string(),
    };

    assert!(!config.enabled);
    assert_eq!(config.verification_mode, "disabled");
}

// ============================================================================
// DiscoveryBackend Tests
// ============================================================================

#[test]
fn test_discovery_backend_dns() {
    let backend = DiscoveryBackend {
        backend_type: "dns".to_string(),
        config: HashMap::new(),
        priority: 1,
        enabled: true,
    };

    assert_eq!(backend.backend_type, "dns");
    assert_eq!(backend.priority, 1);
    assert!(backend.enabled);
}

#[test]
fn test_discovery_backend_consul() {
    let mut config = HashMap::new();
    config.insert("address".to_string(), serde_json::json!("consul:8500"));
    config.insert("datacenter".to_string(), serde_json::json!("dc1"));

    let backend = DiscoveryBackend {
        backend_type: "consul".to_string(),
        config,
        priority: 2,
        enabled: true,
    };

    assert_eq!(backend.backend_type, "consul");
    assert_eq!(backend.config.len(), 2);
}

#[test]
fn test_discovery_backend_etcd() {
    let backend = DiscoveryBackend {
        backend_type: "etcd".to_string(),
        config: HashMap::new(),
        priority: 3,
        enabled: false,
    };

    assert_eq!(backend.backend_type, "etcd");
    assert!(!backend.enabled);
}

#[test]
fn test_discovery_backend_kubernetes() {
    let mut config = HashMap::new();
    config.insert("namespace".to_string(), serde_json::json!("default"));

    let backend = DiscoveryBackend {
        backend_type: "kubernetes".to_string(),
        config,
        priority: 1,
        enabled: true,
    };

    assert_eq!(backend.backend_type, "kubernetes");
}

// ============================================================================
// ServiceDiscoveryConfig Tests
// ============================================================================

#[test]
fn test_service_discovery_enabled() {
    let backend = DiscoveryBackend {
        backend_type: "dns".to_string(),
        config: HashMap::new(),
        priority: 1,
        enabled: true,
    };

    let config = ServiceDiscoveryConfig {
        enabled: true,
        backends: vec![backend],
        refresh_interval: Duration::from_secs(30),
        cache_ttl: Duration::from_mins(5),
        health_check_integration: true,
    };

    assert!(config.enabled);
    assert_eq!(config.backends.len(), 1);
    assert!(config.health_check_integration);
}

#[test]
fn test_service_discovery_multiple_backends() {
    let dns_backend = DiscoveryBackend {
        backend_type: "dns".to_string(),
        config: HashMap::new(),
        priority: 1,
        enabled: true,
    };

    let consul_backend = DiscoveryBackend {
        backend_type: "consul".to_string(),
        config: HashMap::new(),
        priority: 2,
        enabled: true,
    };

    let config = ServiceDiscoveryConfig {
        enabled: true,
        backends: vec![dns_backend, consul_backend],
        refresh_interval: Duration::from_mins(1),
        cache_ttl: Duration::from_mins(10),
        health_check_integration: true,
    };

    assert_eq!(config.backends.len(), 2);
}

// ============================================================================
// NetworkSelector Tests
// ============================================================================

#[test]
fn test_network_selector_ip() {
    let selector = NetworkSelector {
        selector_type: "ip".to_string(),
        value: "192.168.1.100".to_string(),
    };

    assert_eq!(selector.selector_type, "ip");
    assert_eq!(selector.value, "192.168.1.100");
}

#[test]
fn test_network_selector_cidr() {
    let selector = NetworkSelector {
        selector_type: "cidr".to_string(),
        value: "10.0.0.0/8".to_string(),
    };

    assert_eq!(selector.selector_type, "cidr");
}

#[test]
fn test_network_selector_service() {
    let selector = NetworkSelector {
        selector_type: "service".to_string(),
        value: "api-gateway".to_string(),
    };

    assert_eq!(selector.selector_type, "service");
}

#[test]
fn test_network_selector_label() {
    let selector = NetworkSelector {
        selector_type: "label".to_string(),
        value: "app=web".to_string(),
    };

    assert_eq!(selector.selector_type, "label");
}

// ============================================================================
// NetworkPort Tests
// ============================================================================

#[test]
fn test_network_port_tcp() {
    let port = NetworkPort {
        port: 8080,
        protocol: "tcp".to_string(),
        end_port: None,
    };

    assert_eq!(port.port, 8080);
    assert_eq!(port.protocol, "tcp");
    assert!(port.end_port.is_none());
}

#[test]
fn test_network_port_udp() {
    let port = NetworkPort {
        port: 53,
        protocol: "udp".to_string(),
        end_port: None,
    };

    assert_eq!(port.port, 53);
    assert_eq!(port.protocol, "udp");
}

#[test]
fn test_network_port_range() {
    let port = NetworkPort {
        port: 8000,
        protocol: "tcp".to_string(),
        end_port: Some(8999),
    };

    assert_eq!(port.port, 8000);
    assert_eq!(port.end_port, Some(8999));
}

#[test]
fn test_network_port_sctp() {
    let port = NetworkPort {
        port: 3868,
        protocol: "sctp".to_string(),
        end_port: None,
    };

    assert_eq!(port.protocol, "sctp");
}

// ============================================================================
// IngressRule Tests
// ============================================================================

#[test]
fn test_ingress_rule_allow() {
    let selector = NetworkSelector {
        selector_type: "cidr".to_string(),
        value: "0.0.0.0/0".to_string(),
    };

    let port = NetworkPort {
        port: 443,
        protocol: "tcp".to_string(),
        end_port: None,
    };

    let rule = IngressRule {
        name: "allow-https".to_string(),
        from: vec![selector],
        ports: vec![port],
        action: "allow".to_string(),
        priority: 100,
    };

    assert_eq!(rule.name, "allow-https");
    assert_eq!(rule.action, "allow");
    assert_eq!(rule.priority, 100);
}

#[test]
fn test_ingress_rule_deny() {
    let rule = IngressRule {
        name: "deny-all".to_string(),
        from: vec![],
        ports: vec![],
        action: "deny".to_string(),
        priority: 0,
    };

    assert_eq!(rule.action, "deny");
}

// ============================================================================
// EgressRule Tests
// ============================================================================

#[test]
fn test_egress_rule_allow() {
    let selector = NetworkSelector {
        selector_type: "service".to_string(),
        value: "database".to_string(),
    };

    let port = NetworkPort {
        port: 5432,
        protocol: "tcp".to_string(),
        end_port: None,
    };

    let rule = EgressRule {
        name: "allow-database".to_string(),
        to: vec![selector],
        ports: vec![port],
        action: "allow".to_string(),
        priority: 200,
    };

    assert_eq!(rule.name, "allow-database");
    assert_eq!(rule.action, "allow");
}

#[test]
fn test_egress_rule_deny() {
    let rule = EgressRule {
        name: "deny-external".to_string(),
        to: vec![],
        ports: vec![],
        action: "deny".to_string(),
        priority: 1,
    };

    assert_eq!(rule.action, "deny");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_proxy_types_all() {
    let types = ["envoy", "nginx", "haproxy"];
    assert_eq!(types.len(), 3);
}

#[test]
fn test_verification_modes_all() {
    let modes = ["strict", "permissive", "disabled"];
    assert_eq!(modes.len(), 3);
}

#[test]
fn test_network_protocols_all() {
    let protocols = ["tcp", "udp", "sctp"];
    assert_eq!(protocols.len(), 3);
}

#[test]
fn test_backend_types_all() {
    let types = ["dns", "consul", "etcd", "kubernetes"];
    assert_eq!(types.len(), 4);
}
