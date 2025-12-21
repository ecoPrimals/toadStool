//! Primal Identity System - Self-Knowledge Only
//!
//! This module implements the principle that each primal only knows about itself.
//! Other primals are discovered at runtime through capability-based discovery.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge**: Each primal knows only its own identity
//! - **Runtime Discovery**: Other primals discovered via capabilities
//! - **Zero Hardcoding**: No primal-specific code or configuration
//! - **Capability-Based**: Services matched by what they can do, not who they are

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Primal identity trait - defines what a primal knows about itself
pub trait PrimalIdentity: Send + Sync {
    /// Get the primal's name (e.g., "toadstool", "songbird")
    fn primal_name(&self) -> &'static str;

    /// Get the primal's version
    fn version(&self) -> &str;

    /// Get the primal's capabilities
    fn capabilities(&self) -> Vec<Capability>;

    /// Get the primal's endpoints
    fn endpoints(&self) -> Vec<ServiceEndpoint>;

    /// Get additional metadata
    fn metadata(&self) -> HashMap<String, String>;
}

/// Capability that a primal can provide
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    // Compute capabilities
    Compute(ComputeCapability),

    // Storage capabilities
    Storage(StorageCapability),

    // Authentication capabilities
    Authentication(AuthCapability),

    // Coordination capabilities
    Coordination(CoordinationCapability),

    // Discovery capabilities
    Discovery(DiscoveryCapability),

    // Custom capability
    Custom { name: String, version: String },
}

/// Compute capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeCapability {
    /// Native binary execution
    NativeExecution,

    /// Container orchestration (Docker, Podman)
    ContainerOrchestration,

    /// WebAssembly execution
    WasmExecution,

    /// Python runtime
    PythonExecution,

    /// GPU compute
    GpuCompute,

    /// Edge device execution
    EdgeExecution,

    /// Specialty hardware (mainframe, embedded)
    SpecialtyHardware,
}

/// Storage capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageCapability {
    /// Object storage
    ObjectStorage,

    /// Block storage
    BlockStorage,

    /// File storage
    FileStorage,

    /// Database
    Database,

    /// Cache
    Cache,
}

/// Authentication capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthCapability {
    /// User authentication
    UserAuth,

    /// Service authentication
    ServiceAuth,

    /// Token management
    TokenManagement,

    /// Cryptographic operations
    CryptoOperations,
}

/// Coordination capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CoordinationCapability {
    #[default]
    /// Service discovery
    ServiceDiscovery,

    /// Load balancing
    LoadBalancing,

    /// Health checking
    HealthChecking,

    /// Configuration management
    ConfigManagement,

    /// Workflow orchestration
    WorkflowOrchestration,
}

/// Discovery capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoveryCapability {
    /// Capability-based discovery
    CapabilityDiscovery,

    /// DNS-based discovery
    DnsDiscovery,

    /// mDNS/Bonjour
    MdnsDiscovery,

    /// Registry-based
    RegistryDiscovery,
}

/// Service endpoint information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Protocol (http, https, grpc, ws, wss)
    pub protocol: String,

    /// Address (can be hostname or IP)
    pub address: String,

    /// Port number
    pub port: u16,

    /// Path (optional, for HTTP-based protocols)
    pub path: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ServiceEndpoint {
    /// Create a new HTTP endpoint
    pub fn http(host: impl Into<String>, port: u16) -> Self {
        Self {
            protocol: "http".to_string(),
            address: host.into(),
            port,
            path: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new HTTPS endpoint
    pub fn https(host: impl Into<String>, port: u16) -> Self {
        Self {
            protocol: "https".to_string(),
            address: host.into(),
            port,
            path: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new gRPC endpoint
    pub fn grpc(host: impl Into<String>, port: u16) -> Self {
        Self {
            protocol: "grpc".to_string(),
            address: host.into(),
            port,
            path: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new `WebSocket` endpoint
    pub fn websocket(host: impl Into<String>, port: u16) -> Self {
        Self {
            protocol: "ws".to_string(),
            address: host.into(),
            port,
            path: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the path for this endpoint
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add metadata
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get the full URL for this endpoint
    #[must_use]
    pub fn url(&self) -> String {
        let path = self.path.as_deref().unwrap_or("");
        format!("{}://{}:{}{}", self.protocol, self.address, self.port, path)
    }
}

/// ToadStool's self-knowledge implementation
#[derive(Debug, Clone)]
pub struct ToadStoolIdentity {
    /// Version from Cargo.toml
    version: String,

    /// Capabilities we provide
    capabilities: Vec<Capability>,

    /// Our service endpoints
    endpoints: Vec<ServiceEndpoint>,

    /// Additional metadata
    metadata: HashMap<String, String>,
}

impl ToadStoolIdentity {
    /// Create a new ToadStool identity
    #[must_use]
    pub fn new() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert(
            "description".to_string(),
            "Universal Compute Platform".to_string(),
        );
        metadata.insert("platform".to_string(), std::env::consts::OS.to_string());
        metadata.insert("arch".to_string(), std::env::consts::ARCH.to_string());

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: Self::default_capabilities(),
            endpoints: Vec::new(),
            metadata,
        }
    }

    /// Get default capabilities for ToadStool
    fn default_capabilities() -> Vec<Capability> {
        vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Compute(ComputeCapability::ContainerOrchestration),
            Capability::Compute(ComputeCapability::WasmExecution),
            Capability::Compute(ComputeCapability::PythonExecution),
            Capability::Compute(ComputeCapability::GpuCompute),
        ]
    }

    /// Add an endpoint
    pub fn add_endpoint(&mut self, endpoint: ServiceEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Set endpoints from configuration
    #[must_use]
    pub fn with_endpoints(mut self, endpoints: Vec<ServiceEndpoint>) -> Self {
        self.endpoints = endpoints;
        self
    }

    /// Add a custom capability
    pub fn add_capability(&mut self, capability: Capability) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl Default for ToadStoolIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimalIdentity for ToadStoolIdentity {
    fn primal_name(&self) -> &'static str {
        "toadstool"
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.clone()
    }

    fn endpoints(&self) -> Vec<ServiceEndpoint> {
        self.endpoints.clone()
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }
}

/// Discovered service information (what we learn about others)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    /// Service ID (if available)
    pub id: Option<String>,

    /// Capabilities this service provides
    pub capabilities: Vec<Capability>,

    /// Endpoints to reach this service
    pub endpoints: Vec<ServiceEndpoint>,

    /// Health status
    pub healthy: bool,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl DiscoveredService {
    /// Check if this service has a specific capability
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if this service has any compute capability
    #[must_use]
    pub fn has_compute_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Compute(_)))
    }

    /// Check if this service has any storage capability
    #[must_use]
    pub fn has_storage_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Storage(_)))
    }

    /// Check if this service has any auth capability
    #[must_use]
    pub fn has_auth_capability(&self) -> bool {
        self.capabilities
            .iter()
            .any(|cap| matches!(cap, Capability::Authentication(_)))
    }

    /// Get endpoints for a specific protocol
    #[must_use]
    pub fn endpoints_for_protocol(&self, protocol: &str) -> Vec<&ServiceEndpoint> {
        self.endpoints
            .iter()
            .filter(|ep| ep.protocol == protocol)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toadstool_identity() {
        let identity = ToadStoolIdentity::new();

        assert_eq!(identity.primal_name(), "toadstool");
        assert!(!identity.version().is_empty());
        assert!(!identity.capabilities().is_empty());
    }

    #[test]
    fn test_service_endpoint_url() {
        let endpoint = ServiceEndpoint::http("localhost", 8080).with_path("/api/v1");

        assert_eq!(endpoint.url(), "http://localhost:8080/api/v1");
    }

    #[test]
    fn test_capability_matching() {
        let service = DiscoveredService {
            id: Some("test".to_string()),
            capabilities: vec![
                Capability::Compute(ComputeCapability::NativeExecution),
                Capability::Storage(StorageCapability::ObjectStorage),
            ],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_compute_capability());
        assert!(service.has_storage_capability());
        assert!(!service.has_auth_capability());
    }
}
