// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Plugin System
//!
//! This module implements a dynamic plugin system for integrating unknown
//! compute providers without core code changes.
//!
//! # Philosophy
//!
//! **Extensibility Without Modification**: New providers can be added via plugins
//! without touching core code. Open/closed principle in action.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::plugin_system::{PluginManager, PluginManifest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut manager = PluginManager::new();
//!
//! // Load plugin from manifest
//! let manifest = PluginManifest {
//!     name: "custom-cloud".to_string(),
//!     version: "1.0.0".to_string(),
//!     plugin_type: "cloud_provider".to_string(),
//!     entry_point: "libcustom_cloud.so".to_string(),
//!     ..Default::default()
//! };
//!
//! manager.register_plugin(manifest)?;
//!
//! // Plugin is now available for use
//! let available = manager.list_plugins();
//! println!("Available plugins: {:?}", available);
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Plugin manager
///
/// Manages loading, registration, and lifecycle of plugins.
pub struct PluginManager {
    /// Registered plugins
    plugins: HashMap<String, PluginInfo>,

    /// Plugin search paths
    search_paths: Vec<PathBuf>,

    /// Plugin configuration
    config: PluginConfig,
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Enable plugin system
    pub enabled: bool,

    /// Require signed plugins
    pub require_signatures: bool,

    /// Maximum plugins to load
    pub max_plugins: usize,

    /// Plugin timeout (seconds)
    pub plugin_timeout_secs: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            require_signatures: false, // For development
            max_plugins: 100,
            plugin_timeout_secs: 30,
        }
    }
}

/// Plugin manifest
///
/// Describes a plugin's capabilities and requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin type (e.g., "cloud_provider", "storage_backend")
    pub plugin_type: String,

    /// Entry point (library path)
    pub entry_point: String,

    /// Author
    pub author: Option<String>,

    /// Description
    pub description: Option<String>,

    /// Required ToadStool version
    pub requires_toadstool_version: Option<String>,

    /// Dependencies on other plugins
    pub dependencies: Vec<String>,

    /// Capabilities provided
    pub provides: Vec<String>,

    /// Configuration schema
    pub config_schema: Option<String>,

    /// Plugin metadata
    pub metadata: HashMap<String, String>,
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin manifest
    pub manifest: PluginManifest,

    /// Plugin state
    pub state: PluginState,

    /// Load time
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Error information (if failed)
    pub error: Option<String>,
}

/// Plugin state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    /// Not yet loaded
    Registered,

    /// Currently loading
    Loading,

    /// Loaded and active
    Active,

    /// Failed to load
    Failed,

    /// Unloaded
    Unloaded,
}

/// Plugin error
#[derive(Debug, Clone, thiserror::Error)]
pub enum PluginError {
    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin load failed
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    /// Invalid manifest
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// Version mismatch
    #[error("Version mismatch: {0}")]
    VersionMismatch(String),

    /// Dependency not met
    #[error("Dependency not met: {0}")]
    DependencyNotMet(String),

    /// Plugin limit reached
    #[error("Plugin limit reached: max {0}")]
    LimitReached(usize),

    /// Invalid signature
    #[error("Invalid plugin signature")]
    InvalidSignature,

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            search_paths: vec![
                PathBuf::from("/usr/lib/toadstool/plugins"),
                PathBuf::from("/usr/local/lib/toadstool/plugins"),
                PathBuf::from("./plugins"),
            ],
            config: PluginConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: PluginConfig) -> Self {
        let mut manager = Self::new();
        manager.config = config;
        manager
    }

    /// Register a plugin from manifest
    pub fn register_plugin(&mut self, manifest: PluginManifest) -> Result<(), PluginError> {
        if !self.config.enabled {
            return Err(PluginError::ConfigError(
                "Plugin system disabled".to_string(),
            ));
        }

        // Check limits
        if self.plugins.len() >= self.config.max_plugins {
            return Err(PluginError::LimitReached(self.config.max_plugins));
        }

        // Validate manifest
        self.validate_manifest(&manifest)?;

        // Check dependencies
        self.check_dependencies(&manifest)?;

        let plugin_info = PluginInfo {
            manifest: manifest.clone(),
            state: PluginState::Registered,
            loaded_at: None,
            error: None,
        };

        self.plugins.insert(manifest.name.clone(), plugin_info);

        info!(
            "📦 Registered plugin: {} v{}",
            manifest.name, manifest.version
        );

        Ok(())
    }

    /// Load a plugin
    pub fn load_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // Update state
        plugin.state = PluginState::Loading;

        // In a real implementation, this would:
        // 1. Load the dynamic library
        // 2. Verify signature (if required)
        // 3. Call plugin init function
        // 4. Register plugin's providers/handlers

        // For now, simulate successful load
        plugin.state = PluginState::Active;
        plugin.loaded_at = Some(chrono::Utc::now());

        info!("✅ Loaded plugin: {}", name);

        Ok(())
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        // In a real implementation, this would:
        // 1. Call plugin cleanup function
        // 2. Unregister providers/handlers
        // 3. Unload dynamic library

        plugin.state = PluginState::Unloaded;

        info!("🔄 Unloaded plugin: {}", name);

        Ok(())
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get plugin info
    pub fn get_plugin_info(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// Get active plugins
    pub fn active_plugins(&self) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|(_, info)| info.state == PluginState::Active)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get plugins by type
    pub fn plugins_by_type(&self, plugin_type: &str) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|(_, info)| info.manifest.plugin_type == plugin_type)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Validate plugin manifest
    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), PluginError> {
        if manifest.name.is_empty() {
            return Err(PluginError::InvalidManifest("name is required".to_string()));
        }

        if manifest.version.is_empty() {
            return Err(PluginError::InvalidManifest(
                "version is required".to_string(),
            ));
        }

        if manifest.plugin_type.is_empty() {
            return Err(PluginError::InvalidManifest(
                "plugin_type is required".to_string(),
            ));
        }

        if manifest.entry_point.is_empty() {
            return Err(PluginError::InvalidManifest(
                "entry_point is required".to_string(),
            ));
        }

        // Check if plugin already registered
        if self.plugins.contains_key(&manifest.name) {
            warn!("Plugin {} already registered, will replace", manifest.name);
        }

        Ok(())
    }

    /// Check plugin dependencies
    fn check_dependencies(&self, manifest: &PluginManifest) -> Result<(), PluginError> {
        for dep in &manifest.dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(PluginError::DependencyNotMet(format!(
                    "Plugin {} requires {}",
                    manifest.name, dep
                )));
            }
        }

        Ok(())
    }

    /// Add search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        debug!("Added plugin search path: {:?}", path);
        self.search_paths.push(path);
    }

    /// Get search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Discover plugins in search paths
    ///
    /// Scans each search path for plugin manifests (`plugin.json`) and parses them.
    /// Supports both flat directories and nested plugin directories.
    ///
    /// # Discovery Strategy
    ///
    /// For each search path, looks for:
    /// 1. Direct `plugin.json` in the search path
    /// 2. Subdirectories containing `plugin.json` (plugin bundles)
    ///
    /// # Example Directory Structure
    ///
    /// ```text
    /// /usr/share/toadstool/plugins/
    /// ├── aws-provider/
    /// │   ├── plugin.json
    /// │   └── libaws_provider.so
    /// ├── gcp-provider/
    /// │   ├── plugin.json
    /// │   └── libgcp_provider.so
    /// └── local-storage/
    ///     ├── plugin.json
    ///     └── liblocal_storage.so
    /// ```
    pub fn discover_plugins(&mut self) -> Vec<PluginManifest> {
        let mut discovered = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                debug!("Plugin search path does not exist: {:?}", search_path);
                continue;
            }

            debug!("Scanning for plugins in: {:?}", search_path);

            // Check for direct plugin.json in search path
            let direct_manifest = search_path.join("plugin.json");
            if direct_manifest.exists() {
                if let Some(manifest) = self.parse_manifest(&direct_manifest) {
                    discovered.push(manifest);
                }
            }

            // Scan subdirectories for plugin bundles
            if let Ok(entries) = fs::read_dir(search_path) {
                for entry in entries.filter_map(Result::ok) {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        let manifest_path = entry_path.join("plugin.json");
                        if manifest_path.exists() {
                            if let Some(manifest) = self.parse_manifest(&manifest_path) {
                                discovered.push(manifest);
                            }
                        }
                    }
                }
            }
        }

        info!("Discovered {} plugins", discovered.len());
        discovered
    }

    /// Parse a plugin manifest file
    fn parse_manifest(&self, path: &std::path::Path) -> Option<PluginManifest> {
        match fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str::<PluginManifest>(&contents) {
                Ok(manifest) => {
                    debug!(
                        "Parsed plugin manifest: {} v{}",
                        manifest.name, manifest.version
                    );
                    Some(manifest)
                }
                Err(e) => {
                    warn!("Failed to parse plugin manifest {:?}: {}", path, e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read plugin manifest {:?}: {}", path, e);
                None
            }
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin capability interface
///
/// Plugins can implement this to expose their capabilities.
pub trait PluginCapability: Send + Sync {
    /// Get capability name
    fn capability_name(&self) -> &str;

    /// Get capability version
    fn capability_version(&self) -> &str;

    /// Initialize the capability
    fn initialize(&mut self) -> Result<(), String>;

    /// Cleanup the capability
    fn cleanup(&mut self) -> Result<(), String>;
}

/// Plugin registry for specific types
///
/// Manages plugins of a specific type (e.g., cloud providers).
pub struct TypedPluginRegistry<T> {
    plugins: HashMap<String, T>,
}

impl<T> TypedPluginRegistry<T> {
    /// Create a new typed registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, name: impl Into<String>, plugin: T) {
        self.plugins.insert(name.into(), plugin);
    }

    /// Get plugin by name
    pub fn get(&self, name: &str) -> Option<&T> {
        self.plugins.get(name)
    }

    /// Get mutable plugin by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        self.plugins.get_mut(name)
    }

    /// List available plugins
    ///
    /// Returns plugin names as string slices to avoid unnecessary cloning.
    /// Callsites that need owned strings can collect: `list().map(String::from)`
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }

    /// Check if plugin exists
    pub fn has(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
}

impl<T> Default for TypedPluginRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: format!("lib{}.so", name),
            ..Default::default()
        }
    }

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert_eq!(manager.list_plugins().len(), 0);
    }

    #[test]
    fn test_register_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        let result = manager.register_plugin(manifest);
        assert!(result.is_ok());
        assert_eq!(manager.list_plugins().len(), 1);
    }

    #[test]
    fn test_load_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager.register_plugin(manifest).unwrap();
        let result = manager.load_plugin("test-plugin");
        assert!(result.is_ok());

        let active = manager.active_plugins();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_unload_plugin() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager.register_plugin(manifest).unwrap();
        manager.load_plugin("test-plugin").unwrap();

        let result = manager.unload_plugin("test-plugin");
        assert!(result.is_ok());

        let active = manager.active_plugins();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_plugin_dependencies() {
        let mut manager = PluginManager::new();

        let dep_manifest = create_test_manifest("dependency");
        manager.register_plugin(dep_manifest).unwrap();

        let mut main_manifest = create_test_manifest("main-plugin");
        main_manifest.dependencies = vec!["dependency".to_string()];

        let result = manager.register_plugin(main_manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_dependency() {
        let mut manager = PluginManager::new();

        let mut manifest = create_test_manifest("main-plugin");
        manifest.dependencies = vec!["missing-dep".to_string()];

        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_manifest() {
        let mut manager = PluginManager::new();

        let manifest = PluginManifest {
            name: "".to_string(), // Invalid: empty name
            version: "1.0.0".to_string(),
            plugin_type: "test".to_string(),
            entry_point: "lib.so".to_string(),
            ..Default::default()
        };

        let result = manager.register_plugin(manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_limit() {
        let config = PluginConfig {
            max_plugins: 2,
            ..Default::default()
        };

        let mut manager = PluginManager::with_config(config);

        manager
            .register_plugin(create_test_manifest("plugin1"))
            .unwrap();
        manager
            .register_plugin(create_test_manifest("plugin2"))
            .unwrap();

        let result = manager.register_plugin(create_test_manifest("plugin3"));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugins_by_type() {
        let mut manager = PluginManager::new();

        let mut manifest1 = create_test_manifest("provider1");
        manifest1.plugin_type = "cloud_provider".to_string();

        let mut manifest2 = create_test_manifest("provider2");
        manifest2.plugin_type = "cloud_provider".to_string();

        let mut manifest3 = create_test_manifest("storage1");
        manifest3.plugin_type = "storage".to_string();

        manager.register_plugin(manifest1).unwrap();
        manager.register_plugin(manifest2).unwrap();
        manager.register_plugin(manifest3).unwrap();

        let cloud_plugins = manager.plugins_by_type("cloud_provider");
        assert_eq!(cloud_plugins.len(), 2);

        let storage_plugins = manager.plugins_by_type("storage");
        assert_eq!(storage_plugins.len(), 1);
    }

    #[test]
    fn test_typed_registry() {
        let mut registry: TypedPluginRegistry<i32> = TypedPluginRegistry::new();

        registry.register("test1".to_string(), 42);
        registry.register("test2".to_string(), 100);

        assert_eq!(registry.list().len(), 2);
        assert_eq!(*registry.get("test1").unwrap(), 42);
        assert!(registry.has("test2"));
    }

    #[test]
    fn test_plugin_state_transitions() {
        let mut manager = PluginManager::new();
        let manifest = create_test_manifest("test-plugin");

        manager.register_plugin(manifest).unwrap();

        let info = manager.get_plugin_info("test-plugin").unwrap();
        assert_eq!(info.state, PluginState::Registered);

        manager.load_plugin("test-plugin").unwrap();
        let info = manager.get_plugin_info("test-plugin").unwrap();
        assert_eq!(info.state, PluginState::Active);

        manager.unload_plugin("test-plugin").unwrap();
        let info = manager.get_plugin_info("test-plugin").unwrap();
        assert_eq!(info.state, PluginState::Unloaded);
    }
}
