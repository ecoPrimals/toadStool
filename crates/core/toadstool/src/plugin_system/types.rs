// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin system type definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin manifest
    pub manifest: PluginManifest,

    /// Plugin state
    pub state: PluginState,

    /// Load time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub loaded_at: Option<std::time::SystemTime>,

    /// Error information (if failed)
    pub error: Option<String>,
}

/// Plugin state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
