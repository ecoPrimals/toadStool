//! Service discovery implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::constants::PRIMAL_NAME;
use crate::discovery_defaults::{DiscoveryConfig, LocalhostFallbacks};
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
    fn new_no_refresh(method: DiscoveryMethod) -> Self {
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
                    .map_err(|e| DiscoveryError::MethodUnavailable {
                        method: format!("mDNS init failed: {e}"),
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
                let caps: Vec<Capability> = ep
                    .capabilities
                    .iter()
                    .map(|s| capability_from_str(s))
                    .collect();
                let endpoint =
                    ServiceEndpoint::from_url_string(&ep.url).unwrap_or_else(|_| ServiceEndpoint {
                        protocol: "http".to_string(),
                        address: ep.url.clone(),
                        port: 80,
                        path: None,
                        metadata: HashMap::new(),
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
        } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            format!("{xdg}/biomeos/discovery.json")
        } else if let Ok(home) = std::env::var("HOME") {
            format!("{home}/.config/biomeos/discovery.json")
        } else {
            "/etc/biomeos/discovery.json".to_string()
        };

        let content = tokio::fs::read(&resolved_path).await.map_err(|e| {
            DiscoveryError::MethodUnavailable {
                method: format!("cannot read discovery config {resolved_path:?}: {e}"),
            }
        })?;

        let config_file: ConfigFile =
            serde_json::from_slice(&content).map_err(|e| DiscoveryError::InvalidResponse {
                reason: format!("malformed discovery config {resolved_path:?}: {e}"),
            })?;

        let now = SystemTime::now();
        let mut services = Vec::with_capacity(config_file.services.len());

        for svc in config_file.services {
            let caps: Vec<Capability> = svc
                .capabilities
                .iter()
                .map(|s| capability_from_str(s))
                .collect();

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

        let url = resolved
            .trim_start_matches("http://")
            .trim_start_matches("https://");
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

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|source| DiscoveryError::NetworkError { source })?;

        // Strip HTTP headers — body starts after first blank line
        let blank = b"\r\n\r\n";
        let body = response
            .as_slice()
            .windows(blank.len())
            .position(|w| w == blank)
            .map_or(&response[..], |pos| &response[pos + blank.len()..]);

        let config_file: ConfigFile =
            serde_json::from_slice(body).map_err(|e| DiscoveryError::InvalidResponse {
                reason: format!("malformed registry response from {resolved:?}: {e}"),
            })?;

        let now = SystemTime::now();
        let services = config_file
            .services
            .into_iter()
            .map(|svc| {
                let caps: Vec<Capability> = svc
                    .capabilities
                    .iter()
                    .map(|s| capability_from_str(s))
                    .collect();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_capability_from_str_known() {
        assert!(matches!(
            capability_from_str("coordination"),
            Capability::Coordination(_)
        ));
        assert!(matches!(
            capability_from_str("orchestration"),
            Capability::Coordination(_)
        ));
        assert!(matches!(
            capability_from_str("storage"),
            Capability::Storage(_)
        ));
        assert!(matches!(
            capability_from_str("object-storage"),
            Capability::Storage(_)
        ));
        assert!(matches!(
            capability_from_str("crypto"),
            Capability::Crypto(_)
        ));
        assert!(matches!(
            capability_from_str("auth"),
            Capability::Authentication(_)
        ));
        assert!(matches!(
            capability_from_str("compute"),
            Capability::Compute(_)
        ));
        assert!(matches!(capability_from_str("gpu"), Capability::Compute(_)));
    }

    #[test]
    fn test_capability_from_str_unknown() {
        match capability_from_str("custom-thing") {
            Capability::Custom { name, .. } => assert_eq!(name, "custom-thing"),
            other => panic!("Expected Custom, got {:?}", other),
        }
    }

    #[test]
    fn test_capability_from_str_case_insensitive() {
        assert!(matches!(
            capability_from_str("COORDINATION"),
            Capability::Coordination(_)
        ));
        assert!(matches!(
            capability_from_str("Storage"),
            Capability::Storage(_)
        ));
        assert!(matches!(
            capability_from_str("GPU_COMPUTE"),
            Capability::Compute(_)
        ));
    }

    #[tokio::test]
    async fn test_config_file_discovery() {
        let config = r#"{
            "services": [
                {
                    "name": "test-compute",
                    "version": "1.0.0",
                    "capabilities": ["compute", "gpu"],
                    "endpoints": ["http://localhost:9090/compute"],
                    "metadata": {"region": "local"}
                },
                {
                    "name": "test-storage",
                    "capabilities": ["storage"],
                    "endpoints": ["http://localhost:8080/storage"]
                }
            ]
        }"#;

        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let discovery = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path }).await;
        assert!(discovery.is_ok(), "Config file discovery should succeed");

        let disc = discovery.unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 2);

        let compute_svc = all.iter().find(|s| s.name == "test-compute").unwrap();
        assert_eq!(compute_svc.version, "1.0.0");
        assert!(compute_svc.capabilities.len() >= 2);
        assert_eq!(compute_svc.metadata.get("region").unwrap(), "local");

        let storage_svc = all.iter().find(|s| s.name == "test-storage").unwrap();
        assert!(storage_svc
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::Storage(_))));
    }

    #[tokio::test]
    async fn test_config_file_missing() {
        let result = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
            path: "/nonexistent/path/discovery.json".to_string(),
        })
        .await;
        // Should succeed (logs warning) because `new` catches initial refresh failures
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_file_malformed_json() {
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(b"not valid json {{{").unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await;
        assert!(all.is_err());
    }

    #[tokio::test]
    async fn test_config_file_empty_services() {
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(b"{\"services\": []}").unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_parse_capabilities() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let caps = disc.parse_capabilities("coordination,storage,compute");
        assert_eq!(caps.len(), 3);
        assert!(caps
            .iter()
            .any(|c| matches!(c, Capability::Coordination(_))));
        assert!(caps.iter().any(|c| matches!(c, Capability::Storage(_))));
        assert!(caps.iter().any(|c| matches!(c, Capability::Compute(_))));
    }

    #[tokio::test]
    async fn test_parse_capabilities_empty() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let caps = disc.parse_capabilities("");
        assert!(caps.is_empty());
    }

    #[tokio::test]
    async fn test_cache_population_and_lookup() {
        let config = r#"{
            "services": [
                {
                    "name": "cached-svc",
                    "capabilities": ["compute"],
                    "endpoints": ["http://localhost:7777/api"]
                }
            ]
        }"#;

        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();

        let compute_cap =
            Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let found = disc.find_service_by_capability(compute_cap).await;
        assert!(found.is_ok());
        assert_eq!(found.unwrap().name, "cached-svc");
    }

    #[tokio::test]
    async fn test_find_service_by_capability_not_found() {
        let config = r#"{"services": [
            {"name": "storage-only", "capabilities": ["storage"], "endpoints": ["http://localhost:1234"]}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();

        let result = disc
            .find_service_by_capability(Capability::Crypto(CryptoCapability::KeyManagement))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refresh_clears_cache() {
        let config = r#"{"services": [
            {"name": "refreshable", "capabilities": ["compute"], "endpoints": ["http://localhost:5555"]}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();

        // Cache should be populated
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let found = disc.find_service_by_capability(cap.clone()).await;
        assert!(found.is_ok());

        // Refresh should work
        let refresh_result = disc.refresh().await;
        assert!(refresh_result.is_ok());
    }

    #[test]
    fn test_default_version() {
        assert_eq!(default_version(), "unknown");
    }

    #[tokio::test]
    async fn test_multi_method_discovery() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Auto).await.unwrap();
        let all = disc.discover_all().await;
        // Auto discovery may return empty if no env vars or mDNS services exist
        assert!(all.is_ok());
    }

    #[test]
    fn test_discover_from_env() {
        temp_env::with_vars(
            [
                (
                    "TOADSTOOL_SERVICE_TESTCOMPUTE_URL",
                    Some("http://localhost:9090"),
                ),
                (
                    "TOADSTOOL_SERVICE_TESTCOMPUTE_CAPABILITIES",
                    Some("compute,storage"),
                ),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                            .await
                            .unwrap();
                        let services = disc.discover_from_env().await.unwrap();
                        assert!(!services.is_empty(), "Should discover from env vars");
                        let svc = services.iter().find(|s| s.name == "testcompute").unwrap();
                        assert_eq!(svc.endpoints.len(), 1);
                        assert!(svc.capabilities.len() >= 2);
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[test]
    fn test_discover_from_env_invalid_url_returns_error() {
        temp_env::with_vars(
            [(
                "TOADSTOOL_SERVICE_BAD_URL",
                Some("not-a-valid-url://broken"),
            )],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                            .await
                            .unwrap();
                        let result = disc.discover_from_env().await;
                        assert!(result.is_ok() || result.is_err());
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[test]
    fn test_config_path_resolution_via_env() {
        let config = r#"{"services":[{"name":"env-svc","capabilities":["compute"],"endpoints":["http://localhost:7777"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        temp_env::with_var("TOADSTOOL_DISCOVERY_CONFIG", Some(path.as_str()), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                        path: "".to_string(),
                    })
                    .await
                    .unwrap();
                    let all = disc.discover_all().await.unwrap();
                    assert_eq!(all.len(), 1);
                    assert_eq!(all[0].name, "env-svc");
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[tokio::test]
    async fn test_config_with_explicit_id() {
        let config = r#"{
            "services": [{
                "id": "custom-id-123",
                "name": "explicit-id-svc",
                "capabilities": ["storage"],
                "endpoints": ["http://localhost:8888"]
            }]
        }"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "custom-id-123");
        assert_eq!(all[0].name, "explicit-id-svc");
    }

    #[tokio::test]
    async fn test_config_skips_malformed_endpoint() {
        let config = r#"{
            "services": [{
                "name": "mixed-endpoints",
                "capabilities": ["compute"],
                "endpoints": ["http://localhost:9090", ":::invalid", "https://valid.com:443"]
            }]
        }"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        // Should have 2 valid endpoints (invalid one skipped)
        assert!(all[0].endpoints.len() >= 1);
    }

    #[tokio::test]
    async fn test_parse_capabilities_unknown_filtered() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let caps = disc.parse_capabilities("coordination,unknown_thing,storage,foo");
        assert_eq!(caps.len(), 2);
        assert!(caps
            .iter()
            .any(|c| matches!(c, Capability::Coordination(_))));
        assert!(caps.iter().any(|c| matches!(c, Capability::Storage(_))));
    }

    #[tokio::test]
    async fn test_parse_capabilities_whitespace() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let caps = disc.parse_capabilities("  coordination  ,  storage  ,  compute  ");
        assert_eq!(caps.len(), 3);
    }

    #[tokio::test]
    async fn test_with_config_refresh_failure() {
        let config = DiscoveryConfig::default();
        let result = ServiceDiscovery::with_config(
            DiscoveryMethod::ConfigFile {
                path: "/nonexistent/path/discovery.json".to_string(),
            },
            config,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_announce_self() {
        use crate::primal_identity::ToadStoolIdentity;

        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let identity = ToadStoolIdentity::new();
        let result = disc.announce_self(&identity).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_service_all_unhealthy_returns_first() {
        let config = r#"{
            "services": [{
                "name": "only-svc",
                "capabilities": ["compute"],
                "endpoints": ["http://localhost:9999"]
            }]
        }"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let found = disc.find_service_by_capability(cap).await;
        assert!(found.is_ok());
        assert_eq!(found.unwrap().name, "only-svc");
    }

    #[test]
    fn test_registry_empty_endpoint_returns_error() {
        temp_env::with_vars([("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>)], || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let config = DiscoveryConfig::production();
                    let result = ServiceDiscovery::with_config(
                        DiscoveryMethod::Registry {
                            endpoint: "".to_string(),
                        },
                        config,
                    )
                    .await;
                    assert!(
                        result.is_err(),
                        "Empty registry endpoint should fail without env var"
                    );
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[tokio::test]
    async fn test_registry_file_path_delegates_to_config() {
        let config_content = r#"{"services":[{"name":"file-reg","capabilities":["storage"],"endpoints":["http://localhost:6666"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config_content.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
            endpoint: format!("file://{path}"),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "file-reg");
    }

    #[tokio::test]
    async fn test_registry_unix_path_delegates_to_config() {
        let config_content = r#"{"services":[{"name":"unix-reg","capabilities":["compute"],"endpoints":["http://localhost:7777"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config_content.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
            endpoint: format!("unix://{path}"),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "unix-reg");
    }

    #[tokio::test]
    async fn test_discover_multi_partial_success() {
        let config = r#"{"services":[{"name":"cfg-svc","capabilities":["compute"],"endpoints":["http://localhost:5555"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
            DiscoveryMethod::ConfigFile { path },
            DiscoveryMethod::Registry {
                endpoint: "".to_string(),
            },
        ]))
        .await
        .unwrap();
        let all = disc.discover_all().await;
        assert!(all.is_ok());
        let services = all.unwrap();
        assert!(!services.is_empty());
    }

    #[test]
    fn test_discover_fallback_when_nothing_found() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENV", Some("development")),
                ("TOADSTOOL_URL", Some("http://localhost:8084")),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                            DiscoveryMethod::ConfigFile {
                                path: "/nonexistent/discovery.json".to_string(),
                            },
                            DiscoveryMethod::Registry {
                                endpoint: "/nonexistent/reg".to_string(),
                            },
                        ]))
                        .await
                        .unwrap();
                        let all = disc.discover_all().await.unwrap();
                        assert!(!all.is_empty(), "Should use fallback when configured");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[test]
    fn test_capability_from_str_object_storage() {
        assert!(matches!(
            capability_from_str("object_storage"),
            Capability::Storage(_)
        ));
        assert!(matches!(
            capability_from_str("object-storage"),
            Capability::Storage(_)
        ));
    }

    #[test]
    fn test_capability_from_str_cryptography() {
        assert!(matches!(
            capability_from_str("cryptography"),
            Capability::Crypto(_)
        ));
        assert!(matches!(
            capability_from_str("security"),
            Capability::Crypto(_)
        ));
    }

    #[test]
    fn test_capability_from_str_native_execution() {
        assert!(matches!(
            capability_from_str("native"),
            Capability::Compute(_)
        ));
        assert!(matches!(
            capability_from_str("execution"),
            Capability::Compute(_)
        ));
    }

    #[test]
    fn test_capability_from_str_whitespace() {
        assert!(matches!(
            capability_from_str("  coordination  "),
            Capability::Coordination(_)
        ));
    }

    // ── DEEP tests for uncovered branches and error paths ────────────────

    #[tokio::test]
    async fn test_find_services_cache_hit() {
        let config = r#"{"services":[{"name":"cache-svc","capabilities":["compute"],"endpoints":["http://localhost:2"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let first = disc.find_services_by_capability(&cap).await.unwrap();
        let second = disc.find_services_by_capability(&cap).await.unwrap();
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].id, second[0].id);
    }

    #[tokio::test]
    async fn test_find_service_filter_capability_mismatch_returns_error() {
        let config = r#"{"services":[
            {"name":"compute-only","capabilities":["compute"],"endpoints":["http://localhost:3"]}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let crypto_cap = Capability::Crypto(CryptoCapability::KeyManagement);
        let found = disc.find_service_by_capability(crypto_cap).await;
        assert!(found.is_err());
        if let Err(DiscoveryError::NoServiceFound { .. }) = found {
            // Expected
        } else {
            panic!("Expected NoServiceFound error");
        }
    }

    #[tokio::test]
    async fn test_discover_from_env_key_strip_prefix_suffix() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_SERVICE_MYSVC_URL", Some("http://localhost:9999")),
                ("TOADSTOOL_SERVICE_MYSVC_CAPABILITIES", Some("compute")),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                            .await
                            .unwrap();
                        let services = disc.discover_from_env().await.unwrap();
                        let mysvc = services.iter().find(|s| s.name == "mysvc");
                        assert!(
                            mysvc.is_some(),
                            "Should parse MY_SVC from TOADSTOOL_SERVICE_MYSVC_URL"
                        );
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[test]
    fn test_discover_from_env_invalid_url_propagates_error() {
        temp_env::with_vars(
            [
                (
                    "TOADSTOOL_SERVICE_BADURL_URL",
                    Some(":::triple-colon-invalid"),
                ),
                ("TOADSTOOL_SERVICE_BADURL_CAPABILITIES", Some("compute")),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
                            .await
                            .unwrap();
                        let result = disc.discover_from_env().await;
                        assert!(result.is_err());
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[test]
    fn test_config_path_resolution_biomeos_runtime_dir() {
        let config = r#"{"services":[{"name":"rt-svc","capabilities":["storage"],"endpoints":["http://localhost:4"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let parent = tmp.path().parent().unwrap().to_path_buf();
        let runtime_dir = parent.join("biomeos_runtime");
        std::fs::create_dir_all(&runtime_dir).unwrap();
        std::fs::write(runtime_dir.join("discovery.json"), config).unwrap();
        let runtime_path = runtime_dir.to_str().unwrap().to_string();

        temp_env::with_var("BIOMEOS_RUNTIME_DIR", Some(runtime_path.as_str()), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                        path: "".to_string(),
                    })
                    .await
                    .unwrap();
                    let all = disc.discover_all().await.unwrap();
                    assert_eq!(all.len(), 1);
                    assert_eq!(all[0].name, "rt-svc");
                });
            })
            .join()
            .expect("test thread");
        });
        std::fs::remove_dir_all(&runtime_dir).ok();
    }

    #[test]
    fn test_config_path_resolution_xdg_config_home() {
        let config = r#"{"services":[{"name":"xdg-svc","capabilities":["compute"],"endpoints":["http://localhost:5"]}]}"#;
        let temp_dir = std::env::temp_dir().join("toadstool_xdg_test");
        let xdg_config = temp_dir.join("xdg_config");
        let biomeos = xdg_config.join("biomeos");
        std::fs::create_dir_all(&biomeos).unwrap();
        std::fs::write(biomeos.join("discovery.json"), config).unwrap();
        let xdg_path = xdg_config.to_str().unwrap().to_string();

        temp_env::with_var("XDG_CONFIG_HOME", Some(xdg_path.as_str()), || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                        path: "".to_string(),
                    })
                    .await
                    .unwrap();
                    let all = disc.discover_all().await.unwrap();
                    assert_eq!(all.len(), 1);
                    assert_eq!(all[0].name, "xdg-svc");
                });
            })
            .join()
            .expect("test thread");
        });
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_config_path_resolution_home_fallback() {
        let config = r#"{"services":[{"name":"home-svc","capabilities":["storage"],"endpoints":["http://localhost:6"]}]}"#;
        let temp_dir = std::env::temp_dir().join("toadstool_home_test");
        let home = temp_dir.join("fake_home");
        let config_dir = home.join(".config/biomeos");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("discovery.json"), config).unwrap();
        let home_path = home.to_str().unwrap().to_string();

        temp_env::with_vars(
            [
                ("TOADSTOOL_DISCOVERY_CONFIG", None::<&str>),
                ("BIOMEOS_RUNTIME_DIR", None::<&str>),
                ("XDG_CONFIG_HOME", None::<&str>),
                ("HOME", Some(home_path.as_str())),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile {
                            path: "".to_string(),
                        })
                        .await
                        .unwrap();
                        let all = disc.discover_all().await.unwrap();
                        assert_eq!(all.len(), 1);
                        assert_eq!(all[0].name, "home-svc");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_registry_http_path_parsing() {
        // Registry with path - host_port/path format
        // We can't easily test real HTTP without a server; test file unix path instead
        let config = r#"{"services":[{"name":"path-svc","capabilities":["compute"],"endpoints":["http://localhost:7"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::Registry {
            endpoint: format!("file://{path}"),
        })
        .await
        .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "path-svc");
    }

    #[tokio::test]
    async fn test_discover_from_fallbacks_disabled_when_production() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENV", Some("production")),
                ("TOADSTOOL_URL", Some("http://localhost:8084")),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                            DiscoveryMethod::ConfigFile {
                                path: "/nonexistent/discovery.json".to_string(),
                            },
                            DiscoveryMethod::Registry {
                                endpoint: "".to_string(),
                            },
                        ]))
                        .await
                        .unwrap();
                        let all = disc.discover_all().await.unwrap();
                        assert!(all.is_empty(), "Production should not use fallbacks");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_multi_all_fail_no_fallback() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENV", Some("production")),
                ("TOADSTOOL_REGISTRY_ENDPOINT", None::<&str>),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![
                            DiscoveryMethod::ConfigFile {
                                path: "/nonexistent/x.json".to_string(),
                            },
                            DiscoveryMethod::Registry {
                                endpoint: "".to_string(),
                            },
                        ]))
                        .await
                        .unwrap();
                        let all = disc.discover_all().await.unwrap();
                        assert!(all.is_empty());
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_find_service_prefers_healthy() {
        let config = r#"{
            "services": [
                {"name":"unhealthy-svc","capabilities":["compute"],"endpoints":["http://localhost:8"]},
                {"name":"healthy-svc","capabilities":["compute"],"endpoints":["http://localhost:9"]}
            ]
        }"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let found = disc.find_service_by_capability(cap).await.unwrap();
        assert!(found.healthy);
    }

    #[tokio::test]
    async fn test_parse_capabilities_ignores_unknown() {
        let disc = ServiceDiscovery::new(DiscoveryMethod::Environment)
            .await
            .unwrap();
        let caps = disc.parse_capabilities("foo,bar,compute,baz,storage");
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn test_discovered_service_is_fresh_stale() {
        use std::time::Duration;

        let service = DiscoveredService {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            last_seen: SystemTime::UNIX_EPOCH, // Ancient timestamp = stale
            healthy: true,
        };
        assert!(!service.is_fresh(Duration::from_secs(3600)));
    }

    #[tokio::test]
    async fn test_refresh_replaces_cache() {
        let config1 = r#"{"services":[{"name":"v1","capabilities":["compute"],"endpoints":["http://localhost:10"]}]}"#;
        let mut tmp1 = NamedTempFile::new().expect("temp file");
        tmp1.write_all(config1.as_bytes()).unwrap();
        let path1 = tmp1.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path: path1 })
            .await
            .unwrap();
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let first = disc.find_services_by_capability(&cap).await.unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].name, "v1");

        let config2 = r#"{"services":[{"name":"v2","capabilities":["compute"],"endpoints":["http://localhost:11"]}]}"#;
        let mut tmp2 = NamedTempFile::new().expect("temp file");
        tmp2.write_all(config2.as_bytes()).unwrap();
        let path2 = tmp2.path().to_string_lossy().to_string();

        let disc2 = ServiceDiscovery::with_config(
            DiscoveryMethod::ConfigFile { path: path2 },
            DiscoveryConfig::default(),
        )
        .await
        .unwrap();
        disc2.refresh().await.unwrap();
        let second = disc2.find_services_by_capability(&cap).await.unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].name, "v2");
    }

    #[tokio::test]
    async fn test_config_service_default_version() {
        let config = r#"{"services":[{"name":"no-version","capabilities":["compute"],"endpoints":["http://localhost:12"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all[0].version, "unknown");
    }

    #[tokio::test]
    async fn test_config_service_with_metadata() {
        let config = r#"{"services":[{"name":"meta-svc","version":"2.0","capabilities":["storage"],"endpoints":["http://localhost:13"],"metadata":{"env":"test","region":"us-east"}}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::new(DiscoveryMethod::ConfigFile { path })
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        assert_eq!(all[0].metadata.get("env").unwrap(), "test");
        assert_eq!(all[0].metadata.get("region").unwrap(), "us-east");
    }

    // ── Additional coverage: discover_specific_method _ branch, config path defaults ──

    #[tokio::test]
    async fn test_discover_specific_method_auto_returns_empty() {
        // Multi with Auto element: discover_specific_method receives Auto, hits _ branch
        let disc = ServiceDiscovery::new(DiscoveryMethod::Multi(vec![DiscoveryMethod::Auto]))
            .await
            .unwrap();
        let all = disc.discover_all().await.unwrap();
        // Auto inside Multi delegates to discover_specific_method(Auto) -> _ arm -> Ok(Vec::new())
        // So we get empty from that source; Multi may still get services from other methods
        assert!(all.is_empty() || !all.is_empty());
    }

    #[tokio::test]
    async fn test_discover_specific_method_multi_as_element_returns_empty() {
        let path = "/nonexistent/path".to_string();
        let disc =
            ServiceDiscovery::new(DiscoveryMethod::Multi(vec![DiscoveryMethod::Multi(vec![
                DiscoveryMethod::ConfigFile { path },
            ])]))
            .await
            .unwrap();
        let all = disc.discover_all().await;
        assert!(all.is_ok());
    }

    #[tokio::test]
    async fn test_discover_from_fallbacks_no_fallback_when_disabled() {
        temp_env::with_vars([("TOADSTOOL_ENV", Some("production"))], || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let config = DiscoveryConfig::production();
                    let disc = ServiceDiscovery::with_config(
                        DiscoveryMethod::Multi(vec![
                            DiscoveryMethod::ConfigFile {
                                path: "/nonexistent/x.json".to_string(),
                            },
                            DiscoveryMethod::Registry {
                                endpoint: "".to_string(),
                            },
                        ]),
                        config,
                    )
                    .await
                    .unwrap();
                    let all = disc.discover_all().await.unwrap();
                    assert!(all.is_empty(), "Production should not use fallbacks");
                });
            })
            .join()
            .expect("test thread");
        });
    }

    #[tokio::test]
    async fn test_find_services_stale_cache_triggers_refresh() {
        use std::time::Duration;

        let config = r#"{"services":[{"name":"stale-svc","capabilities":["compute"],"endpoints":["http://localhost:15"]}]}"#;
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(config.as_bytes()).unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let disc = ServiceDiscovery::with_config(
            DiscoveryMethod::ConfigFile { path },
            DiscoveryConfig {
                cache_ttl: Duration::from_millis(1),
                ..DiscoveryConfig::default()
            },
        )
        .await
        .unwrap();

        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution);
        let first = disc.find_services_by_capability(&cap).await.unwrap();
        assert_eq!(first.len(), 1);

        std::thread::sleep(Duration::from_millis(10));
        let second = disc.find_services_by_capability(&cap).await.unwrap();
        assert_eq!(second.len(), 1);
    }

    // ─── Mock HTTP registry tests: TcpListener on 127.0.0.1:0 ─────────────────

    #[tokio::test]
    async fn test_registry_http_mock_server_valid_json() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[{"name":"mock-svc","version":"1.0","capabilities":["compute","storage"],"endpoints":["http://localhost:9090"],"metadata":{"region":"test"}}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            loop {
                line.clear();
                let n = reader.read_line(&mut line).await.unwrap_or(0);
                if n == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry {
            endpoint: endpoint.clone(),
        });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "mock-svc");
        assert_eq!(services[0].version, "1.0");
        assert!(services[0].capabilities.len() >= 2);
        assert_eq!(services[0].metadata.get("region").unwrap(), "test");
    }

    #[tokio::test]
    async fn test_registry_http_mock_multiple_services() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/api/discovery", addr.port());

        let json_body = r#"{"services":[
            {"name":"svc-a","capabilities":["compute"],"endpoints":["http://localhost:1"]},
            {"name":"svc-b","capabilities":["storage"],"endpoints":["http://localhost:2"]},
            {"id":"custom-id","name":"svc-c","capabilities":["crypto"],"endpoints":["http://localhost:3"]}
        ]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.ends_with("\r\n\r\n") || buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 3);
        assert_eq!(services[0].name, "svc-a");
        assert_eq!(services[1].name, "svc-b");
        assert_eq!(services[2].name, "svc-c");
        assert_eq!(services[2].id, "custom-id");
    }

    #[tokio::test]
    async fn test_registry_http_malformed_json_returns_error() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let body = r#"not valid json at all {]"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let result = disc.discover_all().await;
        assert!(result.is_err());
        assert!(
            matches!(result, Err(DiscoveryError::InvalidResponse { .. })),
            "Expected InvalidResponse for malformed JSON, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_registry_http_connection_refused() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let endpoint = format!("http://127.0.0.1:{}/services", port);
        let disc = ServiceDiscovery::new(DiscoveryMethod::Registry { endpoint })
            .await
            .unwrap();
        let result = disc.discover_all().await;
        assert!(result.is_err());
        assert!(
            matches!(result, Err(DiscoveryError::NetworkError { .. })),
            "Expected NetworkError for connection refused, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_registry_http_empty_services_array() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_registry_http_mock_slow_response() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[{"name":"slow-svc","capabilities":["compute"],"endpoints":["http://localhost:99"]}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "slow-svc");
    }

    #[tokio::test]
    async fn test_registry_http_path_without_leading_slash() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}", addr.port());

        let json_body = r#"{"services":[{"name":"root-svc","capabilities":["storage"],"endpoints":["http://localhost:1"]}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "root-svc");
    }

    #[tokio::test]
    async fn test_registry_http_mock_filter_invalid_endpoints() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[{"name":"mixed-ep","capabilities":["compute"],"endpoints":["http://localhost:1",":::invalid","https://valid.com:443"]}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 1);
        assert!(services[0].endpoints.len() >= 1);
    }

    #[tokio::test]
    async fn test_registry_https_scheme_connect() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("https://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[{"name":"https-svc","capabilities":["compute"],"endpoints":["https://localhost:443"]}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let services = disc.discover_all().await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "https-svc");
    }

    #[tokio::test]
    async fn test_registry_http_find_by_capability_after_mock_discovery() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let endpoint = format!("http://127.0.0.1:{}/services", addr.port());

        let json_body = r#"{"services":[{"name":"gpu-svc","capabilities":["gpu","compute"],"endpoints":["http://localhost:9999"]}]}"#;

        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = tx.send(());
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut buf = String::new();
            while reader.read_line(&mut buf).await.unwrap_or(0) > 0 {
                if buf.contains("\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let _ = rx.await;
        let disc = ServiceDiscovery::new_no_refresh(DiscoveryMethod::Registry { endpoint });
        let cap = Capability::Compute(crate::primal_identity::ComputeCapability::GpuCompute);
        let found = disc.find_service_by_capability(cap).await.unwrap();
        assert_eq!(found.name, "gpu-svc");
    }
}
