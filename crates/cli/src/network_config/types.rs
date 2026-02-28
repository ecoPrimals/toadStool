//! # Songbird Network Configuration Module - Type Definitions
//!
//! This module contains all type definitions for Songbird network configuration.
//!
//! Many of these types now use base configurations from `toadstool_common::config_bases`
//! for consistency and code reuse.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{
    BackendEndpoint, CacheConfig, ConnectionPoolConfig, HttpHealthCheckConfig, RetryConfig,
    TelemetryConfig, TimeoutConfig,
};

/// Complete Songbird network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdNetworkConfig {
    /// Service mesh configuration
    pub service_mesh: ServiceMeshConfig,
    /// DNS service discovery configuration
    pub dns_discovery: DnsDiscoveryConfig,
    /// Cross-primal security configuration
    pub cross_primal_security: CrossPrimalSecurityConfig,
    /// Network ingress/egress rules
    pub network_policies: NetworkPoliciesConfig,
    /// Traffic management configuration
    pub traffic_management: TrafficManagementConfig,
    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Health monitoring configuration
    pub health_monitoring: HealthMonitoringConfig,
}

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    /// Enable service mesh
    pub enabled: bool,
    /// Service mesh type (Istio, Linkerd, Consul, Native)
    pub mesh_type: String,
    /// Sidecar proxy configuration
    pub sidecar: SidecarConfig,
    /// Mutual TLS configuration
    pub mtls: MutualTLSConfig,
    /// Service discovery integration
    pub service_discovery: ServiceDiscoveryConfig,
    /// Inter-service communication settings
    pub inter_service: InterServiceConfig,
}

/// Sidecar proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarConfig {
    /// Enable sidecar injection
    pub enabled: bool,
    /// Sidecar image
    pub image: String,
    /// Resource limits
    pub resources: SidecarResources,
    /// Proxy configuration
    pub proxy: ProxyConfig,
    /// Telemetry configuration
    pub telemetry: TelemetryConfig,
}

/// Sidecar resource limits
///
/// Follows the ResourceLimit pattern from `toadstool_common::config_bases` for consistency.
/// Uses Kubernetes-style resource specification (e.g., "200m", "256Mi").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarResources {
    /// CPU limit (e.g., "200m")
    pub cpu_limit: String,
    /// Memory limit (e.g., "256Mi")
    pub memory_limit: String,
    /// CPU request (e.g., "100m")
    pub cpu_request: String,
    /// Memory request (e.g., "128Mi")
    pub memory_request: String,
}

// Note: This struct maintains compatibility with Kubernetes resource specifications.
// While it follows a similar pattern to BaseResourceConfig, it uses explicit String
// fields for easier Kubernetes integration rather than the Option<String> pattern.

/// Proxy configuration
///
/// Uses base `TimeoutConfig` for consistent timeout handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy type (envoy, nginx, haproxy)
    pub proxy_type: String,
    /// Listen port
    pub listen_port: u16,
    /// Admin port
    pub admin_port: u16,
    /// Concurrency
    pub concurrency: u32,
    /// Timeout configuration
    #[serde(flatten)]
    pub timeouts: TimeoutConfig,
}

// Note: TelemetryConfig is now imported from toadstool_common::config_bases
// for consistency across the codebase

/// Mutual TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutualTLSConfig {
    /// Enable mTLS
    pub enabled: bool,
    /// Certificate authority
    pub ca_cert: String,
    /// Service certificate
    pub service_cert: String,
    /// Private key
    pub private_key: String,
    /// Certificate rotation interval
    pub rotation_interval: Duration,
    /// Verification mode (strict, permissive, disabled)
    pub verification_mode: String,
}

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryConfig {
    /// Enable service discovery
    pub enabled: bool,
    /// Discovery backends
    pub backends: Vec<DiscoveryBackend>,
    /// Refresh interval
    pub refresh_interval: Duration,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Health check integration
    pub health_check_integration: bool,
}

/// Discovery backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryBackend {
    /// Backend type (dns, consul, etcd, kubernetes)
    pub backend_type: String,
    /// Backend configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Priority
    pub priority: u32,
    /// Enabled
    pub enabled: bool,
}

/// Inter-service communication configuration
///
/// Uses base configurations for retries, timeouts, and connection pooling.
/// All network-related base configs are now imported from `toadstool_common::config_bases`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterServiceConfig {
    /// Default communication protocol
    pub default_protocol: String,
    /// Connection pooling (uses base ConnectionPoolConfig)
    pub connection_pooling: ConnectionPoolConfig,
    /// Retry configuration (uses base RetryConfig)
    pub retry: RetryConfig,
    /// Timeout configuration (uses base TimeoutConfig)
    pub timeouts: TimeoutConfig,
}

// NOTE: ConnectionPoolConfig, RetryConfig, and TimeoutConfig are now imported
// from toadstool_common::config_bases. The previous definitions here have been
// removed to use the shared base types for consistency across the codebase.

/// DNS service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDiscoveryConfig {
    /// Enable DNS discovery
    pub enabled: bool,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
    /// Service domains
    pub service_domains: ServiceDomainsConfig,
    /// DNS resolution timeout
    pub resolution_timeout: Duration,
    /// DNS cache configuration
    pub cache: DnsCacheConfig,
}

/// Service domains configuration
///
/// **DEPRECATED**: This configuration uses hardcoded primal names.
/// New code should use capability-based discovery instead.
///
/// For backward compatibility, this can be constructed from environment
/// variables or a base domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDomainsConfig {
    /// ToadStool domain
    pub toadstool: String,
    /// Songbird domain (DEPRECATED: use ORCHESTRATION capability)
    pub songbird: String,
    /// BearDog domain (DEPRECATED: use PKI capability)
    pub beardog: String,
    /// NestGate domain (DEPRECATED: use STORAGE capability)
    pub nestgate: String,
    /// Squirrel domain (DEPRECATED: use AI_PROCESSING capability)
    pub squirrel: String,
    /// BiomeOS domain
    pub biomeos: String,
}

impl ServiceDomainsConfig {
    /// Create service domains from environment or defaults
    ///
    /// Reads TOADSTOOL_BASE_DOMAIN (default: "primal.local")
    /// and constructs service-specific domains.
    ///
    /// Individual services can be overridden with:
    /// - TOADSTOOL_DOMAIN
    /// - SONGBIRD_DOMAIN
    /// - BEARDOG_DOMAIN
    /// - NESTGATE_DOMAIN
    /// - SQUIRREL_DOMAIN
    /// - BIOMEOS_DOMAIN
    pub fn from_env() -> Self {
        let base_domain =
            std::env::var("TOADSTOOL_BASE_DOMAIN").unwrap_or_else(|_| "primal.local".to_string());

        Self {
            toadstool: std::env::var("TOADSTOOL_DOMAIN")
                .unwrap_or_else(|_| format!("toadstool.{base_domain}")),
            songbird: std::env::var("SONGBIRD_DOMAIN")
                .unwrap_or_else(|_| format!("songbird.{base_domain}")),
            beardog: std::env::var("BEARDOG_DOMAIN")
                .unwrap_or_else(|_| format!("beardog.{base_domain}")),
            nestgate: std::env::var("NESTGATE_DOMAIN")
                .unwrap_or_else(|_| format!("nestgate.{base_domain}")),
            squirrel: std::env::var("SQUIRREL_DOMAIN")
                .unwrap_or_else(|_| format!("squirrel.{base_domain}")),
            biomeos: std::env::var("BIOMEOS_DOMAIN")
                .unwrap_or_else(|_| format!("biomeos.{base_domain}")),
        }
    }

    /// Create with a custom base domain
    pub fn with_base_domain(base_domain: &str) -> Self {
        Self {
            toadstool: format!("toadstool.{base_domain}"),
            songbird: format!("songbird.{base_domain}"),
            beardog: format!("beardog.{base_domain}"),
            nestgate: format!("nestgate.{base_domain}"),
            squirrel: format!("squirrel.{base_domain}"),
            biomeos: format!("biomeos.{base_domain}"),
        }
    }
}

/// DNS cache configuration
///
/// Uses base `CacheConfig` with DNS-specific semantics.
/// The base configuration provides standard caching parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsCacheConfig {
    /// Base cache configuration (enabled, ttl, max_entries, negative_ttl)
    #[serde(flatten)]
    pub base: CacheConfig,
}

/// Cross-primal security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossPrimalSecurityConfig {
    /// Enable cross-primal security
    pub enabled: bool,
    /// Authentication requirements
    pub authentication: AuthenticationConfig,
    /// Authorization policies
    pub authorization: AuthorizationConfig,
    /// Network isolation
    pub network_isolation: NetworkIsolationConfig,
    /// Audit logging
    pub audit_logging: AuditLoggingConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication method (jwt, oauth2, mtls, beardog)
    pub method: String,
    /// Token validation
    pub token_validation: TokenValidationConfig,
    /// Certificate validation
    pub certificate_validation: CertificateValidationConfig,
    /// BearDog integration
    pub beardog_integration: BearDogIntegrationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Issuer validation
    pub validate_issuer: bool,
    /// Audience validation
    pub validate_audience: bool,
    /// Expiration validation
    pub validate_expiration: bool,
    /// Signature validation
    pub validate_signature: bool,
    /// Clock skew tolerance
    pub clock_skew: Duration,
}

/// Certificate validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateValidationConfig {
    /// Validate certificate chain
    pub validate_chain: bool,
    /// Validate certificate expiration
    pub validate_expiration: bool,
    /// Validate certificate usage
    pub validate_usage: bool,
    /// Trusted CA certificates
    pub trusted_cas: Vec<String>,
}

/// BearDog integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogIntegrationConfig {
    /// Enable BearDog integration
    pub enabled: bool,
    /// BearDog endpoint
    pub endpoint: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Signature verification
    pub signature_verification: bool,
    /// Crypto-lock integration
    pub crypto_lock: bool,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization model (rbac, abac, policy)
    pub model: String,
    /// Policy engine
    pub policy_engine: PolicyEngineConfig,
    /// Role definitions
    pub roles: Vec<RoleDefinition>,
    /// Permission matrix
    pub permissions: HashMap<String, Vec<String>>,
}

/// Policy engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngineConfig {
    /// Engine type (opa, casbin, native)
    pub engine_type: String,
    /// Policy files
    pub policy_files: Vec<String>,
    /// Policy endpoints
    pub policy_endpoints: Vec<String>,
    /// Evaluation cache
    pub evaluation_cache: bool,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// Role name
    pub name: String,
    /// Role description
    pub description: String,
    /// Permissions
    pub permissions: Vec<String>,
    /// Inheritance
    pub inherits: Vec<String>,
}

/// Network isolation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIsolationConfig {
    /// Enable network isolation
    pub enabled: bool,
    /// Isolation level (none, basic, strict, paranoid)
    pub isolation_level: String,
    /// Allowed networks
    pub allowed_networks: Vec<String>,
    /// Blocked networks
    pub blocked_networks: Vec<String>,
    /// Firewall rules
    pub firewall_rules: Vec<FirewallRule>,
}

/// Firewall rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    /// Rule name
    pub name: String,
    /// Rule action (allow, deny, log)
    pub action: String,
    /// Source
    pub source: String,
    /// Destination
    pub destination: String,
    /// Protocol
    pub protocol: String,
    /// Port range
    pub port_range: Option<String>,
    /// Priority
    pub priority: u32,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log level
    pub log_level: String,
    /// Log format
    pub log_format: String,
    /// Log destinations
    pub destinations: Vec<LogDestination>,
    /// Retention policy
    pub retention: RetentionPolicy,
}

/// Log destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDestination {
    /// Destination type (file, syslog, elasticsearch, s3)
    pub destination_type: String,
    /// Destination configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Enabled
    pub enabled: bool,
}

/// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Retention duration
    pub duration: Duration,
    /// Compression enabled
    pub compression: bool,
    /// Archive location
    pub archive_location: Option<String>,
}

/// Network policies configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPoliciesConfig {
    /// Enable network policies
    pub enabled: bool,
    /// Default policy (allow, deny)
    pub default_policy: String,
    /// Ingress rules
    pub ingress_rules: Vec<IngressRule>,
    /// Egress rules
    pub egress_rules: Vec<EgressRule>,
    /// Service mesh policies
    pub service_mesh_policies: Vec<ServiceMeshPolicy>,
}

/// Ingress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRule {
    /// Rule name
    pub name: String,
    /// Source selectors
    pub from: Vec<NetworkSelector>,
    /// Port specifications
    pub ports: Vec<NetworkPort>,
    /// Action (allow, deny)
    pub action: String,
    /// Priority
    pub priority: u32,
}

/// Egress rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgressRule {
    /// Rule name
    pub name: String,
    /// Destination selectors
    pub to: Vec<NetworkSelector>,
    /// Port specifications
    pub ports: Vec<NetworkPort>,
    /// Action (allow, deny)
    pub action: String,
    /// Priority
    pub priority: u32,
}

/// Network selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSelector {
    /// Selector type (ip, cidr, service, label)
    pub selector_type: String,
    /// Selector value
    pub value: String,
}

/// Network port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPort {
    /// Port number
    pub port: u16,
    /// Protocol (tcp, udp, sctp)
    pub protocol: String,
    /// End port (for ranges)
    pub end_port: Option<u16>,
}

/// Service mesh policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshPolicy {
    /// Policy name
    pub name: String,
    /// Policy type (traffic, security, observability)
    pub policy_type: String,
    /// Selector
    pub selector: HashMap<String, String>,
    /// Configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Traffic management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficManagementConfig {
    /// Enable traffic management
    pub enabled: bool,
    /// Traffic splitting
    pub traffic_splitting: TrafficSplittingConfig,
    /// Canary deployments
    pub canary: CanaryConfig,
    /// Blue-green deployments
    pub blue_green: BlueGreenConfig,
    /// Rate limiting
    pub rate_limiting: RateLimitingConfig,
    /// Traffic mirroring
    pub traffic_mirroring: TrafficMirroringConfig,
}

/// Traffic splitting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSplittingConfig {
    /// Enable traffic splitting
    pub enabled: bool,
    /// Splitting strategy (weighted, header, cookie)
    pub strategy: String,
    /// Weight distribution
    pub weights: HashMap<String, u32>,
    /// Header-based routing
    pub header_routing: Option<HeaderRoutingConfig>,
}

/// Header routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderRoutingConfig {
    /// Header name
    pub header_name: String,
    /// Header value mappings
    pub value_mappings: HashMap<String, String>,
    /// Default destination
    pub default_destination: String,
}

/// Canary deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Enable canary deployments
    pub enabled: bool,
    /// Canary percentage
    pub percentage: u32,
    /// Success criteria
    pub success_criteria: SuccessCriteria,
    /// Rollback criteria
    pub rollback_criteria: RollbackCriteria,
    /// Automation settings
    pub automation: AutomationConfig,
}

/// Success criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    /// Success rate threshold
    pub success_rate: f64,
    /// Latency threshold
    pub latency_p99: Duration,
    /// Error rate threshold
    pub error_rate: f64,
    /// Evaluation period
    pub evaluation_period: Duration,
}

/// Rollback criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackCriteria {
    /// Error rate threshold
    pub error_rate: f64,
    /// Latency threshold
    pub latency_p99: Duration,
    /// Evaluation period
    pub evaluation_period: Duration,
    /// Automatic rollback
    pub automatic_rollback: bool,
}

/// Automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationConfig {
    /// Enable automation
    pub enabled: bool,
    /// Promotion interval
    pub promotion_interval: Duration,
    /// Maximum promotion steps
    pub max_promotion_steps: u32,
    /// Rollback timeout
    pub rollback_timeout: Duration,
}

/// Blue-green deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenConfig {
    /// Enable blue-green deployments
    pub enabled: bool,
    /// Switch strategy (instant, gradual)
    pub switch_strategy: String,
    /// Validation period
    pub validation_period: Duration,
    /// Rollback timeout
    pub rollback_timeout: Duration,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Global rate limit
    pub global_limit: Option<RateLimit>,
    /// Per-service rate limits
    pub service_limits: HashMap<String, RateLimit>,
    /// Per-user rate limits
    pub user_limits: HashMap<String, RateLimit>,
}

/// Rate limit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requests per second
    pub requests_per_second: u32,
    /// Burst size
    pub burst_size: u32,
    /// Window size
    pub window_size: Duration,
}

/// Traffic mirroring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMirroringConfig {
    /// Enable traffic mirroring
    pub enabled: bool,
    /// Mirror destinations
    pub destinations: Vec<MirrorDestination>,
    /// Mirror percentage
    pub percentage: u32,
    /// Mirror request headers
    pub mirror_headers: bool,
}

/// Mirror destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorDestination {
    /// Destination service
    pub service: String,
    /// Destination weight
    pub weight: u32,
    /// Sampling percentage
    pub sampling: u32,
}

/// Load balancing configuration
///
/// Uses `HttpHealthCheckConfig` for HTTP-based health checks with path and status code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Enable load balancing
    pub enabled: bool,
    /// Load balancing algorithm
    pub algorithm: String,
    /// Health check configuration
    pub health_check: HttpHealthCheckConfig,
    /// Sticky sessions
    pub sticky_sessions: StickySessionsConfig,
    /// Backend configuration
    pub backends: Vec<BackendConfig>,
}

// NOTE: HealthCheckConfig is now imported from toadstool_common::config_bases
// Use HttpHealthCheckConfig for HTTP-specific health checks with path and expected_status

/// Sticky sessions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickySessionsConfig {
    /// Enable sticky sessions
    pub enabled: bool,
    /// Session affinity type (cookie, ip, header)
    pub affinity_type: String,
    /// Cookie configuration
    pub cookie: Option<CookieConfig>,
    /// Session timeout
    pub timeout: Duration,
}

/// Cookie configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieConfig {
    /// Cookie name
    pub name: String,
    /// Cookie domain
    pub domain: Option<String>,
    /// Cookie path
    pub path: Option<String>,
    /// Secure flag
    pub secure: bool,
    /// HttpOnly flag
    pub http_only: bool,
}

/// Backend configuration for load balancing
///
/// Uses base `BackendEndpoint` with additional load balancing fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend endpoint (name, address, port, enabled)
    #[serde(flatten)]
    pub endpoint: BackendEndpoint,
    /// Backend weight for load balancing
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Backend health check configuration
    pub health_check: Option<HttpHealthCheckConfig>,
}

fn default_weight() -> u32 {
    100
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold
    pub success_threshold: u32,
    /// Timeout duration
    pub timeout: Duration,
    /// Half-open timeout
    pub half_open_timeout: Duration,
    /// Reset timeout
    pub reset_timeout: Duration,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringConfig {
    /// Enable health monitoring
    pub enabled: bool,
    /// Monitoring interval
    pub interval: Duration,
    /// Health check endpoints
    pub endpoints: Vec<HealthEndpoint>,
    /// Alerting configuration
    pub alerting: AlertingConfig,
    /// Metrics collection
    pub metrics: MetricsConfig,
}

/// Health endpoint configuration
///
/// Composes HTTP health check configuration with an endpoint name and URL.
/// Uses base `HttpHealthCheckConfig` for consistent health checking across the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEndpoint {
    /// Endpoint name
    pub name: String,
    /// Endpoint URL  
    pub url: String,
    /// HTTP health check configuration (includes timeout, retries, status checks)
    #[serde(flatten)]
    pub health_check: HttpHealthCheckConfig,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Alert channels
    pub channels: Vec<AlertChannel>,
    /// Alert rules
    pub rules: Vec<AlertRule>,
}

/// Alert channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    /// Channel name
    pub name: String,
    /// Channel type (email, slack, webhook)
    pub channel_type: String,
    /// Channel configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: String,
    /// Severity level
    pub severity: String,
    /// Target channels
    pub channels: Vec<String>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Collection interval
    pub interval: Duration,
    /// Metrics exporters
    pub exporters: Vec<MetricsExporter>,
}

/// Metrics exporter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExporter {
    /// Exporter type (prometheus, influx, datadog)
    pub exporter_type: String,
    /// Exporter configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Enabled
    pub enabled: bool,
}
