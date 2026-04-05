// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal service configurations
//!
//! Configuration structures for all Primal services in the ecoPrimals ecosystem.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::resources::{BiomeHealthCheckConfig, PrimalResources};

/// Configuration for ecosystem services (capability-oriented; serde keys use capability names).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalsConfig {
    /// `ToadStool` (Universal Compute) configuration
    pub toadstool: Option<ToadStoolConfig>,
    /// Coordination / network orchestration configuration
    #[serde(alias = "songbird")] // legacy config key
    pub coordination: Option<CoordinationConfig>,
    /// Security / crypto configuration
    #[serde(alias = "beardog")] // legacy config key
    pub security: Option<SecurityConfig>,
    /// Storage configuration
    #[serde(alias = "nestgate")] // legacy config key
    pub storage: Option<StorageConfig>,
    /// Intelligence / routing configuration
    #[serde(alias = "squirrel")] // legacy config key
    pub intelligence: Option<IntelligenceConfig>,
    /// biomeOS (Universal OS) configuration
    pub biomeos: Option<BiomeOSConfig>,
}

/// `ToadStool` Universal Compute configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolConfig {
    /// Enable `ToadStool`
    pub enabled: bool,
    /// Act as primary orchestrator
    pub orchestrator: bool,
    /// Resource allocation
    pub resources: Option<PrimalResources>,
    /// Runtime engines to enable
    pub runtime_engines: Vec<String>,
    /// Execution environments
    pub execution_environments: Vec<String>,
    /// Substrate support
    pub substrates: Vec<String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Network coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    /// Enable coordination service
    pub enabled: bool,
    /// Enable service mesh functionality
    pub service_mesh: bool,
    /// Port range for dynamic allocation
    pub port_range: Option<String>,
    /// Load balancing strategy
    pub load_balancing: Option<String>,
    /// Health check configuration
    pub health_checks: Option<BiomeHealthCheckConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable security service
    pub enabled: bool,
    /// Security level (low, medium, high, maximum)
    pub security_level: String,
    /// Enable crypto-lock functionality
    pub crypto_lock: bool,
    /// Authentication methods
    pub auth_methods: Vec<String>,
    /// Token propagation settings
    pub token_propagation: Option<TokenPropagationConfig>,
    /// Security policies
    pub policies: Vec<String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Token propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationConfig {
    /// Enable token propagation
    pub enabled: bool,
    /// Refresh interval
    pub refresh_interval: Duration,
    /// Validation settings
    pub validation: TokenValidationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Require signature
    pub require_signature: bool,
    /// Timestamp window
    pub timestamp_window: Duration,
    /// Replay protection
    pub replay_protection: bool,
}

/// Storage service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Enable storage service
    pub enabled: bool,
    /// Storage tier (cold, warm, hot)
    pub storage_tier: String,
    /// Volume definitions
    pub volumes: Vec<ServiceVolumeConfig>,
    /// Backup configuration
    pub backup: Option<BackupConfig>,
    /// Replication settings
    pub replication: Option<ReplicationConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Service-level volume configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceVolumeConfig {
    /// Volume name
    pub name: String,
    /// Volume size
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Mount path
    pub mount_path: String,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable backups
    pub enabled: bool,
    /// Backup schedule (cron format)
    pub schedule: String,
    /// Retention period
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

/// Intelligence / AI workload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceConfig {
    /// Enable intelligence service
    pub enabled: bool,
    /// AI agents to deploy
    pub ai_agents: Vec<super::agent::AgentConfig>,
    /// Model configurations
    pub models: Vec<super::agent::ModelConfig>,
    /// MCP (Model Control Protocol) settings
    pub mcp: Option<super::agent::MCPConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

// MCPConfig and BootConfig moved to agent.rs module

/// biomeOS Universal OS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSConfig {
    /// Enable biomeOS
    pub enabled: bool,
    /// OS compatibility layers
    pub compatibility_layers: Vec<String>,
    /// System services
    pub system_services: Vec<String>,
    /// Boot configuration
    pub boot: Option<super::agent::BootConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}
