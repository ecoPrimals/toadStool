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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub auth_type: AuthType,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdIntegrationConfig {
    pub connection_config: SongbirdConnectionConfig,
    pub distribution_config: DistributionConfig,
    pub discovery_config: SongbirdDiscoveryConfig,
    pub load_balancer_config: LoadBalancerConfig,
    pub broadcast_config: BroadcastConfig,
    pub capacity_config: CapacityConfig,
    pub receiver_config: ReceiverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConnectionConfig {
    pub endpoints: Vec<String>,
    pub protocol_config: ProtocolConfig,
    pub auth_config: AuthConfig,
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    pub max_subtasks: usize,
    pub splitting_strategies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdDiscoveryConfig {
    #[serde(with = "humantime_serde")]
    pub discovery_interval: Duration,
    #[serde(with = "humantime_serde")]
    pub node_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: String,
    #[serde(with = "humantime_serde")]
    pub feedback_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    pub channels: Vec<String>,
    #[serde(with = "humantime_serde")]
    pub message_retention: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityConfig {
    #[serde(with = "humantime_serde")]
    pub monitoring_interval: Duration,
    pub resource_buffer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    pub max_concurrent_jobs: usize,
    #[serde(with = "humantime_serde")]
    pub job_timeout: Duration,
}
