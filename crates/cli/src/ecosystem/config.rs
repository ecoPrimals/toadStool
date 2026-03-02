//! Ecosystem configuration management
//!
//! Handles loading and managing service discovery configuration from:
//! - Environment variables
//! - Configuration files (TOML)
//! - Runtime overrides

use crate::{CliContextExt, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceDiscoveryConfig {
    /// Service endpoints by capability category
    #[serde(default)]
    pub services: HashMap<String, ServiceConfig>,

    /// Discovery preferences
    #[serde(default)]
    pub discovery: DiscoveryConfig,
}

/// Configuration for a specific service capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service endpoint URL
    pub url: String,

    /// Priority (0-100, higher = preferred)
    #[serde(default = "default_priority")]
    pub priority: u8,

    /// Enabled/disabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Health check interval (seconds)
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: u64,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_priority() -> u8 {
    50
}
fn default_enabled() -> bool {
    true
}
fn default_health_check_interval() -> u64 {
    30
}

/// Discovery preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable environment variable discovery
    #[serde(default = "default_true")]
    pub enable_env: bool,

    /// Enable configuration file discovery
    #[serde(default = "default_true")]
    pub enable_config: bool,

    /// Enable mDNS discovery
    #[serde(default = "default_false")]
    pub enable_mdns: bool,

    /// Enable service mesh discovery
    #[serde(default = "default_false")]
    pub enable_service_mesh: bool,

    /// Discovery timeout (seconds)
    #[serde(default = "default_discovery_timeout")]
    pub timeout_seconds: u64,
}

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_discovery_timeout() -> u64 {
    10
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_env: true,
            enable_config: true,
            enable_mdns: false,
            enable_service_mesh: false,
            timeout_seconds: 10,
        }
    }
}

impl ServiceDiscoveryConfig {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns an error if:
    /// - The configuration file cannot be read (permissions, not found, etc.)
    /// - The file contents are not valid UTF-8
    /// - The TOML syntax is invalid or cannot be parsed
    #[must_use = "Configuration loading should be checked"]
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read config file: {}",
            path.as_ref().display()
        ))?;

        toml::from_str(&contents).context("Failed to parse TOML configuration")
    }

    /// Load configuration from default locations
    ///
    /// Tries in order using Pure Rust etcetera:
    /// 1. `~/.toadstool/services.toml`
    /// 2. `./.toadstool/config.toml`
    /// 3. `/etc/toadstool/services.toml`
    pub fn load_default() -> Self {
        use etcetera::{choose_base_strategy, BaseStrategy};

        let paths = if let Ok(strategy) = choose_base_strategy() {
            vec![
                Some(strategy.home_dir().join(".toadstool/services.toml")),
                Some(PathBuf::from(".toadstool/config.toml")),
                Some(PathBuf::from(
                    super::constants::paths::SYSTEM_SERVICES_CONFIG,
                )),
            ]
        } else {
            vec![
                Some(PathBuf::from(".toadstool/config.toml")),
                Some(PathBuf::from(
                    super::constants::paths::SYSTEM_SERVICES_CONFIG,
                )),
            ]
        };

        for path in paths.into_iter().flatten() {
            if let Ok(config) = Self::from_file(&path) {
                tracing::info!("Loaded service discovery config from: {}", path.display());
                return config;
            }
        }

        // Return default if no config found
        tracing::debug!("No config file found, using defaults");
        Self::default()
    }

    /// Get service configuration by capability category
    pub fn get_service(&self, capability_category: &str) -> Option<&ServiceConfig> {
        self.services.get(capability_category)
    }

    /// Save configuration to file
    ///
    /// # Errors
    /// Returns an error if:
    /// - The configuration cannot be serialized to TOML
    /// - The file cannot be written (permissions, disk space, etc.)
    #[must_use = "Configuration save should be checked"]
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let toml = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        std::fs::write(path.as_ref(), toml).context(format!(
            "Failed to write config to: {}",
            path.as_ref().display()
        ))?;

        Ok(())
    }

    /// Create example configuration file
    ///
    /// Uses capability-based endpoint discovery: Unix socket paths from
    /// biomeOS runtime directory. Services are discovered at runtime via
    /// well-known capability constants (primal_identity, ecosystem).
    #[allow(deprecated)] // Intentional: IPC addressing requires well-known names
    pub fn create_example() -> Self {
        use toadstool_common::constants::ecosystem::well_known;
        use toadstool_common::primal_sockets::get_biomeos_dir;

        let mut services = HashMap::new();
        let biomeos_dir = get_biomeos_dir();

        // Capability-based: PKI (beardog) Unix socket
        let beardog_socket = biomeos_dir.join(format!("{}.sock", well_known::BEARDOG));
        services.insert(
            "crypto".to_string(),
            ServiceConfig {
                url: format!("unix://{}", beardog_socket.display()),
                priority: 90,
                enabled: false, // Disabled by default - enable via discovery
                health_check_interval: 30,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "discovery_capability".to_string(),
                        toadstool_common::constants::primal_identity::capability::CRYPTO_PROVIDER
                            .to_string(),
                    );
                    meta
                },
            },
        );

        // Capability-based: Storage (nestgate) Unix socket
        let nestgate_socket = biomeos_dir.join(format!("{}.sock", well_known::NESTGATE));
        services.insert(
            "storage".to_string(),
            ServiceConfig {
                url: format!("unix://{}", nestgate_socket.display()),
                priority: 80,
                enabled: false,
                health_check_interval: 60,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "discovery_capability".to_string(),
                        toadstool_common::constants::primal_identity::capability::STORAGE_PROVIDER
                            .to_string(),
                    );
                    meta
                },
            },
        );

        // Capability-based: Coordination (songbird) Unix socket
        let songbird_socket = biomeos_dir.join(format!("{}.sock", well_known::SONGBIRD));
        services.insert(
            "coordination".to_string(),
            ServiceConfig {
                url: format!("unix://{}", songbird_socket.display()),
                priority: 85,
                enabled: true,
                health_check_interval: 15,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "discovery_capability".to_string(),
                        toadstool_common::constants::primal_identity::capability::SERVICE_DISCOVERY
                            .to_string(),
                    );
                    meta
                },
            },
        );

        Self {
            services,
            discovery: DiscoveryConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServiceDiscoveryConfig::default();
        assert!(config.services.is_empty());
        assert!(config.discovery.enable_env);
        assert!(config.discovery.enable_config);
    }

    #[test]
    fn test_example_config() {
        let config = ServiceDiscoveryConfig::create_example();
        assert_eq!(config.services.len(), 3);
        assert!(config.get_service("crypto").is_some());
        assert!(config.get_service("storage").is_some());
        assert!(config.get_service("coordination").is_some());
    }

    #[test]
    fn test_toml_serialization() {
        let config = ServiceDiscoveryConfig::create_example();
        let toml = toml::to_string_pretty(&config).unwrap();

        // Should be able to round-trip
        let parsed: ServiceDiscoveryConfig = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.services.len(), config.services.len());
    }
}
