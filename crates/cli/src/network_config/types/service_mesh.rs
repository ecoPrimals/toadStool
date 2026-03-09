// SPDX-License-Identifier: AGPL-3.0-only

//! Service mesh configuration types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{
    ConnectionPoolConfig, RetryConfig, TelemetryConfig, TimeoutConfig,
};

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
    pub rotation_interval: std::time::Duration,
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
    pub refresh_interval: std::time::Duration,
    /// Cache TTL
    pub cache_ttl: std::time::Duration,
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
