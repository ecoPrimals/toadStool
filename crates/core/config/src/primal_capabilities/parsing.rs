// SPDX-License-Identifier: AGPL-3.0-or-later
//! TOML loading and serde schema for `primal-capabilities.toml`.

use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Primal capability registry error.
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    /// Failed to load capabilities file from disk.
    #[error("Failed to load capabilities file: {0}")]
    LoadFailed(String),

    /// Failed to parse capabilities TOML.
    #[error("Failed to parse capabilities: {0}")]
    ParseFailed(String),

    /// Requested primal not found in registry.
    #[error("Primal not found: {0}")]
    PrimalNotFound(String),

    /// Requested capability not found.
    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    /// No endpoint configured for the primal.
    #[error("No endpoint configured for primal: {0}")]
    NoEndpoint(String),
}

/// Result type for capability registry operations.
pub type CapabilityResult<T> = Result<T, CapabilityError>;

/// Primal capabilities registry
///
/// Loads and manages primal capability information from primal-capabilities.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCapabilitiesRegistry {
    /// Registry metadata
    #[serde(default)]
    pub registry: RegistryMetadata,

    /// Primal definitions
    #[serde(default)]
    pub primals: HashMap<String, PrimalDefinition>,

    /// Discovery configuration
    #[serde(default)]
    pub discovery: DiscoveryConfig,

    /// Migration mappings (deprecated - for backward compatibility)
    #[serde(default)]
    pub migration: HashMap<String, MigrationMapping>,
}

/// Registry metadata from primal-capabilities.toml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryMetadata {
    /// Registry schema version.
    #[serde(default)]
    pub version: String,

    /// Discovery protocol (e.g. mdns, environment).
    #[serde(default)]
    pub discovery_protocol: String,

    /// Fallback strategy when discovery fails.
    #[serde(default)]
    pub fallback_strategy: String,
}

/// Primal definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalDefinition {
    /// Primal name
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: String,

    /// Primary role (compute, security, storage, coordination, intelligence, orchestration)
    pub primary_role: String,

    /// Capabilities this primal provides
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Supported protocols
    #[serde(default)]
    pub protocols: Vec<String>,

    /// Default port (discovered at runtime, not hardcoded in code!)
    pub default_port: u16,

    /// Health check endpoint
    #[serde(default)]
    pub health_endpoint: String,

    /// Metrics endpoint (optional)
    #[serde(default)]
    pub metrics_endpoint: Option<String>,

    /// Discovery endpoint (for coordination services)
    #[serde(default)]
    pub discovery_endpoint: Option<String>,
}

/// Discovery configuration for finding primals at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Discovery methods in priority order (e.g. `["mdns", "environment"]`).
    #[serde(default)]
    pub methods: Vec<String>,

    /// Enable discovery result caching.
    #[serde(default)]
    pub cache_enabled: bool,

    /// Cache TTL in seconds.
    #[serde(default)]
    pub cache_ttl_seconds: u64,

    /// Number of retry attempts on discovery failure.
    #[serde(default)]
    pub retry_attempts: u32,

    /// Delay between retries in milliseconds.
    #[serde(default)]
    pub retry_delay_ms: u64,

    /// Discovery timeout in seconds.
    #[serde(default)]
    pub timeout_seconds: u64,

    /// Enable health checks before using discovered services.
    #[serde(default)]
    pub health_check_enabled: bool,

    /// Health check interval in seconds.
    #[serde(default)]
    pub health_check_interval_seconds: u64,

    /// Health check timeout in seconds.
    #[serde(default)]
    pub health_check_timeout_seconds: u64,

    /// Discovery preferences (local, healthy, load balance).
    #[serde(default)]
    pub preferences: DiscoveryPreferences,
}

/// Discovery preferences for service selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryPreferences {
    /// Prefer services on localhost.
    #[serde(default)]
    pub prefer_local: bool,

    /// Only use services that pass health check.
    #[serde(default)]
    pub require_healthy: bool,

    /// Load balancing strategy (round-robin, random, etc.).
    #[serde(default)]
    pub load_balance: String,

    /// Use any available service if preferred unavailable.
    #[serde(default)]
    pub fallback_to_any: bool,
}

/// Migration mapping (deprecated - for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationMapping {
    /// Capability to discover
    pub capability: String,

    /// Fallback URL (last resort only)
    pub fallback_url: String,
}

impl PrimalCapabilitiesRegistry {
    /// Load registry from file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> CapabilityResult<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| CapabilityError::LoadFailed(e.to_string()))?;

        let registry: Self =
            toml::from_str(&content).map_err(|e| CapabilityError::ParseFailed(e.to_string()))?;

        Ok(registry)
    }

    /// Load registry from default location
    ///
    /// Tries these paths in order:
    /// 1. `PRIMAL_CAPABILITIES_PATH` environment variable
    /// 2. `./primal-capabilities.toml`
    /// 3. `~/.config/toadstool/primal-capabilities.toml`
    ///
    /// # Errors
    /// Returns error if no file found or parse fails
    pub fn load_default() -> CapabilityResult<Self> {
        // Try environment variable
        if let Ok(path) = std::env::var("PRIMAL_CAPABILITIES_PATH") {
            if Path::new(&path).exists() {
                return Self::load_from_file(&path);
            }
        }

        // Try current directory
        let local_path = Path::new("primal-capabilities.toml");
        if local_path.exists() {
            return Self::load_from_file(local_path);
        }

        // Try config directory (Pure Rust Evolution - Jan 17, 2026)
        // OLD: directories::ProjectDirs (pulled in dirs-sys)
        // NEW: etcetera (100% Pure Rust!)
        if let Ok(strategy) = etcetera::choose_base_strategy() {
            let primal_name = toadstool_common::constants::primal_identity::PRIMAL_NAME;
            let config_path = strategy
                .config_dir()
                .join(primal_name)
                .join("primal-capabilities.toml");
            if config_path.exists() {
                return Self::load_from_file(config_path);
            }
        } else {
            // Strategy selection failed, continue to error
        }

        Err(CapabilityError::LoadFailed(
            "No primal-capabilities.toml found in default locations".to_string(),
        ))
    }
}
