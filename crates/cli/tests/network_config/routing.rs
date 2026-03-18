// SPDX-License-Identifier: AGPL-3.0-or-later
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
