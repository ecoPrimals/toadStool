//! Server configuration types and defaults

use std::collections::HashMap;
use std::time::Duration;

/// ToadStool server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server bind address
    pub bind_address: String,

    /// Enable REST API endpoints
    pub enable_api: bool,

    /// Enable WebSocket for real-time events
    pub enable_websocket: bool,

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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            enable_api: true,
            enable_websocket: true,
            enable_cors: true,
            max_concurrent_executions: 100,
            default_timeout: Duration::from_secs(300),
            resource_monitoring_interval: Duration::from_secs(30),
            auth: None,
            rate_limiting: None,
            logging: LoggingConfig::default(),
            health_check: HealthCheckConfig::default(),
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
    pub fn enable_api(mut self, enabled: bool) -> Self {
        self.enable_api = enabled;
        self
    }

    /// Enable or disable WebSocket
    pub fn enable_websocket(mut self, enabled: bool) -> Self {
        self.enable_websocket = enabled;
        self
    }

    /// Set maximum concurrent executions
    pub fn max_concurrent_executions(mut self, max: u32) -> Self {
        self.max_concurrent_executions = max;
        self
    }

    /// Set default execution timeout
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Set authentication configuration
    pub fn auth(mut self, auth: AuthenticationConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set rate limiting configuration
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
            interval: Duration::from_secs(30),
            check_runtime_engines: true,
            check_resources: true,
            memory_threshold_percent: 90.0,
            cpu_threshold_percent: 95.0,
        }
    }
}
