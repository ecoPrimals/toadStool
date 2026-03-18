// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(deprecated)] // Intentional: IPC addressing requires well-known names
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::wildcard_imports,
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
    clippy::uninlined_format_args,
    clippy::format_push_string,
    clippy::used_underscore_binding,
    clippy::unnecessary_wraps,
    clippy::struct_excessive_bools,
    clippy::single_match_else,
    clippy::needless_continue,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::cast_sign_loss,
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

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("System error: {0}")]
    System(String),

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

impl From<ed25519_dalek::ed25519::Error> for CliError {
    fn from(e: ed25519_dalek::ed25519::Error) -> Self {
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated: std::time::SystemTime,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    pub version: String,
    pub source: WorkloadSource,
    pub enabled: bool,
    pub config: HashMap<String, serde_yaml_ng::Value>,
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
        wasi_config: Option<HashMap<String, serde_yaml_ng::Value>>,
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
    pub nestgate_integration: Option<String>, // NestGate version or config
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created: std::time::SystemTime,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub started: Option<std::time::SystemTime>,
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
