// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default [`OrchestrationNetworkConfig`] construction and DNS resolver helpers.

use super::super::*;
use std::collections::HashMap;
use std::time::Duration;
use toadstool_common::config_bases::{
    CacheConfig, ConnectionPoolConfig, HealthCheckConfig, HttpHealthCheckConfig, RetryConfig,
    TelemetryConfig, TimeoutConfig,
};
use toadstool_common::constants::platform_paths::{etc_paths, install_paths};
use toadstool_common::interned_strings::capabilities;
use toadstool_common::interned_strings::socket_env;
use toadstool_common::primal_sockets::SocketPathEnv;
use toadstool_config::ports::{capability_fallback, resolve_capability_port};

// --- Network configurator defaults (overridable via env) ---

const DEFAULT_PROXY_LISTEN_PORT: u16 = 15001;
const DEFAULT_PROXY_ADMIN_PORT: u16 = 15000;
const DEFAULT_METRICS_PORT: u16 = 15090;
const DEFAULT_DNS_PORT: u16 = 53;
const DEFAULT_PROXY_CONCURRENCY: u32 = 2;
const DEFAULT_SIDECAR_IMAGE: &str = "toadstool/service-mesh-proxy:latest";
const RFC1918_RANGES: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];

/// Default DNS search suffixes for the orchestration resolver stack.
///
/// Used when building [`OrchestrationNetworkConfig`] defaults. Override the full list via
/// `dns_discovery.search_domains` in config, or set `TOADSTOOL_DNS_SEARCH_DOMAINS`
/// (comma-separated) for a process-wide default.
mod dns_defaults {
    /// Cluster-local search suffix for mesh-scoped names.
    pub const TOADSTOOL_CLUSTER: &str = "toadstool.local";
    /// Ecosystem-wide search suffix for shared discovery.
    pub const ECOSYSTEM: &str = "ecosystem.local";
    /// Default `TOADSTOOL_BASE_DOMAIN` when that variable is unset (third search label).
    pub const DEFAULT_BASE_DOMAIN: &str = "primal.local";
}

/// Resolver search domains: `TOADSTOOL_DNS_SEARCH_DOMAINS` first, then [`dns_defaults`].
fn default_orchestration_dns_search_domains() -> Vec<String> {
    if let Ok(v) = std::env::var("TOADSTOOL_DNS_SEARCH_DOMAINS") {
        let domains: Vec<String> = v
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if !domains.is_empty() {
            return domains;
        }
    }
    vec![
        dns_defaults::TOADSTOOL_CLUSTER.into(),
        dns_defaults::ECOSYSTEM.into(),
        std::env::var(socket_env::TOADSTOOL_BASE_DOMAIN)
            .unwrap_or_else(|_| dns_defaults::DEFAULT_BASE_DOMAIN.into()),
    ]
}

/// Parse `/etc/resolv.conf` text and return `nameserver` IP (or hostname) entries in order.
fn parse_resolv_conf(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("nameserver")
                .map(|rest| rest.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn default_audit_log_path() -> String {
    std::env::var("TOADSTOOL_AUDIT_LOG_PATH")
        .unwrap_or_else(|_| install_paths::VAR_LOG_TOADSTOOL_AUDIT.into())
}

/// Discover DNS resolvers from the environment or host system.
///
/// Resolution order:
/// 1. `TOADSTOOL_DNS_RESOLVERS` env var (comma-separated IP list)
/// 2. Host system `/etc/resolv.conf` nameserver entries (Linux/macOS)
/// 3. Empty list — fall back to the OS-level resolver
pub(super) fn system_dns_resolvers() -> Vec<String> {
    if let Ok(val) = std::env::var(socket_env::TOADSTOOL_DNS_RESOLVERS) {
        let servers: Vec<String> = val
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if !servers.is_empty() {
            return servers;
        }
    }

    // Parse /etc/resolv.conf on POSIX systems
    if let Ok(contents) = std::fs::read_to_string(etc_paths::RESOLV_CONF) {
        let servers = parse_resolv_conf(&contents);
        if !servers.is_empty() {
            return servers;
        }
    }

    Vec::new()
}

/// Full default network configuration for the orchestration network stack.
pub(super) fn orchestration_default_network_config() -> OrchestrationNetworkConfig {
    OrchestrationNetworkConfig {
        service_mesh: ServiceMeshConfig {
            enabled: true,
            mesh_type: "native".to_string(),
            sidecar: SidecarConfig {
                enabled: true,
                image: std::env::var("TOADSTOOL_SIDECAR_IMAGE")
                    .unwrap_or_else(|_| DEFAULT_SIDECAR_IMAGE.to_string()),
                resources: SidecarResources {
                    cpu_limit: "200m".to_string(),
                    memory_limit: "256Mi".to_string(),
                    cpu_request: "100m".to_string(),
                    memory_request: "128Mi".to_string(),
                },
                proxy: ProxyConfig {
                    proxy_type: "envoy".to_string(),
                    listen_port: DEFAULT_PROXY_LISTEN_PORT,
                    admin_port: DEFAULT_PROXY_ADMIN_PORT,
                    concurrency: DEFAULT_PROXY_CONCURRENCY,
                    timeouts: TimeoutConfig {
                        connection_timeout: Duration::from_secs(10),
                        request_timeout: Duration::from_secs(30),
                        read_timeout: Duration::from_secs(30),
                        write_timeout: Duration::from_secs(30),
                    },
                },
                telemetry: TelemetryConfig {
                    metrics_enabled: true,
                    tracing_enabled: true,
                    access_logs: true,
                    metrics_port: DEFAULT_METRICS_PORT,
                    tracing_endpoint: std::env::var("TOADSTOOL_JAEGER_ENDPOINT").ok(),
                },
            },
            mtls: MutualTLSConfig {
                enabled: std::env::var("TOADSTOOL_CA_CERT").is_ok(),
                ca_cert: std::env::var("TOADSTOOL_CA_CERT").unwrap_or_default(),
                service_cert: std::env::var("TOADSTOOL_SERVICE_CERT").unwrap_or_default(),
                private_key: std::env::var("TOADSTOOL_SERVICE_KEY").unwrap_or_default(),
                rotation_interval: Duration::from_secs(3600),
                verification_mode: "strict".to_string(),
            },
            service_discovery: ServiceDiscoveryConfig {
                enabled: true,
                backends: vec![
                    DiscoveryBackend {
                        backend_type: "dns".to_string(),
                        config: HashMap::new(),
                        priority: 1,
                        enabled: true,
                    },
                    DiscoveryBackend {
                        backend_type: "mdns".to_string(),
                        config: HashMap::new(),
                        priority: 2,
                        enabled: true,
                    },
                ],
                refresh_interval: Duration::from_secs(30),
                cache_ttl: Duration::from_secs(300),
                health_check_integration: true,
            },
            inter_service: InterServiceConfig {
                default_protocol: "jsonrpc".to_string(),
                connection_pooling: ConnectionPoolConfig {
                    enabled: true,
                    max_connections_per_host: 100,
                    max_idle_connections: 10,
                    idle_timeout: Duration::from_secs(300),
                    connection_lifetime: Duration::from_secs(3600),
                },
                retry: RetryConfig {
                    max_retries: 3,
                    base_delay: Duration::from_millis(100),
                    max_delay: Duration::from_secs(30),
                    backoff_multiplier: 2.0,
                    jitter_percent: 0.1,
                },
                timeouts: TimeoutConfig {
                    connection_timeout: Duration::from_secs(10),
                    request_timeout: Duration::from_secs(30),
                    read_timeout: Duration::from_secs(60),
                    write_timeout: Duration::from_secs(60),
                },
            },
        },
        dns_discovery: DnsDiscoveryConfig {
            enabled: true,
            dns_servers: system_dns_resolvers(),
            search_domains: default_orchestration_dns_search_domains(),
            // Use environment-aware service domains instead of hardcoded values
            service_domains: ServiceDomainsConfig::from_env(),
            resolution_timeout: Duration::from_secs(5),
            cache: DnsCacheConfig {
                base: CacheConfig {
                    enabled: true,
                    ttl: Duration::from_secs(300),
                    max_entries: 1000,
                    negative_ttl: Duration::from_secs(60),
                },
            },
        },
        cross_primal_security: CrossPrimalSecurityConfig {
            enabled: true,
            authentication: AuthenticationConfig {
                method: capabilities::CRYPTO.to_string(),
                token_validation: TokenValidationConfig {
                    validate_issuer: true,
                    validate_audience: true,
                    validate_expiration: true,
                    validate_signature: true,
                    clock_skew: Duration::from_secs(300),
                },
                certificate_validation: CertificateValidationConfig {
                    validate_chain: true,
                    validate_expiration: true,
                    validate_usage: true,
                    trusted_cas: vec![],
                },
                security: SecurityServiceIntegrationConfig {
                    enabled: true,
                    endpoint: {
                        let env = SocketPathEnv::from_env();
                        env.security_connection_hint.unwrap_or_else(|| {
                            let domains = ServiceDomainsConfig::from_env();
                            // `SocketPathEnv::security_connection_hint` first; then same port chain as
                            // health probes — env overrides with cold-start fallback matching
                            // `toadstool_common::constants::discovery_ports::DEFAULT_SECURITY_PORT`
                            // (`capability_fallback::SECURITY`).
                            let port =
                                resolve_capability_port("SECURITY", capability_fallback::SECURITY);
                            format!("http://{}:{port}", domains.security)
                        })
                    },
                    auth_token: None,
                    signature_verification: true,
                    crypto_lock: true,
                },
            },
            authorization: AuthorizationConfig {
                model: "rbac".to_string(),
                policy_engine: PolicyEngineConfig {
                    engine_type: "native".to_string(),
                    policy_files: vec![],
                    policy_endpoints: vec![],
                    evaluation_cache: true,
                },
                roles: vec![
                    RoleDefinition {
                        name: "admin".to_string(),
                        description: "Administrative access".to_string(),
                        permissions: vec!["*".to_string()],
                        inherits: vec![],
                    },
                    RoleDefinition {
                        name: "user".to_string(),
                        description: "User access".to_string(),
                        permissions: vec!["read".to_string(), "execute".to_string()],
                        inherits: vec![],
                    },
                ],
                permissions: HashMap::new(),
            },
            network_isolation: NetworkIsolationConfig {
                enabled: true,
                isolation_level: "strict".to_string(),
                allowed_networks: RFC1918_RANGES.iter().map(|s| (*s).to_string()).collect(),
                blocked_networks: vec![],
                firewall_rules: vec![],
            },
            audit_logging: AuditLoggingConfig {
                enabled: true,
                log_level: "info".to_string(),
                log_format: "json".to_string(),
                destinations: vec![LogDestination {
                    destination_type: "file".to_string(),
                    config: HashMap::from([(
                        "path".to_string(),
                        serde_json::Value::String(default_audit_log_path()),
                    )]),
                    enabled: true,
                }],
                retention: RetentionPolicy {
                    duration: Duration::from_secs(30 * 24 * 3600), // 30 days
                    compression: true,
                    archive_location: None,
                },
            },
        },
        network_policies: NetworkPoliciesConfig {
            enabled: true,
            default_policy: "deny".to_string(),
            ingress_rules: vec![IngressRule {
                name: "allow-intra-mesh".to_string(),
                from: vec![NetworkSelector {
                    selector_type: "label".to_string(),
                    value: "service-mesh=toadstool".to_string(),
                }],
                ports: vec![NetworkPort {
                    port: toadstool_config::config_utils::ConfigUtils::get_toadstool_port(),
                    protocol: "tcp".to_string(),
                    end_port: None,
                }],
                action: "allow".to_string(),
                priority: 100,
            }],
            egress_rules: vec![EgressRule {
                name: "allow-dns".to_string(),
                to: vec![NetworkSelector {
                    selector_type: "cidr".to_string(),
                    value: "0.0.0.0/0".to_string(),
                }],
                ports: vec![NetworkPort {
                    port: DEFAULT_DNS_PORT,
                    protocol: "udp".to_string(),
                    end_port: None,
                }],
                action: "allow".to_string(),
                priority: 100,
            }],
            service_mesh_policies: vec![],
        },
        traffic_management: TrafficManagementConfig {
            enabled: true,
            traffic_splitting: TrafficSplittingConfig {
                enabled: true,
                strategy: "weighted".to_string(),
                weights: HashMap::from([("stable".to_string(), 90), ("canary".to_string(), 10)]),
                header_routing: None,
            },
            canary: CanaryConfig {
                enabled: true,
                percentage: 5,
                success_criteria: SuccessCriteria {
                    success_rate: 0.99,
                    latency_p99: Duration::from_millis(500),
                    error_rate: 0.01,
                    evaluation_period: Duration::from_secs(300),
                },
                rollback_criteria: RollbackCriteria {
                    error_rate: 0.05,
                    latency_p99: Duration::from_secs(1),
                    evaluation_period: Duration::from_secs(60),
                    automatic_rollback: true,
                },
                automation: AutomationConfig {
                    enabled: true,
                    promotion_interval: Duration::from_secs(300),
                    max_promotion_steps: 5,
                    rollback_timeout: Duration::from_secs(30),
                },
            },
            blue_green: BlueGreenConfig {
                enabled: true,
                switch_strategy: "instant".to_string(),
                validation_period: Duration::from_secs(300),
                rollback_timeout: Duration::from_secs(30),
            },
            rate_limiting: RateLimitingConfig {
                enabled: true,
                global_limit: Some(RateLimit {
                    requests_per_second: 1000,
                    burst_size: 2000,
                    window_size: Duration::from_secs(60),
                }),
                service_limits: HashMap::new(),
                user_limits: HashMap::new(),
            },
            traffic_mirroring: TrafficMirroringConfig {
                enabled: false,
                destinations: vec![],
                percentage: 0,
                mirror_headers: true,
            },
        },
        load_balancing: LoadBalancingConfig {
            enabled: true,
            algorithm: "round_robin".to_string(),
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
            sticky_sessions: StickySessionsConfig {
                enabled: false,
                affinity_type: "cookie".to_string(),
                cookie: Some(CookieConfig {
                    name: "toadstool-session".to_string(),
                    domain: None,
                    path: Some("/".to_string()),
                    secure: true,
                    http_only: true,
                }),
                timeout: Duration::from_secs(3600),
            },
            backends: vec![],
        },
        circuit_breaker: CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(30),
            reset_timeout: Duration::from_secs(300),
        },
        health_monitoring: HealthMonitoringConfig {
            enabled: true,
            interval: Duration::from_secs(30),
            // Health endpoints now constructed dynamically from service domains + env ports
            endpoints: {
                let domains = ServiceDomainsConfig::from_env();
                let coordination_port =
                    resolve_capability_port("COORDINATION", capability_fallback::COORDINATION);
                let security_port =
                    resolve_capability_port("SECURITY", capability_fallback::SECURITY);
                let storage_port = resolve_capability_port("STORAGE", capability_fallback::STORAGE);
                let ai_processing_port =
                    resolve_capability_port("PLATFORM", capability_fallback::PLATFORM);
                vec![
                    HealthEndpoint {
                        name: "orchestration".to_string(), // Capability-based name
                        url: format!("http://{}:{coordination_port}/health", domains.coordination),
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
                    },
                    HealthEndpoint {
                        name: "pki".to_string(), // Capability-based name
                        url: format!("http://{}:{security_port}/health", domains.security),
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
                    },
                    HealthEndpoint {
                        name: "storage".to_string(), // Capability-based name
                        url: format!("http://{}:{storage_port}/health", domains.storage),
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
                    },
                    HealthEndpoint {
                        name: "ai".to_string(), // Capability-based name
                        url: format!(
                            "http://{}:{ai_processing_port}/health",
                            domains.ai_processing
                        ),
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
                    },
                ]
            },
            alerting: AlertingConfig {
                enabled: true,
                channels: vec![AlertChannel {
                    name: "console".to_string(),
                    channel_type: "console".to_string(),
                    config: HashMap::new(),
                }],
                rules: vec![AlertRule {
                    name: "service-down".to_string(),
                    condition: "health_check_failed".to_string(),
                    severity: "critical".to_string(),
                    channels: vec!["console".to_string()],
                }],
            },
            metrics: MetricsConfig {
                enabled: true,
                endpoint: "/metrics".to_string(),
                interval: Duration::from_secs(15),
                exporters: vec![MetricsExporter {
                    exporter_type: "prometheus".to_string(),
                    config: {
                        let prometheus_port = std::env::var("TOADSTOOL_PROMETHEUS_PORT")
                            .or_else(|_| std::env::var("PROMETHEUS_PORT"))
                            .ok()
                            .and_then(|p| p.parse().ok())
                            .unwrap_or(toadstool_config::ports::toadstool::METRICS);
                        let prometheus_host = std::env::var("TOADSTOOL_PROMETHEUS_HOST")
                            .or_else(|_| std::env::var("PROMETHEUS_HOST"))
                            .unwrap_or_else(|_| "prometheus".to_string());
                        HashMap::from([(
                            "endpoint".to_string(),
                            serde_json::Value::String(format!(
                                "http://{prometheus_host}:{prometheus_port}"
                            )),
                        )])
                    },
                    enabled: true,
                }],
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse_resolv_conf;

    #[test]
    fn parse_resolv_conf_valid_multiple_nameservers() {
        let text = "# Managed by test\nnameserver 8.8.8.8\nnameserver 2001:4860:4860::8888\n";
        assert_eq!(
            parse_resolv_conf(text),
            vec!["8.8.8.8".to_string(), "2001:4860:4860::8888".to_string()]
        );
    }

    #[test]
    fn parse_resolv_conf_no_nameserver_lines() {
        let text = "search example.com\noptions ndots:5\n";
        assert!(parse_resolv_conf(text).is_empty());
    }

    #[test]
    fn parse_resolv_conf_comments_and_ignored_lines() {
        let text = r"# resolv.conf
# nameserver 1.1.1.1
; nameserver 9.9.9.9
nameserver 10.0.0.1
";
        assert_eq!(parse_resolv_conf(text), vec!["10.0.0.1".to_string()]);
    }

    #[test]
    fn parse_resolv_conf_empty() {
        assert!(parse_resolv_conf("").is_empty());
        assert!(parse_resolv_conf("   \n\t\n").is_empty());
    }
}
