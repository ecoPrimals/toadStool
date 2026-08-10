// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

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

pub mod abi;

mod manager;
mod registry;
mod traits;
mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_advanced;

// Re-exports for backward compatibility
pub use manager::PluginManager;
pub use registry::TypedPluginRegistry;
pub use traits::PluginCapability;
pub use types::{PluginConfig, PluginError, PluginId, PluginInfo, PluginManifest, PluginState};
