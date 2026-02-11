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

#[allow(deprecated)]
use crate::interned_strings::primals;

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

    // Cryptographic capabilities
    Crypto(CryptoCapability),

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

    /// Artifact storage
    ArtifactStorage,
}

/// Cryptographic capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoCapability {
    /// Encryption/Decryption operations
    Encryption,

    /// Key management (generation, rotation, storage)
    KeyManagement,

    /// PKI and certificate authority
    CertificateAuthority,

    /// Secrets management (vaults, secret storage)
    SecretsManagement,

    /// Hardware security module (HSM) support
    HardwareSecurity,

    /// Genetic entropy (unique to BearDog)
    GeneticEntropy,

    /// Digital signatures
    DigitalSignatures,

    /// Hashing operations
    Hashing,
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

    /// OAuth/OIDC provider
    OAuthProvider,

    /// SAML provider
    SamlProvider,
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

#[allow(deprecated)] // Self-knowledge: ToadStool knows its own name
impl PrimalIdentity for ToadStoolIdentity {
    fn primal_name(&self) -> &'static str {
        primals::TOADSTOOL
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

    // === New comprehensive tests ===

    #[test]
    fn test_service_endpoint_http() {
        let endpoint = ServiceEndpoint::http("example.com", 8080);
        assert_eq!(endpoint.protocol, "http");
        assert_eq!(endpoint.address, "example.com");
        assert_eq!(endpoint.port, 8080);
        assert_eq!(endpoint.url(), "http://example.com:8080");
    }

    #[test]
    fn test_service_endpoint_https() {
        let endpoint = ServiceEndpoint::https("secure.example.com", 443);
        assert_eq!(endpoint.protocol, "https");
        assert_eq!(endpoint.port, 443);
        assert_eq!(endpoint.url(), "https://secure.example.com:443");
    }

    #[test]
    fn test_service_endpoint_grpc() {
        let endpoint = ServiceEndpoint::grpc("grpc.example.com", 9090);
        assert_eq!(endpoint.protocol, "grpc");
        assert_eq!(endpoint.url(), "grpc://grpc.example.com:9090");
    }

    #[test]
    fn test_service_endpoint_websocket() {
        let endpoint = ServiceEndpoint::websocket("ws.example.com", 8081);
        assert_eq!(endpoint.protocol, "ws");
        assert_eq!(endpoint.url(), "ws://ws.example.com:8081");
    }

    #[test]
    fn test_service_endpoint_with_path() {
        let endpoint = ServiceEndpoint::http("api.example.com", 8080).with_path("/v2/compute");
        assert_eq!(endpoint.url(), "http://api.example.com:8080/v2/compute");
    }

    #[test]
    fn test_service_endpoint_with_metadata() {
        let endpoint = ServiceEndpoint::http("api.example.com", 8080)
            .with_metadata("region", "us-west")
            .with_metadata("tier", "production");

        assert_eq!(
            endpoint.metadata.get("region"),
            Some(&"us-west".to_string())
        );
        assert_eq!(
            endpoint.metadata.get("tier"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_toadstool_identity_default_capabilities() {
        let identity = ToadStoolIdentity::new();
        let caps = identity.capabilities();

        // Should have compute capabilities
        assert!(caps.contains(&Capability::Compute(ComputeCapability::NativeExecution)));
        assert!(caps.contains(&Capability::Compute(ComputeCapability::WasmExecution)));
        assert!(caps.contains(&Capability::Compute(ComputeCapability::GpuCompute)));
    }

    #[test]
    fn test_toadstool_identity_add_endpoint() {
        let mut identity = ToadStoolIdentity::new();
        identity.add_endpoint(ServiceEndpoint::http("localhost", 8080));

        let endpoints = identity.endpoints();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].protocol, "http");
    }

    #[test]
    fn test_toadstool_identity_with_endpoints() {
        let endpoints = vec![
            ServiceEndpoint::http("localhost", 8080),
            ServiceEndpoint::grpc("localhost", 9090),
        ];
        let identity = ToadStoolIdentity::new().with_endpoints(endpoints);

        assert_eq!(identity.endpoints().len(), 2);
    }

    #[test]
    fn test_toadstool_identity_add_capability() {
        let mut identity = ToadStoolIdentity::new();
        let initial_count = identity.capabilities().len();

        identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
        assert_eq!(identity.capabilities().len(), initial_count + 1);

        // Adding duplicate should not increase count
        identity.add_capability(Capability::Storage(StorageCapability::ObjectStorage));
        assert_eq!(identity.capabilities().len(), initial_count + 1);
    }

    #[test]
    fn test_toadstool_identity_add_metadata() {
        let mut identity = ToadStoolIdentity::new();
        identity.add_metadata("custom_key".to_string(), "custom_value".to_string());

        let metadata = identity.metadata();
        assert_eq!(
            metadata.get("custom_key"),
            Some(&"custom_value".to_string())
        );
    }

    #[test]
    fn test_toadstool_identity_metadata_includes_platform() {
        let identity = ToadStoolIdentity::new();
        let metadata = identity.metadata();

        assert!(metadata.contains_key("platform"));
        assert!(metadata.contains_key("arch"));
        assert!(metadata.contains_key("description"));
    }

    #[test]
    fn test_discovered_service_has_capability() {
        let service = DiscoveredService {
            id: Some("test".to_string()),
            capabilities: vec![Capability::Compute(ComputeCapability::GpuCompute)],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_capability(&Capability::Compute(ComputeCapability::GpuCompute)));
        assert!(!service.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
    }

    #[test]
    fn test_discovered_service_has_compute_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![
                Capability::Compute(ComputeCapability::WasmExecution),
                Capability::Storage(StorageCapability::Cache),
            ],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_compute_capability());
    }

    #[test]
    fn test_discovered_service_has_storage_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![Capability::Storage(StorageCapability::Database)],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_storage_capability());
        assert!(!service.has_compute_capability());
    }

    #[test]
    fn test_discovered_service_has_auth_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![Capability::Authentication(AuthCapability::UserAuth)],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_auth_capability());
        assert!(!service.has_compute_capability());
        assert!(!service.has_storage_capability());
    }

    #[test]
    fn test_discovered_service_endpoints_for_protocol() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![],
            endpoints: vec![
                ServiceEndpoint::http("api1.example.com", 8080),
                ServiceEndpoint::https("api2.example.com", 443),
                ServiceEndpoint::http("api3.example.com", 8081),
            ],
            healthy: true,
            metadata: HashMap::new(),
        };

        let http_endpoints = service.endpoints_for_protocol("http");
        assert_eq!(http_endpoints.len(), 2);

        let https_endpoints = service.endpoints_for_protocol("https");
        assert_eq!(https_endpoints.len(), 1);

        let grpc_endpoints = service.endpoints_for_protocol("grpc");
        assert_eq!(grpc_endpoints.len(), 0);
    }

    #[test]
    fn test_capability_equality() {
        let cap1 = Capability::Compute(ComputeCapability::NativeExecution);
        let cap2 = Capability::Compute(ComputeCapability::NativeExecution);
        let cap3 = Capability::Compute(ComputeCapability::WasmExecution);

        assert_eq!(cap1, cap2);
        assert_ne!(cap1, cap3);
    }

    #[test]
    fn test_capability_custom() {
        let cap1 = Capability::Custom {
            name: "custom-service".to_string(),
            version: "1.0".to_string(),
        };
        let cap2 = Capability::Custom {
            name: "custom-service".to_string(),
            version: "1.0".to_string(),
        };

        assert_eq!(cap1, cap2);
    }

    #[test]
    fn test_coordination_capability_default() {
        let cap = CoordinationCapability::default();
        assert_eq!(cap, CoordinationCapability::ServiceDiscovery);
    }

    #[test]
    fn test_service_endpoint_clone() {
        let endpoint1 = ServiceEndpoint::http("localhost", 8080);
        let endpoint2 = endpoint1.clone();

        assert_eq!(endpoint1.protocol, endpoint2.protocol);
        assert_eq!(endpoint1.address, endpoint2.address);
        assert_eq!(endpoint1.port, endpoint2.port);
    }

    #[test]
    fn test_toadstool_identity_default() {
        let identity = ToadStoolIdentity::default();
        assert_eq!(identity.primal_name(), "toadstool");
        assert!(!identity.version().is_empty());
    }

    #[test]
    fn test_discovered_service_with_no_id() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![],
            endpoints: vec![],
            healthy: false,
            metadata: HashMap::new(),
        };

        assert!(service.id.is_none());
        assert!(!service.healthy);
    }

    #[test]
    fn test_all_compute_capabilities() {
        let caps = vec![
            ComputeCapability::NativeExecution,
            ComputeCapability::ContainerOrchestration,
            ComputeCapability::WasmExecution,
            ComputeCapability::PythonExecution,
            ComputeCapability::GpuCompute,
            ComputeCapability::EdgeExecution,
            ComputeCapability::SpecialtyHardware,
        ];

        assert_eq!(caps.len(), 7);
    }

    #[test]
    fn test_all_storage_capabilities() {
        let caps = vec![
            StorageCapability::ObjectStorage,
            StorageCapability::BlockStorage,
            StorageCapability::FileStorage,
            StorageCapability::Database,
            StorageCapability::Cache,
            StorageCapability::ArtifactStorage,
        ];

        assert_eq!(caps.len(), 6);
    }

    #[test]
    fn test_all_auth_capabilities() {
        let caps = vec![
            AuthCapability::UserAuth,
            AuthCapability::ServiceAuth,
            AuthCapability::TokenManagement,
            AuthCapability::OAuthProvider,
            AuthCapability::SamlProvider,
        ];

        assert_eq!(caps.len(), 5);
    }

    // === Capability variant coverage (all branches) ===

    #[test]
    fn test_all_crypto_capabilities() {
        let caps = vec![
            CryptoCapability::Encryption,
            CryptoCapability::KeyManagement,
            CryptoCapability::CertificateAuthority,
            CryptoCapability::SecretsManagement,
            CryptoCapability::HardwareSecurity,
            CryptoCapability::GeneticEntropy,
            CryptoCapability::DigitalSignatures,
            CryptoCapability::Hashing,
        ];

        assert_eq!(caps.len(), 8);
    }

    #[test]
    fn test_all_coordination_capabilities() {
        let caps = vec![
            CoordinationCapability::ServiceDiscovery,
            CoordinationCapability::LoadBalancing,
            CoordinationCapability::HealthChecking,
            CoordinationCapability::ConfigManagement,
            CoordinationCapability::WorkflowOrchestration,
        ];

        assert_eq!(caps.len(), 5);
    }

    #[test]
    fn test_all_discovery_capabilities() {
        let caps = vec![
            DiscoveryCapability::CapabilityDiscovery,
            DiscoveryCapability::DnsDiscovery,
            DiscoveryCapability::MdnsDiscovery,
            DiscoveryCapability::RegistryDiscovery,
        ];

        assert_eq!(caps.len(), 4);
    }

    #[test]
    fn test_discovered_service_with_crypto_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
        assert!(!service.has_compute_capability());
        assert!(!service.has_storage_capability());
        assert!(!service.has_auth_capability());
    }

    #[test]
    fn test_discovered_service_with_discovery_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![Capability::Discovery(
                DiscoveryCapability::CapabilityDiscovery,
            )],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_capability(&Capability::Discovery(
            DiscoveryCapability::CapabilityDiscovery
        )));
    }

    #[test]
    fn test_discovered_service_with_coordination_capability() {
        let service = DiscoveredService {
            id: None,
            capabilities: vec![Capability::Coordination(
                CoordinationCapability::LoadBalancing,
            )],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };

        assert!(service.has_capability(&Capability::Coordination(
            CoordinationCapability::LoadBalancing
        )));
    }

    #[test]
    fn test_capability_debug_formatting() {
        let cap = Capability::Compute(ComputeCapability::WasmExecution);
        let debug_str = format!("{:?}", cap);
        assert!(!debug_str.is_empty());
        assert!(debug_str.contains("Compute"));
        assert!(debug_str.contains("WasmExecution"));

        let custom = Capability::Custom {
            name: "test".to_string(),
            version: "1.0".to_string(),
        };
        let custom_debug = format!("{:?}", custom);
        assert!(!custom_debug.is_empty());
        assert!(custom_debug.contains("Custom"));
    }

    #[test]
    fn test_service_endpoint_debug_formatting() {
        let ep = ServiceEndpoint::http("localhost", 8080);
        let debug_str = format!("{:?}", ep);
        assert!(!debug_str.is_empty());
        assert!(debug_str.contains("localhost"));
    }

    #[test]
    fn test_toadstool_identity_debug_formatting() {
        let identity = ToadStoolIdentity::new();
        let debug_str = format!("{:?}", identity);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_discovered_service_debug_formatting() {
        let service = DiscoveredService {
            id: Some("id".to_string()),
            capabilities: vec![],
            endpoints: vec![],
            healthy: true,
            metadata: HashMap::new(),
        };
        let debug_str = format!("{:?}", service);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_capability_serialize_deserialize() {
        let cap = Capability::Compute(ComputeCapability::GpuCompute);
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);

        let custom = Capability::Custom {
            name: "custom-service".to_string(),
            version: "2.0".to_string(),
        };
        let json = serde_json::to_string(&custom).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(custom, deserialized);
    }
}
