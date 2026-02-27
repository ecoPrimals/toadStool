// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

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
    fn initialize(&mut self) -> Result<(), String>;

    /// Cleanup the capability
    fn cleanup(&mut self) -> Result<(), String>;
}
