// SPDX-License-Identifier: AGPL-3.0-or-later
//! Memory layout and peripheral managers
//!
//! This module contains managers for memory layouts and peripheral configurations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::configs::embedded::PeripheralConfig;
use crate::{LegacyArchitecture, MemoryLayout};

use super::types::*;

/// Memory layout manager
#[derive(Debug)]
pub struct MemoryLayoutManager {
    /// Memory layouts per architecture
    layouts: HashMap<LegacyArchitecture, MemoryLayout>,
}

/// Peripheral manager
#[derive(Debug)]
pub struct PeripheralManager {
    /// Peripheral configurations
    peripherals: HashMap<String, PeripheralConfig>,
    /// Active peripheral instances
    _active_peripherals: Arc<RwLock<HashMap<String, Box<dyn PeripheralInterface>>>>,
}

impl MemoryLayoutManager {
    /// Create a new memory layout manager
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
        }
    }

    /// Add a memory layout for an architecture
    pub fn add_layout(&mut self, architecture: LegacyArchitecture, layout: MemoryLayout) {
        self.layouts.insert(architecture, layout);
    }

    /// Get a memory layout for an architecture
    pub fn get_layout(&self, architecture: &LegacyArchitecture) -> Option<&MemoryLayout> {
        self.layouts.get(architecture)
    }

    /// Remove a memory layout for an architecture
    pub fn remove_layout(&mut self, architecture: &LegacyArchitecture) -> Option<MemoryLayout> {
        self.layouts.remove(architecture)
    }

    /// Check if a layout exists for an architecture
    pub fn has_layout(&self, architecture: &LegacyArchitecture) -> bool {
        self.layouts.contains_key(architecture)
    }
}

impl PeripheralManager {
    /// Create a new peripheral manager
    pub fn new() -> Self {
        Self {
            peripherals: HashMap::new(),
            _active_peripherals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a peripheral configuration
    pub fn add_peripheral(&mut self, name: impl Into<String>, config: PeripheralConfig) {
        self.peripherals.insert(name.into(), config);
    }

    /// Get a peripheral configuration
    pub fn get_peripheral(&self, name: &str) -> Option<&PeripheralConfig> {
        self.peripherals.get(name)
    }

    /// Remove a peripheral configuration
    pub fn remove_peripheral(&mut self, name: &str) -> Option<PeripheralConfig> {
        self.peripherals.remove(name)
    }

    /// Check if a peripheral exists
    pub fn has_peripheral(&self, name: &str) -> bool {
        self.peripherals.contains_key(name)
    }

    /// Get all peripheral names
    pub fn peripheral_names(&self) -> Vec<String> {
        self.peripherals.keys().cloned().collect()
    }
}

impl Default for MemoryLayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PeripheralManager {
    fn default() -> Self {
        Self::new()
    }
}
