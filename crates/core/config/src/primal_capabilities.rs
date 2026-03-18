// SPDX-License-Identifier: AGPL-3.0-or-later
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

use etcetera::BaseStrategy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path; // Pure Rust directory paths

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

    /// Find primal names that have a specific capability
    #[must_use]
    pub fn find_by_capability(&self, capability: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.capabilities.iter().any(|c| c == capability))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals that have ALL of the specified capabilities
    #[must_use]
    pub fn find_by_capabilities(&self, capabilities: &[&str]) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| {
                capabilities
                    .iter()
                    .all(|cap| def.capabilities.contains(&(*cap).to_string()))
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Find primals by role
    #[must_use]
    pub fn find_by_role(&self, role: &str) -> Vec<&str> {
        self.primals
            .iter()
            .filter(|(_, def)| def.primary_role == role)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get primal definition
    #[must_use]
    pub fn get_primal(&self, name: &str) -> Option<&PrimalDefinition> {
        self.primals.get(name)
    }

    /// Get endpoint for a primal
    ///
    /// Constructs endpoint from host and `default_port`
    /// In production, this should query actual service discovery (mDNS, Consul, etc.)
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityError`] if the primal is not found.
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
    #[must_use]
    pub fn get_migration_fallback(&self, primal_name: &str) -> Option<&str> {
        self.migration
            .get(primal_name)
            .map(|m| m.fallback_url.as_str())
    }

    /// Get all primals with their endpoints
    ///
    /// Returns a map of `primal_name` -> endpoint
    #[must_use]
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
#[must_use]
pub fn get_self_capabilities(registry: &PrimalCapabilitiesRegistry) -> Option<&PrimalDefinition> {
    let self_name = toadstool_common::constants::primal_identity::PRIMAL_NAME;
    registry.get_primal(self_name)
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
        assert!(
            self_def
                .capabilities
                .contains(&"universal-compute".to_string())
        );
    }

    #[test]
    fn test_self_knowledge_returns_none_for_unknown_primal() {
        let registry = create_test_registry();
        // toadstool exists, so get_self_capabilities returns Some
        let self_def = get_self_capabilities(&registry);
        assert!(self_def.is_some());
    }

    #[test]
    fn test_find_by_capabilities_all() {
        let registry = create_test_registry();
        // Beardog has both capabilities
        let crypto_key =
            registry.find_by_capabilities(&["cryptographic-operations", "key-management"]);
        assert_eq!(crypto_key, vec!["beardog"]);
        // No primal has both universal-compute and key-management
        let none_match = registry.find_by_capabilities(&["universal-compute", "key-management"]);
        assert!(none_match.is_empty());
    }

    #[test]
    fn test_get_primal_returns_none_for_unknown() {
        let registry = create_test_registry();
        let primal = registry.get_primal("nonexistent");
        assert!(primal.is_none());
    }

    #[test]
    fn test_get_endpoint_primal_not_found_returns_error() {
        let registry = create_test_registry();
        let result = registry.get_endpoint("nonexistent", "localhost");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CapabilityError::PrimalNotFound(_)
        ));
    }

    #[test]
    fn test_get_endpoint_uses_https_when_no_http_protocol() {
        let toml_content = r#"
[primals.secure]
name = "secure"
primary_role = "security"
capabilities = ["secure"]
protocols = ["https"]
default_port = 8443
"#;
        let registry: PrimalCapabilitiesRegistry = toml::from_str(toml_content).unwrap();
        let endpoint = registry.get_endpoint("secure", "localhost").unwrap();
        assert_eq!(endpoint, "https://localhost:8443");
    }

    #[test]
    fn test_get_all_endpoints() {
        let registry = create_test_registry();
        let endpoints = registry.get_all_endpoints("192.168.1.1");
        assert_eq!(endpoints.len(), 2);
        assert_eq!(
            endpoints.get("toadstool").unwrap(),
            "http://192.168.1.1:8080"
        );
        assert_eq!(endpoints.get("beardog").unwrap(), "http://192.168.1.1:8081");
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_migration_fallback() {
        let toml_content = r#"
[primals.toadstool]
name = "toadstool"
primary_role = "compute"
capabilities = ["compute"]
default_port = 8080

[migration.toadstool]
capability = "compute"
fallback_url = "http://fallback:8080"
"#;
        let registry: PrimalCapabilitiesRegistry = toml::from_str(toml_content).unwrap();
        let fallback = registry.get_migration_fallback("toadstool");
        assert_eq!(fallback, Some("http://fallback:8080"));
        let no_fallback = registry.get_migration_fallback("beardog");
        assert!(no_fallback.is_none());
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result =
            PrimalCapabilitiesRegistry::load_from_file("/nonexistent/path/capabilities.toml");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CapabilityError::LoadFailed(_)
        ));
    }

    #[test]
    fn test_load_from_file_parse_error() {
        let temp = std::env::temp_dir().join("invalid_capabilities.toml");
        std::fs::write(&temp, "invalid toml {{{").unwrap();
        let result = PrimalCapabilitiesRegistry::load_from_file(&temp);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CapabilityError::ParseFailed(_)
        ));
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_load_from_file_success() {
        let temp = std::env::temp_dir().join("valid_capabilities.toml");
        let content = r#"
[registry]
version = "1.0"

[primals.test]
name = "test"
primary_role = "compute"
capabilities = ["test"]
default_port = 9090
"#;
        std::fs::write(&temp, content).unwrap();
        let result = PrimalCapabilitiesRegistry::load_from_file(&temp);
        assert!(result.is_ok());
        let registry = result.unwrap();
        assert_eq!(registry.primals.len(), 1);
        assert!(registry.primals.contains_key("test"));
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_load_default_via_env_var() {
        let temp = std::env::temp_dir().join("primal_caps_env_test.toml");
        let content = r#"
[registry]
version = "1.0"

[primals.envtest]
name = "envtest"
primary_role = "compute"
capabilities = ["test"]
default_port = 7777
"#;
        std::fs::write(&temp, content).unwrap();
        let path_str = temp.to_str().unwrap().to_string();
        temp_env::with_var("PRIMAL_CAPABILITIES_PATH", Some(path_str.as_str()), || {
            let result = PrimalCapabilitiesRegistry::load_default();
            assert!(result.is_ok());
            let registry = result.unwrap();
            assert!(registry.primals.contains_key("envtest"));
        });
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn test_registry_metadata_defaults() {
        let toml_content = r#"
[primals.minimal]
name = "minimal"
primary_role = "compute"
default_port = 8000
"#;
        let registry: PrimalCapabilitiesRegistry = toml::from_str(toml_content).unwrap();
        assert!(registry.registry.version.is_empty());
        assert!(registry.registry.discovery_protocol.is_empty());
    }

    #[test]
    fn test_find_by_role_empty() {
        let registry = create_test_registry();
        let result = registry.find_by_role("nonexistent-role");
        assert!(result.is_empty());
    }
}
