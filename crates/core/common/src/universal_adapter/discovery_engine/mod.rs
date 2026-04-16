// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery Engine - Multi-Source Capability Provider Discovery
//!
//! Discovers capability providers from multiple sources:
//! - mDNS (local network)
//! - Environment variables
//! - Configuration files
//! - Service registries (if available)
//!
//! NO hardcoded primal names or endpoints!

use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use super::capability_types::{CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint};
#[expect(deprecated)] // Protocol compatibility: platform path convention
use crate::constants::ecosystem::well_known::BIOMEOS;
use crate::constants::network::{HTTP_PROTOCOL, UNIX_SOCKET_URL_PREFIX};
use crate::{ToadStoolError, ToadStoolResult};

#[cfg(test)]
pub(crate) mod test_mocks;

mod discovery_source_dispatch;
pub use discovery_source_dispatch::DiscoverySourceDispatch;

#[cfg(test)]
mod tests;

/// Simplified registry entry for biomeos registry.json.
#[derive(Debug, Deserialize)]
struct RegistryServiceEntry {
    #[serde(alias = "provider_id", alias = "id")]
    provider_id: String,
    #[serde(alias = "endpoint", alias = "url", alias = "address")]
    endpoint: String,
    #[serde(default)]
    capability: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

/// Discovery engine that finds capability providers
pub struct DiscoveryEngine {
    sources: Vec<DiscoverySourceDispatch>,
    timeout: Duration,
}

impl DiscoveryEngine {
    /// Creates discovery engine with default sources (mDNS, environment, local registry).
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn with_defaults() -> ToadStoolResult<Self> {
        let mut sources: Vec<DiscoverySourceDispatch> = vec![
            DiscoverySourceDispatch::Environment(EnvironmentSource::new()),
            DiscoverySourceDispatch::LocalRegistry(LocalRegistrySource::new()),
        ];
        #[cfg(feature = "mdns")]
        sources.insert(0, DiscoverySourceDispatch::Mdns(MDnsSource::new()));
        Ok(Self {
            sources,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create discovery engine with custom sources.
    ///
    /// # Errors
    ///
    /// This constructor does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn new(sources: Vec<DiscoverySourceDispatch>) -> ToadStoolResult<Self> {
        Ok(Self {
            sources,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create discovery engine with no sources (for testing or manual source addition).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            sources: vec![],
            timeout: Duration::from_secs(1),
        }
    }

    /// # Errors
    ///
    /// Does not return errors; individual source failures are logged. Returns [`ToadStoolResult`] for API consistency.
    pub async fn discover_all(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let mut all_providers = Vec::new();
        let mut seen_ids = HashSet::new();

        for source in &self.sources {
            match tokio::time::timeout(self.timeout, source.discover()).await {
                Ok(Ok(providers)) => {
                    for provider in providers {
                        if seen_ids.insert(provider.provider_id.clone()) {
                            all_providers.push(provider);
                        }
                    }
                }
                Ok(Err(e)) => tracing::warn!("Discovery source failed: {}", e),
                Err(_) => tracing::warn!("Discovery source timed out"),
            }
        }

        Ok(all_providers)
    }

    /// Add a discovery source to the engine.
    pub fn add_source(&mut self, source: DiscoverySourceDispatch) {
        self.sources.push(source);
    }
}

/// Trait for capability provider discovery sources.
///
/// Implement this to add custom discovery backends (e.g., custom registries).
pub trait DiscoverySource: Send + Sync {
    /// Discover capability providers from this source.
    fn discover(
        &self,
    ) -> impl std::future::Future<Output = ToadStoolResult<Vec<CapabilityInfo>>> + Send + '_;
    /// Human-readable source name for logging.
    fn name(&self) -> &str;
}

/// mDNS/DNS-SD discovery source for local network capability providers.
#[cfg(feature = "mdns")]
pub struct MDnsSource {
    browse_timeout_secs: u64,
}

#[cfg(feature = "mdns")]
impl Default for MDnsSource {
    fn default() -> Self {
        Self {
            browse_timeout_secs: 2,
        }
    }
}

#[cfg(feature = "mdns")]
impl MDnsSource {
    /// Create mDNS source with default browse timeout.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create mDNS source with custom browse timeout in seconds.
    #[must_use]
    pub const fn with_timeout(secs: u64) -> Self {
        Self {
            browse_timeout_secs: secs,
        }
    }

    fn parse_txt_records(
        service_name: &str,
        host: &str,
        port: u16,
        txt: &HashMap<String, String>,
    ) -> CapabilityInfo {
        let provider_id = txt
            .get("provider_id")
            .cloned()
            .unwrap_or_else(|| service_name.to_string());

        let endpoint_str = txt.get("endpoint").map_or_else(|| "", String::as_str);
        let endpoint = if endpoint_str.is_empty() {
            ServiceEndpoint::Http(format!("{HTTP_PROTOCOL}{host}:{port}"))
        } else if let Ok(ep) = EnvironmentSource::parse_endpoint(endpoint_str) {
            ep
        } else {
            ServiceEndpoint::Http(format!("{HTTP_PROTOCOL}{host}:{port}"))
        };

        let capability_str = txt.get("capability").map_or("coordination", String::as_str);
        let capability = LocalRegistrySource::capability_from_str(capability_str);

        let metadata: HashMap<String, String> = txt
            .iter()
            .filter(|(k, _)| *k != "provider_id" && *k != "endpoint" && *k != "capability")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        CapabilityInfo {
            provider_id,
            capability,
            metadata,
            endpoint,
            health: HealthStatus::Unknown,
        }
    }
}

#[cfg(feature = "mdns")]
impl DiscoverySource for MDnsSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        use mdns_sd::{ServiceDaemon, ServiceEvent};
        use std::time::Instant;

        let mut providers = Vec::new();

        let mdns = match ServiceDaemon::new() {
            Ok(daemon) => daemon,
            Err(e) => {
                tracing::debug!("mDNS daemon unavailable: {} (continuing without mDNS)", e);
                return Ok(vec![]);
            }
        };

        let service_type = "_toadstool._tcp.local.";
        let receiver = match mdns.browse(service_type) {
            Ok(rx) => rx,
            Err(e) => {
                tracing::debug!("mDNS browse failed for {}: {}", service_type, e);
                let _ = mdns.shutdown();
                return Ok(vec![]);
            }
        };

        let timeout = Duration::from_secs(self.browse_timeout_secs);
        let start = Instant::now();

        while start.elapsed() < timeout {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if let ServiceEvent::ServiceResolved(info) = event {
                        let txt: HashMap<String, String> = info
                            .get_properties()
                            .iter()
                            .map(|p| (p.key().to_string(), p.val_str().to_string()))
                            .collect();

                        let host = info.get_hostname().trim_end_matches('.').to_string();
                        let port = info.get_port();

                        let cap_info =
                            Self::parse_txt_records(info.get_fullname(), &host, port, &txt);
                        tracing::debug!(
                            "mDNS discovered: {} at {}:{}",
                            cap_info.provider_id,
                            host,
                            port
                        );
                        providers.push(cap_info);
                    }
                }
                Err(e) => {
                    if format!("{e:?}").contains("Disconnected") {
                        break;
                    }
                }
            }
        }

        let _ = mdns.stop_browse(service_type);
        let _ = mdns.shutdown();

        tracing::debug!("mDNS discovery found {} providers", providers.len());
        Ok(providers)
    }

    fn name(&self) -> &'static str {
        "mdns"
    }
}

/// Environment variable discovery source (TOADSTOOL_*_PROVIDER vars).
#[derive(Default)]
pub struct EnvironmentSource {}

impl EnvironmentSource {
    /// Create environment source (reads TOADSTOOL_*_PROVIDER vars).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse_endpoint(url: &str) -> ToadStoolResult<ServiceEndpoint> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(ServiceEndpoint::Http(url.to_string()))
        } else if url.starts_with(UNIX_SOCKET_URL_PREFIX) {
            let path = url
                .strip_prefix(UNIX_SOCKET_URL_PREFIX)
                .ok_or_else(|| ToadStoolError::validation("Invalid unix socket URL".to_string()))?;
            Ok(ServiceEndpoint::UnixSocket(path.into()))
        } else if url.starts_with("tcp://") {
            let addr = url
                .strip_prefix("tcp://")
                .ok_or_else(|| ToadStoolError::validation("Invalid TCP URL".to_string()))?;
            let parts: Vec<&str> = addr.split(':').collect();
            if parts.len() != 2 {
                return Err(ToadStoolError::validation(
                    "TCP URL must be tcp://host:port".to_string(),
                ));
            }
            let port = parts[1]
                .parse()
                .map_err(|_| ToadStoolError::validation("Invalid port number".to_string()))?;
            Ok(ServiceEndpoint::Tcp {
                host: parts[0].to_string(),
                port,
            })
        } else {
            Ok(ServiceEndpoint::Custom {
                protocol: "unknown".to_string(),
                address: url.to_string(),
            })
        }
    }
}

impl DiscoverySource for EnvironmentSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let mut providers = Vec::new();

        if let Ok(url) = std::env::var("TOADSTOOL_SECURITY_PROVIDER") {
            if let Ok(endpoint) = Self::parse_endpoint(&url) {
                providers.push(CapabilityInfo {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                    capability: crate::universal_adapter::CapabilityType::Security {
                        features: vec![],
                        min_trust_level: crate::universal_adapter::TrustLevel::Medium,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint,
                    health: HealthStatus::Unknown,
                });
            }
        }

        if let Ok(url) = std::env::var("TOADSTOOL_STORAGE_PROVIDER") {
            if let Ok(endpoint) = Self::parse_endpoint(&url) {
                providers.push(CapabilityInfo {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                    capability: crate::universal_adapter::CapabilityType::Storage {
                        features: vec![],
                        min_throughput_mbps: None,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint,
                    health: HealthStatus::Unknown,
                });
            }
        }

        if let Ok(url) = std::env::var("TOADSTOOL_COORDINATION_PROVIDER") {
            if let Ok(endpoint) = Self::parse_endpoint(&url) {
                providers.push(CapabilityInfo {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                    capability: crate::universal_adapter::CapabilityType::Coordination {
                        features: vec![],
                        max_latency_ms: None,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint,
                    health: HealthStatus::Unknown,
                });
            }
        }

        if let Ok(url) = std::env::var("TOADSTOOL_INTELLIGENCE_PROVIDER") {
            if let Ok(endpoint) = Self::parse_endpoint(&url) {
                providers.push(CapabilityInfo {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                    capability: crate::universal_adapter::CapabilityType::Intelligence {
                        features: vec![],
                        model_types: vec![],
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint,
                    health: HealthStatus::Unknown,
                });
            }
        }

        tracing::debug!("Environment discovery found {} providers", providers.len());
        Ok(providers)
    }

    fn name(&self) -> &'static str {
        "environment"
    }
}

/// Local registry discovery source (XDG config dir / registry.json).
#[derive(Default)]
pub struct LocalRegistrySource {}

impl LocalRegistrySource {
    /// Create local registry source (reads XDG config / registry.json).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn parse_endpoint(url: &str) -> ToadStoolResult<ServiceEndpoint> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(ServiceEndpoint::Http(url.to_string()))
        } else if url.starts_with(UNIX_SOCKET_URL_PREFIX) {
            let path = url
                .strip_prefix(UNIX_SOCKET_URL_PREFIX)
                .ok_or_else(|| ToadStoolError::validation("Invalid unix socket URL".to_string()))?;
            Ok(ServiceEndpoint::UnixSocket(path.into()))
        } else if url.starts_with("tcp://") {
            let addr = url
                .strip_prefix("tcp://")
                .ok_or_else(|| ToadStoolError::validation("Invalid TCP URL".to_string()))?;
            let parts: Vec<&str> = addr.split(':').collect();
            if parts.len() != 2 {
                return Err(ToadStoolError::validation(
                    "TCP URL must be tcp://host:port".to_string(),
                ));
            }
            let port = parts[1]
                .parse()
                .map_err(|_| ToadStoolError::validation("Invalid port number".to_string()))?;
            Ok(ServiceEndpoint::Tcp {
                host: parts[0].to_string(),
                port,
            })
        } else {
            Ok(ServiceEndpoint::Custom {
                protocol: "unknown".to_string(),
                address: url.to_string(),
            })
        }
    }

    pub(crate) fn capability_from_str(s: &str) -> CapabilityType {
        match s.to_lowercase().as_str() {
            "security" => CapabilityType::Security {
                features: vec![],
                min_trust_level: super::capability_types::TrustLevel::Medium,
            },
            "storage" => CapabilityType::Storage {
                features: vec![],
                min_throughput_mbps: None,
            },
            "intelligence" => CapabilityType::Intelligence {
                features: vec![],
                model_types: vec![],
            },
            "compute" => CapabilityType::Compute {
                features: vec![],
                min_memory_gb: None,
            },
            "network" => CapabilityType::Network {
                features: vec![],
                min_bandwidth_mbps: None,
            },
            "monitoring" => CapabilityType::Monitoring {
                features: vec![],
                retention_days: None,
            },
            _ => CapabilityType::Coordination {
                features: vec![],
                max_latency_ms: None,
            },
        }
    }
}

impl DiscoverySource for LocalRegistrySource {
    #[expect(deprecated)] // BIOMEOS used for platform path convention
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{home}/.config")
        });
        let registry_path = Path::new(&config_dir).join(BIOMEOS).join("registry.json");

        if !registry_path.exists() {
            return Ok(vec![]);
        }

        match std::fs::read_to_string(&registry_path) {
            Ok(content) => match serde_json::from_str::<Vec<RegistryServiceEntry>>(&content) {
                Ok(entries) => {
                    let mut providers = Vec::with_capacity(entries.len());
                    for entry in entries {
                        match Self::parse_endpoint(&entry.endpoint) {
                            Ok(endpoint) => {
                                let capability = entry.capability.as_deref().map_or(
                                    CapabilityType::Coordination {
                                        features: vec![],
                                        max_latency_ms: None,
                                    },
                                    Self::capability_from_str,
                                );
                                providers.push(CapabilityInfo {
                                    provider_id: entry.provider_id,
                                    capability,
                                    metadata: entry.metadata,
                                    endpoint,
                                    health: HealthStatus::Unknown,
                                });
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Skipping registry entry {:?}: invalid endpoint - {}",
                                    entry.provider_id,
                                    e
                                );
                            }
                        }
                    }
                    tracing::debug!(
                        "Local registry discovered {} providers from {:?}",
                        providers.len(),
                        registry_path
                    );
                    Ok(providers)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse registry at {:?}: {}", registry_path, e);
                    Ok(vec![])
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read registry at {:?}: {}", registry_path, e);
                Ok(vec![])
            }
        }
    }

    fn name(&self) -> &'static str {
        "local_registry"
    }
}
