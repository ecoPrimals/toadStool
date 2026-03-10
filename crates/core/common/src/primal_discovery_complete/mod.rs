// SPDX-License-Identifier: AGPL-3.0-only
//! Complete Primal Discovery with mDNS Integration
//!
//! This module completes the capability-based discovery system by wiring up
//! mDNS service discovery with the `PrimalDiscovery` interface.
//!
//! # Philosophy
//!
//! - **Each primal knows only itself** - No hardcoded knowledge of other services
//! - **Discover by capability** - Find services by WHAT they do, not WHO they are
//! - **Runtime resolution** - All dependencies discovered at runtime
//! - **Graceful degradation** - Falls back to configuration when mDNS unavailable
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │      Application Layer              │
//! │  (Needs "orchestration" capability) │
//! └──────────────┬──────────────────────┘
//!                │
//!                ▼
//! ┌─────────────────────────────────────┐
//! │    PrimalDiscoveryEngine            │
//! │  - Capability-based interface       │
//! │  - Caching & health checking        │
//! │  - Fallback handling                │
//! └──────────────┬──────────────────────┘
//!                │
//!       ┌────────┴────────┐
//!       ▼                 ▼
//! ┌──────────┐      ┌──────────────┐
//! │   mDNS   │      │ Config       │
//! │ Discovery│      │ Fallbacks    │
//! └──────────┘      └──────────────┘
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::primal_identity::{Capability, DiscoveredService};
use crate::runtime_discovery::DiscoveryClient;
use crate::{ToadStoolError, ToadStoolResult};

#[cfg(test)]
mod tests;

/// Complete primal discovery engine with mDNS integration
pub struct PrimalDiscoveryEngine {
    /// Primary discovery backend (mDNS)
    mdns_client: Option<Arc<dyn DiscoveryClient + Send + Sync>>,

    /// Discovered endpoints cache
    cache: Arc<RwLock<HashMap<String, CachedEndpoint>>>,

    /// Configuration
    config: Arc<DiscoveryConfig>,
}

/// Cached endpoint with freshness tracking
#[derive(Clone, Debug)]
struct CachedEndpoint {
    service: DiscoveredService,
    #[allow(dead_code, reason = "timestamped for future cache expiration logic")]
    discovered_at: Instant,
    last_checked: Instant,
}

impl CachedEndpoint {
    fn is_fresh(&self, ttl: Duration) -> bool {
        self.last_checked.elapsed() < ttl
    }
}

/// Discovery configuration
#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    /// Cache TTL
    pub cache_ttl: Duration,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Configured fallback endpoints by capability
    pub fallbacks: HashMap<String, String>,

    /// Enable mDNS discovery
    pub enable_mdns: bool,

    /// Fail fast if mDNS unavailable (vs falling back)
    pub require_mdns: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        // Environment-aware defaults
        let enable_mdns = std::env::var("TOADSTOOL_MDNS_ENABLE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let require_mdns =
            std::env::var("TOADSTOOL_MDNS_REQUIRE").is_ok_and(|v| v == "true" || v == "1");

        Self {
            cache_ttl: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            fallbacks: Self::default_fallbacks(),
            enable_mdns,
            require_mdns,
        }
    }
}

impl DiscoveryConfig {
    /// Build fallbacks from environment. Fallbacks are only populated when
    /// `TOADSTOOL_DISCOVERY_FALLBACKS` is set or `TOADSTOOL_ENV=development`.
    ///
    /// Each capability URL is resolved from env vars, falling back to
    /// `{bind_host}:{port}` using `toadstool_config::ports::capability_fallback`.
    fn default_fallbacks() -> HashMap<String, String> {
        const COORDINATION_PORT: u16 = 8080;
        const SECURITY_PORT: u16 = 8081;
        const STORAGE_PORT: u16 = 8082;

        let dev_mode = std::env::var("TOADSTOOL_DISCOVERY_FALLBACKS").is_ok()
            || std::env::var("TOADSTOOL_ENV").is_ok_and(|e| e == "development");

        if !dev_mode {
            return HashMap::new();
        }

        let bind_host = std::env::var("TOADSTOOL_BIND_HOST")
            .or_else(|_| std::env::var("BIND_HOST"))
            .unwrap_or_else(|_| crate::constants::network::DEFAULT_HOSTNAME.to_string());

        let specs: &[(&str, &str, u16, &[&str])] = &[
            (
                "TOADSTOOL_COORDINATION_URL",
                "SONGBIRD_URL",
                COORDINATION_PORT,
                &["orchestration", "coordination"],
            ),
            (
                "TOADSTOOL_SECURITY_URL",
                "BEARDOG_URL",
                SECURITY_PORT,
                &["security", "authentication"],
            ),
            (
                "TOADSTOOL_STORAGE_URL",
                "NESTGATE_URL",
                STORAGE_PORT,
                &["storage"],
            ),
        ];

        let mut fallbacks = HashMap::new();
        for (cap_env, legacy_env, port, keys) in specs {
            let url = std::env::var(cap_env)
                .or_else(|_| std::env::var(legacy_env))
                .unwrap_or_else(|_| format!("http://{bind_host}:{port}"));
            for key in *keys {
                fallbacks.insert((*key).to_string(), url.clone());
            }
        }
        fallbacks
    }
}

impl PrimalDiscoveryEngine {
    /// Create new discovery engine with mDNS client
    ///
    /// # Errors
    ///
    /// Returns error if mDNS client is required but unavailable
    pub fn new(
        mdns_client: Option<Arc<dyn DiscoveryClient + Send + Sync>>,
    ) -> ToadStoolResult<Self> {
        Self::with_config(mdns_client, DiscoveryConfig::default())
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid or mDNS client is required but missing
    pub fn with_config(
        mdns_client: Option<Arc<dyn DiscoveryClient + Send + Sync>>,
        config: DiscoveryConfig,
    ) -> ToadStoolResult<Self> {
        // Validate configuration
        if config.require_mdns && mdns_client.is_none() {
            return Err(ToadStoolError::configuration(
                "mDNS client required but not provided",
            ));
        }

        if config.enable_mdns && mdns_client.is_some() {
            info!("✅ mDNS discovery enabled");
        } else if !config.fallbacks.is_empty() {
            info!(
                "⚠️  mDNS unavailable, using {} configured fallbacks",
                config.fallbacks.len()
            );
        } else {
            warn!("⚠️  No mDNS and no fallbacks configured - discovery may fail");
        }

        Ok(Self {
            mdns_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        })
    }

    /// Discover service by capability (main interface)
    ///
    /// Searches in order:
    /// 1. Cache (if fresh)
    /// 2. mDNS discovery (if enabled)
    /// 3. Configured fallback
    ///
    /// # Errors
    ///
    /// Returns error if no service with the capability is found
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use toadstool_common::primal_discovery_complete::PrimalDiscoveryEngine;
    /// # use toadstool_common::primal_identity::Capability;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = PrimalDiscoveryEngine::new(None)?;
    ///
    /// // Find orchestration service (discovers Songbird if available)
    /// let services = engine.discover_by_capability(&Capability::Coordination(
    ///     toadstool_common::primal_identity::CoordinationCapability::ServiceDiscovery
    /// )).await?;
    ///
    /// for service in services {
    ///     println!("Found: {:?}", service.endpoints);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_by_capability(
        &self,
        capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        let capability_str = Self::capability_to_string(capability);

        // 1. Check cache
        if let Some(cached) = self.get_from_cache(&capability_str).await {
            if cached.is_fresh(self.config.cache_ttl) {
                debug!("✅ Cache hit for capability: {}", capability_str);
                return Ok(vec![cached.service]);
            }
        }

        // 2. Try mDNS discovery
        if self.config.enable_mdns {
            if let Some(ref mdns) = self.mdns_client {
                debug!("🔍 Querying mDNS for capability: {}", capability_str);

                match mdns.discover_by_capability(capability).await {
                    Ok(services) if !services.is_empty() => {
                        info!(
                            "✅ mDNS discovered {} services for {}",
                            services.len(),
                            capability_str
                        );

                        // Cache all discovered services
                        for service in &services {
                            self.cache_service(&capability_str, service.clone()).await;
                        }

                        return Ok(services);
                    }
                    Ok(_) => {
                        debug!("mDNS query returned no results for {}", capability_str);
                    }
                    Err(e) => {
                        warn!("mDNS query failed for {}: {}", capability_str, e);
                    }
                }
            }
        }

        // 3. Try configured fallback
        if let Some(url) = self.config.fallbacks.get(&capability_str) {
            info!(
                "⚡ Using configured fallback for {}: {}",
                capability_str, url
            );

            let service = Self::create_fallback_service(&capability_str, url, capability);
            self.cache_service(&capability_str, service.clone()).await;

            return Ok(vec![service]);
        }

        // 4. Not found
        Err(ToadStoolError::not_found(format!(
            "No service found with capability: {capability_str}"
        )))
    }

    /// Convert capability to string key for caching/fallback lookup
    fn capability_to_string(capability: &Capability) -> String {
        match capability {
            Capability::Coordination(_) => "orchestration".to_string(),
            Capability::Compute(_) => "compute".to_string(),
            Capability::Storage(_) => "storage".to_string(),
            Capability::Crypto(_) => "crypto".to_string(),
            Capability::Authentication(_) => "authentication".to_string(),
            Capability::Discovery(_) => "discovery".to_string(),
            Capability::Custom { name, .. } => name.clone(),
        }
    }

    /// Create fallback service from configured URL
    fn create_fallback_service(
        capability_str: &str,
        url: &str,
        capability: &Capability,
    ) -> DiscoveredService {
        // Parse URL to extract host and port
        // Note: For simple parsing, we'll use string operations instead of url crate
        let endpoint = if url.starts_with("http://") || url.starts_with("https://") {
            // Simple URL parsing without external crate
            let parts: Vec<&str> = url.split("://").collect();
            if parts.len() == 2 {
                let protocol = parts[0];
                let remainder = parts[1];
                let (host_port, path) = if let Some(idx) = remainder.find('/') {
                    (&remainder[..idx], &remainder[idx..])
                } else {
                    (remainder, "/")
                };

                let (host, port) = if let Some(idx) = host_port.find(':') {
                    (
                        &host_port[..idx],
                        host_port[idx + 1..]
                            .parse::<u16>()
                            .unwrap_or(crate::constants::network::DEFAULT_HTTP_PORT),
                    )
                } else {
                    (host_port, if protocol == "https" { 443 } else { 80 })
                };

                crate::primal_identity::ServiceEndpoint {
                    protocol: protocol.to_string(),
                    address: host.to_string(),
                    port,
                    path: Some(path.to_string()),
                    metadata: HashMap::new(),
                }
            } else {
                // Fallback when URL parsing fails - use configurable localhost constant
                crate::primal_identity::ServiceEndpoint {
                    protocol: "http".to_string(),
                    address: crate::constants::network::LOCALHOST_IPV4.to_string(),
                    port: crate::constants::network::DEFAULT_HTTP_PORT,
                    path: Some("/".to_string()),
                    metadata: HashMap::new(),
                }
            }
        } else if let Ok(parsed_addr) = url.parse::<std::net::SocketAddr>() {
            // Handle "localhost:PORT" format
            crate::primal_identity::ServiceEndpoint {
                protocol: "http".to_string(),
                address: parsed_addr.ip().to_string(),
                port: parsed_addr.port(),
                path: Some("/".to_string()),
                metadata: HashMap::new(),
            }
        } else {
            // Fallback for any other format - use configurable localhost constant
            crate::primal_identity::ServiceEndpoint {
                protocol: "http".to_string(),
                address: crate::constants::network::LOCALHOST_IPV4.to_string(),
                port: crate::constants::network::DEFAULT_HTTP_PORT,
                path: Some("/".to_string()),
                metadata: HashMap::new(),
            }
        };

        DiscoveredService {
            id: Some(format!("{capability_str}-fallback")),
            capabilities: vec![capability.clone()],
            endpoints: vec![endpoint],
            healthy: true, // Assume healthy until health check proves otherwise
            metadata: {
                let mut map = HashMap::new();
                map.insert("source".to_string(), "configuration".to_string());
                map
            },
        }
    }

    /// Get service from cache
    async fn get_from_cache(&self, capability: &str) -> Option<CachedEndpoint> {
        self.cache.read().await.get(capability).cloned()
    }

    /// Cache discovered service
    async fn cache_service(&self, capability: &str, service: DiscoveredService) {
        let now = Instant::now();
        let cached = CachedEndpoint {
            service,
            discovered_at: now,
            last_checked: now,
        };

        self.cache
            .write()
            .await
            .insert(capability.to_string(), cached);
    }

    /// Clear cache (useful for testing or forced refresh)
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
        debug!("Discovery cache cleared");
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let fresh = cache
            .values()
            .filter(|e| e.is_fresh(self.config.cache_ttl))
            .count();

        CacheStats {
            total_entries: total,
            fresh_entries: fresh,
            stale_entries: total - fresh,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub fresh_entries: usize,
    pub stale_entries: usize,
}
