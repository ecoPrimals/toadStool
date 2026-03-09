// SPDX-License-Identifier: AGPL-3.0-only

//! Load balancing configuration types.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::{BackendEndpoint, HttpHealthCheckConfig};

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
