// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery engine configuration

use std::time::Duration;

/// Service discovery engine configuration
#[derive(Debug, Clone)]
pub struct ServiceDiscoveryConfig {
    /// Enable caching of discovered services.
    pub enable_cache: bool,

    /// Cache TTL.
    pub cache_ttl: Duration,

    /// Default discovery timeout.
    pub default_timeout: Duration,

    /// Number of retry attempts.
    pub retry_attempts: u32,

    /// Retry delay.
    pub retry_delay: Duration,
}

impl Default for ServiceDiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_ttl: Duration::from_secs(300),
            default_timeout: Duration::from_secs(30),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = ServiceDiscoveryConfig::default();
        assert!(config.enable_cache);
        assert_eq!(config.cache_ttl, Duration::from_secs(300));
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
    }

    #[test]
    fn config_is_cloneable() {
        let config = ServiceDiscoveryConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.retry_attempts, config.retry_attempts);
    }

    #[test]
    fn config_is_debuggable() {
        let config = ServiceDiscoveryConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("enable_cache"));
        assert!(debug.contains("cache_ttl"));
    }

    #[test]
    fn config_fields_are_mutable() {
        let config = ServiceDiscoveryConfig {
            enable_cache: false,
            retry_attempts: 10,
            cache_ttl: Duration::from_secs(60),
            ..Default::default()
        };
        assert!(!config.enable_cache);
        assert_eq!(config.retry_attempts, 10);
        assert_eq!(config.cache_ttl, Duration::from_secs(60));
    }
}
