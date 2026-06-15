// SPDX-License-Identifier: AGPL-3.0-or-later
//! Standard capability taxonomy
//!
//! Defines the standard capability identifiers used across the ecosystem.
//! Capabilities are hierarchical: `category.subcategory.specific`
//!
//! # Philosophy
//! Services are discovered by **capability**, not by name. This allows:
//! - Swapping implementations without code changes
//! - Multiple providers for the same capability
//! - Zero hardcoding of service names
//! - Infant discovery (start with zero knowledge)
//!
//! # Zero-Copy Optimization (Phase 2.2)
//! Uses `Arc<str>` for cheap clones in capability registries and lookups.
//! - Clone = just rc increment (no heap allocation)
//! - Thread-safe sharing across operations
//! - Automatic cleanup when no longer needed

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;

/// Capability identifier - hierarchical dotted notation
///
/// **Zero-Copy**: Uses `Arc<str>` internally for cheap clones.
/// Capabilities are frequently cloned in registries and lookups,
/// so using `Arc<str>` eliminates allocations after initial creation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityId(Arc<str>);

// Custom Serialize implementation for Arc<str>
impl Serialize for CapabilityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

// Custom Deserialize implementation for Arc<str>
impl<'de> Deserialize<'de> for CapabilityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(Arc::from(s.as_str())))
    }
}

impl CapabilityId {
    /// Create a new capability ID
    ///
    /// **Zero-Copy**: Converts to `Arc<str>` for cheap clones.
    pub fn new(id: impl Into<String>) -> Self {
        let s: String = id.into();
        Self(Arc::from(s.as_str()))
    }

    /// Get the capability ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the category (first part before first dot)
    pub fn category(&self) -> &str {
        self.0.split('.').next().unwrap_or("")
    }

    /// Get the subcategory (second part)
    pub fn subcategory(&self) -> Option<&str> {
        self.0.split('.').nth(1)
    }

    /// Check if this capability matches a pattern (with wildcards)
    pub fn matches(&self, pattern: &str) -> bool {
        // Support wildcards: "crypto.*" matches "crypto.signature.ed25519"
        pattern.strip_suffix('*').map_or_else(
            || self.0.as_ref() == pattern,
            |prefix| self.0.starts_with(prefix),
        )
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CapabilityId {
    fn from(s: String) -> Self {
        Self(Arc::from(s.as_str()))
    }
}

impl From<&str> for CapabilityId {
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl From<StandardCapability> for CapabilityId {
    fn from(cap: StandardCapability) -> Self {
        // Use as_str() which returns &'static str (zero allocation!)
        Self(Arc::from(cap.as_str()))
    }
}

/// Standard capability definitions
///
/// These are well-known capabilities that services can provide.
/// Custom capabilities can be defined by third parties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardCapability {
    // ===== CRYPTO CAPABILITIES =====
    /// Ed25519 signature generation and verification
    CryptoSignatureEd25519,
    /// ECDSA signature generation and verification
    CryptoSignatureEcdsa,
    /// RSA signature generation and verification
    CryptoSignatureRsa,
    /// AES-256-GCM encryption/decryption
    CryptoEncryptionAes256,
    /// ChaCha20-Poly1305 encryption/decryption
    CryptoEncryptionChaCha20,
    /// Key generation
    CryptoKeyGeneration,
    /// Key derivation (HKDF, PBKDF2, etc.)
    CryptoKeyDerivation,
    /// Secure random number generation
    CryptoRandom,
    /// Permission management (validate, install, revoke crypto permissions)
    CryptoPermissionManagement,

    // ===== STORAGE CAPABILITIES =====
    /// Distributed filesystem (like ZFS, Ceph, GlusterFS)
    StorageDistributedFilesystem,
    /// Object storage (S3-compatible)
    StorageObjectS3,
    /// Block storage
    StorageBlock,
    /// Key-value storage
    StorageKeyValue,
    /// Database (SQL)
    StorageDatabaseSql,
    /// Database (NoSQL)
    StorageDatabaseNosql,

    // ===== COORDINATION CAPABILITIES =====
    /// Service registry and discovery
    CoordinationServiceRegistry,
    /// Peer discovery (mDNS, gossip, etc.)
    CoordinationPeerDiscovery,
    /// Leader election
    CoordinationLeaderElection,
    /// Distributed locking
    CoordinationDistributedLock,
    /// Configuration management
    CoordinationConfigManagement,
    /// Health checking
    CoordinationHealthCheck,

    // ===== COMPUTE CAPABILITIES =====
    /// OCI container execution
    ComputeContainerOci,
    /// WebAssembly component model
    ComputeWasmComponent,
    /// WebAssembly WASI
    ComputeWasmWasi,
    /// Native binary execution
    ComputeNative,
    /// Python runtime
    ComputePython,
    /// GPU compute
    ComputeGpu,
    /// Edge/IoT device execution
    ComputeEdge,

    // ===== MESSAGING CAPABILITIES =====
    /// AMQP message queue
    MessagingQueueAmqp,
    /// MQTT pub/sub
    MessagingPubsubMqtt,
    /// Kafka streaming
    MessagingStreamKafka,
    /// Redis pub/sub
    MessagingPubsubRedis,
    /// WebSocket real-time
    MessagingWebsocket,

    // ===== NETWORKING CAPABILITIES =====
    /// HTTP/REST endpoint
    NetworkingHttp,
    /// GraphQL API
    NetworkingGraphql,
    /// Load balancing
    NetworkingLoadBalancer,
    /// Service mesh integration
    NetworkingServiceMesh,
    /// VPN/tunnel
    NetworkingTunnel,

    // ===== MONITORING CAPABILITIES =====
    /// Metrics collection (Prometheus-compatible)
    MonitoringMetrics,
    /// Distributed tracing (OpenTelemetry)
    MonitoringTracing,
    /// Log aggregation
    MonitoringLogs,
    /// Alerting
    MonitoringAlerting,
    /// Health check aggregation
    MonitoringHealthAggregation,

    // ===== AUTHENTICATION/AUTHORIZATION =====
    /// OAuth2/OIDC provider
    AuthOauth2,
    /// API key management
    AuthApiKey,
    /// JWT validation
    AuthJwt,
    /// Permission evaluation
    AuthPermissions,
    /// Role-based access control
    AuthRbac,

    // ===== INFRASTRUCTURE =====
    /// Kubernetes orchestration
    InfraKubernetes,
    /// Docker container management
    InfraDocker,
    /// Systemd service management
    InfraSystemd,
    /// DNS management
    InfraDns,
    /// Certificate management (ACME, Let's Encrypt)
    InfraCertificates,
}

impl StandardCapability {
    /// Get the capability ID as a string
    pub const fn as_str(&self) -> &'static str {
        match self {
            // Crypto
            Self::CryptoSignatureEd25519 => "crypto.signature.ed25519",
            Self::CryptoSignatureEcdsa => "crypto.signature.ecdsa",
            Self::CryptoSignatureRsa => "crypto.signature.rsa",
            Self::CryptoEncryptionAes256 => "crypto.encryption.aes256",
            Self::CryptoEncryptionChaCha20 => "crypto.encryption.chacha20",
            Self::CryptoKeyGeneration => "crypto.key-generation",
            Self::CryptoKeyDerivation => "crypto.key-derivation",
            Self::CryptoRandom => "crypto.random",
            Self::CryptoPermissionManagement => "crypto.permission-management",

            // Storage
            Self::StorageDistributedFilesystem => "storage.distributed.filesystem",
            Self::StorageObjectS3 => "storage.object.s3",
            Self::StorageBlock => "storage.block",
            Self::StorageKeyValue => "storage.kv",
            Self::StorageDatabaseSql => "storage.database.sql",
            Self::StorageDatabaseNosql => "storage.database.nosql",

            // Coordination
            Self::CoordinationServiceRegistry => "coordination.service-registry",
            Self::CoordinationPeerDiscovery => "coordination.peer-discovery",
            Self::CoordinationLeaderElection => "coordination.leader-election",
            Self::CoordinationDistributedLock => "coordination.distributed-lock",
            Self::CoordinationConfigManagement => "coordination.config-management",
            Self::CoordinationHealthCheck => "coordination.health-check",

            // Compute
            Self::ComputeContainerOci => "compute.container.oci",
            Self::ComputeWasmComponent => "compute.wasm.component-model",
            Self::ComputeWasmWasi => "compute.wasm.wasi",
            Self::ComputeNative => "compute.native",
            Self::ComputePython => "compute.python",
            Self::ComputeGpu => "compute.gpu",
            Self::ComputeEdge => "compute.edge",

            // Messaging
            Self::MessagingQueueAmqp => "messaging.queue.amqp",
            Self::MessagingPubsubMqtt => "messaging.pubsub.mqtt",
            Self::MessagingStreamKafka => "messaging.stream.kafka",
            Self::MessagingPubsubRedis => "messaging.pubsub.redis",
            Self::MessagingWebsocket => "messaging.websocket",

            // Networking
            Self::NetworkingHttp => "networking.http",
            Self::NetworkingGraphql => "networking.graphql",
            Self::NetworkingLoadBalancer => "networking.load-balancer",
            Self::NetworkingServiceMesh => "networking.service-mesh",
            Self::NetworkingTunnel => "networking.tunnel",

            // Monitoring
            Self::MonitoringMetrics => "monitoring.metrics",
            Self::MonitoringTracing => "monitoring.tracing",
            Self::MonitoringLogs => "monitoring.logs",
            Self::MonitoringAlerting => "monitoring.alerting",
            Self::MonitoringHealthAggregation => "monitoring.health-aggregation",

            // Auth
            Self::AuthOauth2 => "auth.oauth2",
            Self::AuthApiKey => "auth.api-key",
            Self::AuthJwt => "auth.jwt",
            Self::AuthPermissions => "auth.permissions",
            Self::AuthRbac => "auth.rbac",

            // Infrastructure
            Self::InfraKubernetes => "infra.kubernetes",
            Self::InfraDocker => "infra.docker",
            Self::InfraSystemd => "infra.systemd",
            Self::InfraDns => "infra.dns",
            Self::InfraCertificates => "infra.certificates",
        }
    }

    /// Get capability ID
    pub fn id(&self) -> CapabilityId {
        CapabilityId::new(self.as_str())
    }
}

impl fmt::Display for StandardCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<StandardCapability> for String {
    fn from(cap: StandardCapability) -> Self {
        cap.as_str().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_id_matching() {
        let cap = CapabilityId::new("crypto.signature.ed25519");

        assert!(cap.matches("crypto.signature.ed25519"));
        assert!(cap.matches("crypto.signature.*"));
        assert!(cap.matches("crypto.*"));
        assert!(!cap.matches("storage.*"));
    }

    #[test]
    fn test_capability_category() {
        let cap = CapabilityId::new("crypto.signature.ed25519");
        assert_eq!(cap.category(), "crypto");
        assert_eq!(cap.subcategory(), Some("signature"));
    }

    #[test]
    fn test_standard_capability_conversion() {
        let cap = StandardCapability::CryptoSignatureEd25519;
        let id: CapabilityId = cap.into();
        assert_eq!(id.as_str(), "crypto.signature.ed25519");
    }
}
