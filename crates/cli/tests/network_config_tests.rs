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
    let config = create_test_songbird_config();
    assert!(config.service_mesh.enabled);
    assert!(config.dns_discovery.enabled);
}

#[test]
fn test_songbird_network_config_clone() {
    let config = create_test_songbird_config();
    let cloned = config.clone();
    assert_eq!(config.service_mesh.enabled, cloned.service_mesh.enabled);
}

#[test]
fn test_songbird_network_config_debug() {
    let config = create_test_songbird_config();
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("SongbirdNetworkConfig"));
}

#[test]
fn test_songbird_network_config_serialization() {
    let config = create_test_songbird_config();
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
    assert!(!domains.toadstool.is_empty());
    assert!(!domains.songbird.is_empty());
    assert!(!domains.beardog.is_empty());
    assert!(!domains.nestgate.is_empty());
    assert!(!domains.squirrel.is_empty());
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
fn test_beardog_integration_config() {
    let config = create_test_beardog_integration_config(true);
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
        };
        assert_eq!(selector.selector_type, selector_type);
    }
}

#[test]
fn test_network_port() {
    let port = NetworkPort {
        port: 8080,
        protocol: "tcp".to_string(),
        end_port: Some(8090),
    };
    assert_eq!(port.port, 8080);
    assert_eq!(port.protocol, "tcp");
    assert_eq!(port.end_port, Some(8090));
}

// ============================================================================
// Traffic Management Tests
// ============================================================================

#[test]
fn test_traffic_management_config() {
    let config = create_test_traffic_management_config();
    assert!(config.enabled);
}

#[test]
fn test_traffic_splitting_config() {
    let config = create_test_traffic_splitting_config();
    assert!(config.enabled);
    assert!(!config.weights.is_empty());
}

#[test]
fn test_canary_config() {
    let config = create_test_canary_config();
    assert!(config.enabled);
    assert!(config.percentage <= 100);
}

#[test]
fn test_blue_green_config() {
    let config = create_test_blue_green_config();
    assert!(config.enabled);
    assert!(!config.switch_strategy.is_empty());
}

#[test]
fn test_rate_limiting_config() {
    let config = create_test_rate_limiting_config();
    assert!(config.enabled);
}

#[test]
fn test_rate_limit() {
    let limit = RateLimit {
        requests_per_second: 100,
        burst_size: 200,
        window_size: Duration::from_secs(60),
    };
    assert_eq!(limit.requests_per_second, 100);
    assert_eq!(limit.burst_size, 200);
}

// ============================================================================
// Load Balancing Tests
// ============================================================================

#[test]
fn test_load_balancing_config() {
    let config = create_test_load_balancing_config();
    assert!(config.enabled);
    assert!(!config.algorithm.is_empty());
}

#[test]
fn test_load_balancing_algorithms() {
    let algorithms = vec!["round_robin", "least_connections", "ip_hash", "random"];
    for algorithm in algorithms {
        let mut config = create_test_load_balancing_config();
        config.algorithm = algorithm.to_string();
        assert_eq!(config.algorithm, algorithm);
    }
}

#[test]
fn test_health_check_config() {
    let config = create_test_health_check_config();
    assert!(config.enabled);
    assert!(config.interval.as_secs() > 0);
}

#[test]
fn test_http_health_check_config() {
    let config = create_test_http_health_check_config();
    assert!(config.base.enabled);
    assert!(!config.path.is_empty());
    assert_eq!(config.expected_status, 200);
}

#[test]
fn test_sticky_sessions_config() {
    let config = create_test_sticky_sessions_config();
    assert!(config.enabled);
    assert!(!config.affinity_type.is_empty());
}

#[test]
fn test_cookie_config() {
    let cookie = create_test_cookie_config();
    assert!(!cookie.name.is_empty());
    assert!(cookie.secure);
    assert!(cookie.http_only);
}

#[test]
fn test_backend_config() {
    let backend = create_test_backend_config();
    assert!(!backend.endpoint.name.is_empty());
    assert!(backend.endpoint.port > 0);
    assert!(backend.weight > 0);
}

// ============================================================================
// Circuit Breaker Tests
// ============================================================================

#[test]
fn test_circuit_breaker_config() {
    let config = create_test_circuit_breaker_config();
    assert!(config.enabled);
    assert!(config.failure_threshold > 0);
    assert!(config.success_threshold > 0);
}

#[test]
fn test_circuit_breaker_thresholds() {
    let config = create_test_circuit_breaker_config();
    assert!(config.failure_threshold <= config.success_threshold * 10);
}

// ============================================================================
// Health Monitoring Tests
// ============================================================================

#[test]
fn test_health_monitoring_config() {
    let config = create_test_health_monitoring_config();
    assert!(config.enabled);
    assert!(!config.endpoints.is_empty());
}

#[test]
fn test_health_endpoint() {
    let endpoint = create_test_health_endpoint();
    assert!(!endpoint.name.is_empty());
    assert!(!endpoint.url.is_empty());
    assert_eq!(endpoint.health_check.expected_status, 200);
}

#[test]
fn test_alerting_config() {
    let config = create_test_alerting_config();
    assert!(config.enabled);
}

#[test]
fn test_metrics_config() {
    let config = create_test_metrics_config();
    assert!(config.enabled);
    assert!(!config.endpoint.is_empty());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_songbird_config() -> SongbirdNetworkConfig {
    SongbirdNetworkConfig {
        service_mesh: create_test_service_mesh_config(true),
        dns_discovery: create_test_dns_discovery_config(),
        cross_primal_security: create_test_cross_primal_security_config(),
        network_policies: create_test_network_policies_config(),
        traffic_management: create_test_traffic_management_config(),
        load_balancing: create_test_load_balancing_config(),
        circuit_breaker: create_test_circuit_breaker_config(),
        health_monitoring: create_test_health_monitoring_config(),
    }
}

fn create_test_service_mesh_config(enabled: bool) -> ServiceMeshConfig {
    ServiceMeshConfig {
        enabled,
        mesh_type: "Istio".to_string(),
        sidecar: create_test_sidecar_config(),
        mtls: create_test_mtls_config(true),
        service_discovery: create_test_service_discovery_config(),
        inter_service: create_test_inter_service_config(),
    }
}

fn create_test_sidecar_config() -> SidecarConfig {
    SidecarConfig {
        enabled: true,
        image: "envoy:latest".to_string(),
        resources: SidecarResources {
            cpu_limit: "1.0".to_string(),
            memory_limit: "512Mi".to_string(),
            cpu_request: "0.5".to_string(),
            memory_request: "256Mi".to_string(),
        },
        proxy: create_test_proxy_config(),
        telemetry: create_test_telemetry_config(true, true, true),
    }
}

fn create_test_proxy_config() -> ProxyConfig {
    ProxyConfig {
        proxy_type: "envoy".to_string(),
        listen_port: 15001,
        admin_port: 15000,
        concurrency: 2,
        timeouts: TimeoutConfig {
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
        },
    }
}

fn create_test_telemetry_config(metrics: bool, tracing: bool, logs: bool) -> TelemetryConfig {
    TelemetryConfig {
        metrics_enabled: metrics,
        tracing_enabled: tracing,
        access_logs: logs,
        metrics_port: 9090,
        tracing_endpoint: Some("http://jaeger:14268".to_string()),
    }
}

fn create_test_mtls_config(enabled: bool) -> MutualTLSConfig {
    MutualTLSConfig {
        enabled,
        ca_cert: "/etc/certs/ca.crt".to_string(),
        service_cert: "/etc/certs/service.crt".to_string(),
        private_key: "/etc/certs/service.key".to_string(),
        rotation_interval: Duration::from_secs(86400),
        verification_mode: "strict".to_string(),
    }
}

fn create_test_service_discovery_config() -> ServiceDiscoveryConfig {
    ServiceDiscoveryConfig {
        enabled: true,
        backends: vec![DiscoveryBackend {
            backend_type: "dns".to_string(),
            config: HashMap::new(),
            priority: 1,
            enabled: true,
        }],
        refresh_interval: Duration::from_secs(30),
        cache_ttl: Duration::from_secs(300),
        health_check_integration: true,
    }
}

fn create_test_inter_service_config() -> InterServiceConfig {
    InterServiceConfig {
        default_protocol: "http2".to_string(),
        connection_pooling: create_test_connection_pool_config(),
        retry: create_test_retry_config(),
        timeouts: create_test_timeout_config(),
    }
}

fn create_test_connection_pool_config() -> ConnectionPoolConfig {
    ConnectionPoolConfig {
        enabled: true,
        max_connections_per_host: 100,
        max_idle_connections: 10,
        idle_timeout: Duration::from_secs(60),
        connection_lifetime: Duration::from_secs(600),
    }
}

fn create_test_retry_config() -> RetryConfig {
    RetryConfig {
        max_retries: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(10),
        backoff_multiplier: 2.0,
        jitter_percent: 10.0,
    }
}

fn create_test_timeout_config() -> TimeoutConfig {
    TimeoutConfig {
        connection_timeout: Duration::from_secs(5),
        request_timeout: Duration::from_secs(30),
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
    }
}

fn create_test_dns_discovery_config() -> DnsDiscoveryConfig {
    DnsDiscoveryConfig {
        enabled: true,
        dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
        search_domains: vec!["local".to_string()],
        service_domains: create_test_service_domains_config(),
        resolution_timeout: Duration::from_secs(5),
        cache: create_test_dns_cache_config(true),
    }
}

fn create_test_service_domains_config() -> ServiceDomainsConfig {
    ServiceDomainsConfig {
        toadstool: "toadstool.svc.local".to_string(),
        songbird: "songbird.svc.local".to_string(),
        beardog: "beardog.svc.local".to_string(),
        nestgate: "nestgate.svc.local".to_string(),
        squirrel: "squirrel.svc.local".to_string(),
        biomeos: "biomeos.svc.local".to_string(),
    }
}

fn create_test_dns_cache_config(enabled: bool) -> DnsCacheConfig {
    DnsCacheConfig {
        base: CacheConfig {
            enabled,
            ttl: Duration::from_secs(300),
            max_entries: 1000,
            negative_ttl: Duration::from_secs(60),
        },
    }
}

fn create_test_cross_primal_security_config() -> CrossPrimalSecurityConfig {
    CrossPrimalSecurityConfig {
        enabled: true,
        authentication: create_test_authentication_config(),
        authorization: create_test_authorization_config(),
        network_isolation: create_test_network_isolation_config(),
        audit_logging: create_test_audit_logging_config(),
    }
}

fn create_test_authentication_config() -> AuthenticationConfig {
    AuthenticationConfig {
        method: "beardog".to_string(),
        token_validation: create_test_token_validation_config(),
        certificate_validation: create_test_certificate_validation_config(),
        beardog_integration: create_test_beardog_integration_config(true),
    }
}

fn create_test_token_validation_config() -> TokenValidationConfig {
    TokenValidationConfig {
        validate_issuer: true,
        validate_audience: true,
        validate_expiration: true,
        validate_signature: true,
        clock_skew: Duration::from_secs(60),
    }
}

fn create_test_certificate_validation_config() -> CertificateValidationConfig {
    CertificateValidationConfig {
        validate_chain: true,
        validate_expiration: true,
        validate_usage: true,
        trusted_cas: vec!["/etc/certs/ca.crt".to_string()],
    }
}

fn create_test_beardog_integration_config(enabled: bool) -> BearDogIntegrationConfig {
    BearDogIntegrationConfig {
        enabled,
        endpoint: "http://beardog:9000".to_string(),
        auth_token: Some("test-token".to_string()),
        signature_verification: true,
        crypto_lock: true,
    }
}

fn create_test_authorization_config() -> AuthorizationConfig {
    AuthorizationConfig {
        model: "rbac".to_string(),
        policy_engine: create_test_policy_engine_config(),
        roles: vec![],
        permissions: HashMap::new(),
    }
}

fn create_test_policy_engine_config() -> PolicyEngineConfig {
    PolicyEngineConfig {
        engine_type: "native".to_string(),
        policy_files: vec![],
        policy_endpoints: vec![],
        evaluation_cache: true,
    }
}

fn create_test_network_isolation_config() -> NetworkIsolationConfig {
    NetworkIsolationConfig {
        enabled: true,
        isolation_level: "strict".to_string(),
        allowed_networks: vec!["10.0.0.0/8".to_string()],
        blocked_networks: vec![],
        firewall_rules: vec![],
    }
}

fn create_test_audit_logging_config() -> AuditLoggingConfig {
    AuditLoggingConfig {
        enabled: true,
        log_level: "info".to_string(),
        log_format: "json".to_string(),
        destinations: vec![],
        retention: create_test_retention_policy(),
    }
}

fn create_test_retention_policy() -> RetentionPolicy {
    RetentionPolicy {
        duration: Duration::from_secs(86400 * 30),
        compression: true,
        archive_location: Some("/var/log/archive".to_string()),
    }
}

fn create_test_network_policies_config() -> NetworkPoliciesConfig {
    NetworkPoliciesConfig {
        enabled: true,
        default_policy: "deny".to_string(),
        ingress_rules: vec![create_test_ingress_rule()],
        egress_rules: vec![create_test_egress_rule()],
        service_mesh_policies: vec![],
    }
}

fn create_test_ingress_rule() -> IngressRule {
    IngressRule {
        name: "allow-web".to_string(),
        from: vec![NetworkSelector {
            selector_type: "cidr".to_string(),
            value: "0.0.0.0/0".to_string(),
        }],
        ports: vec![NetworkPort {
            port: 80,
            protocol: "tcp".to_string(),
            end_port: None,
        }],
        action: "allow".to_string(),
        priority: 100,
    }
}

fn create_test_egress_rule() -> EgressRule {
    EgressRule {
        name: "allow-dns".to_string(),
        to: vec![NetworkSelector {
            selector_type: "service".to_string(),
            value: "dns".to_string(),
        }],
        ports: vec![NetworkPort {
            port: 53,
            protocol: "udp".to_string(),
            end_port: None,
        }],
        action: "allow".to_string(),
        priority: 100,
    }
}

fn create_test_traffic_management_config() -> TrafficManagementConfig {
    TrafficManagementConfig {
        enabled: true,
        traffic_splitting: create_test_traffic_splitting_config(),
        canary: create_test_canary_config(),
        blue_green: create_test_blue_green_config(),
        rate_limiting: create_test_rate_limiting_config(),
        traffic_mirroring: create_test_traffic_mirroring_config(),
    }
}

fn create_test_traffic_splitting_config() -> TrafficSplittingConfig {
    let mut weights = HashMap::new();
    weights.insert("v1".to_string(), 80);
    weights.insert("v2".to_string(), 20);

    TrafficSplittingConfig {
        enabled: true,
        strategy: "weighted".to_string(),
        weights,
        header_routing: None,
    }
}

fn create_test_canary_config() -> CanaryConfig {
    CanaryConfig {
        enabled: true,
        percentage: 10,
        success_criteria: create_test_success_criteria(),
        rollback_criteria: create_test_rollback_criteria(),
        automation: create_test_automation_config(),
    }
}

fn create_test_success_criteria() -> SuccessCriteria {
    SuccessCriteria {
        success_rate: 0.99,
        latency_p99: Duration::from_millis(500),
        error_rate: 0.01,
        evaluation_period: Duration::from_secs(300),
    }
}

fn create_test_rollback_criteria() -> RollbackCriteria {
    RollbackCriteria {
        error_rate: 0.05,
        latency_p99: Duration::from_secs(1),
        evaluation_period: Duration::from_secs(60),
        automatic_rollback: true,
    }
}

fn create_test_automation_config() -> AutomationConfig {
    AutomationConfig {
        enabled: true,
        promotion_interval: Duration::from_secs(600),
        max_promotion_steps: 5,
        rollback_timeout: Duration::from_secs(300),
    }
}

fn create_test_blue_green_config() -> BlueGreenConfig {
    BlueGreenConfig {
        enabled: true,
        switch_strategy: "instant".to_string(),
        validation_period: Duration::from_secs(300),
        rollback_timeout: Duration::from_secs(600),
    }
}

fn create_test_rate_limiting_config() -> RateLimitingConfig {
    RateLimitingConfig {
        enabled: true,
        global_limit: Some(RateLimit {
            requests_per_second: 1000,
            burst_size: 2000,
            window_size: Duration::from_secs(1),
        }),
        service_limits: HashMap::new(),
        user_limits: HashMap::new(),
    }
}

fn create_test_traffic_mirroring_config() -> TrafficMirroringConfig {
    TrafficMirroringConfig {
        enabled: true,
        destinations: vec![],
        percentage: 10,
        mirror_headers: true,
    }
}

fn create_test_load_balancing_config() -> LoadBalancingConfig {
    LoadBalancingConfig {
        enabled: true,
        algorithm: "round_robin".to_string(),
        health_check: create_test_http_health_check_config(),
        sticky_sessions: create_test_sticky_sessions_config(),
        backends: vec![create_test_backend_config()],
    }
}

fn create_test_health_check_config() -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        interval: Duration::from_secs(10),
        timeout: Duration::from_secs(5),
        healthy_threshold: 2,
        unhealthy_threshold: 3,
        retry_count: 3,
    }
}

fn create_test_http_health_check_config() -> HttpHealthCheckConfig {
    HttpHealthCheckConfig {
        base: create_test_health_check_config(),
        path: "/health".to_string(),
        expected_status: 200,
        method: "GET".to_string(),
    }
}

fn create_test_sticky_sessions_config() -> StickySessionsConfig {
    StickySessionsConfig {
        enabled: true,
        affinity_type: "cookie".to_string(),
        cookie: Some(create_test_cookie_config()),
        timeout: Duration::from_secs(3600),
    }
}

fn create_test_cookie_config() -> CookieConfig {
    CookieConfig {
        name: "session".to_string(),
        domain: Some(".example.com".to_string()),
        path: Some("/".to_string()),
        secure: true,
        http_only: true,
    }
}

fn create_test_backend_config() -> BackendConfig {
    BackendConfig {
        endpoint: BackendEndpoint {
            name: "backend-1".to_string(),
            address: "10.0.1.100".to_string(),
            port: 8080,
            enabled: true,
        },
        weight: 100,
        health_check: None,
    }
}

fn create_test_circuit_breaker_config() -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        enabled: true,
        failure_threshold: 5,
        success_threshold: 2,
        timeout: Duration::from_secs(60),
        half_open_timeout: Duration::from_secs(30),
        reset_timeout: Duration::from_secs(120),
    }
}

fn create_test_health_monitoring_config() -> HealthMonitoringConfig {
    HealthMonitoringConfig {
        enabled: true,
        interval: Duration::from_secs(30),
        endpoints: vec![create_test_health_endpoint()],
        alerting: create_test_alerting_config(),
        metrics: create_test_metrics_config(),
    }
}

fn create_test_health_endpoint() -> HealthEndpoint {
    HealthEndpoint {
        name: "api-health".to_string(),
        url: "http://api:8080/health".to_string(),
        health_check: HttpHealthCheckConfig {
            base: HealthCheckConfig {
                enabled: true,
                interval: Duration::from_secs(30),
                timeout: Duration::from_secs(5),
                healthy_threshold: 2,
                unhealthy_threshold: 3,
                retry_count: 3,
            },
            path: "/health".to_string(),
            expected_status: 200,
            method: "GET".to_string(),
        },
    }
}

fn create_test_alerting_config() -> AlertingConfig {
    AlertingConfig {
        enabled: true,
        channels: vec![],
        rules: vec![],
    }
}

fn create_test_metrics_config() -> MetricsConfig {
    MetricsConfig {
        enabled: true,
        endpoint: "/metrics".to_string(),
        interval: Duration::from_secs(15),
        exporters: vec![],
    }
}
