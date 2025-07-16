//! ToadStool CLI - Universal Compute Command Center
//!
//! The gateway to SOVEREIGN SCIENCE and universal compute capabilities.
//! Commands for managing biome.yaml manifests and orchestrating distributed workloads.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use toadstool_config::network::DEFAULT_CONNECTION_TIMEOUT_SECS;
use tokio::fs;
use uuid::Uuid;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CliError {
    #[error("Biome not found: {0}")]
    BiomeNotFound(String),

    #[error("Biome already exists: {0}")]
    BiomeAlreadyExists(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System error: {0}")]
    System(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// ToadStool - Universal Compute Platform for Sovereign Science
#[derive(Parser)]
#[command(name = "toadstool")]
#[command(about = "🍄 Universal Compute Platform - The backbone of SOVEREIGN SCIENCE")]
#[command(version = "0.1.0")]
#[command(author = "ToadStool Development Team")]
#[command(long_about = "
ToadStool is the universal runtime environment for the ecoPrimals ecosystem.
It bootstraps, manages, and isolates complete biomeOS instances from declarative
manifest files (biome.yaml).

🎯 SOVEREIGN SCIENCE: Your compute, your data, your control
🚀 UNIVERSAL COMPUTE: If it has a chip and memory, ToadStool runs on it
🔒 ZERO TRUST: BearDog cryptographic security by default
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Working directory
    #[arg(short = 'C', long, global = true)]
    pub directory: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start and run a biome in the foreground
    Run {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Override biome name
        #[arg(short, long)]
        name: Option<String>,

        /// Environment variables to set
        #[arg(short, long)]
        env: Vec<String>,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,

        /// Resource limits override
        #[arg(long)]
        cpu_limit: Option<f64>,
        #[arg(long)]
        memory_limit: Option<String>,

        /// Security level (low, medium, high, maximum)
        #[arg(long, default_value = "high")]
        security: String,
    },

    /// Start a biome in the background (detached mode)
    Up {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Run in detached mode (background)
        #[arg(short, long)]
        detach: bool,

        /// Override biome name
        #[arg(short, long)]
        name: Option<String>,

        /// Environment variables to set
        #[arg(short, long)]
        env: Vec<String>,

        /// Auto-restart on failure
        #[arg(long)]
        restart: bool,

        /// Health check interval in seconds
        #[arg(long, default_value = "30")]
        health_interval: u64,
    },

    /// Stop a running biome
    Down {
        /// Biome name or ID to stop
        biome: String,

        /// Force stop (SIGKILL)
        #[arg(short, long)]
        force: bool,

        /// Timeout for graceful shutdown
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Remove all associated data
        #[arg(long)]
        purge: bool,
    },

    /// List all running biomes on the host
    Ps {
        /// Show all biomes (including stopped)
        #[arg(short, long)]
        all: bool,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show resource usage
        #[arg(short, long)]
        resources: bool,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// View logs for a specific biome or service
    Logs {
        /// Biome name or service name (biome.service)
        target: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: usize,

        /// Show timestamps
        #[arg(short, long)]
        timestamps: bool,

        /// Filter by log level
        #[arg(long)]
        level: Option<String>,

        /// Search pattern
        #[arg(long)]
        grep: Option<String>,
    },

    /// Validate a biome.yaml manifest
    Validate {
        /// Path to biome.yaml manifest file
        manifest: PathBuf,

        /// Check resource availability
        #[arg(long)]
        check_resources: bool,

        /// Validate security policies
        #[arg(long)]
        check_security: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Initialize a new biome.yaml template
    Init {
        /// Directory to create biome.yaml in
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Biome template type
        #[arg(short, long, default_value = "basic")]
        template: String,

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },

    /// Show system capabilities and detected platforms
    Capabilities {
        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Show detailed platform information
        #[arg(short, long)]
        detailed: bool,

        /// Test specific platform
        #[arg(long)]
        test_platform: Option<String>,
    },

    /// Ecosystem integration commands
    Ecosystem {
        #[command(subcommand)]
        action: EcosystemCommands,
    },

    /// Advanced universal compute operations
    Universal {
        #[command(subcommand)]
        operation: UniversalCommands,
    },

    /// Zero-configuration rapid deployment
    ZeroConfig {
        /// Save configuration to file
        #[arg(short, long)]
        save_config: Option<PathBuf>,

        /// Skip service discovery
        #[arg(long)]
        skip_discovery: bool,

        /// Target deployment time in seconds
        #[arg(long, default_value = "60")]
        target_time: u64,

        /// Dry run (don't deploy)
        #[arg(long)]
        dry_run: bool,
    },

    /// Configure Songbird service mesh networking
    NetworkConfig {
        /// Apply network configuration
        #[arg(long)]
        apply: bool,

        /// Validate configuration
        #[arg(long)]
        validate: bool,

        /// Show configuration summary
        #[arg(long)]
        summary: bool,

        /// Configuration file path
        #[arg(short = 'f', long)]
        config_file: Option<PathBuf>,

        /// Test connectivity
        #[arg(long)]
        test: bool,

        /// Export configuration
        #[arg(long)]
        export: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum EcosystemCommands {
    /// Discover and connect to ecosystem services
    Discover {
        /// Service types to discover
        #[arg(short, long)]
        services: Vec<String>,

        /// Network scan timeout
        #[arg(long, default_value_t = DEFAULT_CONNECTION_TIMEOUT_SECS)]
        timeout: u64,
    },

    /// Register with Songbird discovery service
    Register {
        /// Songbird endpoint
        endpoint: String,

        /// Authentication token
        #[arg(short, long)]
        token: Option<String>,
    },

    /// Install BearDog crypto permissions
    Auth {
        /// Permission file path
        permission_file: PathBuf,

        /// Validate only (don't install)
        #[arg(long)]
        validate_only: bool,
    },

    /// Connect to NestGate storage
    Storage {
        /// NestGate endpoint
        endpoint: String,

        /// Mount point
        #[arg(short, long, default_value = "/data")]
        mount: PathBuf,

        /// ZFS dataset name
        #[arg(long)]
        dataset: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum UniversalCommands {
    /// Detect all available compute substrates
    Detect {
        /// Platform categories to detect
        #[arg(short, long)]
        categories: Vec<String>,

        /// Run detection tests
        #[arg(long)]
        test: bool,

        /// Save results to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Benchmark compute capabilities
    Benchmark {
        /// Benchmark suite to run
        #[arg(short, long, default_value = "standard")]
        suite: String,

        /// Target platforms
        #[arg(short, long)]
        platforms: Vec<String>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Migrate workloads between substrates
    Migrate {
        /// Source biome
        source: String,

        /// Target platform
        target: String,

        /// Pause source during migration
        #[arg(long)]
        pause: bool,

        /// Verify after migration
        #[arg(long)]
        verify: bool,
    },

    /// Federate with other ToadStool instances
    Federate {
        /// Remote ToadStool endpoint
        endpoint: String,

        /// Federation mode (peer, leader, follower)
        #[arg(short, long, default_value = "peer")]
        mode: String,

        /// Shared resources
        #[arg(short, long)]
        resources: Vec<String>,
    },
}

/// Biome manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// Biome metadata
    pub metadata: BiomeMetadata,

    /// Primal configurations
    pub primals: HashMap<String, PrimalConfig>,

    /// Service definitions
    pub services: HashMap<String, ServiceConfig>,

    /// Resource requirements
    pub resources: BiomeResources,

    /// Security policies
    pub security: BiomeSecurity,

    /// Network configuration
    pub networking: BiomeNetworking,

    /// Storage configuration
    pub storage: BiomeStorage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    pub version: String,
    pub source: WorkloadSource,
    pub enabled: bool,
    pub config: HashMap<String, serde_yaml::Value>,
    pub dependencies: Vec<String>,
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub version: String,
    pub source: WorkloadSource,
    pub replicas: Option<u32>,
    pub resources: ServiceResources,
    pub environment: HashMap<String, String>,
    pub ports: Vec<ServicePort>,
    pub volumes: Vec<ServiceVolume>,
    pub dependencies: Vec<String>,
    pub health_check: Option<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkloadSource {
    /// OCI container registry
    Container {
        registry: String,
        image: String,
        tag: String,
        digest: Option<String>,
    },
    /// WebAssembly module
    Wasm {
        source: String,
        checksum: String,
        wasi_config: Option<HashMap<String, serde_yaml::Value>>,
    },
    /// Git repository
    Git {
        repository: String,
        branch: Option<String>,
        commit: Option<String>,
        path: Option<String>,
    },
    /// IPFS content
    Ipfs {
        hash: String,
        gateway: Option<String>,
    },
    /// Local file path
    Local { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<String>,
    pub storage_limit: Option<String>,
    pub gpu_limit: Option<u32>,
    pub network_bandwidth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    pub isolation_level: String,
    pub trust_level: String,
    pub beardog_required: bool,
    pub crypto_policies: Vec<String>,
    pub allowed_networks: Vec<String>,
    pub forbidden_syscalls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    pub mode: String, // bridge, host, none
    pub dns_servers: Vec<String>,
    pub port_mappings: Vec<PortMapping>,
    pub network_policies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    pub nestgate_integration: bool,
    pub datasets: Vec<DatasetConfig>,
    pub volumes: Vec<VolumeConfig>,
    pub backup_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResources {
    pub cpu_limit: Option<f64>,
    pub memory_limit: Option<String>,
    pub storage_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String, // tcp, udp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceVolume {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub command: Vec<String>,
    pub interval: u64,
    pub timeout: u64,
    pub retries: u32,
    pub start_period: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub name: String,
    pub size: Option<String>,
    pub compression: Option<String>,
    pub encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub name: String,
    pub driver: String,
    pub options: HashMap<String, String>,
}

/// Running biome information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeInfo {
    pub id: Uuid,
    pub name: String,
    pub status: BiomeStatus,
    pub created: DateTime<Utc>,
    pub started: Option<DateTime<Utc>>,
    pub manifest_path: PathBuf,
    pub resource_usage: ResourceUsage,
    pub services: Vec<ServiceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomeStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
    Migrating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: String,
    pub replicas: u32,
    pub ports: Vec<u16>,
    pub health: String,
}

/// CLI execution context
pub struct CliContext {
    pub config_path: Option<PathBuf>,
    pub working_dir: PathBuf,
    pub verbose: bool,
}

impl CliContext {
    pub fn new(cli: &Cli) -> Result<Self> {
        let working_dir = if let Some(dir) = &cli.directory {
            dir.clone()
        } else {
            std::env::current_dir()?
        };

        Ok(Self {
            config_path: cli.config.clone(),
            working_dir,
            verbose: cli.verbose,
        })
    }
}

/// Load biome manifest from file
pub async fn load_biome_manifest(path: &PathBuf) -> Result<BiomeManifest> {
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read manifest file: {}", path.display()))?;

    let manifest: BiomeManifest = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse manifest file: {}", path.display()))?;

    Ok(manifest)
}

/// Validate biome manifest
pub fn validate_manifest(manifest: &BiomeManifest) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Check for required primals
    if !manifest.primals.contains_key("beardog") && manifest.security.beardog_required {
        warnings.push("BearDog is required but not configured".to_string());
    }

    // Validate service dependencies
    for (service_name, service) in &manifest.services {
        for dep in &service.dependencies {
            if !manifest.services.contains_key(dep) && !manifest.primals.contains_key(dep) {
                warnings.push(format!(
                    "Service '{service_name}' depends on undefined service '{dep}'"
                ));
            }
        }
    }

    // Check resource limits
    if manifest.resources.cpu_limit.is_none() {
        warnings.push("No CPU limit specified - consider setting resource limits".to_string());
    }

    Ok(warnings)
}

pub mod ecosystem;
pub mod executor;
pub mod monitoring;
pub mod network_config;
pub mod templates;
pub mod universal;
pub mod zero_config;
