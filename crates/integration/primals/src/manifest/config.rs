// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use toadstool_common::config_bases::HealthCheckConfig;
use toadstool_common::config_bases::HttpHealthCheckConfig;

/// Configuration for a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    pub primal_type: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub settings: HashMap<String, serde_json::Value>,

    /// Health check configuration (using base pattern)
    #[serde(flatten)]
    pub health_check: Option<HttpHealthCheckConfig>,
}

/// Biome storage configuration
///
/// Defines storage settings for biome persistent data including capacity,
/// persistence options, and backup policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    /// Storage backend type (e.g., "local", "s3", "distributed")
    pub storage_type: String,
    /// Storage capacity in gigabytes
    pub capacity_gb: u64,
    /// Enable data persistence across restarts
    pub persistence: bool,
    /// Enable automatic backups
    pub backup_enabled: bool,
}

impl Default for BiomeStorage {
    fn default() -> Self {
        Self {
            storage_type: "local".to_string(),
            capacity_gb: 10,
            persistence: true,
            backup_enabled: true,
        }
    }
}

/// Agent configuration
///
/// Defines configuration for autonomous agents within a biome including
/// capabilities and agent-specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent identifier
    pub agent_id: String,
    /// Agent type or class (e.g., "monitor", "optimizer", "coordinator")
    pub agent_type: String,
    /// List of agent capabilities
    pub capabilities: Vec<String>,
    /// Agent-specific configuration parameters
    pub config: HashMap<String, serde_json::Value>,
}

/// Biome security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    pub encryption_enabled: bool,
    pub authentication_required: bool,
    pub access_control: HashMap<String, Vec<String>>,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub image: String,
    pub ports: Vec<u16>,
    pub environment: HashMap<String, String>,

    /// Resource configuration (using base pattern)
    #[serde(flatten)]
    pub resources: ServiceResourcesConfig,
}

/// Service resources configuration
///
/// Uses base resource pattern with service-specific extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResourcesConfig {
    /// CPU cores allocation
    pub cpu_cores: f64,
    /// Memory in megabytes
    pub memory_mb: u64,
    /// Storage in gigabytes
    pub storage_gb: u64,
}

/// Biome networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    pub network_type: String,
    pub subnets: Vec<String>,
    pub external_access: bool,
}

/// Biome resources configuration
///
/// Specifies resource allocation for biome instances with GPU support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU cores allocation
    pub cpu_cores: f64,
    /// Memory in gigabytes
    pub memory_gb: u64,
    /// Storage in gigabytes
    pub storage_gb: u64,
    /// Number of GPUs allocated
    pub gpu_count: u32,
}

impl Default for BiomeResources {
    fn default() -> Self {
        Self {
            cpu_cores: 2.0,
            memory_gb: 4,
            storage_gb: 20,
            gpu_count: 0,
        }
    }
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    pub federation_id: String,
    pub peers: Vec<String>,
    pub sync_interval_seconds: u64,
}
