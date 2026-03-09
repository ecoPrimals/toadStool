// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Capability-based discovery system - zero hardcoded primal names
//!
//! This module implements pure capability-based service discovery where
//! `ToadStool` discovers services by what they do, not by who they are.
//!
//! # Core Principle
//! **"Each primal knows only itself. Everything else is discovered."**

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Service endpoint discovered through capability matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Capability this service provides
    pub capability: String,

    /// Endpoint URL (protocol-agnostic)
    pub endpoint: String,

    /// Supported protocols (http, rpc, grpc, etc.)
    pub protocols: Vec<String>,

    /// Service metadata
    pub metadata: ServiceMetadata,

    /// Discovery source (how we found it)
    pub source: DiscoverySource,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    /// Service version
    pub version: Option<String>,

    /// Service health status
    pub health: ServiceHealth,

    /// Last seen timestamp
    pub last_seen: std::time::SystemTime,

    /// Priority/preference score (0-100)
    pub priority: u8,

    /// Additional arbitrary metadata
    pub extra: std::collections::HashMap<String, String>,
}

impl Default for ServiceMetadata {
    fn default() -> Self {
        Self {
            version: None,
            health: ServiceHealth::Unknown,
            last_seen: std::time::SystemTime::now(),
            priority: 50,
            extra: std::collections::HashMap::new(),
        }
    }
}

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceHealth {
    Unknown,
    Degraded,
    Healthy,
}

/// Source of service discovery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Discovered via environment variable
    Environment,

    /// Discovered via mDNS/Bonjour
    MDNS,

    /// Discovered via service mesh (consul, etcd, etc.)
    ServiceMesh(String),

    /// Discovered via configuration file
    ConfigFile,

    /// Using fallback default
    Fallback,

    /// Discovered via universal adapter
    UniversalAdapter,
}

/// Capability discovery preferences
#[derive(Debug, Clone)]
pub struct DiscoveryPreferences {
    /// Prefer local services
    pub prefer_local: bool,

    /// Required protocols (empty = any)
    pub required_protocols: Vec<String>,

    /// Timeout for discovery
    pub timeout: Option<Duration>,

    /// Minimum health level
    pub min_health: ServiceHealth,

    /// Preferred discovery sources (in order)
    pub preferred_sources: Vec<DiscoverySource>,
}

impl Default for DiscoveryPreferences {
    fn default() -> Self {
        Self {
            prefer_local: false,
            required_protocols: Vec::new(),
            timeout: None,
            min_health: ServiceHealth::Unknown,
            preferred_sources: Vec::new(),
        }
    }
}

/// Capability discovery trait - implemented by discovery engines
pub trait CapabilityDiscovery: Send + Sync {
    /// Discover a service by capability name (not primal name!)
    ///
    /// # Examples
    /// ```ignore
    /// // ✅ GOOD - capability-based
    /// let ai = discovery.discover("ai_processing").await?;
    /// let auth = discovery.discover("authentication").await?;
    ///
    /// // ❌ BAD - primal name hardcoding
    /// // let songbird = SongbirdClient::new();  // DON'T DO THIS
    /// ```
    fn discover(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    >;

    /// Discover with preferences
    fn discover_with_preferences(
        &self,
        capability: &str,
        preferences: DiscoveryPreferences,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    >;

    /// Discover all services providing a capability
    fn discover_all(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<DiscoveredService>, DiscoveryError>>
                + Send
                + '_,
        >,
    >;

    /// Check if a capability is available
    fn is_available<'a>(
        &'a self,
        capability: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move { self.discover(capability).await.is_ok() })
    }
}

/// Errors during capability discovery
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Capability '{0}' not found")]
    CapabilityNotFound(String),

    #[error("Discovery timeout after {0:?}")]
    Timeout(Duration),

    #[error("No healthy services found for capability '{0}'")]
    NoHealthyServices(String),

    #[error("Protocol '{0}' not supported by any discovered service")]
    ProtocolNotSupported(String),

    #[error("Discovery source failed: {0}")]
    SourceFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Standard capability names - expand as needed
#[allow(clippy::module_inception)]
pub mod capabilities {
    /// AI processing and machine learning
    pub const AI_PROCESSING: &str = "ai_processing";

    /// Natural language processing
    pub const NLP: &str = "natural_language_processing";

    /// Authentication and authorization
    pub const AUTHENTICATION: &str = "authentication";

    /// Authorization and access control
    pub const AUTHORIZATION: &str = "authorization";

    /// Persistent data storage
    pub const STORAGE: &str = "persistent_storage";

    /// Key-value storage
    pub const KEY_VALUE_STORE: &str = "key_value_storage";

    /// Service orchestration
    pub const ORCHESTRATION: &str = "service_orchestration";

    /// Load balancing
    pub const LOAD_BALANCING: &str = "load_balancing";

    /// Service mesh coordination
    pub const SERVICE_MESH: &str = "service_mesh";

    /// Monitoring and observability
    pub const MONITORING: &str = "monitoring";

    /// Distributed tracing
    pub const TRACING: &str = "distributed_tracing";

    /// Secret management
    pub const SECRETS: &str = "secret_management";

    /// Certificate authority
    pub const PKI: &str = "public_key_infrastructure";

    /// Message queue
    pub const MESSAGE_QUEUE: &str = "message_queue";

    /// Event streaming
    pub const EVENT_STREAM: &str = "event_streaming";

    /// Cache service
    pub const CACHE: &str = "caching";

    /// Search indexing
    pub const SEARCH: &str = "search_indexing";
}

/// Substrate capability detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstrateCapability {
    /// Container orchestration (k8s, nomad, etc.)
    ContainerOrchestration,

    /// Container runtime (docker, podman, containerd)
    ContainerRuntime,

    /// Service mesh (consul, linkerd, istio)
    ServiceMesh,

    /// Service discovery (consul, etcd, zookeeper)
    ServiceDiscovery,

    /// Cloud compute (AWS, GCP, Azure, etc.)
    CloudCompute,

    /// Bare metal / no orchestration
    BareMetal,
}

/// Detected substrate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSubstrate {
    /// Type of substrate detected
    pub substrate_type: SubstrateType,

    /// Capabilities this substrate provides
    pub capabilities: Vec<SubstrateCapability>,

    /// Substrate-specific metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Types of substrates (detected, not hardcoded!)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubstrateType {
    /// Container orchestrator detected (could be k8s, nomad, etc.)
    ContainerOrchestrator,

    /// Container runtime detected (docker, podman, etc.)
    ContainerRuntime,

    /// Cloud environment detected
    Cloud,

    /// Bare metal / direct execution
    Bare,
}

impl DetectedSubstrate {
    /// Check if substrate has a capability
    #[must_use]
    pub fn has_capability(&self, capability: &SubstrateCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get substrate-specific metadata value
    #[must_use]
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Substrate detection trait - implemented by specific detectors
///
/// Migrated from `async_trait` to native async for zero-cost abstraction.
pub trait SubstrateDetector: Send + Sync {
    /// Try to detect this substrate type
    fn detect(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<DetectedSubstrate>, DiscoveryError>>
                + Send
                + '_,
        >,
    >;

    /// Name of this detector (for logging)
    fn name(&self) -> &str;
}

/// Endpoint resolution chain - tries multiple sources
pub struct EndpointResolver {
    sources: Vec<Box<dyn EndpointSource>>,
}

impl EndpointResolver {
    /// Create new resolver with default sources
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add an endpoint source to the resolution chain
    pub fn add_source(&mut self, source: Box<dyn EndpointSource>) {
        self.sources.push(source);
    }

    /// Resolve an endpoint by trying each source in order
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError::CapabilityNotFound` if no source can resolve the service.
    pub async fn resolve(&self, service: &str) -> Result<String, DiscoveryError> {
        for source in &self.sources {
            if let Some(endpoint) = source.resolve(service).await? {
                return Ok(endpoint);
            }
        }
        Err(DiscoveryError::CapabilityNotFound(service.to_string()))
    }
}

impl Default for EndpointResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Endpoint source trait - one way to find service endpoints
///
/// Migrated from `async_trait` to native async for zero-cost abstraction.
pub trait EndpointSource: Send + Sync {
    /// Try to resolve service endpoint from this source
    fn resolve(
        &self,
        service: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>,
    >;

    /// Source name (for logging/debugging)
    fn source_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_capability_names() {
        // Ensure capability names are stable
        assert_eq!(capabilities::AI_PROCESSING, "ai_processing");
        assert_eq!(capabilities::AUTHENTICATION, "authentication");
        assert_eq!(capabilities::STORAGE, "persistent_storage");
    }

    #[test]
    fn test_substrate_capabilities() {
        let substrate = DetectedSubstrate {
            substrate_type: SubstrateType::ContainerOrchestrator,
            capabilities: vec![
                SubstrateCapability::ContainerOrchestration,
                SubstrateCapability::ServiceDiscovery,
            ],
            metadata: std::collections::HashMap::new(),
        };

        assert!(substrate.has_capability(&SubstrateCapability::ContainerOrchestration));
        assert!(substrate.has_capability(&SubstrateCapability::ServiceDiscovery));
        assert!(!substrate.has_capability(&SubstrateCapability::CloudCompute));
    }

    #[test]
    fn test_discovered_service_serialization() {
        let service = DiscoveredService {
            capability: "test_capability".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            protocols: vec!["http".to_string(), "grpc".to_string()],
            metadata: ServiceMetadata {
                version: Some("1.0.0".to_string()),
                health: ServiceHealth::Healthy,
                last_seen: std::time::SystemTime::now(),
                priority: 80,
                extra: HashMap::new(),
            },
            source: DiscoverySource::Environment,
        };

        let json = serde_json::to_string(&service).expect("Failed to serialize");
        let deserialized: DiscoveredService =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.capability, service.capability);
        assert_eq!(deserialized.endpoint, service.endpoint);
        assert_eq!(deserialized.protocols.len(), 2);
    }

    #[test]
    fn test_service_health_variants() {
        assert_eq!(ServiceHealth::Healthy, ServiceHealth::Healthy);
        assert_ne!(ServiceHealth::Healthy, ServiceHealth::Degraded);
        assert_ne!(ServiceHealth::Degraded, ServiceHealth::Unknown);
    }

    #[test]
    fn test_discovery_source_variants() {
        let env = DiscoverySource::Environment;
        let mdns = DiscoverySource::MDNS;
        let mesh = DiscoverySource::ServiceMesh("consul".to_string());
        let config = DiscoverySource::ConfigFile;
        let fallback = DiscoverySource::Fallback;
        let adapter = DiscoverySource::UniversalAdapter;

        assert_eq!(env, DiscoverySource::Environment);
        assert_ne!(env, mdns);
        assert_ne!(mesh, config);
        assert_ne!(fallback, adapter);
    }

    #[test]
    fn test_discovery_source_serialization() {
        let sources = vec![
            DiscoverySource::Environment,
            DiscoverySource::MDNS,
            DiscoverySource::ServiceMesh("etcd".to_string()),
            DiscoverySource::ConfigFile,
            DiscoverySource::Fallback,
            DiscoverySource::UniversalAdapter,
        ];

        for source in sources {
            let json = serde_json::to_string(&source).expect("Failed to serialize");
            let _deserialized: DiscoverySource =
                serde_json::from_str(&json).expect("Failed to deserialize");
        }
    }

    #[test]
    fn test_discovery_preferences_default() {
        let prefs = DiscoveryPreferences::default();

        assert!(!prefs.prefer_local);
        assert!(prefs.required_protocols.is_empty());
        assert!(prefs.timeout.is_none());
        assert_eq!(prefs.min_health, ServiceHealth::Unknown);
        assert!(prefs.preferred_sources.is_empty());
    }

    #[test]
    fn test_discovery_preferences_with_values() {
        let prefs = DiscoveryPreferences {
            prefer_local: true,
            required_protocols: vec!["grpc".to_string()],
            timeout: Some(Duration::from_secs(5)),
            min_health: ServiceHealth::Healthy,
            preferred_sources: vec![DiscoverySource::Environment],
        };

        assert!(prefs.prefer_local);
        assert_eq!(prefs.required_protocols.len(), 1);
        assert_eq!(prefs.timeout, Some(Duration::from_secs(5)));
        assert_eq!(prefs.min_health, ServiceHealth::Healthy);
    }

    #[test]
    fn test_substrate_capability_variants() {
        let caps = [
            SubstrateCapability::ContainerOrchestration,
            SubstrateCapability::ContainerRuntime,
            SubstrateCapability::ServiceMesh,
            SubstrateCapability::ServiceDiscovery,
            SubstrateCapability::CloudCompute,
            SubstrateCapability::BareMetal,
        ];

        assert_eq!(caps.len(), 6);
        assert_eq!(caps[0], SubstrateCapability::ContainerOrchestration);
    }

    #[test]
    fn test_substrate_type_variants() {
        let types = [
            SubstrateType::ContainerOrchestrator,
            SubstrateType::ContainerRuntime,
            SubstrateType::Cloud,
            SubstrateType::Bare,
        ];

        assert_eq!(types.len(), 4);
        assert_eq!(types[3], SubstrateType::Bare);
    }

    #[test]
    fn test_detected_substrate_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.20.0".to_string());
        metadata.insert("provider".to_string(), "k8s".to_string());

        let substrate = DetectedSubstrate {
            substrate_type: SubstrateType::ContainerOrchestrator,
            capabilities: vec![SubstrateCapability::ContainerOrchestration],
            metadata,
        };

        assert_eq!(
            substrate.get_metadata("version"),
            Some(&"1.20.0".to_string())
        );
        assert_eq!(substrate.get_metadata("provider"), Some(&"k8s".to_string()));
        assert_eq!(substrate.get_metadata("missing"), None);
    }

    #[test]
    fn test_detected_substrate_serialization() {
        let substrate = DetectedSubstrate {
            substrate_type: SubstrateType::Cloud,
            capabilities: vec![
                SubstrateCapability::CloudCompute,
                SubstrateCapability::ServiceDiscovery,
            ],
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&substrate).expect("Failed to serialize");
        let deserialized: DetectedSubstrate =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.substrate_type, SubstrateType::Cloud);
        assert_eq!(deserialized.capabilities.len(), 2);
    }

    #[test]
    fn test_endpoint_resolver_creation() {
        let resolver = EndpointResolver::new();
        assert_eq!(resolver.sources.len(), 0);

        let default_resolver = EndpointResolver::default();
        assert_eq!(default_resolver.sources.len(), 0);
    }

    #[test]
    fn test_discovery_error_variants() {
        let err1 = DiscoveryError::CapabilityNotFound("test".to_string());
        let err2 = DiscoveryError::Timeout(Duration::from_secs(30));
        let err3 = DiscoveryError::NoHealthyServices("test".to_string());
        let err4 = DiscoveryError::ProtocolNotSupported("mqtt".to_string());
        let err5 = DiscoveryError::SourceFailed("mdns error".to_string());
        let err6 = DiscoveryError::ConfigError("invalid config".to_string());

        assert_eq!(err1.to_string(), "Capability 'test' not found");
        assert!(err2.to_string().contains("Discovery timeout"));
        assert!(err3.to_string().contains("No healthy services"));
        assert!(err4.to_string().contains("Protocol 'mqtt' not supported"));
        assert!(err5.to_string().contains("Discovery source failed"));
        assert!(err6.to_string().contains("Configuration error"));
    }

    #[test]
    fn test_capability_constants() {
        // Verify all standard capabilities are defined with expected values
        assert_eq!(capabilities::AI_PROCESSING, "ai_processing");
        assert_eq!(capabilities::NLP, "natural_language_processing");
        assert_eq!(capabilities::AUTHENTICATION, "authentication");
        assert_eq!(capabilities::AUTHORIZATION, "authorization");
        assert_eq!(capabilities::STORAGE, "persistent_storage");
        assert_eq!(capabilities::KEY_VALUE_STORE, "key_value_storage");
        assert_eq!(capabilities::ORCHESTRATION, "service_orchestration");
        assert_eq!(capabilities::LOAD_BALANCING, "load_balancing");
        assert_eq!(capabilities::SERVICE_MESH, "service_mesh");
        assert_eq!(capabilities::MONITORING, "monitoring");
        assert_eq!(capabilities::TRACING, "distributed_tracing");
        assert_eq!(capabilities::SECRETS, "secret_management");
        assert_eq!(capabilities::PKI, "public_key_infrastructure");
        assert_eq!(capabilities::MESSAGE_QUEUE, "message_queue");
        assert_eq!(capabilities::EVENT_STREAM, "event_streaming");
        assert_eq!(capabilities::CACHE, "caching");
        assert_eq!(capabilities::SEARCH, "search_indexing");
    }

    #[test]
    fn test_service_metadata_priority_range() {
        let metadata = ServiceMetadata {
            version: None,
            health: ServiceHealth::Healthy,
            last_seen: std::time::SystemTime::now(),
            priority: 95,
            extra: HashMap::new(),
        };

        assert!(metadata.priority <= 100);
        // Note: priority is u8, so >= 0 check is redundant
    }
}
