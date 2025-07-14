use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    pub primal_type: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub settings: HashMap<String, serde_json::Value>,
    pub health_check: Option<HealthCheckConfig>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub endpoint: String,
}

/// Biome storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    pub storage_type: String,
    pub capacity_gb: u64,
    pub persistence: bool,
    pub backup_enabled: bool,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
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
    pub resources: ServiceResources,
}

/// Service resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResources {
    pub cpu_cores: f64,
    pub memory_mb: u64,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    pub cpu_cores: f64,
    pub memory_gb: u64,
    pub storage_gb: u64,
    pub gpu_count: u32,
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub enabled: bool,
    pub federation_id: String,
    pub peers: Vec<String>,
    pub sync_interval_seconds: u64,
}
