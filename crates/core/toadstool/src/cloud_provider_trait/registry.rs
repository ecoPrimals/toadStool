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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_provider_trait::provider::NoopCloudProvider;

    #[test]
    fn registry_default_is_empty() {
        let reg = CloudProviderRegistry::<NoopCloudProvider>::default();
        assert!(reg.available_providers().is_empty());
    }

    #[test]
    fn register_and_get_provider() {
        let mut reg = CloudProviderRegistry::new();
        reg.register(Box::new(NoopCloudProvider));
        assert!(reg.has_provider("noop"));
        assert!(reg.get("noop").is_some());
        assert_eq!(reg.get("noop").unwrap().name(), "noop");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let reg = CloudProviderRegistry::<NoopCloudProvider>::new();
        assert!(reg.get("nonexistent").is_none());
        assert!(!reg.has_provider("nonexistent"));
    }

    #[test]
    fn available_providers_lists_all() {
        let mut reg = CloudProviderRegistry::new();
        reg.register(Box::new(NoopCloudProvider));
        let names = reg.available_providers();
        assert_eq!(names, vec!["noop"]);
    }

    #[test]
    fn register_overwrites_existing() {
        let mut reg = CloudProviderRegistry::new();
        reg.register(Box::new(NoopCloudProvider));
        reg.register(Box::new(NoopCloudProvider));
        assert_eq!(reg.available_providers().len(), 1);
    }
}
