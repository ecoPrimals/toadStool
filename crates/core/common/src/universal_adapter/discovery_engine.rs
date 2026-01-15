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
use std::collections::HashSet;
use std::time::Duration;

use super::capability_types::{CapabilityInfo, HealthStatus, ServiceEndpoint};
use crate::{ToadStoolError, ToadStoolResult};

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
        // TODO: Implement actual mDNS discovery
        // For now, return empty list
        tracing::debug!("mDNS discovery not yet implemented");
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
/// Discovers providers from a local configuration file.
/// Default location: ~/.toadstool/providers.toml or /etc/toadstool/providers.toml
#[derive(Default)]
pub struct LocalRegistrySource {
    // Configuration if needed
}

impl LocalRegistrySource {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DiscoverySource for LocalRegistrySource {
    async fn discover(&self) -> ToadStoolResult<Vec<CapabilityInfo>> {
        // TODO: Implement local registry file reading
        // For now, return empty list
        tracing::debug!("Local registry discovery not yet implemented");
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "local_registry"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn test_environment_source_parsing() {
        let endpoint = EnvironmentSource::parse_endpoint("http://localhost:8080");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Http(_)));

        let endpoint = EnvironmentSource::parse_endpoint("tcp://localhost:9000");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::Tcp { .. }));

        let endpoint = EnvironmentSource::parse_endpoint("unix:///var/run/test.sock");
        assert!(endpoint.is_ok());
        assert!(matches!(endpoint.unwrap(), ServiceEndpoint::UnixSocket(_)));
    }

    #[tokio::test]
    async fn test_environment_discovery() {
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
    async fn test_mdns_source() {
        let source = MDnsSource::new();
        let providers = source.discover().await.unwrap();
        assert_eq!(providers.len(), 0, "mDNS not yet implemented");
    }

    #[tokio::test]
    async fn test_local_registry_source() {
        let source = LocalRegistrySource::new();
        let providers = source.discover().await.unwrap();
        assert_eq!(providers.len(), 0, "Local registry not yet implemented");
    }
}
