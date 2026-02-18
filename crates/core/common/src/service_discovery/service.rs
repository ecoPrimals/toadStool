//! Service discovery implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::discovery_defaults::{DiscoveryConfig, LocalhostFallbacks};
#[allow(deprecated)]
use crate::interned_strings::primals;
use crate::primal_identity::{
    AuthCapability, Capability, CoordinationCapability, CryptoCapability, PrimalIdentity,
    ServiceEndpoint, StorageCapability,
};

use super::types::{
    DiscoveredService, DiscoveryError, DiscoveryMethod, DiscoveryResult, ServiceDiscoveryTrait,
};

// ── Config-file discovery types ───────────────────────────────────────────────

/// A single service entry in a discovery config file.
///
/// Config files are JSON, searched in order:
/// 1. `$TOADSTOOL_DISCOVERY_CONFIG` env var (full path)
/// 2. `$BIOMEOS_RUNTIME_DIR/discovery.json` (biomeOS runtime dir)
/// 3. `/etc/biomeos/discovery.json` (system-wide)
#[derive(Debug, Deserialize)]
struct ConfigFileService {
    id: Option<String>,
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    endpoints: Vec<String>,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    services: Vec<ConfigFileService>,
}

fn default_version() -> String {
    "unknown".to_string()
}

/// Map capability string (from config/mDNS TXT records) to typed `Capability`.
fn capability_from_str(s: &str) -> Capability {
    match s.trim().to_lowercase().as_str() {
        "coordination" | "orchestration" => {
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        }
        "storage" | "object_storage" | "object-storage" => {
            Capability::Storage(StorageCapability::ObjectStorage)
        }
        "security" | "crypto" | "cryptography" => {
            Capability::Crypto(CryptoCapability::KeyManagement)
        }
        "authentication" | "auth" => Capability::Authentication(AuthCapability::TokenManagement),
        "compute" | "native" | "execution" => {
            Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution)
        }
        "gpu" | "gpu_compute" | "gpu-compute" => {
            Capability::Compute(crate::primal_identity::ComputeCapability::GpuCompute)
        }
        other => Capability::Custom {
            name: other.to_string(),
            version: "0".to_string(),
        },
    }
}

/// Main service discovery implementation
pub struct ServiceDiscovery {
    pub(crate) config: DiscoveryConfig,
    pub(crate) method: DiscoveryMethod,
    cache: Arc<RwLock<HashMap<String, DiscoveredService>>>,
    fallbacks: LocalhostFallbacks,
}

impl ServiceDiscovery {
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
            DiscoveryMethod::Mdns => self.discover_via_mdns().await,
            DiscoveryMethod::ConfigFile { path } => self.discover_from_config(path).await,
            DiscoveryMethod::Registry { endpoint } => self.discover_from_registry(endpoint).await,
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
            return self.discover_from_fallbacks().await;
        }
        Ok(all_services)
    }

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

    /// Discover from environment variables (pub for tests)
    pub async fn discover_from_env(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("TOADSTOOL_SERVICE_") && key.ends_with("_URL") {
                let service_name = key
                    .strip_prefix("TOADSTOOL_SERVICE_")
                    .and_then(|s| s.strip_suffix("_URL"))
                    .unwrap_or("unknown");
                let cap_key = format!("TOADSTOOL_SERVICE_{}_CAPABILITIES", service_name);
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

    async fn discover_via_mdns(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        use crate::primal_discovery::DiscoveryConfig as PrimalDiscoveryConfig;
        use crate::primal_discovery_mdns::MdnsAdapter;

        let mdns_config = PrimalDiscoveryConfig {
            enable_mdns: true,
            ..Default::default()
        };

        // MdnsAdapter::discover_all() uses blocking recv_timeout internally;
        // run on the blocking thread pool to avoid starving the async executor.
        let endpoints = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                MdnsAdapter::new(mdns_config)
                    .await
                    .map_err(|e| {
                        DiscoveryError::MethodUnavailable {
                            method: format!("mDNS init failed: {e}"),
                        }
                    })?
                    .discover_all()
                    .await
                    .map_err(|e| DiscoveryError::MethodUnavailable {
                        method: format!("mDNS browse failed: {e}"),
                    })
            })
        })
        .await
        .map_err(|e| DiscoveryError::MethodUnavailable {
            method: format!("spawn_blocking failed: {e}"),
        })??;

        let now = SystemTime::now();
        let services: Vec<DiscoveredService> = endpoints
            .into_iter()
            .map(|ep| {
                let caps: Vec<Capability> =
                    ep.capabilities.iter().map(|s| capability_from_str(s)).collect();
                let endpoint = ServiceEndpoint::from_url_string(&ep.url).unwrap_or_else(|_| {
                    ServiceEndpoint {
                        protocol: "http".to_string(),
                        address: ep.url.clone(),
                        port: 80,
                        path: None,
                        metadata: HashMap::new(),
                    }
                });
                DiscoveredService {
                    id: ep.service_id.clone(),
                    name: ep.service_id,
                    version: "mdns".to_string(),
                    capabilities: caps,
                    endpoints: vec![endpoint],
                    metadata: HashMap::new(),
                    discovered_at: now,
                    last_seen: now,
                    healthy: true,
                }
            })
            .collect();

        info!("mDNS discovery: found {} services", services.len());
        Ok(services)
    }

    async fn discover_from_config(&self, path: &str) -> DiscoveryResult<Vec<DiscoveredService>> {
        // Resolve path: explicit arg → env var → default locations
        let resolved_path = if !path.is_empty() {
            path.to_string()
        } else if let Ok(p) = std::env::var("TOADSTOOL_DISCOVERY_CONFIG") {
            p
        } else if let Ok(runtime) = std::env::var("BIOMEOS_RUNTIME_DIR") {
            format!("{runtime}/discovery.json")
        } else {
            "/etc/biomeos/discovery.json".to_string()
        };

        let content = tokio::fs::read_to_string(&resolved_path).await.map_err(|e| {
            DiscoveryError::MethodUnavailable {
                method: format!("cannot read discovery config {resolved_path:?}: {e}"),
            }
        })?;

        let config_file: ConfigFile =
            serde_json::from_str(&content).map_err(|e| DiscoveryError::InvalidResponse {
                reason: format!("malformed discovery config {resolved_path:?}: {e}"),
            })?;

        let now = SystemTime::now();
        let mut services = Vec::with_capacity(config_file.services.len());

        for svc in config_file.services {
            let caps: Vec<Capability> =
                svc.capabilities.iter().map(|s| capability_from_str(s)).collect();

            let mut endpoints = Vec::with_capacity(svc.endpoints.len());
            for url in &svc.endpoints {
                match ServiceEndpoint::from_url_string(url) {
                    Ok(ep) => endpoints.push(ep),
                    Err(e) => {
                        warn!("Skipping malformed endpoint {url:?} in discovery config: {e}");
                    }
                }
            }

            let id = svc
                .id
                .unwrap_or_else(|| format!("config-{}", svc.name.to_lowercase()));
            services.push(DiscoveredService {
                id,
                name: svc.name,
                version: svc.version,
                capabilities: caps,
                endpoints,
                metadata: svc.metadata,
                discovered_at: now,
                last_seen: now,
                healthy: true,
            });
        }

        info!(
            "Config discovery: loaded {} services from {:?}",
            services.len(),
            resolved_path
        );
        Ok(services)
    }

    async fn discover_from_registry(
        &self,
        endpoint: &str,
    ) -> DiscoveryResult<Vec<DiscoveredService>> {
        // Registry protocol: GET {endpoint}/services → JSON array of ConfigFileService.
        // Pure Rust via tokio UnixStream or TCP — no external HTTP client.
        // Resolution order: arg → TOADSTOOL_REGISTRY_ENDPOINT env → error.
        let resolved = if !endpoint.is_empty() {
            endpoint.to_string()
        } else if let Ok(env_ep) = std::env::var("TOADSTOOL_REGISTRY_ENDPOINT") {
            env_ep
        } else {
            return Err(DiscoveryError::MethodUnavailable {
                method: "registry endpoint not configured (set TOADSTOOL_REGISTRY_ENDPOINT)"
                    .to_string(),
            });
        };

        // For Unix socket registries (file:// or unix://) delegate to config discovery
        // since the registry serves the same JSON format over a socket.
        if resolved.starts_with("file://") || resolved.starts_with("unix://") {
            let path = resolved
                .trim_start_matches("file://")
                .trim_start_matches("unix://");
            return self.discover_from_config(path).await;
        }

        // HTTP registry: use tokio TCP to issue a minimal HTTP/1.1 GET request —
        // no reqwest or ring, pure Rust stdlib + tokio.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let url = resolved.trim_start_matches("http://").trim_start_matches("https://");
        let (host_port, path) = url.split_once('/').unwrap_or((url, "services"));
        let path = format!("/{path}");

        let mut stream = TcpStream::connect(host_port)
            .await
            .map_err(|source| DiscoveryError::NetworkError { source })?;

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|source| DiscoveryError::NetworkError { source })?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .await
            .map_err(|source| DiscoveryError::NetworkError { source })?;

        // Strip HTTP headers — body starts after first blank line
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or(&response);

        let config_file: ConfigFile =
            serde_json::from_str(body).map_err(|e| DiscoveryError::InvalidResponse {
                reason: format!("malformed registry response from {resolved:?}: {e}"),
            })?;

        let now = SystemTime::now();
        let services = config_file
            .services
            .into_iter()
            .map(|svc| {
                let caps: Vec<Capability> =
                    svc.capabilities.iter().map(|s| capability_from_str(s)).collect();
                let endpoints: Vec<ServiceEndpoint> = svc
                    .endpoints
                    .iter()
                    .filter_map(|url| ServiceEndpoint::from_url_string(url).ok())
                    .collect();
                let id = svc
                    .id
                    .unwrap_or_else(|| format!("registry-{}", svc.name.to_lowercase()));
                DiscoveredService {
                    id,
                    name: svc.name,
                    version: svc.version,
                    capabilities: caps,
                    endpoints,
                    metadata: svc.metadata,
                    discovered_at: now,
                    last_seen: now,
                    healthy: true,
                }
            })
            .collect::<Vec<_>>();

        info!(
            "Registry discovery: loaded {} services from {:?}",
            services.len(),
            resolved
        );
        Ok(services)
    }

    async fn discover_from_fallbacks(&self) -> DiscoveryResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();
        if !self.fallbacks.should_use_fallback() {
            return Ok(services);
        }
        info!("Using localhost fallbacks for development");
        #[allow(deprecated)]
        if let Some(url) = self.fallbacks.get_fallback_url(primals::TOADSTOOL) {
            services.push(DiscoveredService {
                id: format!("fallback-{}", primals::TOADSTOOL),
                name: primals::TOADSTOOL.to_string(),
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
        // Self-announcement via mDNS TXT records.
        // We log the intent; full registration is handled by `infant_discovery`
        // when the caller opts into the full biomeOS advertise loop.
        // Self-announcement via mDNS TXT records is handled by calling code that
        // uses `primal_discovery_mdns::MdnsAdapter` or `infant_discovery`.
        // `ServiceDiscovery` focuses on *finding* services; callers that want to
        // announce themselves should use the biomeOS MdnsAdapter directly.
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
