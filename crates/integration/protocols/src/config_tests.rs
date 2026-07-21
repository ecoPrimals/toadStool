// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_protocol_config_default() {
    let config = ProtocolConfig::default();
    assert!(!config.service_id.is_empty());
    assert_eq!(config.default_format, MessageFormat::Json);
    assert_eq!(config.supported_transports.len(), 2);
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    assert!(config.auth_config.is_none());
}

#[test]
fn test_connection_pool_config_default() {
    let config = ConnectionPoolConfig::default();
    assert_eq!(config.max_connections_per_service, 10);
    assert_eq!(config.idle_timeout, Duration::from_mins(5));
    assert_eq!(config.keep_alive_interval, Duration::from_secs(30));
    assert_eq!(config.max_concurrent_requests, 100);
}

#[test]
fn test_routing_config_default() {
    let config = RoutingConfig::default();
    assert!(matches!(
        config.default_strategy,
        RoutingStrategy::RoundRobin
    ));
    assert!(config.load_balancing.health_check_enabled);
    assert!(config.circuit_breaker.is_none());
}

#[test]
fn test_load_balancing_config_default() {
    let config = LoadBalancingConfig::default();
    assert!(config.health_check_enabled);
    assert_eq!(config.health_check_interval, Duration::from_secs(30));
    assert_eq!(config.unhealthy_threshold, 3);
    assert_eq!(config.healthy_threshold, 2);
}

#[test]
fn test_health_config_default() {
    let config = HealthConfig::default();
    assert_eq!(config.endpoint, "/health");
}

#[test]
fn test_discovery_type_variants() {
    let static_discovery = DiscoveryType::Static;
    let dns_discovery = DiscoveryType::Dns;
    let consul_discovery = DiscoveryType::Consul;
    let etcd_discovery = DiscoveryType::Etcd;
    let k8s_discovery = DiscoveryType::Kubernetes;
    let custom_discovery = DiscoveryType::Custom("my-discovery".to_string());

    // All variants should be Debug
    assert!(format!("{static_discovery:?}").contains("Static"));
    assert!(format!("{dns_discovery:?}").contains("Dns"));
    assert!(format!("{consul_discovery:?}").contains("Consul"));
    assert!(format!("{etcd_discovery:?}").contains("Etcd"));
    assert!(format!("{k8s_discovery:?}").contains("Kubernetes"));
    assert!(format!("{custom_discovery:?}").contains("Custom"));
}

#[test]
fn test_routing_strategy_variants() {
    let strategies = vec![
        RoutingStrategy::RoundRobin,
        RoutingStrategy::Random,
        RoutingStrategy::LeastConnections,
        RoutingStrategy::Weighted,
        RoutingStrategy::Sticky,
        RoutingStrategy::Custom("custom-strategy".to_string()),
    ];

    assert_eq!(strategies.len(), 6);
}

#[test]
fn test_circuit_breaker_config() {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        recovery_timeout: Duration::from_mins(1),
        success_threshold: 2,
    };

    assert_eq!(config.failure_threshold, 5);
    assert_eq!(config.recovery_timeout, Duration::from_mins(1));
    assert_eq!(config.success_threshold, 2);
}

#[test]
fn test_service_discovery_config() {
    let expected_endpoint = sample_consul_registry_url();
    let config = ServiceDiscoveryConfig {
        discovery_type: DiscoveryType::Consul,
        registry_endpoint: Some(expected_endpoint.clone()),
        registration_ttl: Duration::from_mins(5),
        refresh_interval: Duration::from_mins(1),
        auto_register: true,
    };

    assert!(matches!(config.discovery_type, DiscoveryType::Consul));
    assert_eq!(config.registry_endpoint, Some(expected_endpoint));
    assert!(config.auto_register);
}

#[test]
fn test_protocol_config_clone() {
    let config1 = ProtocolConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.default_format, config2.default_format);
    assert_eq!(config1.request_timeout, config2.request_timeout);
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

#[test]
fn test_routing_config_clone() {
    let config1 = RoutingConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.load_balancing.health_check_enabled,
        config2.load_balancing.health_check_enabled
    );
}

#[test]
fn test_load_balancing_thresholds() {
    let config = LoadBalancingConfig {
        health_check_enabled: false,
        health_check_interval: Duration::from_secs(10),
        unhealthy_threshold: 5,
        healthy_threshold: 3,
    };

    assert!(!config.health_check_enabled);
    assert_eq!(config.unhealthy_threshold, 5);
    assert_eq!(config.healthy_threshold, 3);
}

#[test]
fn test_health_config_custom_endpoint() {
    let config = HealthConfig {
        base: toadstool_common::config_bases::HealthCheckConfig::default(),
        endpoint: "/api/health".to_string(),
    };

    assert_eq!(config.endpoint, "/api/health");
}

#[test]
fn test_discovery_type_custom() {
    let custom = DiscoveryType::Custom("my-service-mesh".to_string());
    assert!(matches!(custom, DiscoveryType::Custom(_)));
}

#[test]
fn test_routing_strategy_custom() {
    let custom = RoutingStrategy::Custom("geo-based".to_string());
    assert!(matches!(custom, RoutingStrategy::Custom(_)));
}
