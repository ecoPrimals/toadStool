// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Typed plugin registry.

use std::collections::HashMap;

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
