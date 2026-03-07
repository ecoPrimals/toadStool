// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Coverage tests for network_config/configurator/ modules
//!
//! Targets discovery, reliability, security, service_mesh, and traffic extension traits.

use toadstool_cli::network_config::*;
use toadstool_common::config_bases::HttpHealthCheckConfig;

// ============================================================================
// Discovery extension (discovery.rs)
// ============================================================================

#[tokio::test]
async fn discovery_apply_dns_discovery_config_succeeds() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[test]
fn discovery_validate_dns_empty_servers_when_enabled_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.dns_discovery.enabled = true;
    configurator.config.dns_discovery.dns_servers = vec![];
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("DNS server"));
}

#[test]
fn discovery_validate_dns_with_servers_succeeds() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.dns_discovery.enabled = true;
    configurator.config.dns_discovery.dns_servers = vec!["8.8.8.8".to_string()];
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[test]
fn discovery_validate_dns_disabled_with_empty_servers_succeeds() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.dns_discovery.enabled = false;
    configurator.config.dns_discovery.dns_servers = vec![];
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

// ============================================================================
// Reliability extension (reliability.rs)
// ============================================================================

#[test]
fn reliability_validate_circuit_breaker_zero_threshold_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.circuit_breaker.enabled = true;
    configurator.config.circuit_breaker.failure_threshold = 0;
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("failure threshold"));
}

#[test]
fn reliability_validate_circuit_breaker_valid_succeeds() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.circuit_breaker.enabled = true;
    configurator.config.circuit_breaker.failure_threshold = 5;
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[test]
fn reliability_validate_health_monitoring_empty_endpoints_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.health_monitoring.enabled = true;
    configurator.config.health_monitoring.endpoints = vec![];
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("health endpoint"));
}

#[test]
fn reliability_validate_health_monitoring_with_endpoints_succeeds() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.health_monitoring.enabled = true;
    configurator.config.health_monitoring.endpoints =
        vec![toadstool_cli::network_config::HealthEndpoint {
            name: "test".to_string(),
            url: "http://localhost:8080/health".to_string(),
            health_check: HttpHealthCheckConfig::default(),
        }];
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[tokio::test]
async fn reliability_apply_circuit_breaker_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn reliability_apply_health_monitoring_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

// ============================================================================
// Security extension (security.rs)
// ============================================================================

#[tokio::test]
async fn security_apply_cross_primal_security_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn security_apply_network_policies_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[test]
fn security_validate_cross_primal_succeeds() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[test]
fn security_validate_network_policies_succeeds() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

// ============================================================================
// Service mesh extension (service_mesh.rs)
// ============================================================================

#[test]
fn service_mesh_validate_invalid_mesh_type_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.service_mesh.enabled = true;
    configurator.config.service_mesh.mesh_type = "invalid_mesh".to_string();
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid mesh type"));
}

#[test]
fn service_mesh_validate_valid_mesh_types_succeed() {
    for mesh_type in ["istio", "linkerd", "consul", "native"] {
        let mut configurator = SongbirdNetworkConfigurator::new();
        configurator.config.service_mesh.enabled = true;
        configurator.config.service_mesh.mesh_type = mesh_type.to_string();
        let result = configurator.validate_configuration();
        assert!(result.is_ok(), "mesh_type={mesh_type} should succeed");
    }
}

#[test]
fn service_mesh_validate_sidecar_zero_listen_port_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.service_mesh.enabled = true;
    configurator.config.service_mesh.mesh_type = "istio".to_string();
    configurator.config.service_mesh.sidecar.enabled = true;
    configurator.config.service_mesh.sidecar.proxy.listen_port = 0;
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("listen port"));
}

#[test]
fn service_mesh_validate_disabled_skips_validation() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.service_mesh.enabled = false;
    configurator.config.service_mesh.mesh_type = "invalid".to_string();
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[tokio::test]
async fn service_mesh_apply_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

// ============================================================================
// Traffic extension (traffic.rs)
// ============================================================================

#[tokio::test]
async fn traffic_apply_traffic_management_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn traffic_apply_load_balancing_config() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.apply_configuration().await;
    assert!(result.is_ok());
}

#[test]
fn traffic_validate_load_balancing_invalid_algorithm_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.load_balancing.enabled = true;
    configurator.config.load_balancing.algorithm = "invalid_algo".to_string();
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid load balancing"));
}

#[test]
fn traffic_validate_load_balancing_valid_algorithms_succeed() {
    for algo in ["round_robin", "least_conn", "random", "ip_hash"] {
        let mut configurator = SongbirdNetworkConfigurator::new();
        configurator.config.load_balancing.enabled = true;
        configurator.config.load_balancing.algorithm = algo.to_string();
        let result = configurator.validate_configuration();
        assert!(result.is_ok(), "algorithm={algo} should succeed");
    }
}

#[test]
fn traffic_validate_load_balancing_zero_healthy_threshold_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.load_balancing.enabled = true;
    configurator.config.load_balancing.algorithm = "round_robin".to_string();
    configurator.config.load_balancing.health_check.base.enabled = true;
    configurator
        .config
        .load_balancing
        .health_check
        .base
        .healthy_threshold = 0;
    configurator
        .config
        .load_balancing
        .health_check
        .base
        .unhealthy_threshold = 3;
    let result = configurator.validate_configuration();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Health check thresholds"));
}

#[test]
fn traffic_validate_load_balancing_zero_unhealthy_threshold_fails() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.load_balancing.enabled = true;
    configurator.config.load_balancing.algorithm = "round_robin".to_string();
    configurator.config.load_balancing.health_check.base.enabled = true;
    configurator
        .config
        .load_balancing
        .health_check
        .base
        .healthy_threshold = 2;
    configurator
        .config
        .load_balancing
        .health_check
        .base
        .unhealthy_threshold = 0;
    let result = configurator.validate_configuration();
    assert!(result.is_err());
}

#[test]
fn traffic_validate_load_balancing_disabled_skips_validation() {
    let mut configurator = SongbirdNetworkConfigurator::new();
    configurator.config.load_balancing.enabled = false;
    configurator.config.load_balancing.algorithm = "invalid".to_string();
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}

#[test]
fn traffic_validate_traffic_management_succeeds() {
    let configurator = SongbirdNetworkConfigurator::new();
    let result = configurator.validate_configuration();
    assert!(result.is_ok());
}
