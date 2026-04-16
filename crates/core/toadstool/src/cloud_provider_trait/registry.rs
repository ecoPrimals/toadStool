// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime registry of [`super::CloudProvider`] implementations.

use std::collections::HashMap;

use super::provider::CloudProvider;

/// Cloud provider registry
///
/// Maintains a registry of available cloud providers.
pub struct CloudProviderRegistry<P: CloudProvider> {
    providers: HashMap<String, Box<P>>,
}

impl<P: CloudProvider> CloudProviderRegistry<P> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Box<P>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get provider by name
    pub fn get(&self, name: &str) -> Option<&P> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Get all provider names
    pub fn available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Check if provider is available
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

impl<P: CloudProvider> Default for CloudProviderRegistry<P> {
    fn default() -> Self {
        Self::new()
    }
}
