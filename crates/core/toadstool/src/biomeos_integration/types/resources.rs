use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeHealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Number of retries before marking as unhealthy
    pub retries: u32,
    /// Initial delay before first check (startup grace period)
    pub initial_delay: Duration,
}

impl Default for BiomeHealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            retries: 3,
            initial_delay: Duration::from_secs(30),
        }
    }
}

/// Token propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationConfig {
    /// Enable cross-Primal token propagation
    pub enabled: bool,
    /// Token refresh interval
    pub refresh_interval: Duration,
    /// Token validation settings
    pub validation: TokenValidationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Require signature validation
    pub require_signature: bool,
    /// Timestamp validation window
    pub timestamp_window: Duration,
    /// Replay attack protection
    pub replay_protection: bool,
}

/// Volume configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Volume size (e.g., "100Gi", "1TB")
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Mount path
    pub mount_path: Option<String>,
    /// Backup policy
    pub backup_policy: Option<String>,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automated backups
    pub enabled: bool,
    /// Backup schedule (cron format)
    pub schedule: String,
    /// Retention policy
    pub retention: String,
    /// Backup destination
    pub destination: String,
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Enable replication
    pub enabled: bool,
    /// Replication factor
    pub factor: u32,
    /// Replication strategy
    pub strategy: String,
}

// Agent, Model, MCP, and Boot configurations moved to agent.rs module

/// Storage configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorageConfig {
    /// Enable storage
    pub enabled: bool,
    /// Storage backend type
    pub backend: String,
    /// Storage capacity
    pub capacity: Option<String>,
}

/// Primal resource allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResources {
    /// CPU cores allocation
    pub cpu_cores: Option<f64>,
    /// Memory allocation in GB
    pub memory_gb: Option<f64>,
    /// Storage allocation in GB
    pub storage_gb: Option<f64>,
    /// GPU allocation
    pub gpu: Option<GpuAllocation>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Number of GPUs
    pub count: u32,
    /// GPU type/model
    pub gpu_type: Option<String>,
    /// GPU memory requirement
    pub memory: Option<String>,
}

/// Resource configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU limits
    pub cpu_limit: Option<f64>,
    /// Memory limits
    pub memory_limit: Option<String>,
    /// Storage limits
    pub storage_limit: Option<String>,
    /// GPU limits
    pub gpu_limit: Option<u32>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}
