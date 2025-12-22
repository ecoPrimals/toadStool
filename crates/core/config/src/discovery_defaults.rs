//! # Discovery-Based Configuration Defaults
//!
//! EVOLVED: Configuration that uses runtime discovery instead of hardcoded addresses.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge**: Know what YOU can do
//! - **Runtime Discovery**: Find others by capability at runtime
//! - **No Hardcoding**: Zero hardcoded primal addresses
//! - **Capability-Based**: Discover by WHAT you need, not WHO
//!
//! ## Evolution Strategy
//!
//! Instead of:
//! ```ignore
//! const SONGBIRD_ENDPOINT: &str = "http://localhost:9080";  // ❌ Hardcoded
//! ```
//!
//! Use:
//! ```ignore
//! // Discover songbird by capability
//! let songbird = discovery.find_by_capability("message-routing").await?;
//! ```

use std::time::Duration;

/// Discovery configuration with sensible defaults
#[derive(Debug, Clone)]
pub struct DiscoveryDefaults {
    /// How long to wait for service discovery
    pub discovery_timeout: Duration,

    /// How often to refresh service cache
    pub refresh_interval: Duration,

    /// How long before cached services are considered stale
    pub cache_ttl: Duration,

    /// Maximum number of retry attempts for discovery
    pub max_retries: u32,

    /// Delay between retry attempts
    pub retry_delay: Duration,
}

impl Default for DiscoveryDefaults {
    fn default() -> Self {
        Self {
            discovery_timeout: Duration::from_secs(5),
            refresh_interval: Duration::from_secs(30),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Capability names for ecoPrimals services
///
/// These are WHAT services can do, not WHO they are
pub mod capabilities {
    /// Message routing and delivery
    pub const MESSAGE_ROUTING: &str = "message-routing";

    /// Coordination and consensus
    pub const COORDINATION: &str = "coordination";

    /// Object storage
    pub const STORAGE: &str = "storage";

    /// Compute orchestration
    pub const COMPUTE: &str = "compute";

    /// Identity and authentication
    pub const IDENTITY: &str = "identity";

    /// Monitoring and observability
    pub const MONITORING: &str = "monitoring";

    /// Configuration management
    pub const CONFIGURATION: &str = "configuration";

    /// AI/ML orchestration
    pub const AI_ORCHESTRATION: &str = "ai-orchestration";
}

/// Service discovery helper
///
/// EVOLVED: Uses mDNS discovery instead of hardcoded endpoints
pub struct ServiceDiscoveryHelper {
    defaults: DiscoveryDefaults,
}

impl ServiceDiscoveryHelper {
    /// Create a new discovery helper with default settings
    pub fn new() -> Self {
        Self {
            defaults: DiscoveryDefaults::default(),
        }
    }

    /// Create with custom defaults
    pub fn with_defaults(defaults: DiscoveryDefaults) -> Self {
        Self { defaults }
    }

    /// Get discovery timeout
    pub fn discovery_timeout(&self) -> Duration {
        self.defaults.discovery_timeout
    }

    /// Get refresh interval
    pub fn refresh_interval(&self) -> Duration {
        self.defaults.refresh_interval
    }

    /// Get cache TTL
    pub fn cache_ttl(&self) -> Duration {
        self.defaults.cache_ttl
    }
}

impl Default for ServiceDiscoveryHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// Fallback configuration for when discovery is not available
///
/// This should ONLY be used in constrained environments where mDNS is unavailable
#[derive(Debug, Clone)]
pub struct FallbackEndpoints {
    /// Enable fallback to localhost
    pub enable_localhost_fallback: bool,

    /// Base port for localhost services
    pub localhost_base_port: u16,
}

impl Default for FallbackEndpoints {
    fn default() -> Self {
        Self {
            enable_localhost_fallback: true, // Development convenience
            localhost_base_port: 9080,
        }
    }
}

impl FallbackEndpoints {
    /// Get localhost endpoint for a service by offset
    ///
    /// This is a FALLBACK ONLY - prefer discovery
    ///
    /// # Errors
    /// Returns error if localhost fallback is disabled
    pub fn localhost_endpoint(&self, offset: u16) -> Result<String, std::io::Error> {
        if self.enable_localhost_fallback {
            Ok(format!(
                "http://localhost:{}",
                self.localhost_base_port + offset
            ))
        } else {
            // Localhost fallback disabled - return error instead of panic
            tracing::error!("Localhost fallback disabled but endpoint requested");
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Localhost fallback disabled - use discovery instead",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_defaults() {
        let defaults = DiscoveryDefaults::default();
        assert_eq!(defaults.discovery_timeout, Duration::from_secs(5));
        assert_eq!(defaults.refresh_interval, Duration::from_secs(30));
        assert_eq!(defaults.cache_ttl, Duration::from_secs(300));
        assert_eq!(defaults.max_retries, 3);
    }

    #[test]
    fn test_service_discovery_helper() {
        let helper = ServiceDiscoveryHelper::new();
        assert_eq!(helper.discovery_timeout(), Duration::from_secs(5));
        assert_eq!(helper.refresh_interval(), Duration::from_secs(30));
    }

    #[test]
    fn test_capability_constants() {
        // Verify capability names are defined
        assert_eq!(capabilities::MESSAGE_ROUTING, "message-routing");
        assert_eq!(capabilities::COORDINATION, "coordination");
        assert_eq!(capabilities::STORAGE, "storage");
        assert_eq!(capabilities::COMPUTE, "compute");
    }

    #[test]
    fn test_fallback_endpoints() {
        let fallback = FallbackEndpoints::default();
        assert!(fallback.enable_localhost_fallback);

        let endpoint = fallback.localhost_endpoint(0).expect("Should succeed");
        assert_eq!(endpoint, "http://localhost:9080");

        let endpoint2 = fallback.localhost_endpoint(1).expect("Should succeed");
        assert_eq!(endpoint2, "http://localhost:9081");
    }
}
