//! Discovery Engine - Multi-Source Capability Provider Discovery
//!
//! Discovers capability providers from multiple sources:
//! - mDNS (local network)
//! - Environment variables
//! - Configuration files
//! - Service registries (if available)
//!
//! NO hardcoded primal names or endpoints!

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use super::capability_types::{CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint};
use crate::{ToadStoolError, ToadStoolResult};

/// Simplified registry entry for biomeos registry.json.
/// Deserializes from JSON and converts to CapabilityInfo.
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
    sources: Vec<Box<dyn DiscoverySource>>,
    timeout: Duration,
}

impl DiscoveryEngine {
    /// Create discovery engine with default sources
    pub fn with_defaults() -> ToadStoolResult<Self> {
        let sources: Vec<Box<dyn DiscoverySource>> = vec![
            Box::new(MDnsSource::new()),
            Box::new(EnvironmentSource::new()),
            Box::new(LocalRegistrySource::new()),
        ];

        Ok(Self {
            sources,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create discovery engine with custom sources
    pub fn new(sources: Vec<Box<dyn DiscoverySource>>) -> ToadStoolResult<Self> {
        Ok(Self {
            sources,
            timeout: Duration::from_secs(5),
        })
    }

    /// Create an empty discovery engine (for testing)
    pub fn empty() -> Self {
        Self {
            sources: vec![],
            timeout: Duration::from_secs(1),
        }
    }

    /// Discover all available capability providers
    pub async fn discover_all(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let mut all_providers = Vec::new();
        let mut seen_ids = HashSet::new();

        for source in &self.sources {
            match tokio::time::timeout(self.timeout, source.discover()).await {
                Ok(Ok(providers)) => {
                    for provider in providers {
                        // Deduplicate by provider ID
                        if seen_ids.insert(provider.provider_id.clone()) {
                            all_providers.push(provider);
                        }
                    }
                }
                Ok(Err(e)) => {
                    // Log but continue with other sources
                    tracing::warn!("Discovery source failed: {}", e);
                }
                Err(_) => {
                    // Timeout - continue with other sources
                    tracing::warn!("Discovery source timed out");
                }
            }
        }

        Ok(all_providers)
    }

    /// Add a new discovery source at runtime
    pub fn add_source(&mut self, source: Box<dyn DiscoverySource>) {
        self.sources.push(source);
    }
}

/// Trait for discovery sources
///
/// Implement this to add new discovery mechanisms (k8s, consul, etc.)
/// without hardcoding them into the core system.
#[async_trait]
pub trait DiscoverySource: Send + Sync {
    /// Discover capability providers from this source
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>>;

    /// Name of this discovery source (for logging)
    fn name(&self) -> &str;
}

/// mDNS-based discovery source
///
/// Discovers capability providers on the local network via mDNS.
/// Providers advertise their capabilities via mDNS service records.
#[derive(Default)]
pub struct MDnsSource {
    // TODO: Integrate with mdns crate when available
}

impl MDnsSource {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DiscoverySource for MDnsSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        // mDNS discovery pending mdns-sd crate integration (network access required).
        // Leaving Ok(vec![]) until we add the external dependency.
        tracing::debug!("mDNS discovery pending mdns-sd integration; returning empty");
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "mdns"
    }
}

/// Environment variable-based discovery
///
/// Discovers providers from environment variables:
/// - TOADSTOOL_SECURITY_PROVIDER=http://localhost:9000
/// - TOADSTOOL_STORAGE_PROVIDER=unix:///var/run/storage.sock
/// - etc.
#[derive(Default)]
pub struct EnvironmentSource {
    // Configuration if needed
}

impl EnvironmentSource {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse_endpoint(url: &str) -> ToadStoolResult<ServiceEndpoint> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(ServiceEndpoint::Http(url.to_string()))
        } else if url.starts_with("unix://") {
            let path = url
                .strip_prefix("unix://")
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

#[async_trait]
impl DiscoverySource for EnvironmentSource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let mut providers = Vec::new();

        // Check for security provider
        if let Ok(url) = std::env::var("TOADSTOOL_SECURITY_PROVIDER") {
            if let Ok(endpoint) = Self::parse_endpoint(&url) {
                providers.push(CapabilityInfo {
                    provider_id: uuid::Uuid::new_v4().to_string(),
                    capability: crate::universal_adapter::CapabilityType::Security {
                        features: vec![], // Will be queried from provider
                        min_trust_level: crate::universal_adapter::TrustLevel::Medium,
                    },
                    metadata: std::collections::HashMap::new(),
                    endpoint,
                    health: HealthStatus::Unknown,
                });
            }
        }

        // Check for storage provider
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

        // Check for coordination provider
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

        // Check for intelligence provider
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

    fn name(&self) -> &str {
        "environment"
    }
}

/// Local registry file-based discovery
///
/// Reads from biomeos registry: `~/.config/biomeos/registry.json` or
/// `$XDG_CONFIG_HOME/biomeos/registry.json`.
#[derive(Default)]
pub struct LocalRegistrySource {
    // Configuration if needed
}

impl LocalRegistrySource {
    pub fn new() -> Self {
        Self::default()
    }

    fn parse_endpoint(url: &str) -> ToadStoolResult<ServiceEndpoint> {
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(ServiceEndpoint::Http(url.to_string()))
        } else if url.starts_with("unix://") {
            let path = url
                .strip_prefix("unix://")
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

    fn capability_from_str(s: &str) -> CapabilityType {
        match s.to_lowercase().as_str() {
            "security" => CapabilityType::Security {
                features: vec![],
                min_trust_level: super::capability_types::TrustLevel::Medium,
            },
            "storage" => CapabilityType::Storage {
                features: vec![],
                min_throughput_mbps: None,
            },
            "coordination" => CapabilityType::Coordination {
                features: vec![],
                max_latency_ms: None,
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

#[async_trait]
impl DiscoverySource for LocalRegistrySource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{home}/.config")
        });
        let registry_path = Path::new(&config_dir).join("biomeos/registry.json");

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

    fn name(&self) -> &str {
        "local_registry"
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn test_discovery_engine_creation() {
        let engine = DiscoveryEngine::with_defaults();
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_empty_discovery() {
        let engine = DiscoveryEngine::empty();
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 0);
    }

    #[tokio::test]
    async fn test_discovery_engine_new_with_custom_sources() {
        let sources: Vec<Box<dyn DiscoverySource>> = vec![Box::new(MDnsSource::new())];
        let engine = DiscoveryEngine::new(sources).unwrap();
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 0);
    }

    #[tokio::test]
    async fn test_add_source() {
        let mut engine = DiscoveryEngine::empty();
        engine.add_source(Box::new(MDnsSource::new()));
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 0);
    }

    #[tokio::test]
    async fn test_discover_all_deduplication() {
        struct MockSource;
        #[async_trait::async_trait]
        impl DiscoverySource for MockSource {
            async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
                Ok(vec![
                    CapabilityInfo {
                        provider_id: "dup-1".to_string(),
                        capability: CapabilityType::Storage {
                            features: vec![],
                            min_throughput_mbps: None,
                        },
                        metadata: std::collections::HashMap::new(),
                        endpoint: ServiceEndpoint::Http("http://a".to_string()),
                        health: HealthStatus::Unknown,
                    },
                    CapabilityInfo {
                        provider_id: "dup-1".to_string(),
                        capability: CapabilityType::Storage {
                            features: vec![],
                            min_throughput_mbps: None,
                        },
                        metadata: std::collections::HashMap::new(),
                        endpoint: ServiceEndpoint::Http("http://b".to_string()),
                        health: HealthStatus::Unknown,
                    },
                ])
            }
            fn name(&self) -> &str {
                "mock"
            }
        }
        let engine = DiscoveryEngine::new(vec![Box::new(MockSource)]).unwrap();
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 1, "Should deduplicate by provider_id");
        assert_eq!(providers[0].provider_id, "dup-1");
    }

    #[tokio::test]
    async fn test_discover_all_source_error() {
        struct FailingSource;
        #[async_trait::async_trait]
        impl DiscoverySource for FailingSource {
            async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
                Err(ToadStoolError::configuration("config error".to_string()))
            }
            fn name(&self) -> &str {
                "failing"
            }
        }
        let engine = DiscoveryEngine::new(vec![Box::new(FailingSource)]).unwrap();
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 0, "Should continue past failing source");
    }

    #[tokio::test]
    async fn test_discover_all_timeout() {
        struct SlowSource;
        #[async_trait::async_trait]
        impl DiscoverySource for SlowSource {
            async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok(vec![])
            }
            fn name(&self) -> &str {
                "slow"
            }
        }
        let mut engine = DiscoveryEngine::empty();
        engine.add_source(Box::new(SlowSource));
        // empty() has 1s timeout, slow source sleeps 10s - will timeout
        let providers = engine.discover_all().await.unwrap();
        assert_eq!(providers.len(), 0, "Should handle timeout gracefully");
    }

    #[tokio::test]
    async fn test_environment_source_parsing() {
        let endpoint = EnvironmentSource::parse_endpoint("http://localhost:8080");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Http(_)));

        let endpoint = EnvironmentSource::parse_endpoint("https://example.com");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Http(_)));

        let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost:9000");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Tcp { .. }));

        let endpoint = EnvironmentSource::parse_endpoint("unix:///var/run/test.sock");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::UnixSocket(_)));

        let endpoint = EnvironmentSource::parse_endpoint("custom://something");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Custom { .. }));
    }

    #[tokio::test]
    async fn test_environment_source_parse_endpoint_errors() {
        // TCP without port (host:port format required)
        let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost");
        assert!(endpoint.is_err());

        // TCP with invalid port number
        let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost:invalid");
        assert!(endpoint.is_err());
    }

    #[tokio::test]
    async fn test_environment_discovery() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("TOADSTOOL_SECURITY_PROVIDER", "http://localhost:9000");

        let source = EnvironmentSource::new();
        let providers = source.discover().await.unwrap();

        assert!(
            !providers.is_empty(),
            "Should find at least one provider from env"
        );

        std::env::remove_var("TOADSTOOL_SECURITY_PROVIDER");
    }

    #[tokio::test]
    async fn test_environment_discovery_storage_provider() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("TOADSTOOL_STORAGE_PROVIDER", "http://localhost:8083");

        let source = EnvironmentSource::new();
        let providers = source.discover().await.unwrap();

        assert!(
            !providers.is_empty(),
            "Should find storage provider from env"
        );
        std::env::remove_var("TOADSTOOL_STORAGE_PROVIDER");
    }

    #[tokio::test]
    async fn test_environment_discovery_coordination_provider() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("TOADSTOOL_COORDINATION_PROVIDER", "tcp://host:1234");

        let source = EnvironmentSource::new();
        let providers = source.discover().await.unwrap();

        assert!(!providers.is_empty(), "Should find coordination provider");
        std::env::remove_var("TOADSTOOL_COORDINATION_PROVIDER");
    }

    #[tokio::test]
    async fn test_environment_discovery_intelligence_provider() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("TOADSTOOL_INTELLIGENCE_PROVIDER", "unix:///tmp/ai.sock");

        let source = EnvironmentSource::new();
        let providers = source.discover().await.unwrap();

        assert!(!providers.is_empty(), "Should find intelligence provider");
        std::env::remove_var("TOADSTOOL_INTELLIGENCE_PROVIDER");
    }

    #[tokio::test]
    async fn test_mdns_source() {
        let source = MDnsSource::new();
        assert_eq!(source.name(), "mdns");
        let providers = source.discover().await.unwrap();
        assert_eq!(providers.len(), 0, "mDNS not yet implemented");
    }

    #[tokio::test]
    async fn test_local_registry_source() {
        let source = LocalRegistrySource::new();
        assert_eq!(source.name(), "local_registry");
        let providers = source.discover().await.unwrap();
        assert!(providers.iter().all(|p| !p.provider_id.is_empty()) || providers.is_empty());
    }

    #[tokio::test]
    async fn test_local_registry_capability_from_str() {
        let cap = LocalRegistrySource::capability_from_str("security");
        assert!(matches!(cap, CapabilityType::Security { .. }));

        let cap = LocalRegistrySource::capability_from_str("compute");
        assert!(matches!(cap, CapabilityType::Compute { .. }));

        let cap = LocalRegistrySource::capability_from_str("unknown");
        assert!(matches!(cap, CapabilityType::Coordination { .. }));
    }

    #[tokio::test]
    async fn test_local_registry_with_valid_file() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let config_dir = temp_dir.join("toadstool_test_config");
        let biomeos_dir = config_dir.join("biomeos");
        std::fs::create_dir_all(&biomeos_dir).unwrap();
        let registry_path = biomeos_dir.join("registry.json");
        std::fs::write(
            &registry_path,
            r#"[{"provider_id":"p1","endpoint":"http://localhost:8080","capability":"storage"}]"#,
        )
        .unwrap();

        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", config_dir.to_str().unwrap());

        let source = LocalRegistrySource::new();
        let providers = source.discover().await.unwrap();

        if let Some(ref xdg) = old_xdg {
            std::env::set_var("XDG_CONFIG_HOME", xdg);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        std::fs::remove_dir_all(&config_dir).ok();

        assert!(!providers.is_empty(), "Should discover from registry file");
        assert_eq!(providers[0].provider_id, "p1");
    }

    #[tokio::test]
    async fn test_local_registry_invalid_json() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir();
        let config_dir = temp_dir.join("toadstool_test_config2");
        let biomeos_dir = config_dir.join("biomeos");
        std::fs::create_dir_all(&biomeos_dir).unwrap();
        std::fs::write(biomeos_dir.join("registry.json"), "not valid json").unwrap();

        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", config_dir.to_str().unwrap());

        let source = LocalRegistrySource::new();
        let providers = source.discover().await.unwrap();

        if let Some(ref xdg) = old_xdg {
            std::env::set_var("XDG_CONFIG_HOME", xdg);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        std::fs::remove_dir_all(&config_dir).ok();

        assert!(providers.is_empty(), "Invalid JSON should return empty");
    }
}
