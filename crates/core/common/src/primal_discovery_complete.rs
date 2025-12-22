//! Complete Primal Discovery with mDNS Integration
//!
//! This module completes the capability-based discovery system by wiring up
//! mDNS service discovery with the PrimalDiscovery interface.
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
    #[allow(dead_code)] // Reserved for future cache expiration logic
    discovered_at: Instant,
    #[allow(dead_code)] // Reserved for future health check logic
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
        
        let require_mdns = std::env::var("TOADSTOOL_MDNS_REQUIRE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        
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
    /// Get default fallbacks from environment or use defaults
    fn default_fallbacks() -> HashMap<String, String> {
        let mut fallbacks = HashMap::new();
        
        let bind_host = std::env::var("TOADSTOOL_BIND_HOST")
            .or_else(|_| std::env::var("BIND_HOST"))
            .unwrap_or_else(|_| "localhost".to_string());
        
        // Only add fallbacks if explicitly configured or in development mode
        if std::env::var("TOADSTOOL_DISCOVERY_FALLBACKS").is_ok() 
            || std::env::var("TOADSTOOL_ENV").map(|e| e == "development").unwrap_or(false) {
            
            // Use environment-specific endpoints if available
            if let Ok(url) = std::env::var("SONGBIRD_URL") {
                fallbacks.insert("orchestration".to_string(), url);
                fallbacks.insert("coordination".to_string(), fallbacks["orchestration"].clone());
            } else {
                fallbacks.insert("orchestration".to_string(), format!("http://{bind_host}:8081"));
                fallbacks.insert("coordination".to_string(), format!("http://{bind_host}:8081"));
            }
            
            if let Ok(url) = std::env::var("BEARDOG_URL") {
                fallbacks.insert("security".to_string(), url);
                fallbacks.insert("authentication".to_string(), fallbacks["security"].clone());
            } else {
                fallbacks.insert("security".to_string(), format!("http://{bind_host}:8082"));
                fallbacks.insert("authentication".to_string(), format!("http://{bind_host}:8082"));
            }
            
            if let Ok(url) = std::env::var("NESTGATE_URL") {
                fallbacks.insert("storage".to_string(), url);
            } else {
                fallbacks.insert("storage".to_string(), format!("http://{bind_host}:8083"));
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
    pub async fn new(mdns_client: Option<Arc<dyn DiscoveryClient + Send + Sync>>) -> ToadStoolResult<Self> {
        Self::with_config(mdns_client, DiscoveryConfig::default()).await
    }
    
    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid or mDNS client is required but missing
    pub async fn with_config(
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
            info!("⚠️  mDNS unavailable, using {} configured fallbacks", config.fallbacks.len());
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
    /// ```rust,no_run
    /// # use toadstool_common::primal_discovery_complete::PrimalDiscoveryEngine;
    /// # use toadstool_common::primal_identity::Capability;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = PrimalDiscoveryEngine::new(None).await?;
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
    pub async fn discover_by_capability(&self, capability: &Capability) -> ToadStoolResult<Vec<DiscoveredService>> {
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
                        info!("✅ mDNS discovered {} services for {}", services.len(), capability_str);
                        
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
            info!("⚡ Using configured fallback for {}: {}", capability_str, url);
            
            let service = Self::create_fallback_service(&capability_str, url, capability);
            self.cache_service(&capability_str, service.clone()).await;
            
            return Ok(vec![service]);
        }
        
        // 4. Not found
        Err(ToadStoolError::not_found(format!(
            "No service found with capability: {}",
            capability_str
        )))
    }
    
    /// Convert capability to string key for caching/fallback lookup
    fn capability_to_string(capability: &Capability) -> String {
        match capability {
            Capability::Coordination(_) => "orchestration".to_string(),
            Capability::Compute(_) => "compute".to_string(),
            Capability::Storage(_) => "storage".to_string(),
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
                    (&host_port[..idx], host_port[idx+1..].parse::<u16>().unwrap_or(8080))
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
                // Fallback
                crate::primal_identity::ServiceEndpoint {
                    protocol: "http".to_string(),
                    address: "localhost".to_string(),
                    port: 8080,
                    path: Some("/".to_string()),
                    metadata: HashMap::new(),
                }
            }
        } else if let Ok(parsed_addr) = url.parse::<std::net::SocketAddr>() {
            // Handle "localhost:8080" format
            crate::primal_identity::ServiceEndpoint {
                protocol: "http".to_string(),
                address: parsed_addr.ip().to_string(),
                port: parsed_addr.port(),
                path: Some("/".to_string()),
                metadata: HashMap::new(),
            }
        } else {
            // Fallback for any other format - create basic HTTP endpoint
            crate::primal_identity::ServiceEndpoint {
                protocol: "http".to_string(),
                address: "localhost".to_string(),
                port: 8080,
                path: Some("/".to_string()),
                metadata: HashMap::new(),
            }
        };
        
        DiscoveredService {
            id: Some(format!("{}-fallback", capability_str)),
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
        
        self.cache.write().await.insert(capability.to_string(), cached);
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
        let fresh = cache.values().filter(|e| e.is_fresh(self.config.cache_ttl)).count();
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primal_identity::CoordinationCapability;
    
    #[tokio::test]
    async fn test_discovery_with_fallback() {
        let mut config = DiscoveryConfig::default();
        config.fallbacks.insert("orchestration".to_string(), "http://localhost:9999".to_string());
        config.enable_mdns = false; // Disable mDNS for this test
        
        let engine = PrimalDiscoveryEngine::with_config(None, config).await.expect("Failed to create engine");
        
        let capability = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
        let result = engine.discover_by_capability(&capability).await;
        
        assert!(result.is_ok(), "Should find orchestration via fallback");
        let services = result.expect("Services should be present");
        assert!(!services.is_empty(), "Should have at least one service");
        assert_eq!(services[0].endpoints[0].port, 9999);
    }
    
    #[tokio::test]
    async fn test_cache_freshness() {
        let engine = PrimalDiscoveryEngine::new(None).await.expect("Failed to create engine");
        
        let service = DiscoveredService {
            id: Some("test-service".to_string()),
            capabilities: vec![Capability::Coordination(CoordinationCapability::ServiceDiscovery)],
            endpoints: vec![crate::primal_identity::ServiceEndpoint {
                protocol: "http".to_string(),
                address: "localhost".to_string(),
                port: 8080,
                path: Some("/".to_string()),
                metadata: HashMap::new(),
            }],
            healthy: true,
            metadata: HashMap::new(),
        };
        
        engine.cache_service("orchestration", service.clone()).await;
        
        // Should be in cache
        let cached = engine.get_from_cache("orchestration").await;
        assert!(cached.is_some(), "Service should be cached");
        assert!(cached.unwrap().is_fresh(Duration::from_secs(300)), "Service should be fresh");
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let engine = PrimalDiscoveryEngine::new(None).await.expect("Failed to create engine");
        
        let stats = engine.cache_stats().await;
        assert_eq!(stats.total_entries, 0, "Cache should be empty initially");
        
        let service = DiscoveredService {
            id: Some("test-service".to_string()),
            capabilities: vec![],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };
        
        engine.cache_service("test", service).await;
        
        let stats = engine.cache_stats().await;
        assert_eq!(stats.total_entries, 1, "Cache should have one entry");
        assert_eq!(stats.fresh_entries, 1, "Entry should be fresh");
    }
}

