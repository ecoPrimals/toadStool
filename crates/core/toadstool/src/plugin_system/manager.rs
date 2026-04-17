// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin manager - loading, registration, and lifecycle.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::types::{PluginConfig, PluginError, PluginInfo, PluginManifest, PluginState};

#[cfg(feature = "plugin-loading")]
use super::types::PluginId;

#[cfg(feature = "plugin-loading")]
use super::ffi_loader::LoadedPlugin;

/// Plugin manager
///
/// Manages loading, registration, and lifecycle of plugins.
pub struct PluginManager {
    /// Registered plugins
    plugins: HashMap<String, PluginInfo>,

    /// Dynamically loaded libraries (only when `plugin-loading` is enabled).
    #[cfg(feature = "plugin-loading")]
    loaded_plugins: HashMap<PluginId, LoadedPlugin>,

    /// Plugin search paths
    search_paths: Vec<PathBuf>,

    /// Plugin configuration
    config: PluginConfig,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            #[cfg(feature = "plugin-loading")]
            loaded_plugins: HashMap::new(),
            search_paths: vec![
                PathBuf::from("/usr/lib/toadstool/plugins"),
                PathBuf::from("/usr/local/lib/toadstool/plugins"),
                PathBuf::from("./plugins"),
            ],
            config: PluginConfig::default(),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(config: PluginConfig) -> Self {
        let mut manager = Self::new();
        manager.config = config;
        manager
    }

    /// Register a plugin from manifest
    ///
    /// # Errors
    ///
    /// Returns error if plugins are disabled, limits are reached, the manifest is invalid, or dependencies are unsatisfied.
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
    ///
    /// # Errors
    ///
    /// Returns error if the plugin is not registered.
    pub fn load_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        if !self.plugins.contains_key(name) {
            return Err(PluginError::NotFound(name.to_string()));
        }

        #[cfg(feature = "plugin-loading")]
        let native: Result<(PathBuf, LoadedPlugin), PluginError> = {
            let manifest = &self.plugins[name].manifest;
            let path = self.resolve_plugin_library(manifest)?;
            LoadedPlugin::load(&path, name).map(|loaded| (path, loaded))
        };

        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        plugin.state = PluginState::Loading;
        plugin.error = None;

        #[cfg(feature = "plugin-loading")]
        {
            match native {
                Ok((path, loaded)) => {
                    self.loaded_plugins.insert(name.to_string(), loaded);
                    plugin.state = PluginState::Active;
                    plugin.loaded_at = Some(std::time::SystemTime::now());
                    info!(
                        "✅ Loaded plugin (native): {} from {}",
                        name,
                        path.display()
                    );
                }
                Err(e) => {
                    plugin.state = PluginState::Failed;
                    plugin.error = Some(e.to_string());
                    return Err(e);
                }
            }
        }

        #[cfg(not(feature = "plugin-loading"))]
        {
            warn!(
                "plugin-loading feature disabled: simulated load for `{}` (no dlopen)",
                name
            );
            plugin.state = PluginState::Active;
            plugin.loaded_at = Some(std::time::SystemTime::now());
            info!("✅ Loaded plugin (simulated): {}", name);
        }

        Ok(())
    }

    /// Unload a plugin
    ///
    /// # Errors
    ///
    /// Returns error if the plugin is not registered.
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        #[cfg(feature = "plugin-loading")]
        {
            if let Some(mut loaded) = self.loaded_plugins.remove(name) {
                loaded.unload();
            }
        }

        #[cfg(not(feature = "plugin-loading"))]
        {
            warn!(
                "plugin-loading feature disabled: simulated unload for `{}` (no dlclose)",
                name
            );
        }

        plugin.state = PluginState::Unloaded;
        plugin.loaded_at = None;

        info!("🔄 Unloaded plugin: {}", name);

        Ok(())
    }

    /// List all plugins
    pub fn list_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }

    /// Get plugin info
    pub fn get_plugin_info(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// Get active plugins
    pub fn active_plugins(&self) -> Vec<&str> {
        self.plugins
            .iter()
            .filter(|(_, info)| info.state == PluginState::Active)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get plugins by type
    pub fn plugins_by_type(&self, plugin_type: &str) -> Vec<&str> {
        self.plugins
            .iter()
            .filter(|(_, info)| info.manifest.plugin_type == plugin_type)
            .map(|(name, _)| name.as_str())
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

    /// Resolve `manifest.entry_point` to an existing file under [`Self::search_paths`].
    #[cfg(feature = "plugin-loading")]
    fn resolve_plugin_library(&self, manifest: &PluginManifest) -> Result<PathBuf, PluginError> {
        for base in &self.search_paths {
            let candidates = [
                base.join(&manifest.entry_point),
                base.join(&manifest.name).join(&manifest.entry_point),
            ];
            for p in candidates {
                if p.exists() && p.is_file() {
                    return Ok(p);
                }
            }
        }
        Err(PluginError::LoadFailed(format!(
            "could not find plugin library `{}` for plugin `{}` in search paths",
            manifest.entry_point, manifest.name
        )))
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
    fn parse_manifest(&self, path: &Path) -> Option<PluginManifest> {
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
