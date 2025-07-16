//! Runtime defaults and constants for ToadStool
//!
//! This file centralizes all default values, timeouts, and configuration constants
//! to eliminate hardcoded values scattered throughout the codebase.

use std::time::Duration;

/// Network configuration defaults with environment variable support
pub mod network {
    use std::env;
    
    /// Default Songbird port - configurable via TOADSTOOL_SONGBIRD_PORT
    pub fn default_songbird_port() -> u16 {
        env::var("TOADSTOOL_SONGBIRD_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(8080)
    }
    
    /// Default ToadStool API port - configurable via TOADSTOOL_API_PORT
    pub fn default_toadstool_port() -> u16 {
        env::var("TOADSTOOL_API_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(7000)
    }
    
    /// Default BearDog port - configurable via TOADSTOOL_BEARDOG_PORT
    pub fn default_beardog_port() -> u16 {
        env::var("TOADSTOOL_BEARDOG_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(9000)
    }
    
    /// Default NestGate port - configurable via TOADSTOOL_NESTGATE_PORT
    pub fn default_nestgate_port() -> u16 {
        env::var("TOADSTOOL_NESTGATE_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(3000)
    }
    
    /// Default bind address - configurable via TOADSTOOL_BIND_ADDRESS
    pub fn default_bind_address() -> String {
        env::var("TOADSTOOL_BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string())
    }
    
    /// Default federation port - configurable via TOADSTOOL_FEDERATION_PORT
    pub fn default_federation_port() -> u16 {
        env::var("TOADSTOOL_FEDERATION_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(7777)
    }
    
    /// Default health check port - configurable via TOADSTOOL_HEALTH_PORT
    pub fn default_health_port() -> u16 {
        env::var("TOADSTOOL_HEALTH_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(8080)
    }
    
    /// Default metrics port - configurable via TOADSTOOL_METRICS_PORT
    pub fn default_metrics_port() -> u16 {
        env::var("TOADSTOOL_METRICS_PORT")
            .ok()
            .and_then(|port| port.parse().ok())
            .unwrap_or(9090)
    }
    
    /// Legacy constants for backward compatibility
    pub const DEFAULT_SONGBIRD_PORT: u16 = 8080;
    pub const DEFAULT_TOADSTOOL_PORT: u16 = 7000;
    pub const DEFAULT_BEARDOG_PORT: u16 = 9000;
    pub const DEFAULT_NESTGATE_PORT: u16 = 3000;
    pub const DEFAULT_LOCALHOST: &str = "127.0.0.1";
    pub const DEFAULT_FEDERATION_PORT: u16 = 7777;
}

/// Runtime configuration defaults with environment variable support
pub mod runtime {
    use std::env;
    use std::time::Duration;
    
    /// Default execution timeout - configurable via TOADSTOOL_EXECUTION_TIMEOUT_MS
    pub fn default_execution_timeout() -> Duration {
        let ms = env::var("TOADSTOOL_EXECUTION_TIMEOUT_MS")
            .ok()
            .and_then(|timeout| timeout.parse().ok())
            .unwrap_or(30000);
        Duration::from_millis(ms)
    }
    
    /// Default health check interval - configurable via TOADSTOOL_HEALTH_CHECK_INTERVAL_MS
    pub fn default_health_check_interval() -> Duration {
        let ms = env::var("TOADSTOOL_HEALTH_CHECK_INTERVAL_MS")
            .ok()
            .and_then(|interval| interval.parse().ok())
            .unwrap_or(5000);
        Duration::from_millis(ms)
    }
    
    /// Default retry count - configurable via TOADSTOOL_RETRY_COUNT
    pub fn default_retry_count() -> u32 {
        env::var("TOADSTOOL_RETRY_COUNT")
            .ok()
            .and_then(|count| count.parse().ok())
            .unwrap_or(3)
    }
    
    /// Default connection timeout - configurable via TOADSTOOL_CONNECTION_TIMEOUT_MS
    pub fn default_connection_timeout() -> Duration {
        let ms = env::var("TOADSTOOL_CONNECTION_TIMEOUT_MS")
            .ok()
            .and_then(|timeout| timeout.parse().ok())
            .unwrap_or(10000);
        Duration::from_millis(ms)
    }
    
    /// Default max concurrent executions - configurable via TOADSTOOL_MAX_CONCURRENT_EXECUTIONS
    pub fn default_max_concurrent_executions() -> usize {
        env::var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS")
            .ok()
            .and_then(|count| count.parse().ok())
            .unwrap_or(100)
    }
    
    /// Default memory limit per execution - configurable via TOADSTOOL_MEMORY_LIMIT_MB
    pub fn default_memory_limit_mb() -> u64 {
        env::var("TOADSTOOL_MEMORY_LIMIT_MB")
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or(512)
    }
    
    /// Default CPU limit per execution - configurable via TOADSTOOL_CPU_LIMIT_CORES
    pub fn default_cpu_limit_cores() -> f64 {
        env::var("TOADSTOOL_CPU_LIMIT_CORES")
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or(2.0)
    }
    
    /// Default storage limit per execution - configurable via TOADSTOOL_STORAGE_LIMIT_GB
    pub fn default_storage_limit_gb() -> u64 {
        env::var("TOADSTOOL_STORAGE_LIMIT_GB")
            .ok()
            .and_then(|limit| limit.parse().ok())
            .unwrap_or(10)
    }
    
    /// Default log level - configurable via TOADSTOOL_LOG_LEVEL
    pub fn default_log_level() -> String {
        env::var("TOADSTOOL_LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    }
    
    /// Default work directory - configurable via TOADSTOOL_WORK_DIR
    pub fn default_work_dir() -> String {
        env::var("TOADSTOOL_WORK_DIR").unwrap_or_else(|_| "/tmp/toadstool".to_string())
    }
    
    /// Default data directory - configurable via TOADSTOOL_DATA_DIR
    pub fn default_data_dir() -> String {
        env::var("TOADSTOOL_DATA_DIR").unwrap_or_else(|_| "/var/lib/toadstool".to_string())
    }
    
    /// Default config directory - configurable via TOADSTOOL_CONFIG_DIR
    pub fn default_config_dir() -> String {
        env::var("TOADSTOOL_CONFIG_DIR").unwrap_or_else(|_| "/etc/toadstool".to_string())
    }
    
    /// Default cache directory - configurable via TOADSTOOL_CACHE_DIR
    pub fn default_cache_dir() -> String {
        env::var("TOADSTOOL_CACHE_DIR").unwrap_or_else(|_| "/tmp/toadstool/cache".to_string())
    }
    
    /// Default enable debug mode - configurable via TOADSTOOL_DEBUG
    pub fn default_debug_mode() -> bool {
        env::var("TOADSTOOL_DEBUG")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default enable telemetry - configurable via TOADSTOOL_TELEMETRY
    pub fn default_telemetry_enabled() -> bool {
        env::var("TOADSTOOL_TELEMETRY")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable auto-scaling - configurable via TOADSTOOL_AUTO_SCALE
    pub fn default_auto_scale_enabled() -> bool {
        env::var("TOADSTOOL_AUTO_SCALE")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable federation - configurable via TOADSTOOL_FEDERATION
    pub fn default_federation_enabled() -> bool {
        env::var("TOADSTOOL_FEDERATION")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default enable GPU acceleration - configurable via TOADSTOOL_GPU_ENABLED
    pub fn default_gpu_enabled() -> bool {
        env::var("TOADSTOOL_GPU_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default enable container runtime - configurable via TOADSTOOL_CONTAINER_RUNTIME
    pub fn default_container_runtime_enabled() -> bool {
        env::var("TOADSTOOL_CONTAINER_RUNTIME")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable WASM runtime - configurable via TOADSTOOL_WASM_RUNTIME
    pub fn default_wasm_runtime_enabled() -> bool {
        env::var("TOADSTOOL_WASM_RUNTIME")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Legacy constants for backward compatibility
    pub const DEFAULT_EXECUTION_TIMEOUT_MS: u64 = 30000;
    pub const DEFAULT_HEALTH_CHECK_INTERVAL_MS: u64 = 5000;
    pub const DEFAULT_RETRY_COUNT: u32 = 3;
    pub const DEFAULT_CONNECTION_TIMEOUT_MS: u64 = 10000;
    pub const DEFAULT_MAX_CONCURRENT_EXECUTIONS: usize = 100;
    pub const DEFAULT_MEMORY_LIMIT_MB: u64 = 512;
    pub const DEFAULT_CPU_LIMIT_CORES: f64 = 2.0;
    pub const DEFAULT_STORAGE_LIMIT_GB: u64 = 10;
}

/// Security configuration defaults with environment variable support
pub mod security {
    use std::env;
    
    /// Default enable sandboxing - configurable via TOADSTOOL_SANDBOX_ENABLED
    pub fn default_sandbox_enabled() -> bool {
        env::var("TOADSTOOL_SANDBOX_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable encryption - configurable via TOADSTOOL_ENCRYPTION_ENABLED
    pub fn default_encryption_enabled() -> bool {
        env::var("TOADSTOOL_ENCRYPTION_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable authentication - configurable via TOADSTOOL_AUTH_ENABLED
    pub fn default_auth_enabled() -> bool {
        env::var("TOADSTOOL_AUTH_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable authorization - configurable via TOADSTOOL_AUTHZ_ENABLED
    pub fn default_authz_enabled() -> bool {
        env::var("TOADSTOOL_AUTHZ_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable audit logging - configurable via TOADSTOOL_AUDIT_ENABLED
    pub fn default_audit_enabled() -> bool {
        env::var("TOADSTOOL_AUDIT_ENABLED")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable network security - configurable via TOADSTOOL_NETWORK_SECURITY
    pub fn default_network_security_enabled() -> bool {
        env::var("TOADSTOOL_NETWORK_SECURITY")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
    
    /// Default enable strict mode - configurable via TOADSTOOL_STRICT_MODE
    pub fn default_strict_mode_enabled() -> bool {
        env::var("TOADSTOOL_STRICT_MODE")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default JWT secret - configurable via TOADSTOOL_JWT_SECRET
    pub fn default_jwt_secret() -> String {
        env::var("TOADSTOOL_JWT_SECRET").unwrap_or_else(|_| "toadstool-default-secret-change-me".to_string())
    }
    
    /// Default encryption key - configurable via TOADSTOOL_ENCRYPTION_KEY
    pub fn default_encryption_key() -> String {
        env::var("TOADSTOOL_ENCRYPTION_KEY").unwrap_or_else(|_| "toadstool-default-key-change-me".to_string())
    }
    
    /// Default API key - configurable via TOADSTOOL_API_KEY
    pub fn default_api_key() -> String {
        env::var("TOADSTOOL_API_KEY").unwrap_or_else(|_| "toadstool-default-api-key".to_string())
    }
    
    /// Default admin token - configurable via TOADSTOOL_ADMIN_TOKEN
    pub fn default_admin_token() -> String {
        env::var("TOADSTOOL_ADMIN_TOKEN").unwrap_or_else(|_| "toadstool-admin-token".to_string())
    }
}

/// Cloud configuration defaults with environment variable support
pub mod cloud {
    use std::env;
    
    /// Default cloud provider - configurable via TOADSTOOL_CLOUD_PROVIDER
    pub fn default_cloud_provider() -> String {
        env::var("TOADSTOOL_CLOUD_PROVIDER").unwrap_or_else(|_| "localhost".to_string())
    }
    
    /// Default cloud region - configurable via TOADSTOOL_CLOUD_REGION
    pub fn default_cloud_region() -> String {
        env::var("TOADSTOOL_CLOUD_REGION").unwrap_or_else(|_| "us-east-1".to_string())
    }
    
    /// Default cloud endpoint - configurable via TOADSTOOL_CLOUD_ENDPOINT
    pub fn default_cloud_endpoint() -> String {
        env::var("TOADSTOOL_CLOUD_ENDPOINT").unwrap_or_else(|_| "http://localhost:8080".to_string())
    }
    
    /// Default enable multi-cloud - configurable via TOADSTOOL_MULTI_CLOUD
    pub fn default_multi_cloud_enabled() -> bool {
        env::var("TOADSTOOL_MULTI_CLOUD")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default enable hybrid cloud - configurable via TOADSTOOL_HYBRID_CLOUD
    pub fn default_hybrid_cloud_enabled() -> bool {
        env::var("TOADSTOOL_HYBRID_CLOUD")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(false)
    }
    
    /// Default enable auto-provisioning - configurable via TOADSTOOL_AUTO_PROVISION
    pub fn default_auto_provision_enabled() -> bool {
        env::var("TOADSTOOL_AUTO_PROVISION")
            .map(|val| val.to_lowercase() == "true" || val == "1")
            .unwrap_or(true)
    }
}

/// Helper functions for generating default endpoints
pub mod endpoints {
    use super::network;
    
    /// Generate default Songbird endpoint
    pub fn default_songbird_endpoint() -> String {
        format!("http://{}:{}", network::default_bind_address(), network::default_songbird_port())
    }
    
    /// Generate default BearDog endpoint
    pub fn default_beardog_endpoint() -> String {
        format!("http://{}:{}", network::default_bind_address(), network::default_beardog_port())
    }
    
    /// Generate default NestGate endpoint
    pub fn default_nestgate_endpoint() -> String {
        format!("http://{}:{}", network::default_bind_address(), network::default_nestgate_port())
    }
    
    /// Generate default federation endpoint
    pub fn default_federation_endpoint() -> String {
        format!("http://{}:{}", network::default_bind_address(), network::default_federation_port())
    }
    
    /// Generate default health check endpoint
    pub fn default_health_endpoint() -> String {
        format!("http://{}:{}/health", network::default_bind_address(), network::default_health_port())
    }
    
    /// Generate default metrics endpoint
    pub fn default_metrics_endpoint() -> String {
        format!("http://{}:{}/metrics", network::default_bind_address(), network::default_metrics_port())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_default_ports() {
        // Test that ports are within valid range
        assert!(network::default_songbird_port() > 0);
        assert!(network::default_songbird_port() < 65536);
        
        assert!(network::default_toadstool_port() > 0);
        assert!(network::default_toadstool_port() < 65536);
        
        assert!(network::default_beardog_port() > 0);
        assert!(network::default_beardog_port() < 65536);
        
        assert!(network::default_nestgate_port() > 0);
        assert!(network::default_nestgate_port() < 65536);
    }
    
    #[test]
    fn test_environment_variable_override() {
        // Test that environment variables are respected
        env::set_var("TOADSTOOL_SONGBIRD_PORT", "9080");
        assert_eq!(network::default_songbird_port(), 9080);
        
        env::set_var("TOADSTOOL_BIND_ADDRESS", "0.0.0.0");
        assert_eq!(network::default_bind_address(), "0.0.0.0");
        
        env::set_var("TOADSTOOL_DEBUG", "true");
        assert!(runtime::default_debug_mode());
        
        env::set_var("TOADSTOOL_DEBUG", "false");
        assert!(!runtime::default_debug_mode());
        
        // Clean up
        env::remove_var("TOADSTOOL_SONGBIRD_PORT");
        env::remove_var("TOADSTOOL_BIND_ADDRESS");
        env::remove_var("TOADSTOOL_DEBUG");
    }
    
    #[test]
    fn test_default_bind_address() {
        // Test that bind address is valid
        let addr = network::default_bind_address();
        assert!(!addr.is_empty());
        assert!(addr.contains('.') || addr == "localhost");
    }
    
    #[test]
    fn test_default_endpoints() {
        // Test that endpoints are generated correctly
        let songbird_endpoint = endpoints::default_songbird_endpoint();
        assert!(songbird_endpoint.starts_with("http://"));
        assert!(songbird_endpoint.contains(&network::default_songbird_port().to_string()));
        
        let beardog_endpoint = endpoints::default_beardog_endpoint();
        assert!(beardog_endpoint.starts_with("http://"));
        assert!(beardog_endpoint.contains(&network::default_beardog_port().to_string()));
        
        let nestgate_endpoint = endpoints::default_nestgate_endpoint();
        assert!(nestgate_endpoint.starts_with("http://"));
        assert!(nestgate_endpoint.contains(&network::default_nestgate_port().to_string()));
    }
    
    #[test]
    fn test_runtime_defaults() {
        // Test that runtime defaults are reasonable
        assert!(runtime::default_execution_timeout().as_millis() > 0);
        assert!(runtime::default_health_check_interval().as_millis() > 0);
        assert!(runtime::default_retry_count() > 0);
        assert!(runtime::default_connection_timeout().as_millis() > 0);
        assert!(runtime::default_max_concurrent_executions() > 0);
        assert!(runtime::default_memory_limit_mb() > 0);
        assert!(runtime::default_cpu_limit_cores() > 0.0);
        assert!(runtime::default_storage_limit_gb() > 0);
        
        // Test that paths are not empty
        assert!(!runtime::default_work_dir().is_empty());
        assert!(!runtime::default_data_dir().is_empty());
        assert!(!runtime::default_config_dir().is_empty());
        assert!(!runtime::default_cache_dir().is_empty());
        
        // Test that log level is valid
        let log_level = runtime::default_log_level();
        assert!(["trace", "debug", "info", "warn", "error"].contains(&log_level.as_str()));
    }
    
    #[test]
    fn test_security_defaults() {
        // Test that security is enabled by default
        assert!(security::default_sandbox_enabled());
        assert!(security::default_encryption_enabled());
        assert!(security::default_auth_enabled());
        assert!(security::default_authz_enabled());
        assert!(security::default_audit_enabled());
        assert!(security::default_network_security_enabled());
        
        // Test that strict mode is disabled by default
        assert!(!security::default_strict_mode_enabled());
        
        // Test that secrets are provided (even if default)
        assert!(!security::default_jwt_secret().is_empty());
        assert!(!security::default_encryption_key().is_empty());
        assert!(!security::default_api_key().is_empty());
        assert!(!security::default_admin_token().is_empty());
    }
    
    #[test]
    fn test_cloud_defaults() {
        // Test that cloud defaults are reasonable
        assert!(!cloud::default_cloud_provider().is_empty());
        assert!(!cloud::default_cloud_region().is_empty());
        assert!(!cloud::default_cloud_endpoint().is_empty());
        
        // Test that multi-cloud is disabled by default
        assert!(!cloud::default_multi_cloud_enabled());
        assert!(!cloud::default_hybrid_cloud_enabled());
        
        // Test that auto-provisioning is enabled by default
        assert!(cloud::default_auto_provision_enabled());
    }
} 