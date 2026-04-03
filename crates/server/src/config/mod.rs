// SPDX-License-Identifier: AGPL-3.0-only
//! Server configuration types and defaults

use std::collections::HashMap;
use std::time::Duration;

// Centralized timeout constants (Deep Debt evolution)
use toadstool_common::constants::timeouts::{HEALTH_CHECK_INTERVAL, WORKLOAD_EXECUTION_TIMEOUT};

/// `ToadStool` server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,

    /// Enable HTTP API endpoints
    pub enable_api: bool,

    /// Enable CORS for API access
    pub enable_cors: bool,

    /// Maximum concurrent executions
    pub max_concurrent_executions: u32,

    /// Default execution timeout
    pub default_timeout: Duration,

    /// Resource monitoring interval
    pub resource_monitoring_interval: Duration,

    /// Authentication configuration
    pub auth: Option<AuthenticationConfig>,

    /// Rate limiting configuration
    pub rate_limiting: Option<RateLimitingConfig>,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Health check configuration
    pub health_check: HealthCheckConfig,

    /// Primal capability system configuration (optional)
    pub primal_capabilities: Option<PrimalCapabilitiesConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let host = std::env::var("TOADSTOOL_BIND_ADDRESS")
            .unwrap_or_else(|_| toadstool_config::constants::network::LOCALHOST.to_string());
        let port = toadstool_config::ports::daemon_port();
        Self {
            bind_address: format!("{host}:{port}"),
            enable_api: true,
            enable_cors: true,
            max_concurrent_executions: 100,
            default_timeout: WORKLOAD_EXECUTION_TIMEOUT,
            resource_monitoring_interval: HEALTH_CHECK_INTERVAL,
            auth: None,
            rate_limiting: None,
            logging: LoggingConfig::default(),
            health_check: HealthCheckConfig::default(),
            primal_capabilities: Some(PrimalCapabilitiesConfig::default()),
        }
    }
}

impl ServerConfig {
    /// Set bind address
    pub fn bind_address<S: Into<String>>(mut self, address: S) -> Self {
        self.bind_address = address.into();
        self
    }

    /// Enable or disable API endpoints
    #[must_use]
    pub fn enable_api(mut self, enabled: bool) -> Self {
        self.enable_api = enabled;
        self
    }

    /// Set maximum concurrent executions
    #[must_use]
    pub fn max_concurrent_executions(mut self, max: u32) -> Self {
        self.max_concurrent_executions = max;
        self
    }

    /// Set default execution timeout
    #[must_use]
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set authentication configuration
    #[must_use]
    pub fn auth(mut self, auth: AuthenticationConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set rate limiting configuration
    #[must_use]
    pub fn rate_limiting(mut self, rate_limiting: RateLimitingConfig) -> Self {
        self.rate_limiting = Some(rate_limiting);
        self
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Default)]
pub struct AuthenticationConfig {
    /// Require authentication for all endpoints
    pub required: bool,

    /// Valid API keys
    pub api_keys: Vec<String>,

    /// JWT secret for token validation
    pub jwt_secret: Option<String>,

    /// Basic auth credentials (username -> password)
    pub basic_auth: HashMap<String, String>,

    /// Custom authentication handler
    pub custom_validator: Option<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitingConfig {
    /// Maximum requests per minute per client
    pub requests_per_minute: u32,

    /// Maximum concurrent executions per client
    pub concurrent_executions_per_client: u32,

    /// Enable rate limiting by IP address
    pub limit_by_ip: bool,

    /// Enable rate limiting by API key
    pub limit_by_api_key: bool,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            concurrent_executions_per_client: 10,
            limit_by_ip: true,
            limit_by_api_key: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (debug, info, warn, error)
    pub level: String,

    /// Enable request logging
    pub log_requests: bool,

    /// Enable execution logging
    pub log_executions: bool,

    /// Enable performance metrics logging
    pub log_metrics: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_requests: true,
            log_executions: true,
            log_metrics: true,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub interval: Duration,

    /// Enable runtime engine health checks
    pub check_runtime_engines: bool,

    /// Enable resource health checks
    pub check_resources: bool,

    /// Memory usage threshold for unhealthy status
    pub memory_threshold_percent: f64,

    /// CPU usage threshold for unhealthy status
    pub cpu_threshold_percent: f64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: HEALTH_CHECK_INTERVAL,
            check_runtime_engines: true,
            check_resources: true,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
        }
    }
}

/// Primal capability system configuration.
///
/// Fields use capability-domain names per `CAPABILITY_BASED_DISCOVERY_STANDARD.md`.
/// Legacy primal-name env vars (`SONGBIRD_ENDPOINT`, `SQUIRREL_ENDPOINT`) are accepted
/// as fallbacks for backward compatibility.
#[derive(Debug, Clone)]
pub struct PrimalCapabilitiesConfig {
    /// Enable capability provider
    pub enabled: bool,

    /// Coordination capability endpoint (formerly songbird)
    pub coordination_endpoint: Option<String>,

    /// AI processing capability endpoint (formerly squirrel)
    pub ai_processing_endpoint: Option<String>,

    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,

    /// Auto-register on startup
    pub auto_register: bool,
}

impl Default for PrimalCapabilitiesConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("ENABLE_PRIMAL_CAPABILITIES")
                .map(|v| v == "true")
                .unwrap_or_default(),
            coordination_endpoint: std::env::var("COORDINATION_ENDPOINT")
                .or_else(|_| std::env::var("SONGBIRD_ENDPOINT"))
                .ok(),
            ai_processing_endpoint: std::env::var("AI_PROCESSING_ENDPOINT")
                .or_else(|_| std::env::var("SQUIRREL_ENDPOINT"))
                .ok(),
            heartbeat_interval_secs: std::env::var("PRIMAL_HEARTBEAT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            auto_register: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_config_default() {
        let config = AuthenticationConfig::default();
        assert!(!config.required);
        assert!(config.api_keys.is_empty());
        assert!(config.jwt_secret.is_none());
        assert!(config.basic_auth.is_empty());
        assert!(config.custom_validator.is_none());
    }

    #[test]
    fn test_rate_limiting_config_default() {
        let config = RateLimitingConfig::default();
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.concurrent_executions_per_client, 10);
        assert!(config.limit_by_ip);
        assert!(config.limit_by_api_key);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.log_requests);
        assert!(config.log_executions);
        assert!(config.log_metrics);
    }

    #[test]
    fn test_health_check_config_default() {
        let config = HealthCheckConfig::default();
        assert!(config.check_runtime_engines);
        assert!(config.check_resources);
        assert_eq!(config.memory_threshold_percent, 90.0);
        assert_eq!(config.cpu_threshold_percent, 95.0);
    }

    #[test]
    fn test_server_config_builder_chain() {
        let config = ServerConfig::default()
            .bind_address("127.0.0.1:0".to_string())
            .enable_api(false)
            .max_concurrent_executions(50)
            .default_timeout(Duration::from_secs(120));

        assert_eq!(config.bind_address, "127.0.0.1:0");
        assert!(!config.enable_api);
        assert_eq!(config.max_concurrent_executions, 50);
        assert_eq!(config.default_timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert!(config.enable_api);
        assert!(config.enable_cors);
        assert_eq!(config.max_concurrent_executions, 100);
        assert!(!config.bind_address.is_empty());
    }

    #[test]
    fn test_server_config_auth() {
        let auth = AuthenticationConfig::default();
        let config = ServerConfig::default().auth(auth);
        assert!(config.auth.is_some());
    }

    #[test]
    fn test_rate_limiting_config_default_values() {
        let config = RateLimitingConfig::default();
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.concurrent_executions_per_client, 10);
    }

    #[test]
    fn test_primal_capabilities_config_default() {
        let config = PrimalCapabilitiesConfig::default();
        assert!(config.heartbeat_interval_secs >= 1);
    }
}
