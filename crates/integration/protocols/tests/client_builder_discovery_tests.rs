// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for protocol client builder, discovery, and health check types
//!
//! Pure unit tests - no network or hardware required.

use std::time::Duration;

use toadstool_integration_protocols::config::{
    ConnectionPoolConfig, DiscoveryType, HealthConfig, LoadBalancingConfig, ProtocolConfig,
    RoutingConfig, RoutingStrategy, ServiceDiscoveryConfig,
};
use toadstool_integration_protocols::types::{
    HealthStatus, ServiceEndpoint, ServiceInfo, TransportType,
};

// ============================================================================
// Client Builder / Config Tests
// ============================================================================

#[test]
fn test_protocol_config_builder_defaults() {
    let config = ProtocolConfig::default();
    assert!(!config.service_id.is_empty());
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    assert!(!config.supported_transports.is_empty());
}

#[test]
fn test_connection_pool_defaults() {
    let config = ConnectionPoolConfig::default();
    assert_eq!(config.max_connections_per_service, 10);
    assert_eq!(config.idle_timeout, Duration::from_secs(300));
    assert_eq!(config.max_concurrent_requests, 100);
}

#[test]
fn test_service_discovery_consul_default() {
    let config = ServiceDiscoveryConfig::consul_default();
    assert!(matches!(config.discovery_type, DiscoveryType::Consul));
    assert!(config.registry_endpoint.is_some());
    assert_eq!(config.registration_ttl, Duration::from_secs(300));
    assert_eq!(config.refresh_interval, Duration::from_secs(60));
    assert!(config.auto_register);
}

#[test]
fn test_routing_config_default_strategy() {
    let config = RoutingConfig::default();
    assert!(matches!(
        config.default_strategy,
        RoutingStrategy::RoundRobin
    ));
}

#[test]
fn test_health_config_endpoint() {
    let config = HealthConfig::default();
    assert_eq!(config.endpoint, "/health");
}

// ============================================================================
// Discovery Type Tests
// ============================================================================

#[test]
fn test_discovery_type_variants() {
    let _ = DiscoveryType::Static;
    let _ = DiscoveryType::Dns;
    let _ = DiscoveryType::Consul;
    let _ = DiscoveryType::Etcd;
    let _ = DiscoveryType::Kubernetes;
    let custom = DiscoveryType::Custom("custom".to_string());
    assert!(matches!(custom, DiscoveryType::Custom(_)));
}

// ============================================================================
// Health Status Tests
// ============================================================================

#[test]
fn test_health_status_variants() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Unknown);
}

#[test]
fn test_health_status_ordering() {
    let statuses = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
        HealthStatus::Unknown,
    ];
    for s in &statuses {
        assert!(!format!("{s:?}").is_empty());
    }
}

// ============================================================================
// ServiceInfo / ServiceEndpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_construction() {
    let endpoint = ServiceEndpoint {
        id: "ep-1".to_string(),
        transport: TransportType::Http,
        address: "127.0.0.1".to_string(),
        port: 8080,
        path: Some("/api".to_string()),
        tls_enabled: false,
        health_status: HealthStatus::Healthy,
    };
    assert_eq!(endpoint.id, "ep-1");
    assert_eq!(endpoint.port, 8080);
    assert_eq!(endpoint.path.as_deref(), Some("/api"));
}

#[test]
fn test_service_info_construction() {
    let service = ServiceInfo {
        id: "svc-1".to_string(),
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        metadata: std::collections::HashMap::new(),
        health_status: HealthStatus::Healthy,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec!["cap1".to_string()],
    };
    assert_eq!(service.id, "svc-1");
    assert_eq!(service.name, "test-service");
    assert_eq!(service.capabilities.len(), 1);
}

// ============================================================================
// Load Balancing Config Tests
// ============================================================================

#[test]
fn test_load_balancing_config_defaults() {
    let config = LoadBalancingConfig::default();
    assert!(config.health_check_enabled);
    assert_eq!(config.unhealthy_threshold, 3);
    assert_eq!(config.healthy_threshold, 2);
}
