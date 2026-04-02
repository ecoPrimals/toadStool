// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-only

//! Plugin capability traits.

/// Plugin capability interface
///
/// Plugins can implement this to expose their capabilities.
pub trait PluginCapability: Send + Sync {
    /// Get capability name
    fn capability_name(&self) -> &str;

    /// Get capability version
    fn capability_version(&self) -> &str;

    /// Initialize the capability
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails.
    fn initialize(&mut self) -> Result<(), String>;

    /// Cleanup the capability
    ///
    /// # Errors
    ///
    /// Returns error if cleanup fails.
    fn cleanup(&mut self) -> Result<(), String>;
}
