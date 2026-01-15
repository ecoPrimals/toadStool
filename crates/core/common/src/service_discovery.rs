//! Service Discovery - Capability-Based Runtime Discovery
//!
//! This module implements the infant discovery pattern where services are
//! discovered at runtime based on their capabilities, not hardcoded names.
//!
//! ## Philosophy
//!
//! - **Zero Hardcoding**: No primal names in code
//! - **Capability Matching**: Find services by what they do, not who they are
//! - **Runtime Discovery**: Services announce themselves and are discovered
//! - **Self-Knowledge Only**: Each primal knows only itself
//!
//! ## Usage
//!
//! ```rust,no_run
//! use toadstool_common::service_discovery::{ServiceDiscovery, DiscoveryMethod};
//! use toadstool_common::primal_identity::{Capability, CoordinationCapability};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize discovery
//! let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await?;
//!
//! // Find a service by capability (not by name!)
//! let coordinator = discovery
//!     .find_service_by_capability(
//!         Capability::Coordination(CoordinationCapability::ServiceDiscovery)
//!     )
//!     .await?;
//!
//! // Connect to discovered service
//! if let Some(endpoint) = coordinator.primary_endpoint() {
//!     println!("Coordinator at: {}", endpoint.url());
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::discovery_defaults::{DiscoveryConfig, LocalhostFallbacks};
use crate::primal_identity::{Capability, PrimalIdentity, ServiceEndpoint};

/// Service discovery error types
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("No services found with capability: {capability:?}")]
    NoServiceFound { capability: Capability },

    #[error("Discovery timeout after {duration:?}")]
    Timeout { duration: Duration },

    #[error("Discovery method unavailable: {method}")]
    MethodUnavailable { method: String },

    #[error("Invalid service response: {reason}")]
    InvalidResponse { reason: String },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        source: std::io::Error,
    },
}

pub type DiscoveryResult<T> = Result<T, DiscoveryError>;

/// Discovered service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Service unique identifier
    pub id: String,

    /// Service name (for logging/debugging only, not for matching)
    pub name: String,

    /// Service version
    pub version: String,

    /// Capabilities this service provides
    pub capabilities: Vec<Capability>,

    /// Available endpoints
    pub endpoints: Vec<ServiceEndpoint>,

    /// Service metadata
    pub metadata: HashMap<String, String>,

    /// When this service was discovered
    pub discovered_at: SystemTime,

    /// Last health check time
    pub last_seen: SystemTime,

    /// Health status
    pub healthy: bool,
}

impl DiscoveredService {
    /// Check if service has a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get primary endpoint (first healthy one)
    pub fn primary_endpoint(&self) -> Option<&ServiceEndpoint> {
        self.endpoints.first()
    }

    /// Get all healthy endpoints
    pub fn healthy_endpoints(&self) -> Vec<&ServiceEndpoint> {
        // For now, return all endpoints
        // Future: track endpoint health separately
        self.endpoints.iter().collect()
    }

    /// Check if service is still fresh (within TTL)
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.last_seen
            .elapsed()
            .map(|elapsed| elapsed < ttl)
            .unwrap_or(false)
    }
}

/// Discovery method types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Automatic discovery using available methods
    Auto,

    /// mDNS/DNS-SD discovery
    Mdns,

    /// Environment variables (TOADSTOOL_SERVICE_*)
    Environment,

    /// Configuration file
    ConfigFile { path: String },

    /// Registry service (Consul, etcd, etc.)
    Registry { endpoint: String },

    /// Multiple methods in priority order
    Multi(Vec<DiscoveryMethod>),
}

/// Service discovery trait
#[async_trait]
pub trait ServiceDiscoveryTrait: Send + Sync {
    /// Discover services by capability
    async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> DiscoveryResult<Vec<DiscoveredService>>;

    /// Discover all available services
    async fn discover_all(&self) -> DiscoveryResult<Vec<DiscoveredService>>;

    /// Register this service for discovery by others
    async fn announce_self(&self, identity: &dyn PrimalIdentity) -> DiscoveryResult<()>;

    /// Refresh discovery cache
    async fn refresh(&self) -> DiscoveryResult<()>;
}

/// Main service discovery implementation
pub struct ServiceDiscovery {
    config: DiscoveryConfig,
    method: DiscoveryMethod,
    cache: Arc<RwLock<HashMap<String, DiscoveredService>>>,
    fallbacks: LocalhostFallbacks,
}

impl ServiceDiscovery {
    /// Create new service discovery
    pub async fn new(method: DiscoveryMethod) -> DiscoveryResult<Self> {
        let config = DiscoveryConfig::default();
        let fallbacks = LocalhostFallbacks::default();

        let discovery = Self {
            config,
            method,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallbacks,
        };

        // Initial discovery
        if let Err(e) = discovery.refresh().await {
            warn!("Initial discovery failed: {}", e);
            // Non-fatal in development mode
        }

        Ok(discovery)
    }

    /// Create with custom configuration
    pub async fn with_config(
        method: DiscoveryMethod,
        config: DiscoveryConfig,
    ) -> DiscoveryResult<Self> {
        let fallbacks = LocalhostFallbacks::default();

        let discovery = Self {
            config,
            method,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallbacks,
        };

        discovery.refresh().await?;

        Ok(discovery)
    }

    /// Find a single service by capability (convenience method)
    pub async fn find_service_by_capability(
        &self,
        capability: Capability,
    ) -> DiscoveryResult<DiscoveredService> {
        let services = self.find_services_by_capability(&capability).await?;

        // First try to find a healthy service
        if let Some(service) = services.iter().find(|s| s.healthy).cloned() {
            return Ok(service);
        }

        // Fall back to any service
        services
            .into_iter()
            .next()
            .ok_or(DiscoveryError::NoServiceFound { capability })
    }

    /// Discover services using configured method
    async fn discover_via_method(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let services = match &self.method {
            DiscoveryMethod::Auto => self.discover_auto().await,
            DiscoveryMethod::Environment => self.discover_from_env().await,
            DiscoveryMethod::Mdns => self.discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => self.discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => self.discover_from_registry(endpoint).await,
            DiscoveryMethod::Multi(methods) => self.discover_multi(methods).await,
        }?;

        Ok(services)
    }

    /// Auto discovery - try methods in order
    async fn discover_auto(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let methods = vec![
            DiscoveryMethod::Environment,
            DiscoveryMethod::Mdns,
            // Add more methods as implemented
        ];

        self.discover_multi(&methods).await
    }

    /// Discover from multiple methods
    async fn discover_multi(
        &self,
        methods: &[DiscoveryMethod],
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut all_services = Vec::new();
        let mut successful = false;

        for method in methods {
            match self.discover_specific_method(method).await {
                Ok(services) => {
                    debug!("Discovered {} services via {:?}", services.len(), method);
                    all_services.extend(services);
                    successful = true;
                }
                Err(e) => {
                    debug!("Discovery via {:?} failed: {}", method, e);
                }
            }
        }

        if !successful && all_services.is_empty() {
            // Try fallbacks if configured
            if self.fallbacks.should_use_fallback() {
                info!("Using localhost fallbacks for development");
                return self.discover_from_fallbacks().await;
            }
        }

        Ok(all_services)
    }

    /// Discover from specific method
    async fn discover_specific_method(
        &self,
        method: &DiscoveryMethod,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        match method {
            DiscoveryMethod::Environment => self.discover_from_env().await,
            DiscoveryMethod::Mdns => self.discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => self.discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => self.discover_from_registry(endpoint).await,
            _ => Ok(Vec::new()),
        }
    }

    /// Discover from environment variables
    async fn discover_from_env(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Look for environment variables like:
        // TOADSTOOL_SERVICE_COORDINATOR_URL=http://localhost:5000
        // TOADSTOOL_SERVICE_COORDINATOR_CAPABILITIES=coordination,discovery

        for (key, value) in std::env::vars() {
            if key.starts_with("TOADSTOOL_SERVICE_") && key.ends_with("_URL") {
                // Extract service name
                let service_name = key
                    .strip_prefix("TOADSTOOL_SERVICE_")
                    .and_then(|s| s.strip_suffix("_URL"))
                    .unwrap_or("unknown"); // Safe: Always provides fallback

                // Get capabilities from companion variable
                let cap_key = format!("TOADSTOOL_SERVICE_{}_CAPABILITIES", service_name);
                let capabilities_str = std::env::var(&cap_key).unwrap_or_default(); // Safe: Empty string on missing var

                let service = DiscoveredService {
                    id: format!("env-{}", service_name.to_lowercase()),
                    name: service_name.to_lowercase(),
                    version: "unknown".to_string(),
                    capabilities: self.parse_capabilities(&capabilities_str),
                    endpoints: vec![ServiceEndpoint::from_url_string(&value)?],
                    metadata: HashMap::new(),
                    discovered_at: SystemTime::now(),
                    last_seen: SystemTime::now(),
                    healthy: true,
                };

                debug!("Discovered service from environment: {}", service.name);
                services.push(service);
            }
        }

        Ok(services)
    }

    /// Discover via mDNS (placeholder - needs implementation)
    async fn discover_via_mdns(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        // TODO(future): Implement mDNS discovery
        // This requires the mdns crate and proper service announcement
        debug!("mDNS discovery not yet implemented");
        Ok(Vec::new())
    }

    /// Discover from configuration file
    async fn discover_from_config(&self, _path: &str) -> DiscoveryResult<Vec<DiscoveredService>> {
        // TODO(future): Implement config file discovery
        debug!("Config file discovery not yet implemented");
        Ok(Vec::new())
    }

    /// Discover from service registry
    async fn discover_from_registry(
        &self,
        _endpoint: &str,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        // TODO(future): Implement registry discovery (Consul, etcd, etc.)
        debug!("Registry discovery not yet implemented");
        Ok(Vec::new())
    }

    /// Discover from localhost fallbacks (development only)
    async fn discover_from_fallbacks(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Only in development
        if !self.fallbacks.should_use_fallback() {
            return Ok(services);
        }

        info!("Using localhost fallbacks for development");

        // ToadStool itself
        if let Some(url) = self.fallbacks.get_fallback_url("toadstool") {
            services.push(DiscoveredService {
                id: "fallback-toadstool".to_string(),
                name: "toadstool".to_string(),
                version: "dev".to_string(),
                capabilities: vec![Capability::Compute(
                    crate::primal_identity::ComputeCapability::NativeExecution,
                )],
                endpoints: vec![ServiceEndpoint::from_url_string(&url)?],
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source".to_string(), "fallback".to_string());
                    meta
                },
                discovered_at: SystemTime::now(),
                last_seen: SystemTime::now(),
                healthy: true,
            });
        }

        Ok(services)
    }

    /// Parse capability strings
    fn parse_capabilities(&self, capabilities_str: &str) -> Vec<Capability> {
        // Simple parsing for now
        // Format: "coordination,storage,compute"
        capabilities_str
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                match s {
                    "coordination" => Some(Capability::Coordination(
                        crate::primal_identity::CoordinationCapability::ServiceDiscovery,
                    )),
                    "storage" => Some(Capability::Storage(
                        crate::primal_identity::StorageCapability::ObjectStorage,
                    )),
                    "compute" => Some(Capability::Compute(
                        crate::primal_identity::ComputeCapability::NativeExecution,
                    )),
                    _ => None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl ServiceDiscoveryTrait for ServiceDiscovery {
    async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            let cached: Vec<DiscoveredService> = cache
                .values()
                .filter(|s| s.has_capability(capability))
                .filter(|s| s.is_fresh(self.config.cache_ttl))
                .cloned()
                .collect();

            if !cached.is_empty() {
                debug!(
                    "Found {} services in cache for capability: {:?}",
                    cached.len(),
                    capability
                );
                return Ok(cached);
            }
        }

        // Discover and cache
        let services = self.discover_via_method().await?;

        {
            let mut cache = self.cache.write().await;
            // ✅ OPTIMIZED: Use Entry API - only clone if entry doesn't exist
            for service in &services {
                cache
                    .entry(service.id.clone())
                    .or_insert_with(|| service.clone());
            }
        }

        // Filter by capability
        let matching: Vec<DiscoveredService> = services
            .into_iter()
            .filter(|s| s.has_capability(capability))
            .collect();

        Ok(matching)
    }

    async fn discover_all(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        self.discover_via_method().await
    }

    async fn announce_self(&self, _identity: &dyn PrimalIdentity) -> DiscoveryResult<()> {
        // TODO(future): Implement service announcement
        // This would announce via mDNS, registry, etc.
        Ok(())
    }

    async fn refresh(&self) -> DiscoveryResult<()> {
        let services = self.discover_via_method().await?;

        let mut cache = self.cache.write().await;
        cache.clear();
        for service in services {
            cache.insert(service.id.clone(), service);
        }

        info!("Discovery cache refreshed: {} services", cache.len());
        Ok(())
    }
}

/// Helper functions for ServiceEndpoint (extends primal_identity::ServiceEndpoint)
impl ServiceEndpoint {
    /// Create endpoint from URL string
    pub fn from_url_string(url: &str) -> DiscoveryResult<Self> {
        // Simple URL parsing
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() != 2 {
            return Err(DiscoveryError::InvalidResponse {
                reason: format!("Invalid URL format: {}", url),
            });
        }

        let protocol = parts[0];
        let rest = parts[1];

        let host_port: Vec<&str> = rest.split(':').collect();
        let address = host_port
            .first()
            .ok_or_else(|| DiscoveryError::InvalidResponse {
                reason: format!("Missing host in URL: {}", url),
            })?;
        let port = host_port.get(1).and_then(|p| p.parse().ok()).unwrap_or(80); // Safe: Default port

        Ok(Self {
            protocol: protocol.to_string(),
            address: (*address).to_string(),
            port,
            path: None,
            metadata: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_discovery_creation() {
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto).await;
        assert!(discovery.is_ok());
    }

    #[tokio::test]
    async fn test_discover_from_env() {
        // Set test environment variables
        std::env::set_var("TOADSTOOL_SERVICE_TEST_URL", "http://localhost:9000");
        std::env::set_var("TOADSTOOL_SERVICE_TEST_CAPABILITIES", "coordination");

        let discovery = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();

        let services = discovery.discover_from_env().await.unwrap();
        assert!(!services.is_empty());
        assert_eq!(services[0].name, "test");

        // Cleanup
        std::env::remove_var("TOADSTOOL_SERVICE_TEST_URL");
        std::env::remove_var("TOADSTOOL_SERVICE_TEST_CAPABILITIES");
    }

    #[tokio::test]
    async fn test_service_endpoint_from_url() {
        let endpoint = ServiceEndpoint::from_url_string("http://localhost:8080").unwrap();
        assert_eq!(endpoint.protocol, "http");
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.port, 8080);
        assert_eq!(endpoint.url(), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_discovered_service_has_capability() {
        let service = DiscoveredService {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Coordination(
                crate::primal_identity::CoordinationCapability::ServiceDiscovery,
            )],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };

        assert!(service.has_capability(&Capability::Coordination(
            crate::primal_identity::CoordinationCapability::ServiceDiscovery
        )));
    }

    #[tokio::test]
    async fn test_service_freshness() {
        let service = DiscoveredService {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };

        // Should be fresh with long TTL
        assert!(service.is_fresh(Duration::from_secs(3600)));
    }

    #[test]
    fn test_parse_capabilities() {
        let discovery = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(ServiceDiscovery::new(DiscoveryMethod::Auto))
            .unwrap();

        let caps = discovery.parse_capabilities("coordination,storage,compute");
        assert_eq!(caps.len(), 3);
    }
}
