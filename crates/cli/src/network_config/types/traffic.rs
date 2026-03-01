// SPDX-License-Identifier: AGPL-3.0-or-later

//! Traffic management configuration types.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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
