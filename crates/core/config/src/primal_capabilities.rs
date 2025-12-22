//! # Primal Capabilities Loader
//!
//! **Universal, Agnostic, Sovereignty-First**
//!
//! This module provides runtime loading of primal capabilities from `primal-capabilities.toml`,
//! eliminating ALL hardcoded endpoint/port references.
//!
//! ## Philosophy
//!
//! **"Each primal knows only itself. Everything else is discovered."**
//!
//! - **Self-Knowledge**: Toadstool knows what IT can do
//! - **Runtime Discovery**: Find other primals by capability, not name
//! - **No Hardcoding**: Zero assumptions about other primals
//! - **Capability-Based**: Discover by WHAT you need, not WHO
//!
//! ## Usage
//!
//! ```no_run
//! # use toadstool_config::primal_capabilities::*;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load capabilities at runtime
//! let registry = PrimalCapabilitiesRegistry::load_from_file("primal-capabilities.toml")?;
//!
//! // Find primals by what they can do, not their name
//! let crypto_services = registry.find_by_capability("cryptographic-operations");
//! let storage_services = registry.find_by_capability("storage");
//!
//! // Get endpoint for first available service
//! if let Some(crypto) = crypto_services.first() {
//!     let endpoint = registry.get_endpoint(crypto, "localhost")?;
//!     println!("Crypto service: {}", endpoint);
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Primal capability registry error
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("Failed to load capabilities file: {0}")]
    LoadFailed(String),

    #[error("Failed to parse capabilities: {0}")]
    ParseFailed(String),

    #[error("Primal not found: {0}")]
    PrimalNotFound(String),

    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("No endpoint configured for primal: {0}")]
    NoEndpoint(String),
}

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

/// Registry metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryMetadata {
    #[serde(default)]
    pub version: String,

    #[serde(default)]
    pub discovery_protocol: String,

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

/// Discovery configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Discovery methods in priority order
    #[serde(default)]
    pub methods: Vec<String>,

    /// Cache settings
    #[serde(default)]
    pub cache_enabled: bool,

    #[serde(default)]
    pub cache_ttl_seconds: u64,

    /// Retry settings
    #[serde(default)]
    pub retry_attempts: u32,

    #[serde(default)]
    pub retry_delay_ms: u64,

    #[serde(default)]
    pub timeout_seconds: u64,

    /// Health check settings
    #[serde(default)]
    pub health_check_enabled: bool,

    #[serde(default)]
    pub health_check_interval_seconds: u64,

    #[serde(default)]
    pub health_check_timeout_seconds: u64,

    /// Preferences
    #[serde(default)]
    pub preferences: DiscoveryPreferences,
}

/// Discovery preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryPreferences {
    /// Prefer local services
    #[serde(default)]
    pub prefer_local: bool,

    /// Require healthy services
    #[serde(default)]
    pub require_healthy: bool,

    /// Load balancing strategy
    #[serde(default)]
    pub load_balance: String,

    /// Fallback to any available service
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

        // Try config directory
        if let Some(config_dir) = directories::ProjectDirs::from("", "", "toadstool") {
            let config_path = config_dir.config_dir().join("primal-capabilities.toml");
            if config_path.exists() {
                return Self::load_from_file(config_path);
            }
        }

        Err(CapabilityError::LoadFailed(
            "No primal-capabilities.toml found in default locations".to_string(),
        ))
    }

    /// Find primal names that have a specific capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.capabilities.iter().any(|c| c == capability))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals that have ALL of the specified capabilities
    pub fn find_by_capabilities(&self, capabilities: &[&str]) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| {
                capabilities
                    .iter()
                    .all(|cap| def.capabilities.contains(&cap.to_string()))
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals by role
    pub fn find_by_role(&self, role: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.primary_role == role)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get primal definition
    pub fn get_primal(&self, name: &str) -> Option<&PrimalDefinition> {
        self.primals.get(name)
    }

    /// Get endpoint for a primal
    ///
    /// Constructs endpoint from host and default_port
    /// In production, this should query actual service discovery (mDNS, Consul, etc.)
    pub fn get_endpoint(&self, primal_name: &str, host: &str) -> CapabilityResult<String> {
        let primal = self
            .primals
            .get(primal_name)
            .ok_or_else(|| CapabilityError::PrimalNotFound(primal_name.to_string()))?;

        // In development/local: use host:port
        // In production: should use real service discovery
        let protocol = if primal.protocols.contains(&"http".to_string()) {
            "http"
        } else {
            "https"
        };

        Ok(format!("{}://{}:{}", protocol, host, primal.default_port))
    }

    /// Get migration fallback URL (deprecated)
    #[deprecated(note = "Use capability discovery instead of migration fallbacks")]
    pub fn get_migration_fallback(&self, primal_name: &str) -> Option<&str> {
        self.migration
            .get(primal_name)
            .map(|m| m.fallback_url.as_str())
    }

    /// Get all primals with their endpoints
    ///
    /// Returns a map of primal_name -> endpoint
    pub fn get_all_endpoints(&self, host: &str) -> HashMap<String, String> {
        self.primals
            .iter()
            .map(|(name, primal)| {
                let protocol = if primal.protocols.contains(&"http".to_string()) {
                    "http"
                } else {
                    "https"
                };
                (
                    name.clone(),
                    format!("{}://{}:{}", protocol, host, primal.default_port),
                )
            })
            .collect()
    }
}

/// Helper function to get self-knowledge (Toadstool's own capabilities)
///
/// This is the ONLY place where hardcoding is acceptable:
/// **"Know thyself"** - a primal should know its own capabilities
pub fn get_self_capabilities(registry: &PrimalCapabilitiesRegistry) -> Option<&PrimalDefinition> {
    registry.get_primal("toadstool")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> PrimalCapabilitiesRegistry {
        let toml_content = r#"
[registry]
version = "1.0.0"
discovery_protocol = "capability-based"

[primals.toadstool]
name = "toadstool"
description = "Universal compute platform"
primary_role = "compute"
capabilities = ["universal-compute", "wasm-execution"]
protocols = ["http"]
default_port = 8080
health_endpoint = "/health"

[primals.beardog]
name = "beardog"
description = "Cryptographic security"
primary_role = "security"
capabilities = ["cryptographic-operations", "key-management"]
protocols = ["http"]
default_port = 8081
health_endpoint = "/health"

[discovery]
methods = ["environment", "mdns"]
cache_enabled = true
cache_ttl_seconds = 300
"#;

        toml::from_str(toml_content).unwrap()
    }

    #[test]
    fn test_find_by_capability() {
        let registry = create_test_registry();
        let compute_primals = registry.find_by_capability("universal-compute");
        assert_eq!(compute_primals, vec!["toadstool"]);

        let crypto_primals = registry.find_by_capability("cryptographic-operations");
        assert_eq!(crypto_primals, vec!["beardog"]);
    }

    #[test]
    fn test_find_by_role() {
        let registry = create_test_registry();
        let compute_primals = registry.find_by_role("compute");
        assert_eq!(compute_primals, vec!["toadstool"]);

        let security_primals = registry.find_by_role("security");
        assert_eq!(security_primals, vec!["beardog"]);
    }

    #[test]
    fn test_get_endpoint() {
        let registry = create_test_registry();
        let endpoint = registry.get_endpoint("toadstool", "localhost").unwrap();
        assert_eq!(endpoint, "http://localhost:8080");

        let endpoint = registry.get_endpoint("beardog", "localhost").unwrap();
        assert_eq!(endpoint, "http://localhost:8081");
    }

    #[test]
    fn test_self_knowledge() {
        let registry = create_test_registry();
        let self_def = get_self_capabilities(&registry).unwrap();
        assert_eq!(self_def.name, "toadstool");
        assert!(self_def
            .capabilities
            .contains(&"universal-compute".to_string()));
    }
}
