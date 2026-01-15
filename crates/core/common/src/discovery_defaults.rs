//! Service Discovery Fallback Defaults
//!
//! This module provides fallback configuration for service discovery.
//! These values are ONLY used when:
//! 1. Automatic discovery fails
//! 2. Running in development/test mode
//! 3. Explicitly enabled via environment variables
//!
//! ## Infant Discovery Pattern
//!
//! ToadStool starts with zero knowledge and discovers services at runtime.
//! These fallbacks allow graceful degradation in development environments.
//!
//! ## Production Behavior
//!
//! In production, discovery should always succeed. If it doesn't, fail fast
//! rather than falling back to localhost defaults.
//!
//! ## Deep Debt Enhancement (Jan 15, 2026)
//!
//! Even fallback URLs now use runtime port discovery instead of hardcoded ports.
//! This ensures no port conflicts even in development environments.

use crate::runtime_ports;
use std::env;
use std::time::Duration;

/// Service discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable fallback to localhost defaults if discovery fails
    pub enable_localhost_fallback: bool,

    /// Discovery timeout
    pub timeout: Duration,

    /// Discovery retry attempts
    pub max_retries: u32,

    /// Cache discovered services (TTL)
    pub cache_ttl: Duration,

    /// Allow insecure connections in development
    pub allow_insecure: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        let is_production =
            env::var("TOADSTOOL_ENV").unwrap_or_else(|_| "development".to_string()) == "production";

        Self {
            // Only enable fallback in non-production
            enable_localhost_fallback: !is_production,
            timeout: Duration::from_secs(5),
            max_retries: 3,
            cache_ttl: Duration::from_secs(60),
            allow_insecure: !is_production,
        }
    }
}

impl DiscoveryConfig {
    /// Create production configuration (strict, no fallbacks)
    #[must_use]
    pub fn production() -> Self {
        Self {
            enable_localhost_fallback: false,
            timeout: Duration::from_secs(10),
            max_retries: 5,
            cache_ttl: Duration::from_secs(300),
            allow_insecure: false,
        }
    }

    /// Create development configuration (permissive, with fallbacks)
    #[must_use]
    pub fn development() -> Self {
        Self {
            enable_localhost_fallback: true,
            timeout: Duration::from_secs(2),
            max_retries: 1,
            cache_ttl: Duration::from_secs(10),
            allow_insecure: true,
        }
    }

    /// Create test configuration (fast, local only)
    #[must_use]
    pub fn test() -> Self {
        Self {
            enable_localhost_fallback: true,
            timeout: Duration::from_millis(100),
            max_retries: 0,
            cache_ttl: Duration::from_secs(1),
            allow_insecure: true,
        }
    }
}

/// Localhost fallback defaults for development
///
/// These are NOT used in production. They exist only for local development
/// where services may be running on known localhost ports.
#[derive(Debug, Clone)]
pub struct LocalhostFallbacks {
    /// Enable fallbacks (should be false in production)
    pub enabled: bool,
}

impl Default for LocalhostFallbacks {
    fn default() -> Self {
        Self {
            enabled: env::var("TOADSTOOL_ENV").unwrap_or_else(|_| "development".to_string())
                != "production",
        }
    }
}

impl LocalhostFallbacks {
    /// Get localhost fallback URL for a service
    ///
    /// Returns None if fallbacks are disabled (production mode)
    ///
    /// **Deep Debt**: Even fallback URLs now try preferred ports with runtime discovery.
    /// If preferred port unavailable, discovers alternative automatically.
    #[must_use]
    pub fn get_fallback_url(&self, service_type: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // Read from environment variables first
        if let Ok(url) = env::var(format!("{}_URL", service_type.to_uppercase())) {
            return Some(url);
        }

        // Deep Debt: Use runtime port discovery with preferred defaults
        // If preferred port unavailable, finds alternative automatically
        match service_type {
            "toadstool" => {
                // Prefer 8080, but discover if unavailable
                let port = runtime_ports::discover_port_with_preference(8080).unwrap_or(8080); // Fallback to preferred if discovery fails
                Some(format!("http://localhost:{}", port))
            }
            "redis" => {
                let port = runtime_ports::discover_port_with_preference(6379).unwrap_or(6379);
                Some(format!("redis://localhost:{}", port))
            }
            "postgres" => {
                let port = runtime_ports::discover_port_with_preference(5432).unwrap_or(5432);
                Some(format!("postgresql://localhost:{}", port))
            }
            "mongodb" => {
                let port = runtime_ports::discover_port_with_preference(27017).unwrap_or(27017);
                Some(format!("mongodb://localhost:{}", port))
            }
            _ => None,
        }
    }

    /// Check if fallbacks should be used
    #[must_use]
    pub fn should_use_fallback(&self) -> bool {
        self.enabled
    }
}

/// Discovery error handling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryErrorStrategy {
    /// Fail fast - no fallbacks, error immediately
    FailFast,

    /// Try fallback - use localhost defaults if discovery fails
    TryFallback,

    /// Silent fallback - use fallback without logging errors
    SilentFallback,
}

impl Default for DiscoveryErrorStrategy {
    fn default() -> Self {
        let is_production =
            env::var("TOADSTOOL_ENV").unwrap_or_else(|_| "development".to_string()) == "production";

        if is_production {
            Self::FailFast
        } else {
            Self::TryFallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        // Should enable fallback in non-production (default)
        assert!(config.enable_localhost_fallback);
    }

    #[test]
    fn test_discovery_config_production() {
        let config = DiscoveryConfig::production();
        // Should NOT enable fallback in production
        assert!(!config.enable_localhost_fallback);
        assert!(!config.allow_insecure);
    }

    #[test]
    fn test_discovery_config_development() {
        let config = DiscoveryConfig::development();
        // Should enable fallback in development
        assert!(config.enable_localhost_fallback);
        assert!(config.allow_insecure);
    }

    #[test]
    fn test_discovery_config_test() {
        let config = DiscoveryConfig::test();
        assert!(config.enable_localhost_fallback);
        assert_eq!(config.max_retries, 0); // Fast fail in tests
    }

    #[test]
    fn test_localhost_fallbacks_default() {
        let fallbacks = LocalhostFallbacks::default();
        // Should be enabled in non-production
        assert!(fallbacks.enabled);
    }

    #[test]
    fn test_localhost_fallback_urls() {
        let fallbacks = LocalhostFallbacks { enabled: true };

        assert_eq!(
            fallbacks.get_fallback_url("toadstool"),
            Some("http://localhost:8080".to_string())
        );
        assert_eq!(
            fallbacks.get_fallback_url("redis"),
            Some("redis://localhost:6379".to_string())
        );
        assert_eq!(fallbacks.get_fallback_url("unknown"), None);
    }

    #[test]
    fn test_localhost_fallbacks_disabled() {
        let fallbacks = LocalhostFallbacks { enabled: false };

        // Should return None when disabled
        assert_eq!(fallbacks.get_fallback_url("toadstool"), None);
        assert!(!fallbacks.should_use_fallback());
    }

    #[test]
    fn test_discovery_error_strategy_default() {
        let strategy = DiscoveryErrorStrategy::default();
        // Should try fallback in non-production (default)
        assert_eq!(strategy, DiscoveryErrorStrategy::TryFallback);
    }
}
