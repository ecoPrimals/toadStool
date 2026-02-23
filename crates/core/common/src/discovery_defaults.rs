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
//! ## Evolution History
//!
//! - **Feb 14, 2026**: Removed hardcoded localhost URLs. All fallbacks now require
//!   explicit environment variable configuration (self-knowledge principle).
//! - **Phase 3/4 (Complete)**: mDNS/DNS-SD discovery fully deployed via
//!   `MdnsDiscoveryService`, `primal_discovery_mdns`, and `discovery_engine`.
//!   This module retained for development/test graceful degradation only.

// EVOLVED (Feb 14, 2026): Removed unused imports after localhost fallback removal
// - DEFAULT_HTTP_PORT, primals, runtime_ports no longer needed
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
    pub const fn production() -> Self {
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
    pub const fn development() -> Self {
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
    pub const fn test() -> Self {
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
/// **Note**: Production environments should use capability-based discovery
/// via mDNS/DNS-SD (fully deployed as of Phase 4, Feb 2026).
/// These fallbacks exist only for development/test graceful degradation.
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
    /// Get fallback URL for a service from environment only
    ///
    /// Returns None if:
    /// - Fallbacks are disabled (production mode)
    /// - No environment variable is set for the service
    ///
    /// **EVOLVED (Feb 14, 2026)**: Removed hardcoded localhost URLs.
    /// Primal self-knowledge principle: a primal cannot assume localhost endpoints exist.
    /// All fallback URLs must come from explicit environment configuration.
    ///
    /// Environment variable format: `{SERVICE_TYPE}_URL` (e.g., `REDIS_URL`, `TOADSTOOL_URL`)
    #[must_use]
    pub fn get_fallback_url(&self, service_type: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // EVOLVED: Only read from environment variables
        // No hardcoded localhost - violates self-knowledge principle
        env::var(format!("{}_URL", service_type.to_uppercase())).ok()
    }

    /// Check if fallbacks should be used
    #[must_use]
    pub const fn should_use_fallback(&self) -> bool {
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

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        // EVOLVED (Feb 14, 2026): Fallbacks now require explicit env vars
        // No hardcoded localhost - self-knowledge principle
        let fallbacks = LocalhostFallbacks { enabled: true };

        // Without env vars set, fallback returns None
        // (unless env var is set by another test)
        let unknown_url = fallbacks.get_fallback_url("unknown_service_xyz");
        assert_eq!(unknown_url, None);
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

    #[test]
    fn test_discovery_config_default_production_mode() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "production");

        let config = DiscoveryConfig::default();
        assert!(!config.enable_localhost_fallback);
        assert!(!config.allow_insecure);

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_ENV", p);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_localhost_fallbacks_default_production_mode() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "production");

        let fallbacks = LocalhostFallbacks::default();
        assert!(!fallbacks.enabled);

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_ENV", p);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_discovery_error_strategy_default_production_mode() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "production");

        let strategy = DiscoveryErrorStrategy::default();
        assert_eq!(strategy, DiscoveryErrorStrategy::FailFast);

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_ENV", p);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_get_fallback_url_env_override() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("REDIS_URL").ok();
        env::set_var("REDIS_URL", "redis://custom.example.com:6380");

        let fallbacks = LocalhostFallbacks { enabled: true };
        let url = fallbacks.get_fallback_url("redis");
        assert_eq!(url.as_deref(), Some("redis://custom.example.com:6380"));

        if let Some(p) = prev {
            env::set_var("REDIS_URL", p);
        } else {
            env::remove_var("REDIS_URL");
        }
    }

    #[test]
    fn test_get_fallback_url_toadstool_env_override() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_URL").ok();
        env::set_var("TOADSTOOL_URL", "http://toadstool.local:9999");

        let fallbacks = LocalhostFallbacks { enabled: true };
        let url = fallbacks.get_fallback_url("toadstool");
        assert_eq!(url.as_deref(), Some("http://toadstool.local:9999"));

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_URL", p);
        } else {
            env::remove_var("TOADSTOOL_URL");
        }
    }

    #[test]
    fn test_get_fallback_url_postgres_env_override() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("POSTGRES_URL").ok();
        env::set_var("POSTGRES_URL", "postgresql://db.example.com:5433");

        let fallbacks = LocalhostFallbacks { enabled: true };
        let url = fallbacks.get_fallback_url("postgres");
        assert_eq!(url.as_deref(), Some("postgresql://db.example.com:5433"));

        if let Some(p) = prev {
            env::set_var("POSTGRES_URL", p);
        } else {
            env::remove_var("POSTGRES_URL");
        }
    }

    #[test]
    fn test_get_fallback_url_mongodb_env_override() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("MONGODB_URL").ok();
        env::set_var("MONGODB_URL", "mongodb://mongo.example.com:27018");

        let fallbacks = LocalhostFallbacks { enabled: true };
        let url = fallbacks.get_fallback_url("mongodb");
        assert_eq!(url.as_deref(), Some("mongodb://mongo.example.com:27018"));

        if let Some(p) = prev {
            env::set_var("MONGODB_URL", p);
        } else {
            env::remove_var("MONGODB_URL");
        }
    }

    #[test]
    fn test_discovery_config_timeouts() {
        let config = DiscoveryConfig::production();
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.cache_ttl, Duration::from_secs(300));

        let config = DiscoveryConfig::development();
        assert_eq!(config.timeout, Duration::from_secs(2));
        assert_eq!(config.max_retries, 1);
        assert_eq!(config.cache_ttl, Duration::from_secs(10));

        let config = DiscoveryConfig::test();
        assert_eq!(config.timeout, Duration::from_millis(100));
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.cache_ttl, Duration::from_secs(1));
    }

    #[test]
    fn test_should_use_fallback_when_enabled() {
        let fallbacks = LocalhostFallbacks { enabled: true };
        assert!(fallbacks.should_use_fallback());
    }

    #[test]
    fn test_discovery_error_strategy_variants() {
        assert_eq!(
            DiscoveryErrorStrategy::FailFast,
            DiscoveryErrorStrategy::FailFast
        );
        assert_eq!(
            DiscoveryErrorStrategy::TryFallback,
            DiscoveryErrorStrategy::TryFallback
        );
        assert_eq!(
            DiscoveryErrorStrategy::SilentFallback,
            DiscoveryErrorStrategy::SilentFallback
        );
        assert_ne!(
            DiscoveryErrorStrategy::FailFast,
            DiscoveryErrorStrategy::TryFallback
        );
    }

    #[test]
    fn test_discovery_config_default_staging_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "staging");

        let config = DiscoveryConfig::default();
        assert!(config.enable_localhost_fallback);
        assert!(config.allow_insecure);

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_ENV", p);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_discovery_config_default_empty_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("TOADSTOOL_ENV").ok();
        env::remove_var("TOADSTOOL_ENV");

        let config = DiscoveryConfig::default();
        assert!(config.enable_localhost_fallback);

        if let Some(p) = prev {
            env::set_var("TOADSTOOL_ENV", p);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_get_fallback_url_service_type_case_insensitive() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let prev = env::var("REDIS_URL").ok();
        env::set_var("REDIS_URL", "redis://custom:6379");

        let fallbacks = LocalhostFallbacks { enabled: true };
        assert_eq!(
            fallbacks.get_fallback_url("redis").as_deref(),
            Some("redis://custom:6379")
        );
        assert_eq!(
            fallbacks.get_fallback_url("Redis").as_deref(),
            Some("redis://custom:6379")
        );

        if let Some(p) = prev {
            env::set_var("REDIS_URL", p);
        } else {
            env::remove_var("REDIS_URL");
        }
    }

    #[test]
    fn test_discovery_config_debug() {
        let config = DiscoveryConfig::production();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("DiscoveryConfig"));
        assert!(debug_str.contains("enable_localhost_fallback"));
    }

    #[test]
    fn test_localhost_fallbacks_debug() {
        let fallbacks = LocalhostFallbacks { enabled: true };
        let debug_str = format!("{:?}", fallbacks);
        assert!(debug_str.contains("LocalhostFallbacks"));
        assert!(debug_str.contains("enabled"));
    }

    #[test]
    fn test_discovery_config_clone() {
        let config = DiscoveryConfig::production();
        let cloned = config.clone();
        assert_eq!(
            config.enable_localhost_fallback,
            cloned.enable_localhost_fallback
        );
        assert_eq!(config.timeout, cloned.timeout);
    }
}
