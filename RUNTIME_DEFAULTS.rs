//! Runtime defaults and constants for ToadStool
//!
//! This file centralizes all default values, timeouts, and configuration constants
//! to eliminate hardcoded values scattered throughout the codebase.

use std::time::Duration;

/// Default timeout values for various operations
pub mod timeouts {
    use super::Duration;
    
    /// Default execution timeout (5 minutes)
    pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(300);
    
    /// Default health check interval (10 seconds)
    pub const DEFAULT_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);
    
    /// Default heartbeat interval for Songbird integration (15 seconds)
    pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
    
    /// Default registration interval for Songbird (30 seconds)
    pub const DEFAULT_REGISTRATION_INTERVAL: Duration = Duration::from_secs(30);
    
    /// Default capability update interval (60 seconds)
    pub const DEFAULT_CAPABILITY_UPDATE_INTERVAL: Duration = Duration::from_secs(60);
    
    /// Default monitoring interval (5 seconds)
    pub const DEFAULT_MONITORING_INTERVAL: Duration = Duration::from_secs(5);
    
    /// Default retry timeout (30 seconds)
    pub const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(30);
    
    /// Default network timeout (10 seconds)
    pub const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(10);
}

/// Default network configuration
pub mod network {
    /// Default ToadStool port
    pub const DEFAULT_TOADSTOOL_PORT: u16 = 8081;
    
    /// Default Songbird port
    pub const DEFAULT_SONGBIRD_PORT: u16 = 8080;
    
    /// Default BearDog port
    pub const DEFAULT_BEARDOG_PORT: u16 = 8082;
    
    /// Default NestGate port
    pub const DEFAULT_NESTGATE_PORT: u16 = 8083;
    
    /// Default bootstrap URL for federation
    pub const DEFAULT_BOOTSTRAP_URL: &str = "http://bootstrap.toadstool.org";
    
    /// Default localhost address
    pub const DEFAULT_LOCALHOST: &str = "127.0.0.1";
    
    /// Default binding address
    pub const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0";
    
    /// Maximum connections per service
    pub const DEFAULT_MAX_CONNECTIONS: usize = 1000;
}

/// Default resource limits
pub mod resources {
    /// Default maximum CPU percentage usage (90%)
    pub const DEFAULT_MAX_CPU_PERCENT: f64 = 90.0;
    
    /// Default maximum memory bytes (8GB)
    pub const DEFAULT_MAX_MEMORY_BYTES: u64 = 8 * 1024 * 1024 * 1024;
    
    /// Default maximum storage bytes (100GB)
    pub const DEFAULT_MAX_STORAGE_BYTES: u64 = 100 * 1024 * 1024 * 1024;
    
    /// Default maximum network bandwidth (1Gbps)
    pub const DEFAULT_MAX_NETWORK_MBPS: f64 = 1000.0;
    
    /// Default maximum GPU percentage usage (95%)
    pub const DEFAULT_MAX_GPU_PERCENT: f64 = 95.0;
    
    /// Default maximum concurrent executions
    pub const DEFAULT_MAX_CONCURRENT_EXECUTIONS: usize = 100;
    
    /// Default resource monitoring interval (5 seconds)
    pub const DEFAULT_RESOURCE_MONITORING_INTERVAL: Duration = Duration::from_secs(5);
}

/// Default security configuration
pub mod security {
    /// Default permission expiry time (24 hours)
    pub const DEFAULT_PERMISSION_EXPIRY_HOURS: u64 = 24;
    
    /// Default isolation level
    pub const DEFAULT_ISOLATION_LEVEL: &str = "container";
    
    /// Default security profile
    pub const DEFAULT_SECURITY_PROFILE: &str = "medium";
    
    /// Maximum permission delegation depth
    pub const DEFAULT_MAX_DELEGATION_DEPTH: u32 = 3;
}

/// Default retry configuration
pub mod retry {
    /// Default maximum retry attempts
    pub const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 3;
    
    /// Default retry base delay (1 second)
    pub const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
    
    /// Default retry backoff multiplier
    pub const DEFAULT_RETRY_BACKOFF_MULTIPLIER: f64 = 2.0;
    
    /// Default retry jitter percentage (10%)
    pub const DEFAULT_RETRY_JITTER_PERCENT: f64 = 0.1;
}

/// Default job distribution configuration
pub mod job_distribution {
    /// Maximum subtasks for ultra-massive jobs
    pub const DEFAULT_MAX_SUBTASKS: usize = 10000;
    
    /// Default job complexity threshold for massive distribution
    pub const DEFAULT_MASSIVE_JOB_THRESHOLD: u64 = 1000;
    
    /// Default node selection algorithm
    pub const DEFAULT_NODE_SELECTION_ALGORITHM: &str = "resource_aware";
    
    /// Default load balancing algorithm
    pub const DEFAULT_LOAD_BALANCING_ALGORITHM: &str = "round_robin";
}

/// Default platform detection configuration
pub mod platform {
    /// Default platform detection timeout (5 seconds)
    pub const DEFAULT_PLATFORM_DETECTION_TIMEOUT: Duration = Duration::from_secs(5);
    
    /// Default platform capability refresh interval (300 seconds)
    pub const DEFAULT_CAPABILITY_REFRESH_INTERVAL: Duration = Duration::from_secs(300);
    
    /// Default substrate detection methods
    pub const DEFAULT_SUBSTRATE_DETECTION_METHODS: &[&str] = &[
        "hardware", "software", "container", "cloud", "edge"
    ];
}

/// Default monitoring configuration
pub mod monitoring {
    /// Default metrics collection interval (10 seconds)
    pub const DEFAULT_METRICS_COLLECTION_INTERVAL: Duration = Duration::from_secs(10);
    
    /// Default metrics retention period (7 days)
    pub const DEFAULT_METRICS_RETENTION_DAYS: u64 = 7;
    
    /// Default alert threshold for CPU usage (85%)
    pub const DEFAULT_CPU_ALERT_THRESHOLD: f64 = 85.0;
    
    /// Default alert threshold for memory usage (90%)
    pub const DEFAULT_MEMORY_ALERT_THRESHOLD: f64 = 90.0;
    
    /// Default alert threshold for storage usage (95%)
    pub const DEFAULT_STORAGE_ALERT_THRESHOLD: f64 = 95.0;
}

/// Helper functions for runtime defaults
pub mod helpers {
    use super::*;
    
    /// Get default Songbird endpoint
    pub fn default_songbird_endpoint() -> String {
        format!("http://{}:{}", network::DEFAULT_LOCALHOST, network::DEFAULT_SONGBIRD_PORT)
    }
    
    /// Get default BearDog endpoint
    pub fn default_beardog_endpoint() -> String {
        format!("http://{}:{}", network::DEFAULT_LOCALHOST, network::DEFAULT_BEARDOG_PORT)
    }
    
    /// Get default NestGate endpoint
    pub fn default_nestgate_endpoint() -> String {
        format!("http://{}:{}", network::DEFAULT_LOCALHOST, network::DEFAULT_NESTGATE_PORT)
    }
    
    /// Get default ToadStool bind address
    pub fn default_toadstool_bind_address() -> String {
        format!("{}:{}", network::DEFAULT_BIND_ADDRESS, network::DEFAULT_TOADSTOOL_PORT)
    }
    
    /// Calculate retry delay with exponential backoff
    pub fn calculate_retry_delay(attempt: u32) -> Duration {
        let base_delay_ms = retry::DEFAULT_RETRY_BASE_DELAY.as_millis() as f64;
        let backoff_delay = base_delay_ms * retry::DEFAULT_RETRY_BACKOFF_MULTIPLIER.powi(attempt as i32);
        let jitter = backoff_delay * retry::DEFAULT_RETRY_JITTER_PERCENT * rand::random::<f64>();
        Duration::from_millis((backoff_delay + jitter) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timeout_constants() {
        assert!(timeouts::DEFAULT_EXECUTION_TIMEOUT.as_secs() > 0);
        assert!(timeouts::DEFAULT_HEALTH_CHECK_INTERVAL.as_secs() > 0);
        assert!(timeouts::DEFAULT_HEARTBEAT_INTERVAL.as_secs() > 0);
    }
    
    #[test]
    fn test_network_constants() {
        assert!(network::DEFAULT_TOADSTOOL_PORT > 0);
        assert!(network::DEFAULT_SONGBIRD_PORT > 0);
        assert!(!network::DEFAULT_LOCALHOST.is_empty());
        assert!(!network::DEFAULT_BIND_ADDRESS.is_empty());
    }
    
    #[test]
    fn test_resource_constants() {
        assert!(resources::DEFAULT_MAX_CPU_PERCENT > 0.0);
        assert!(resources::DEFAULT_MAX_MEMORY_BYTES > 0);
        assert!(resources::DEFAULT_MAX_CONCURRENT_EXECUTIONS > 0);
    }
    
    #[test]
    fn test_helper_functions() {
        let songbird_endpoint = helpers::default_songbird_endpoint();
        assert!(songbird_endpoint.starts_with("http://"));
        assert!(songbird_endpoint.contains("8080"));
        
        let bind_address = helpers::default_toadstool_bind_address();
        assert!(bind_address.contains(":"));
        assert!(bind_address.contains("8081"));
    }
    
    #[test]
    fn test_retry_delay_calculation() {
        let delay_0 = helpers::calculate_retry_delay(0);
        let delay_1 = helpers::calculate_retry_delay(1);
        let delay_2 = helpers::calculate_retry_delay(2);
        
        assert!(delay_1 > delay_0);
        assert!(delay_2 > delay_1);
    }
} 