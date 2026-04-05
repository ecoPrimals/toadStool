// SPDX-License-Identifier: AGPL-3.0-or-later
        network_isolation: create_test_network_isolation_config(),
        audit_logging: create_test_audit_logging_config(),
    }
}

fn create_test_authentication_config() -> AuthenticationConfig {
    AuthenticationConfig {
        method: "beardog".to_string(),
        token_validation: create_test_token_validation_config(),
        certificate_validation: create_test_certificate_validation_config(),
        security: create_test_security_config(true),
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

fn create_test_security_config(enabled: bool) -> BearDogIntegrationConfig {
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
