// SPDX-License-Identifier: AGPL-3.0-only
//! Configuration types for Songbird integration

use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::auth::{AuthType, ServiceAuthConfig};
use toadstool_common::config_bases::ConnectionPoolConfig;

use super::protocols::ProtocolConfig;

pub type AuthConfig = ServiceAuthConfig;

// ============================================================================
// Authentication and Config
// ============================================================================

/// Authentication configuration for Songbird.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Auth type (api_key, token, basic, etc.).
    pub auth_type: AuthType,
    /// API key (optional).
    pub api_key: Option<String>,
    /// Bearer token (optional).
    pub token: Option<String>,
    /// Username for basic auth (optional).
    pub username: Option<String>,
    /// Password for basic auth (optional).
    pub password: Option<String>,
}

/// Songbird integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdIntegrationConfig {
    /// Connection config.
    pub connection_config: SongbirdConnectionConfig,
    /// Distribution config.
    pub distribution_config: DistributionConfig,
    /// Discovery config.
    pub discovery_config: SongbirdDiscoveryConfig,
    /// Load balancer config.
    pub load_balancer_config: LoadBalancerConfig,
    /// Broadcast config.
    pub broadcast_config: BroadcastConfig,
    /// Capacity config.
    pub capacity_config: CapacityConfig,
    /// Receiver config.
    pub receiver_config: ReceiverConfig,
}

/// Songbird connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConnectionConfig {
    /// Endpoint URLs.
    pub endpoints: Vec<String>,
    /// Protocol config.
    pub protocol_config: ProtocolConfig,
    /// Auth config.
    pub auth_config: AuthConfig,
    /// Connection pool config.
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
}

/// Distribution configuration for job splitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Max subtasks per job.
    pub max_subtasks: usize,
    /// Splitting strategy name to config.
    pub splitting_strategies: std::collections::HashMap<String, String>,
}

/// Songbird discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdDiscoveryConfig {
    /// Discovery poll interval.
    #[serde(with = "humantime_serde")]
    pub discovery_interval: Duration,
    /// Node timeout before considered dead.
    #[serde(with = "humantime_serde")]
    pub node_timeout: Duration,
}

/// Load balancer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Strategy name (round_robin, least_loaded, etc.).
    pub strategy: String,
    /// Feedback interval for load updates.
    #[serde(with = "humantime_serde")]
    pub feedback_interval: Duration,
}

/// Broadcast configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    /// Channel names.
    pub channels: Vec<String>,
    /// Message retention duration.
    #[serde(with = "humantime_serde")]
    pub message_retention: Duration,
}

/// Capacity monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityConfig {
    /// Monitoring interval.
    #[serde(with = "humantime_serde")]
    pub monitoring_interval: Duration,
    /// Resource buffer fraction (0.0–1.0).
    pub resource_buffer: f64,
}

/// Receiver configuration for job consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    /// Max concurrent jobs per receiver.
    pub max_concurrent_jobs: usize,
    /// Job timeout.
    #[serde(with = "humantime_serde")]
    pub job_timeout: Duration,
}
