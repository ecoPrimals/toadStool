// SPDX-License-Identifier: AGPL-3.0-or-later
//! Feature flags configuration
//!
//! This module contains configuration for feature toggles that control
//! experimental, beta, and optional platform features.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Feature flags configuration
///
/// Controls which features are enabled at runtime. This allows for:
/// - Gradual rollout of new features
/// - A/B testing
/// - Environment-specific feature sets
/// - Experimental feature gating
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "bool fields map directly to hardware flags"
)]
pub struct FeatureFlags {
    /// Enable experimental features (unstable, may change)
    pub enable_experimental: bool,

    /// Enable beta features (stable API, testing phase)
    pub enable_beta: bool,

    /// Enable debug features (development only)
    pub enable_debug: bool,

    /// Enable profiling features
    pub enable_profiling: bool,

    /// Enable distributed mode
    pub enable_distributed: bool,

    /// Enable federation across instances
    pub enable_federation: bool,

    /// Enable `GraphQL` API
    pub enable_graphql: bool,

    /// Enable `OpenAPI` documentation
    pub enable_openapi: bool,

    /// Enable automatic configuration
    pub enable_auto_config: bool,

    /// Enable hot reload of configuration
    pub enable_hot_reload: bool,

    /// Enable live reload of code (development)
    pub enable_live_reload: bool,

    /// Enable watch mode for file changes
    pub enable_watch_mode: bool,

    /// Custom feature flags (extensible)
    pub custom: HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            enable_beta: false,
            enable_debug: cfg!(debug_assertions),
            enable_profiling: false,
            enable_distributed: true,
            enable_federation: true,
            enable_graphql: false,
            enable_openapi: true,
            enable_auto_config: true,
            enable_hot_reload: cfg!(debug_assertions),
            enable_live_reload: cfg!(debug_assertions),
            enable_watch_mode: cfg!(debug_assertions),
            custom: HashMap::new(),
        }
    }
}

impl FeatureFlags {
    /// Check if a custom feature is enabled
    #[must_use]
    pub fn is_custom_enabled(&self, feature_name: &str) -> bool {
        self.custom.get(feature_name).copied().unwrap_or(false)
    }

    /// Enable a custom feature
    pub fn enable_custom(&mut self, feature_name: impl Into<String>) {
        self.custom.insert(feature_name.into(), true);
    }

    /// Disable a custom feature
    pub fn disable_custom(&mut self, feature_name: impl Into<String>) {
        self.custom.insert(feature_name.into(), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_feature_flags() {
        let flags = FeatureFlags::default();
        assert!(!flags.enable_experimental);
        assert!(flags.enable_distributed);
    }

    #[test]
    fn test_custom_features() {
        let mut flags = FeatureFlags::default();
        assert!(!flags.is_custom_enabled("my_feature"));

        flags.enable_custom("my_feature");
        assert!(flags.is_custom_enabled("my_feature"));

        flags.disable_custom("my_feature");
        assert!(!flags.is_custom_enabled("my_feature"));
    }

    #[test]
    fn test_debug_features_in_debug_mode() {
        let flags = FeatureFlags::default();
        assert_eq!(flags.enable_debug, cfg!(debug_assertions));
        assert_eq!(flags.enable_hot_reload, cfg!(debug_assertions));
    }
}
