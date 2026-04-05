// SPDX-License-Identifier: AGPL-3.0-or-later

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

const fn default_weight() -> u32 {
    100
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use toadstool_common::config_bases::{BackendEndpoint, HttpHealthCheckConfig};

    use super::{BackendConfig, CookieConfig, LoadBalancingConfig, StickySessionsConfig};

    fn sample_http_health() -> HttpHealthCheckConfig {
        HttpHealthCheckConfig::default()
    }

    #[test]
    fn cookie_config_serde_roundtrip_clone_debug() {
        let c = CookieConfig {
            name: "sid".to_string(),
            domain: Some("example.com".to_string()),
            path: Some("/app".to_string()),
            secure: true,
            http_only: false,
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let back: CookieConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.name, c.name);
        assert_eq!(back.domain, c.domain);
        assert_eq!(back.path, c.path);
        assert_eq!(back.secure, c.secure);
        assert_eq!(back.http_only, c.http_only);

        let cloned = c.clone();
        assert_eq!(cloned.name, c.name);

        let dbg = format!("{c:?}");
        assert!(dbg.contains("sid"));
    }

    #[test]
    fn sticky_sessions_serde_roundtrip_clone_debug() {
        let s = StickySessionsConfig {
            enabled: true,
            affinity_type: "cookie".to_string(),
            cookie: None,
            timeout: Duration::from_secs(3600),
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let back: StickySessionsConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.enabled, s.enabled);
        assert_eq!(back.affinity_type, s.affinity_type);
        assert!(back.cookie.is_none() && s.cookie.is_none());
        assert_eq!(back.timeout, s.timeout);

        let cloned = s.clone();
        assert_eq!(cloned.timeout, s.timeout);
        assert!(format!("{s:?}").contains("cookie"));
    }

    #[test]
    fn backend_config_default_weight_and_flattened_endpoint() {
        let json = json!({
            "name": "be1",
            "address": "10.0.0.1",
            "port": 9000,
            "enabled": true
        });
        let bc: BackendConfig = serde_json::from_value(json).expect("deserialize");
        assert_eq!(bc.weight, 100);
        assert_eq!(bc.endpoint.name, "be1");
        assert_eq!(bc.endpoint.address, "10.0.0.1");
        assert_eq!(bc.endpoint.port, 9000);
        assert!(bc.endpoint.enabled);
        assert!(bc.health_check.is_none());

        let back = serde_json::to_string(&bc).expect("serialize");
        let bc2: BackendConfig = serde_json::from_str(&back).expect("round-trip");
        assert_eq!(bc2.weight, bc.weight);
        assert_eq!(bc2.endpoint.name, bc.endpoint.name);
    }

    #[test]
    fn backend_config_explicit_weight_roundtrip() {
        let bc = BackendConfig {
            endpoint: BackendEndpoint::new("api", "127.0.0.1", 8443),
            weight: 42,
            health_check: Some(sample_http_health()),
        };
        let json = serde_json::to_string(&bc).expect("serialize");
        let back: BackendConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.weight, 42);
        assert_eq!(back.endpoint.port, 8443);
        assert!(back.health_check.is_some());
    }

    #[test]
    fn load_balancing_config_full_roundtrip() {
        let cfg = LoadBalancingConfig {
            enabled: true,
            algorithm: "least_conn".to_string(),
            health_check: sample_http_health(),
            sticky_sessions: StickySessionsConfig {
                enabled: false,
                affinity_type: "ip".to_string(),
                cookie: None,
                timeout: Duration::from_secs(120),
            },
            backends: vec![BackendConfig {
                endpoint: BackendEndpoint::new("b1", "192.168.1.1", 80),
                weight: 10,
                health_check: None,
            }],
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: LoadBalancingConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.enabled, cfg.enabled);
        assert_eq!(back.algorithm, cfg.algorithm);
        assert_eq!(back.sticky_sessions.affinity_type, "ip");
        assert_eq!(back.backends.len(), 1);
        assert_eq!(back.backends[0].endpoint.name, "b1");
        assert!(format!("{cfg:?}").contains("least_conn"));
    }
}
