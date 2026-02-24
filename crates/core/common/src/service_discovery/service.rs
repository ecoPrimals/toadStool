//! Service discovery implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::constants::PRIMAL_NAME;
use crate::discovery_defaults::{DiscoveryConfig, LocalhostFallbacks};
use crate::primal_identity::{
    Capability, CoordinationCapability, PrimalIdentity, ServiceEndpoint, StorageCapability,
};

use super::discovery_config::discover_from_config;
use super::discovery_mdns::discover_via_mdns;
use super::discovery_registry::discover_from_registry;
use super::types::{
    DiscoveredService, DiscoveryError, DiscoveryMethod, DiscoveryResult, ServiceDiscoveryTrait,
};

/// Main service discovery implementation
pub struct ServiceDiscovery {
    pub(crate) config: DiscoveryConfig,
    pub(crate) method: DiscoveryMethod,
    cache: Arc<RwLock<HashMap<String, DiscoveredService>>>,
    fallbacks: LocalhostFallbacks,
}

impl ServiceDiscovery {
    /// Create discovery with specified method
    ///
    /// # Errors
    ///
    /// Never fails; initial refresh failures are logged but not propagated.
    pub async fn new(method: DiscoveryMethod) -> DiscoveryResult<Self> {
        let config = DiscoveryConfig::default();
        let fallbacks = LocalhostFallbacks::default();
        let discovery = Self {
            config,
            method,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallbacks,
        };
        if let Err(e) = discovery.refresh().await {
            warn!("Initial discovery failed: {}", e);
        }
        Ok(discovery)
    }

    /// Create discovery with config
    ///
    /// # Errors
    ///
    /// Returns error if initial discovery refresh fails.
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

    /// Create discovery without initial refresh (for tests with mock servers).
    #[cfg(test)]
    pub(crate) fn new_no_refresh(method: DiscoveryMethod) -> Self {
        let config = DiscoveryConfig::default();
        let fallbacks = LocalhostFallbacks::default();
        Self {
            config,
            method,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fallbacks,
        }
    }

    /// Find a single service by capability
    ///
    /// # Errors
    ///
    /// Returns error if no service with the capability is found.
    pub async fn find_service_by_capability(
        &self,
        capability: Capability,
    ) -> DiscoveryResult<DiscoveredService> {
        let services = self.find_services_by_capability(&capability).await?;
        if let Some(service) = services.iter().find(|s| s.healthy).cloned() {
            return Ok(service);
        }
        services
            .into_iter()
            .next()
            .ok_or(DiscoveryError::NoServiceFound { capability })
    }

    async fn discover_via_method(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let services = match &self.method {
            DiscoveryMethod::Auto => self.discover_auto().await,
            DiscoveryMethod::Environment => self.discover_from_env().await,
            DiscoveryMethod::Mdns => discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => discover_from_registry(endpoint).await,
            DiscoveryMethod::Multi(methods) => self.discover_multi(methods).await,
        }?;
        Ok(services)
    }

    async fn discover_auto(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let methods = vec![DiscoveryMethod::Environment, DiscoveryMethod::Mdns];
        self.discover_multi(&methods).await
    }

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
                Err(e) => debug!("Discovery via {:?} failed: {}", method, e),
            }
        }
        if !successful && all_services.is_empty() && self.fallbacks.should_use_fallback() {
            info!("Using localhost fallbacks for development");
            return self.discover_from_fallbacks();
        }
        Ok(all_services)
    }

    async fn discover_specific_method(
        &self,
        method: &DiscoveryMethod,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        match method {
            DiscoveryMethod::Environment => self.discover_from_env().await,
            DiscoveryMethod::Mdns => discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => discover_from_registry(endpoint).await,
            _ => Ok(Vec::new()),
        }
    }

    /// Discover from environment variables (pub for tests)
    pub async fn discover_from_env(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("TOADSTOOL_SERVICE_") && key.ends_with("_URL") {
                let service_name = key
                    .strip_prefix("TOADSTOOL_SERVICE_")
                    .and_then(|s| s.strip_suffix("_URL"))
                    .unwrap_or("unknown");
                let cap_key = format!("TOADSTOOL_SERVICE_{service_name}_CAPABILITIES");
                let capabilities_str = std::env::var(&cap_key).unwrap_or_default();
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

    fn discover_from_fallbacks(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        if !self.fallbacks.should_use_fallback() {
            return Ok(services);
        }
        info!("Using localhost fallbacks for development");
        if let Some(url) = self.fallbacks.get_fallback_url(PRIMAL_NAME) {
            services.push(DiscoveredService {
                id: format!("fallback-{}", PRIMAL_NAME),
                name: PRIMAL_NAME.to_string(),
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

    pub(crate) fn parse_capabilities(&self, capabilities_str: &str) -> Vec<Capability> {
        capabilities_str
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                match s {
                    "coordination" => Some(Capability::Coordination(
                        CoordinationCapability::ServiceDiscovery,
                    )),
                    "storage" => Some(Capability::Storage(StorageCapability::ObjectStorage)),
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
        let services = self.discover_via_method().await?;
        {
            let mut cache = self.cache.write().await;
            for service in &services {
                cache
                    .entry(service.id.clone())
                    .or_insert_with(|| service.clone());
            }
        }
        Ok(services
            .into_iter()
            .filter(|s| s.has_capability(capability))
            .collect())
    }

    async fn discover_all(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        self.discover_via_method().await
    }

    async fn announce_self(&self, identity: &dyn PrimalIdentity) -> DiscoveryResult<()> {
        debug!(
            "announce_self: {} ({} capabilities) — use MdnsAdapter::announce() \
             from primal_discovery_mdns for full mDNS registration",
            identity.primal_name(),
            identity.capabilities().len()
        );
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::primal_identity::{
        Capability, ComputeCapability, CoordinationCapability, ServiceEndpoint, StorageCapability,
    };

    use super::*;

    #[test]
    fn test_parse_capabilities_empty() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("");
        assert!(caps.is_empty());
    }

    #[test]
    fn test_parse_capabilities_coordination() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("coordination");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        ));
    }

    #[test]
    fn test_parse_capabilities_storage() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("storage");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_parse_capabilities_compute() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("compute");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_parse_capabilities_multiple() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("coordination, storage, compute");
        assert_eq!(caps.len(), 3);
    }

    #[test]
    fn test_parse_capabilities_unknown_filtered() {
        let discovery = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
        let caps = discovery.parse_capabilities("coordination, unknown_cap, storage");
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn test_discovered_service_has_capability() {
        let service = DiscoveredService {
            id: "test-1".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![
                Capability::Compute(ComputeCapability::NativeExecution),
                Capability::Storage(StorageCapability::ObjectStorage),
            ],
            endpoints: vec![],
            metadata: std::collections::HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };
        assert!(service.has_capability(&Capability::Compute(ComputeCapability::NativeExecution)));
        assert!(service.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
        assert!(!service.has_capability(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery
        )));
    }

    #[test]
    fn test_discovered_service_is_fresh() {
        let now = SystemTime::now();
        let service = DiscoveredService {
            id: "test-1".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: std::collections::HashMap::new(),
            discovered_at: now,
            last_seen: now,
            healthy: true,
        };
        assert!(service.is_fresh(Duration::from_secs(60)));
    }

    #[test]
    fn test_discovered_service_is_stale() {
        let old = UNIX_EPOCH;
        let service = DiscoveredService {
            id: "test-1".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: std::collections::HashMap::new(),
            discovered_at: old,
            last_seen: old,
            healthy: true,
        };
        assert!(!service.is_fresh(Duration::from_secs(1)));
    }

    #[test]
    fn test_discovered_service_primary_endpoint() {
        let endpoint =
            ServiceEndpoint::from_url_string("http://localhost:8080").expect("valid url");
        let service = DiscoveredService {
            id: "test-1".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![endpoint.clone()],
            metadata: std::collections::HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };
        assert_eq!(service.primary_endpoint(), Some(&endpoint));
    }

    #[test]
    fn test_discovered_service_healthy_endpoints() {
        let endpoint =
            ServiceEndpoint::from_url_string("http://localhost:8080").expect("valid url");
        let service = DiscoveredService {
            id: "test-1".to_string(),
            name: "test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![endpoint.clone()],
            metadata: std::collections::HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::now(),
            healthy: true,
        };
        let healthy = service.healthy_endpoints();
        assert_eq!(healthy.len(), 1);
    }

    #[test]
    fn test_discovery_method_variants() {
        assert_eq!(DiscoveryMethod::Auto, DiscoveryMethod::Auto);
        assert_eq!(DiscoveryMethod::Mdns, DiscoveryMethod::Mdns);
        assert_eq!(DiscoveryMethod::Environment, DiscoveryMethod::Environment);
        assert_ne!(DiscoveryMethod::Auto, DiscoveryMethod::Mdns);
    }
}
