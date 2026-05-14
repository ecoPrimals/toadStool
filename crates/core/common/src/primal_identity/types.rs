// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal identity type definitions
//!
//! Capability enums and `ServiceEndpoint` - the core types for capability-based discovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Capability that a primal can provide
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Compute capabilities
    Compute(ComputeCapability),

    /// Storage capabilities
    Storage(StorageCapability),

    /// Cryptographic capabilities
    Crypto(CryptoCapability),

    /// Authentication capabilities
    Authentication(AuthCapability),

    /// Coordination capabilities
    Coordination(CoordinationCapability),

    /// Discovery capabilities
    Discovery(DiscoveryCapability),

    /// Custom capability
    Custom {
        /// Capability name
        name: String,
        /// Capability version
        version: String,
    },
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
    #[deprecated(
        since = "0.5.0",
        note = "WebSocket is deprecated. Use JSON-RPC 2.0 polling instead."
    )]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_endpoint_url_without_path() {
        let endpoint = ServiceEndpoint::http("localhost", 8080);
        assert_eq!(endpoint.url(), "http://localhost:8080");
    }

    #[test]
    fn test_service_endpoint_url_with_empty_path() {
        let endpoint = ServiceEndpoint::http("localhost", 8080).with_path("");
        assert_eq!(endpoint.url(), "http://localhost:8080");
    }

    #[test]
    fn test_service_endpoint_url_with_root_path() {
        let endpoint = ServiceEndpoint::https("api.example.com", 443).with_path("/");
        assert_eq!(endpoint.url(), "https://api.example.com:443/");
    }

    #[test]
    fn test_service_endpoint_url_with_query_path() {
        let endpoint = ServiceEndpoint::http("api.example.com", 80).with_path("/v1/query?foo=bar");
        assert_eq!(endpoint.url(), "http://api.example.com:80/v1/query?foo=bar");
    }

    #[test]
    fn test_service_endpoint_with_metadata_chain() {
        let endpoint = ServiceEndpoint::grpc("localhost", 50051)
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        assert_eq!(endpoint.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(endpoint.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_capability_hash_for_hashmap_key() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Capability::Compute(ComputeCapability::GpuCompute));
        set.insert(Capability::Compute(ComputeCapability::GpuCompute)); // duplicate
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_capability_serialize_all_variants() {
        let variants = [
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
            Capability::Crypto(CryptoCapability::Encryption),
            Capability::Authentication(AuthCapability::UserAuth),
            Capability::Coordination(CoordinationCapability::ServiceDiscovery),
            Capability::Discovery(DiscoveryCapability::CapabilityDiscovery),
        ];
        for cap in &variants {
            let json = serde_json::to_string(cap).expect("serialize");
            let restored: Capability = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(cap, &restored);
        }
    }

    #[test]
    fn test_service_endpoint_serialize_deserialize() {
        let endpoint = ServiceEndpoint::http("example.com", 8080)
            .with_path("/api")
            .with_metadata("region", "us-west");
        let json = serde_json::to_string(&endpoint).expect("serialize");
        let restored: ServiceEndpoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(endpoint.protocol, restored.protocol);
        assert_eq!(endpoint.address, restored.address);
        assert_eq!(endpoint.port, restored.port);
        assert_eq!(endpoint.path, restored.path);
        assert_eq!(endpoint.metadata, restored.metadata);
    }

    #[test]
    #[expect(deprecated, reason = "tests exercise deprecated websocket constructor")]
    fn test_service_endpoint_websocket_deprecated() {
        let endpoint = ServiceEndpoint::websocket("ws.example.com", 8081);
        assert_eq!(endpoint.protocol, "ws");
        assert_eq!(endpoint.url(), "ws://ws.example.com:8081");
    }
}
