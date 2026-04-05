// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service discovery implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

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
    /// Tracks when the cache was last fully refreshed so that
    /// `find_services_by_capability` avoids redundant mDNS rounds
    /// when a recent refresh already returned zero results.
    last_refreshed: Arc<RwLock<Option<Instant>>>,
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
            last_refreshed: Arc::new(RwLock::new(None)),
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
            last_refreshed: Arc::new(RwLock::new(None)),
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
            last_refreshed: Arc::new(RwLock::new(None)),
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
            DiscoveryMethod::Environment => self.discover_from_env(),
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
            info!(
                "Discovery produced no results; trying fallbacks (ecoPrimals runtime sockets, then TCP)"
            );
            return self.discover_from_fallbacks();
        }
        Ok(all_services)
    }

    async fn discover_specific_method(
        &self,
        method: &DiscoveryMethod,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        match method {
            DiscoveryMethod::Environment => self.discover_from_env(),
            DiscoveryMethod::Mdns => discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => discover_from_registry(endpoint).await,
            _ => Ok(Vec::new()),
        }
    }

    /// Discover from environment variables (pub for tests)
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if a service URL cannot be parsed.
    pub fn discover_from_env(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
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
                    capabilities: Self::parse_capabilities(&capabilities_str),
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

        // c) wateringHole: prefer Unix sockets under $XDG_RUNTIME_DIR/ecoPrimals/ before TCP.
        let socket_services = Self::services_from_eco_primals_runtime_sockets();
        if !socket_services.is_empty() {
            info!(
                count = socket_services.len(),
                "Discovered service(s) via ecoPrimals runtime sockets ($XDG_RUNTIME_DIR/ecoPrimals/{{capability}}.sock)"
            );
            return Ok(socket_services);
        }

        // d) Last resort: explicit TCP URL from env (deprecated for inter-primal use).
        if let Some(url) = self.fallbacks.get_fallback_url(PRIMAL_NAME) {
            warn!(
                "TCP URL fallback for {} is deprecated for inter-primal discovery; prefer Unix sockets at $XDG_RUNTIME_DIR/ecoPrimals/{{capability}}.sock (wateringHole), or TOADSTOOL_SERVICE_*_URL endpoints",
                PRIMAL_NAME
            );
            services.push(DiscoveredService {
                id: format!("fallback-{PRIMAL_NAME}"),
                name: PRIMAL_NAME.to_string(),
                version: "dev".to_string(),
                capabilities: vec![Capability::Compute(
                    crate::primal_identity::ComputeCapability::NativeExecution,
                )],
                endpoints: vec![ServiceEndpoint::from_url_string(&url)?],
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source".to_string(), "fallback-tcp".to_string());
                    meta.insert("deprecation".to_string(), "tcp_url_fallback".to_string());
                    meta
                },
                discovered_at: SystemTime::now(),
                last_seen: SystemTime::now(),
                healthy: true,
            });
        }
        Ok(services)
    }

    /// Probe `$XDG_RUNTIME_DIR/ecoPrimals/{capability}.sock` (with TMPDIR/temp fallbacks when
    /// `XDG_RUNTIME_DIR` is unset) and build [`DiscoveredService`] entries for existing paths.
    fn services_from_eco_primals_runtime_sockets() -> Vec<DiscoveredService> {
        const SOCKET_SPECS: &[(&str, Capability)] = &[
            (
                PRIMAL_NAME,
                Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution),
            ),
            (
                "compute",
                Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution),
            ),
            (
                "coordination",
                Capability::Coordination(CoordinationCapability::ServiceDiscovery),
            ),
            (
                "storage",
                Capability::Storage(StorageCapability::ObjectStorage),
            ),
        ];

        let runtime_base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
            .unwrap_or_else(std::env::temp_dir);
        let eco_dir = runtime_base.join("ecoPrimals");
        let now = SystemTime::now();

        let mut out = Vec::new();
        for &(slug, ref cap) in SOCKET_SPECS {
            let sock_path = eco_dir.join(format!("{slug}.sock"));
            if !sock_path.exists() {
                continue;
            }
            let url = format!("unix://{}", sock_path.display());
            let endpoint = match ServiceEndpoint::from_url_string(&url) {
                Ok(ep) => ep,
                Err(e) => {
                    warn!(path = %sock_path.display(), error = %e, "invalid unix socket URL for ecoPrimals fallback");
                    continue;
                }
            };
            out.push(DiscoveredService {
                id: format!("fallback-socket-{slug}"),
                name: slug.to_string(),
                version: "dev".to_string(),
                capabilities: vec![cap.clone()],
                endpoints: vec![endpoint],
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("source".to_string(), "fallback-unix-socket".to_string());
                    m
                },
                discovered_at: now,
                last_seen: now,
                healthy: true,
            });
        }
        out
    }

    pub(crate) fn parse_capabilities(capabilities_str: &str) -> Vec<Capability> {
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

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ServiceDiscoveryTrait for ServiceDiscovery {
    async fn find_services_by_capability(
        &self,
        capability: &Capability,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        let cached: Vec<DiscoveredService> = {
            let cache = self.cache.read().await;
            cache
                .values()
                .filter(|s| s.has_capability(capability))
                .filter(|s| s.is_fresh(self.config.cache_ttl))
                .cloned()
                .collect()
        };
        if !cached.is_empty() {
            debug!(
                "Found {} services in cache for capability: {:?}",
                cached.len(),
                capability
            );
            return Ok(cached);
        }

        // Avoid redundant mDNS/network scans: if a full discovery pass ran
        // within the cache TTL and found nothing, return empty rather than
        // re-scanning for every capability lookup.
        {
            let lr = self.last_refreshed.read().await;
            if let Some(t) = *lr {
                if t.elapsed() < self.config.cache_ttl {
                    debug!(
                        "Cache recently refreshed ({}ms ago); skipping re-discovery for {:?}",
                        t.elapsed().as_millis(),
                        capability
                    );
                    return Ok(vec![]);
                }
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
        *self.last_refreshed.write().await = Some(Instant::now());
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
        let count = {
            let mut cache = self.cache.write().await;
            cache.clear();
            for service in services {
                cache.insert(service.id.clone(), service);
            }
            cache.len()
        };
        *self.last_refreshed.write().await = Some(Instant::now());
        info!("Discovery cache refreshed: {count} services");
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
        let caps = ServiceDiscovery::parse_capabilities("");
        assert!(caps.is_empty());
    }

    #[test]
    fn test_parse_capabilities_coordination() {
        let caps = ServiceDiscovery::parse_capabilities("coordination");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        ));
    }

    #[test]
    fn test_parse_capabilities_storage() {
        let caps = ServiceDiscovery::parse_capabilities("storage");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_parse_capabilities_compute() {
        let caps = ServiceDiscovery::parse_capabilities("compute");
        assert_eq!(caps.len(), 1);
        assert!(matches!(
            caps[0],
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_parse_capabilities_multiple() {
        let caps = ServiceDiscovery::parse_capabilities("coordination, storage, compute");
        assert_eq!(caps.len(), 3);
    }

    #[test]
    fn test_parse_capabilities_unknown_filtered() {
        let caps = ServiceDiscovery::parse_capabilities("coordination, unknown_cap, storage");
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
            endpoints: vec![endpoint],
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

    #[test]
    fn test_discover_from_fallbacks_prefers_eco_primals_unix_socket() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let eco = tmp.path().join("ecoPrimals");
        std::fs::create_dir_all(&eco).expect("mkdir");
        std::fs::File::create(eco.join("toadstool.sock")).expect("touch");

        temp_env::with_var(
            "XDG_RUNTIME_DIR",
            Some(tmp.path().to_str().unwrap()),
            || {
                let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
                let services = disc.discover_from_fallbacks().expect("ok");
                assert_eq!(services.len(), 1);
                assert_eq!(services[0].endpoints[0].protocol, "unix");
                assert!(services[0].endpoints[0].address.contains("toadstool.sock"));
                assert_eq!(
                    services[0].metadata.get("source").map(String::as_str),
                    Some("fallback-unix-socket")
                );
            },
        );
    }

    #[test]
    fn test_discover_from_fallbacks_tcp_when_no_socket() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let eco = tmp.path().join("ecoPrimals");
        std::fs::create_dir_all(&eco).expect("mkdir");

        temp_env::with_vars(
            [
                ("XDG_RUNTIME_DIR", Some(tmp.path().to_str().unwrap())),
                ("TOADSTOOL_URL", Some("http://localhost:8084")),
            ],
            || {
                let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Environment);
                let services = disc.discover_from_fallbacks().expect("ok");
                assert_eq!(services.len(), 1);
                assert_eq!(services[0].endpoints[0].protocol, "http");
                assert_eq!(
                    services[0].metadata.get("source").map(String::as_str),
                    Some("fallback-tcp")
                );
                assert_eq!(
                    services[0].metadata.get("deprecation").map(String::as_str),
                    Some("tcp_url_fallback")
                );
            },
        );
    }
}
