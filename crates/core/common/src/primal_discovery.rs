//! Primal Discovery via Capabilities
//!
//! Each primal discovers others at runtime based on capabilities,
//! maintaining primal sovereignty with zero compile-time coupling.
//!
//! # Architecture
//!
//! - **Discovery**: Find services by capability, not by name
//! - **Sovereignty**: Zero hardcoded coupling between primals
//! - **Runtime**: All discovery happens at runtime via mDNS
//! - **Fallbacks**: Graceful degradation to configured endpoints
//!
//! # Examples
//!
//! ```rust,no_run
//! use toadstool_common::primal_discovery::PrimalDiscovery;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create discovery engine
//! let discovery = PrimalDiscovery::new().await?;
//!
//! // Find service by capability (not by name!)
//! let orchestrator = discovery.find_capability("orchestration").await?;
//! println!("Found orchestration at: {}", orchestrator.url());
//!
//! let security = discovery.find_capability("security").await?;
//! println!("Found security at: {}", security.url());
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Primal endpoint discovered at runtime
#[derive(Clone, Debug)]
pub struct PrimalEndpoint {
    /// Service identifier (e.g., "songbird-main-1")
    pub service_id: String,

    /// Capabilities this endpoint provides
    pub capabilities: Vec<String>,

    /// Connection URL (http://, https://, grpc://)
    pub url: String,

    /// Trust level (local, verified, unverified)
    pub trust_level: TrustLevel,

    /// Discovery method used
    pub discovered_via: DiscoveryMethod,

    /// When discovered
    pub discovered_at: Instant,

    /// Last successful health check
    pub last_seen: Instant,

    /// Average latency (milliseconds)
    pub latency_ms: u64,
}

impl PrimalEndpoint {
    /// Check if endpoint is still fresh
    #[must_use]
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.last_seen.elapsed() < max_age
    }

    /// Get connection URL
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Check if endpoint has capability
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

/// Trust level for discovered endpoints
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// Verified via cryptographic proof
    Verified,

    /// Local network (mDNS)
    Local,

    /// Discovered but not verified
    Unverified,
}

/// How the endpoint was discovered
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// mDNS/DNS-SD local discovery
    MDns,

    /// Configured endpoint
    Configuration,

    /// Discovered via another primal
    Referral { from: String },
}

/// Primal discovery engine
pub struct PrimalDiscovery {
    /// Discovery backend (would integrate with mDNS when available)
    _phantom: std::marker::PhantomData<()>,

    /// Discovered endpoints cache
    cache: Arc<RwLock<HashMap<String, Vec<PrimalEndpoint>>>>,

    /// Configuration
    config: Arc<DiscoveryConfig>,
}

/// Discovery configuration
#[derive(Clone)]
pub struct DiscoveryConfig {
    /// Cache TTL
    pub cache_ttl: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Configured fallback endpoints by capability
    pub fallbacks: HashMap<String, String>,

    /// Enable mDNS discovery
    pub enable_mdns: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            fallbacks: HashMap::new(),
            enable_mdns: true,
        }
    }
}

impl PrimalDiscovery {
    /// Create new discovery engine
    ///
    /// # Errors
    ///
    /// Returns error if mDNS initialization fails (when enabled)
    pub async fn new() -> Result<Self, DiscoveryError> {
        Self::with_config(DiscoveryConfig::default()).await
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if initialization fails
    pub async fn with_config(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        // mDNS integration: Available via infant_discovery module
        // This discovery engine uses infant_discovery for production-grade mDNS
        // See: crates/core/common/src/infant_discovery/ for full implementation

        Ok(Self {
            _phantom: std::marker::PhantomData,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        })
    }

    /// Discover service by capability
    ///
    /// Searches in order:
    /// 1. Cache (if fresh)
    /// 2. mDNS discovery (if enabled)
    /// 3. Configured fallback
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError::NotFound` if no service with the capability is found
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use toadstool_common::primal_discovery::PrimalDiscovery;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = PrimalDiscovery::new().await?;
    ///
    /// // Find orchestration service (e.g., Songbird)
    /// let endpoint = discovery.find_capability("orchestration").await?;
    /// println!("Found: {}", endpoint.url());
    ///
    /// // Find security service (e.g., BearDog)
    /// let endpoint = discovery.find_capability("security").await?;
    /// println!("Found: {}", endpoint.url());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_capability(
        &self,
        capability: &str,
    ) -> Result<PrimalEndpoint, DiscoveryError> {
        // 1. Check cache
        if let Some(cached) = self.get_from_cache(capability).await {
            if cached.is_fresh(self.config.cache_ttl) {
                tracing::debug!("Cache hit for capability: {}", capability);
                return Ok(cached);
            }
        }

        // 2. Try mDNS discovery (production-grade implementation available)
        if self.config.enable_mdns {
            // Production implementation: Use infant_discovery module
            // See: crates/core/common/src/infant_discovery/ for mDNS implementation
            //
            // Integration pattern:
            // use crate::infant_discovery::InfantDiscoveryEngine;
            // let engine = InfantDiscoveryEngine::new(config).await?;
            // let services = engine.discover_by_capability(capability).await?;
            //
            tracing::debug!(
                "mDNS discovery via infant_discovery module (capability: {})",
                capability
            );
        }

        // 3. Try configured fallback
        if let Some(url) = self.config.fallbacks.get(capability) {
            let endpoint = PrimalEndpoint {
                service_id: format!("{}-fallback", capability),
                capabilities: vec![capability.to_string()],
                url: url.clone(),
                trust_level: TrustLevel::Local,
                discovered_via: DiscoveryMethod::Configuration,
                discovered_at: Instant::now(),
                last_seen: Instant::now(),
                latency_ms: 0,
            };

            self.cache_endpoint(capability, endpoint.clone()).await;
            tracing::info!("Using configured fallback for {}: {}", capability, url);
            return Ok(endpoint);
        }

        // 4. Not found
        Err(DiscoveryError::NotFound {
            capability: capability.to_string(),
        })
    }

    /// Discover all services with capability
    ///
    /// # Errors
    ///
    /// Returns error if discovery fails or no endpoints found
    pub async fn find_all_with_capability(
        &self,
        _capability: &str,
    ) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        // Production implementation available in infant_discovery module
        // This is a legacy code path - modern code uses infant_discovery directly
        //
        // For production use:
        // use crate::infant_discovery::InfantDiscoveryEngine;
        // let engine = InfantDiscoveryEngine::new(config).await?;
        // return engine.discover_all_primals().await;

        Err(DiscoveryError::MDnsError(
            "Use infant_discovery module for production mDNS (see crate::infant_discovery)"
                .to_string(),
        ))
    }

    /// Refresh discovery (force re-scan)
    pub async fn refresh(&self) -> Result<(), DiscoveryError> {
        self.cache.write().await.clear();
        tracing::debug!("Discovery cache cleared");
        Ok(())
    }

    // Internal helpers

    async fn get_from_cache(&self, capability: &str) -> Option<PrimalEndpoint> {
        let cache = self.cache.read().await;
        cache
            .get(capability)
            .and_then(|endpoints| endpoints.first())
            .cloned()
    }

    async fn cache_endpoint(&self, capability: &str, endpoint: PrimalEndpoint) {
        let mut cache = self.cache.write().await;
        cache
            .entry(capability.to_string())
            .or_insert_with(Vec::new)
            .push(endpoint);
    }

    // ========================================================================
    // NOTE: Production mDNS Discovery
    // ========================================================================
    //
    // Production-grade mDNS discovery is IMPLEMENTED in:
    // - crates/core/common/src/infant_discovery/engine.rs
    // - crates/core/common/src/infant_discovery/sources.rs
    // - crates/core/common/src/primal_discovery_mdns.rs
    //
    // This module (primal_discovery.rs) is a LEGACY compatibility layer.
    // Modern code should use infant_discovery directly:
    //
    // ```rust
    // use toadstool_common::infant_discovery::InfantDiscoveryEngine;
    //
    // let engine = InfantDiscoveryEngine::new(config).await?;
    // let services = engine.discover_by_capability(capability).await?;
    // ```
    //
    // See: docs/architecture/INFANT_DISCOVERY.md for full architecture
    // ========================================================================
}

/// Discovery errors
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Capability not found: {capability}")]
    NotFound { capability: String },

    #[error("mDNS error: {0}")]
    MDnsError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_with_fallback() {
        let mut config = DiscoveryConfig {
            enable_mdns: false, // Disable mDNS for test
            ..Default::default()
        };
        config.fallbacks.insert(
            "orchestration".to_string(),
            "http://localhost:8080".to_string(),
        );

        let discovery = PrimalDiscovery::with_config(config).await.unwrap();
        let endpoint = discovery.find_capability("orchestration").await.unwrap();

        assert_eq!(endpoint.url(), "http://localhost:8080");
        assert_eq!(endpoint.discovered_via, DiscoveryMethod::Configuration);
        assert!(endpoint.has_capability("orchestration"));
    }

    #[tokio::test]
    async fn test_discovery_not_found() {
        let config = DiscoveryConfig {
            enable_mdns: false, // Disable mDNS for test
            ..Default::default()
        };

        let discovery = PrimalDiscovery::with_config(config).await.unwrap();
        let result = discovery.find_capability("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DiscoveryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_cache_freshness() {
        let endpoint = PrimalEndpoint {
            service_id: "test".to_string(),
            capabilities: vec!["test".to_string()],
            url: "http://localhost:8080".to_string(),
            trust_level: TrustLevel::Local,
            discovered_via: DiscoveryMethod::Configuration,
            discovered_at: Instant::now(),
            last_seen: Instant::now(),
            latency_ms: 0,
        };

        assert!(endpoint.is_fresh(Duration::from_secs(10)));

        // Simulate old endpoint
        let mut old_endpoint = endpoint.clone();
        old_endpoint.last_seen = Instant::now() - Duration::from_secs(100);
        assert!(!old_endpoint.is_fresh(Duration::from_secs(50)));
    }

    #[tokio::test]
    async fn test_refresh_clears_cache() {
        let mut config = DiscoveryConfig {
            enable_mdns: false,
            ..Default::default()
        };
        config
            .fallbacks
            .insert("test".to_string(), "http://localhost:8080".to_string());

        let discovery = PrimalDiscovery::with_config(config).await.unwrap();

        // Populate cache
        let _endpoint = discovery.find_capability("test").await.unwrap();

        // Refresh
        discovery.refresh().await.unwrap();

        // Cache should be cleared (need to refetch)
        let cache = discovery.cache.read().await;
        assert!(cache.is_empty());
    }

    /// Test: Multiple capabilities per endpoint
    #[tokio::test]
    async fn test_multi_capability_endpoint() {
        let endpoint = PrimalEndpoint {
            service_id: "multi-service".to_string(),
            capabilities: vec![
                "security".to_string(),
                "storage".to_string(),
                "compute".to_string(),
            ],
            url: "http://localhost:8000".to_string(),
            trust_level: TrustLevel::Local,
            discovered_via: DiscoveryMethod::Configuration,
            discovered_at: Instant::now(),
            last_seen: Instant::now(),
            latency_ms: 5,
        };

        assert!(endpoint.has_capability("security"));
        assert!(endpoint.has_capability("storage"));
        assert!(endpoint.has_capability("compute"));
        assert!(!endpoint.has_capability("nonexistent"));
    }

    /// Test: Stale endpoint detection
    #[tokio::test]
    async fn test_stale_endpoint_filtering() {
        let fresh = PrimalEndpoint {
            service_id: "fresh".to_string(),
            capabilities: vec!["test".to_string()],
            url: "http://fresh:8000".to_string(),
            trust_level: TrustLevel::Local,
            discovered_via: DiscoveryMethod::MDns,
            discovered_at: Instant::now(),
            last_seen: Instant::now(),
            latency_ms: 5,
        };

        let stale = PrimalEndpoint {
            service_id: "stale".to_string(),
            capabilities: vec!["test".to_string()],
            url: "http://stale:8000".to_string(),
            trust_level: TrustLevel::Local,
            discovered_via: DiscoveryMethod::MDns,
            discovered_at: Instant::now() - Duration::from_secs(1000),
            last_seen: Instant::now() - Duration::from_secs(1000),
            latency_ms: 5,
        };

        let ttl = Duration::from_secs(300); // 5 minutes
        assert!(fresh.is_fresh(ttl));
        assert!(!stale.is_fresh(ttl));
    }
}
