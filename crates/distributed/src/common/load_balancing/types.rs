//! Common Load Balancing Types
//!
//! Generic load balancing abstractions used across Songbird, Cloud, and other distributed systems.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic load balancing strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LoadBalancingStrategy {
    /// Use only the primary target
    PrimaryOnly,
    /// Round-robin across all targets
    #[default]
    RoundRobin,
    /// Choose target with least active connections
    LeastConnections,
    /// Choose target with least resource usage
    LeastLoaded,
    /// Optimize based on latency measurements
    LatencyBased,
    /// Optimize for cost (use cheapest resources)
    CostOptimized,
    /// Prefer targets in specific regions
    RegionalAffinity { preferred_regions: Vec<String> },
    /// Weighted distribution (some targets get more work)
    Weighted { weights: HashMap<String, u32> },
    /// Random selection
    Random,
    /// Consistent hashing (for session affinity)
    ConsistentHashing,
    /// IP hash (for session affinity)
    IpHash,
    /// Custom strategy (identified by name)
    Custom(String),
}

/// Load balancing algorithm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoadBalancingAlgorithm {
    /// Simple round-robin
    RoundRobin,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Least connections
    LeastConnections,
    /// Least response time
    LeastResponseTime,
    /// Resource-based (CPU, memory, etc.)
    ResourceBased,
    /// Resource-aware (considers resource requirements and availability)
    ResourceAware,
    /// Cost-aware (optimizes for cost)
    CostAware,
    /// Random selection
    Random,
    /// Power of two choices
    PowerOfTwoChoices,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Strategy to use
    pub strategy: LoadBalancingStrategy,
    /// Health check configuration
    pub health_check: HealthCheckConfig,
    /// Session affinity (sticky sessions)
    pub session_affinity: bool,
    /// Failover configuration
    pub failover: FailoverConfig,
    /// Feedback interval for performance metrics (seconds)
    pub feedback_interval_secs: u64,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig::default(),
            session_affinity: false,
            failover: FailoverConfig::default(),
            feedback_interval_secs: 10,
        }
    }
}

/// Health check configuration (re-exported from common config bases)
///
/// **Modernization Note**: This now uses the base `HealthCheckConfig` from
/// `toadstool_common::config_bases`, which provides:
/// - `enabled`: bool
/// - `interval`: Duration (modernized from `interval_secs: u64`)
/// - `timeout`: Duration (modernized from `timeout_secs: u64`)
/// - `healthy_threshold`: u32
/// - `unhealthy_threshold`: u32
/// - `retry_count`: u32 (bonus field!)
pub use toadstool_common::config_bases::HealthCheckConfig;

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// Enable automatic failover
    pub enabled: bool,
    /// Maximum failover attempts
    pub max_attempts: u32,
    /// Retry delay between failover attempts (seconds)
    pub retry_delay_secs: u64,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            retry_delay_secs: 5,
        }
    }
}

/// Load balancing advice/recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingAdvice {
    /// Recommended target ID
    pub target_id: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Reason for recommendation
    pub reason: String,
    /// Alternative targets (in order of preference)
    pub alternatives: Vec<String>,
}

/// Target health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    /// Target is healthy and accepting work
    Healthy,
    /// Target is degraded but still usable
    Degraded,
    /// Target is unhealthy and should not receive work
    Unhealthy,
    /// Target health is unknown
    Unknown,
}

/// Load metrics for a target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetrics {
    /// Target identifier
    pub target_id: String,
    /// Active connections/tasks
    pub active_count: u64,
    /// CPU usage (0.0-1.0)
    pub cpu_usage: f64,
    /// Memory usage (0.0-1.0)
    pub memory_usage: f64,
    /// Average response time (milliseconds)
    pub avg_response_time_ms: f64,
    /// Request rate (requests/sec)
    pub request_rate: f64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Health status
    pub health: HealthStatus,
    /// Timestamp of measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
