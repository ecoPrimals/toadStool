// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(deprecated)] // Intentional: IPC addressing requires well-known names
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::wildcard_imports,
    clippy::uninlined_format_args,
    clippy::must_use_candidate,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls,
    clippy::no_effect_underscore_binding,
    clippy::inefficient_to_string,
    clippy::unused_self,
    clippy::ref_option,
    clippy::explicit_iter_loop,
    clippy::return_self_not_must_use,
    clippy::match_same_arms,
    clippy::unused_async,
    clippy::format_push_string,
    clippy::used_underscore_binding,
    clippy::unnecessary_wraps,
    clippy::struct_excessive_bools,
    clippy::single_match_else,
    clippy::needless_continue,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::cast_lossless,
    clippy::implicit_clone,
    clippy::implicit_hasher,
    clippy::fn_params_excessive_bools,
    clippy::default_trait_access,
    clippy::float_cmp,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::assigning_clones,
    clippy::needless_raw_string_hashes
)]

//! `ToadStool` CLI - Universal Compute Command Center
//!
//! The gateway to SOVEREIGN SCIENCE and universal compute capabilities.
//! Commands for managing biome.yaml manifests and orchestrating distributed workloads.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use toadstool_common::constants::ecosystem::well_known;
use tokio::fs;
use uuid::Uuid;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CliError {
    /// Biome not found by name or path
    #[error("Biome not found: {0}")]
    BiomeNotFound(String),

    /// Biome already exists when attempting to create
    #[error("Biome already exists: {0}")]
    BiomeAlreadyExists(String),

    /// Invalid configuration or manifest
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// I/O error during file or system operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// System or hardware error (e.g. NPU)
    #[error("System error: {0}")]
    System(String),

    /// Catch-all for other errors
    #[error("Other error: {0}")]
    Other(String),
}

impl From<base64::DecodeError> for CliError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::net::AddrParseError> for CliError {
    fn from(e: std::net::AddrParseError) -> Self {
        Self::Other(e.to_string())
    }
}

#[cfg(feature = "npu")]
impl From<akida_driver::AkidaError> for CliError {
    fn from(e: akida_driver::AkidaError) -> Self {
        Self::System(e.to_string())
    }
}

impl From<toadstool::ToadStoolError> for CliError {
    fn from(e: toadstool::ToadStoolError) -> Self {
        Self::Other(e.to_string())
    }
}

/// CLI result type alias. Use `Result<T>` for CliError, or `Result<T, E>` for other errors (e.g. serde).
pub type Result<T, E = CliError> = std::result::Result<T, E>;

/// Add context to errors (replacement for anyhow::Context)
pub trait CliContextExt<T> {
    /// Attach a context message to the error for better diagnostics
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

impl<T, E> CliContextExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| CliError::Other(format!("{context}: {e}")))
    }
}

pub use commands::{
    Commands, EcosystemCommands, ModeCommand, TransportCommands, UniversalCommands,
};

/// `ToadStool` - Universal Compute Platform for Sovereign Science
#[derive(Parser)]
#[command(name = "toadstool")]
#[command(about = "🍄 Universal Compute Platform - The backbone of SOVEREIGN SCIENCE")]
#[command(version = env!("CARGO_PKG_VERSION"))]
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
    /// Top-level subcommand (ecosystem, universal, daemon, etc.)
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

/// Biome manifest metadata (name, version, timestamps)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Biome display name
    pub name: String,
    /// Semantic version string
    pub version: String,
    /// Optional human-readable description
    pub description: Option<String>,
    /// Optional author or maintainer
    pub author: Option<String>,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    /// Last update timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated: std::time::SystemTime,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Configuration for a primal (workload) in the biome manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    /// Primal version
    pub version: String,
    /// Where to load the workload from (container, wasm, git, etc.)
    pub source: WorkloadSource,
    /// Whether the primal is enabled
    pub enabled: bool,
    /// Arbitrary config key-value pairs
    pub config: HashMap<String, serde_yaml_ng::Value>,
    /// Names of other primals or services this depends on
    pub dependencies: Vec<String>,
    /// Optional health check configuration
    pub health_check: Option<HealthCheck>,
}

/// Configuration for a service in the biome manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service version
    pub version: String,
    /// Where to load the service workload from
    pub source: WorkloadSource,
    /// Number of replicas (None = 1)
    pub replicas: Option<u32>,
    /// CPU, memory, storage limits
    pub resources: ServiceResources,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<ServicePort>,
    /// Volume mounts
    pub volumes: Vec<ServiceVolume>,
    /// Names of services this depends on
    pub dependencies: Vec<String>,
    /// Optional health check configuration
    pub health_check: Option<HealthCheck>,
}

/// Source type for loading a workload (container, wasm, git, ipfs, local)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkloadSource {
    /// OCI container registry
    Container {
        /// Registry host (e.g. docker.io)
        registry: String,
        /// Image name
        image: String,
        /// Image tag
        tag: String,
        /// Optional digest for pinning
        digest: Option<String>,
    },
    /// WebAssembly module
    Wasm {
        /// URL or path to the WASM module
        source: String,
        /// SHA256 or similar checksum for verification
        checksum: String,
        /// Optional WASI runtime config
        wasi_config: Option<HashMap<String, serde_yaml_ng::Value>>,
    },
    /// Git repository
    Git {
        /// Repository URL
        repository: String,
        /// Branch to checkout
        branch: Option<String>,
        /// Commit or tag to pin
        commit: Option<String>,
        /// Subpath within the repo
        path: Option<String>,
    },
    /// IPFS content
    Ipfs {
        /// IPFS CID
        hash: String,
        /// Gateway URL for fetching
        gateway: Option<String>,
    },
    /// Local file path
    Local {
        /// Path to the workload file or directory
        path: PathBuf,
    },
}

/// Resource limits for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU cores limit
    pub cpu_limit: Option<f64>,
    /// Memory limit (e.g. "512Mi")
    pub memory_limit: Option<String>,
    /// Storage limit (e.g. "10Gi")
    pub storage_limit: Option<String>,
    /// GPU count limit
    pub gpu_limit: Option<u32>,
    /// Network bandwidth limit
    pub network_bandwidth: Option<String>,
}

/// Security policies for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    /// Isolation level (e.g. process, container)
    pub isolation_level: String,
    /// Trust level for workload execution
    pub trust_level: String,
    /// Whether BearDog crypto is required
    pub beardog_required: bool,
    /// Crypto policy names
    pub crypto_policies: Vec<String>,
    /// Allowed network CIDRs
    pub allowed_networks: Vec<String>,
    /// Syscalls to forbid
    pub forbidden_syscalls: Vec<String>,
}

/// Network configuration for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    /// Network mode (bridge, host, none)
    pub mode: String,
    /// DNS server addresses
    pub dns_servers: Vec<String>,
    /// Host-to-container port mappings
    pub port_mappings: Vec<PortMapping>,
    /// Network policy names
    pub network_policies: Vec<String>,
}

/// Storage configuration for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    /// NestGate integration version or config
    pub nestgate_integration: Option<String>,
    /// Dataset definitions
    pub datasets: Vec<DatasetConfig>,
    /// Volume definitions
    pub volumes: Vec<VolumeConfig>,
    /// Backup policy name or config
    pub backup_policy: Option<String>,
}

/// Resource limits for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResources {
    /// CPU cores limit
    pub cpu_limit: Option<f64>,
    /// Memory limit (e.g. "256Mi")
    pub memory_limit: Option<String>,
    /// Storage limit
    pub storage_limit: Option<String>,
}

/// Port mapping for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    /// Port inside the container
    pub container_port: u16,
    /// Host port (None = same as container)
    pub host_port: Option<u16>,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Volume mount for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceVolume {
    /// Host path or volume name
    pub source: String,
    /// Mount path inside the container
    pub target: String,
    /// Whether the mount is read-only
    pub read_only: bool,
}

/// Health check configuration for a service or primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Command and args to run (e.g. `["curl", "-f", "http://localhost/health"]`)
    pub command: Vec<String>,
    /// Interval between checks in seconds
    pub interval: u64,
    /// Timeout per check in seconds
    pub timeout: u64,
    /// Consecutive failures before unhealthy
    pub retries: u32,
    /// Grace period before first check in seconds
    pub start_period: u64,
}

/// Host-to-container port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Port on the host
    pub host_port: u16,
    /// Port in the container
    pub container_port: u16,
    /// Protocol (tcp, udp)
    pub protocol: String,
}

/// Dataset configuration for NestGate storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Dataset name
    pub name: String,
    /// Size limit (e.g. "10Gi")
    pub size: Option<String>,
    /// Compression algorithm
    pub compression: Option<String>,
    /// Whether to encrypt at rest
    pub encryption: bool,
}

/// Volume driver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Storage driver (e.g. local, nfs)
    pub driver: String,
    /// Driver-specific options
    pub options: HashMap<String, String>,
}

/// Running biome information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeInfo {
    /// Unique biome instance ID
    pub id: Uuid,
    /// Biome name from manifest
    pub name: String,
    /// Current lifecycle status
    pub status: BiomeStatus,
    /// When the biome was created
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    /// When the biome was started (if running)
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub started: Option<std::time::SystemTime>,
    /// Path to the biome manifest file
    pub manifest_path: PathBuf,
    /// Current CPU, memory, network usage
    pub resource_usage: ResourceUsage,
    /// Status of each service
    pub services: Vec<ServiceInfo>,
}

/// Lifecycle status of a biome instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiomeStatus {
    /// Biome is bootstrapping
    Starting,
    /// Biome is running
    Running,
    /// Biome is shutting down
    Stopping,
    /// Biome has stopped
    Stopped,
    /// Biome failed with error message
    Error(String),
    /// Biome is being migrated
    Migrating,
}

/// Current resource usage for a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU utilization percentage
    pub cpu_percent: f64,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Storage usage in bytes
    pub storage_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// Network bytes transmitted
    pub network_tx_bytes: u64,
}

/// Status of a single service within a biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Current status (running, stopped, etc.)
    pub status: String,
    /// Number of replicas
    pub replicas: u32,
    /// Exposed port numbers
    pub ports: Vec<u16>,
    /// Health status (healthy, unhealthy, unknown)
    pub health: String,
}

/// CLI execution context
pub struct CliContext {
    /// Path to config file (if -c/--config specified)
    pub config_path: Option<PathBuf>,
    /// Working directory (-C/--directory or cwd)
    pub working_dir: PathBuf,
    /// Whether verbose output is enabled (-v/--verbose)
    pub verbose: bool,
}

impl CliContext {
    /// Create context from parsed CLI args (config path, working dir, verbose)
    pub fn new(cli: &Cli) -> crate::Result<Self> {
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
///
/// Supports both TOML (preferred, ecoBin compliant) and YAML formats.
/// Format is detected by file extension:
/// - `.toml` → TOML parser (pure Rust, no C dependencies)
/// - `.yaml`, `.yml` → YAML parser (legacy support)
/// - Other → Try TOML first, fall back to YAML
pub async fn load_biome_manifest(path: &PathBuf) -> crate::Result<BiomeManifest> {
    use crate::CliContextExt;

    let content = fs::read_to_string(path)
        .await
        .context(format!("Failed to read manifest file: {}", path.display()))?;

    // Determine format from extension (TOML preferred for ecoBin compliance)
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let manifest: BiomeManifest = match extension.to_lowercase().as_str() {
        "toml" => toml::from_str(&content)
            .context(format!("Failed to parse TOML manifest: {}", path.display()))?,
        "yaml" | "yml" => serde_yaml_ng::from_str(&content)
            .context(format!("Failed to parse YAML manifest: {}", path.display()))?,
        _ => {
            // Unknown extension: try TOML first (ecoBin preferred), then YAML
            toml::from_str(&content).or_else(|_| {
                serde_yaml_ng::from_str(&content)
                    .context(format!("Failed to parse manifest: {}", path.display()))
            })?
        }
    };

    Ok(manifest)
}

/// Validate biome manifest
#[expect(deprecated, reason = "IPC addressing requires well-known names")]
pub fn validate_manifest(manifest: &BiomeManifest) -> crate::Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Check for required primals
    if !manifest.primals.contains_key(well_known::BEARDOG) && manifest.security.beardog_required {
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

pub mod commands;
pub mod daemon;
pub mod ecosystem;
pub mod executor;
pub mod monitoring;
pub mod network_config;
pub mod setup;
pub mod templates;
pub mod universal;
pub mod utils;
pub mod zero_config;
