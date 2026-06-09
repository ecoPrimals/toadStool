// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Songbird network configuration types

use std::collections::HashMap;
use std::time::Duration;
use toadstool_cli::network_config::*;
use toadstool_common::config_bases::{
    BackendEndpoint, CacheConfig, ConnectionPoolConfig, HealthCheckConfig, HttpHealthCheckConfig,
    RetryConfig, TelemetryConfig, TimeoutConfig,
};

// ============================================================================
// Top-Level Configuration Tests
// ============================================================================

#[test]
fn test_songbird_network_config_creation() {
    let config = create_test_coordination_config();
    assert!(config.service_mesh.enabled);
    assert!(config.dns_discovery.enabled);
}

#[test]
fn test_songbird_network_config_clone() {
    let config = create_test_coordination_config();
    let cloned = config.clone();
    assert_eq!(config.service_mesh.enabled, cloned.service_mesh.enabled);
}

#[test]
fn test_songbird_network_config_debug() {
    let config = create_test_coordination_config();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("OrchestrationNetworkConfig"));
}

#[test]
fn test_songbird_network_config_serialization() {
    let config = create_test_coordination_config();
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("service_mesh"));
    assert!(json.contains("dns_discovery"));
}

// ============================================================================
// Service Mesh Configuration Tests
// ============================================================================

#[test]
fn test_service_mesh_config_enabled() {
    let config = create_test_service_mesh_config(true);
    assert!(config.enabled);
    assert_eq!(config.mesh_type, "Istio");
}

#[test]
fn test_service_mesh_config_disabled() {
    let config = create_test_service_mesh_config(false);
    assert!(!config.enabled);
}

#[test]
fn test_service_mesh_config_types() {
    let mesh_types = vec!["Istio", "Linkerd", "Consul", "Native"];
    for mesh_type in mesh_types {
        let mut config = create_test_service_mesh_config(true);
        config.mesh_type = mesh_type.to_string();
        assert_eq!(config.mesh_type, mesh_type);
    }
}

#[test]
fn test_sidecar_config_creation() {
    let sidecar = create_test_sidecar_config();
    assert!(sidecar.enabled);
    assert!(!sidecar.image.is_empty());
}

#[test]
fn test_sidecar_resources() {
    let resources = SidecarResources {
        cpu_limit: "1.0".to_string(),
        memory_limit: "512Mi".to_string(),
        cpu_request: "0.5".to_string(),
        memory_request: "256Mi".to_string(),
    };
    assert_eq!(resources.cpu_limit, "1.0");
    assert_eq!(resources.memory_limit, "512Mi");
}

#[test]
fn test_proxy_config_creation() {
    let proxy = create_test_proxy_config();
    assert_eq!(proxy.proxy_type, "envoy");
    assert!(proxy.listen_port > 0);
    assert!(proxy.admin_port > 0);
}

#[test]
fn test_proxy_config_port_validation() {
    let proxy = create_test_proxy_config();
    assert_ne!(proxy.listen_port, proxy.admin_port);
}

#[test]
fn test_telemetry_config_all_enabled() {
    let telemetry = create_test_telemetry_config(true, true, true);
    assert!(telemetry.metrics_enabled);
    assert!(telemetry.tracing_enabled);
    assert!(telemetry.access_logs);
}

#[test]
fn test_telemetry_config_all_disabled() {
    let telemetry = create_test_telemetry_config(false, false, false);
    assert!(!telemetry.metrics_enabled);
    assert!(!telemetry.tracing_enabled);
    assert!(!telemetry.access_logs);
}

// ============================================================================
// mTLS Configuration Tests
// ============================================================================

#[test]
fn test_mtls_config_enabled() {
    let mtls = create_test_mtls_config(true);
    assert!(mtls.enabled);
    assert!(!mtls.ca_cert.is_empty());
}

#[test]
fn test_mtls_config_verification_modes() {
    let modes = vec!["strict", "permissive", "disabled"];
    for mode in modes {
        let mut mtls = create_test_mtls_config(true);
        mtls.verification_mode = mode.to_string();
        assert_eq!(mtls.verification_mode, mode);
    }
}

// ============================================================================
// Service Discovery Tests
// ============================================================================

#[test]
fn test_service_discovery_config() {
    let config = create_test_service_discovery_config();
    assert!(config.enabled);
    assert!(!config.backends.is_empty());
}

#[test]
fn test_discovery_backend_types() {
    let backend_types = vec!["dns", "consul", "etcd", "kubernetes"];
    for backend_type in backend_types {
        let backend = DiscoveryBackend {
            backend_type: backend_type.to_string(),
            config: HashMap::new(),
            priority: 1,
            enabled: true,
        };
        assert_eq!(backend.backend_type, backend_type);
    }
}

#[test]
fn test_discovery_backend_priority() {
    let high_priority = DiscoveryBackend {
        backend_type: "dns".to_string(),
        config: HashMap::new(),
        priority: 1,
        enabled: true,
    };
    let low_priority = DiscoveryBackend {
        backend_type: "consul".to_string(),
        config: HashMap::new(),
        priority: 10,
        enabled: true,
    };
    assert!(high_priority.priority < low_priority.priority);
}

// ============================================================================
// Inter-Service Communication Tests
// ============================================================================

#[test]
fn test_inter_service_config() {
    let config = create_test_inter_service_config();
    assert!(!config.default_protocol.is_empty());
    assert!(config.connection_pooling.enabled);
}

#[test]
fn test_connection_pool_config() {
    let pool = create_test_connection_pool_config();
    assert!(pool.enabled);
    assert!(pool.max_connections_per_host > 0);
    assert!(pool.max_idle_connections > 0);
}

#[test]
fn test_retry_config() {
    let retry = create_test_retry_config();
    assert!(retry.max_retries > 0);
    assert!(retry.backoff_multiplier > 1.0);
    assert!(retry.jitter_percent >= 0.0 && retry.jitter_percent <= 100.0);
}

#[test]
fn test_timeout_config() {
    let timeout = create_test_timeout_config();
    assert!(timeout.connection_timeout > Duration::from_secs(0));
    assert!(timeout.request_timeout > Duration::from_secs(0));
}

// ============================================================================
// DNS Discovery Tests
// ============================================================================

#[test]
fn test_dns_discovery_config() {
    let config = create_test_dns_discovery_config();
    assert!(config.enabled);
    assert!(!config.dns_servers.is_empty());
}

#[test]
fn test_service_domains_config() {
    let domains = create_test_service_domains_config();
    assert!(!domains.compute.is_empty());
    assert!(!domains.coordination.is_empty());
    assert!(!domains.security.is_empty());
    assert!(!domains.storage.is_empty());
    assert!(!domains.ai_processing.is_empty());
    assert!(!domains.biomeos.is_empty());
}

#[test]
fn test_dns_cache_config() {
    let cache = create_test_dns_cache_config(true);
    assert!(cache.base.enabled);
    assert!(cache.base.max_entries > 0);
}

// ============================================================================
// Cross-Primal Security Tests
// ============================================================================

#[test]
fn test_cross_primal_security_config() {
    let config = create_test_cross_primal_security_config();
    assert!(config.enabled);
}

#[test]
fn test_authentication_config() {
    let auth = create_test_authentication_config();
    assert!(!auth.method.is_empty());
}

#[test]
fn test_authentication_methods() {
    let methods = vec!["jwt", "oauth2", "mtls", "beardog"];
    for method in methods {
        let mut auth = create_test_authentication_config();
        auth.method = method.to_string();
        assert_eq!(auth.method, method);
    }
}

#[test]
fn test_token_validation_config() {
    let validation = create_test_token_validation_config();
    assert!(validation.validate_issuer);
    assert!(validation.validate_signature);
}

#[test]
fn test_security_config() {
    let config = create_test_security_config(true);
    assert!(config.enabled);
    assert!(config.signature_verification);
    assert!(config.crypto_lock);
}

#[test]
fn test_authorization_config() {
    let authz = create_test_authorization_config();
    assert!(!authz.model.is_empty());
}

#[test]
fn test_authorization_models() {
    let models = vec!["rbac", "abac", "policy"];
    for model in models {
        let mut authz = create_test_authorization_config();
        authz.model = model.to_string();
        assert_eq!(authz.model, model);
    }
}

// ============================================================================
// Network Policies Tests
// ============================================================================

#[test]
fn test_network_policies_config() {
    let config = create_test_network_policies_config();
    assert!(config.enabled);
    assert!(!config.default_policy.is_empty());
}

#[test]
fn test_default_policies() {
    let policies = vec!["allow", "deny"];
    for policy in policies {
        let mut config = create_test_network_policies_config();
        config.default_policy = policy.to_string();
        assert_eq!(config.default_policy, policy);
    }
}

#[test]
fn test_ingress_rule() {
    let rule = create_test_ingress_rule();
    assert!(!rule.name.is_empty());
    assert!(!rule.from.is_empty());
}

#[test]
fn test_egress_rule() {
    let rule = create_test_egress_rule();
    assert!(!rule.name.is_empty());
    assert!(!rule.to.is_empty());
}

#[test]
fn test_network_selector_types() {
    let selector_types = vec!["ip", "cidr", "service", "label"];
    for selector_type in selector_types {
        let selector = NetworkSelector {
            selector_type: selector_type.to_string(),
            value: "test".to_string(),
