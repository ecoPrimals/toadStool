// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability Type Definitions
//!
//! Defines WHAT capabilities exist, without hardcoding WHO provides them.
//! This is the core abstraction that breaks primal hardcoding.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core capability types in the ecoPrimals ecosystem
///
/// Each variant represents a capability that can be provided by ANY primal.
/// NO primal names are referenced here - only capabilities and features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilityType {
    /// Security capabilities (encryption, signing, audit, key management)
    ///
    /// Could be provided by: beardog, cloud HSM, local keyring, etc.
    Security {
        /// Security features this provider supports
        features: Vec<SecurityFeature>,
        /// Minimum trust level required
        min_trust_level: TrustLevel,
    },

    /// Storage capabilities (persistence, compression, versioning)
    ///
    /// Could be provided by: nestgate, S3, local filesystem, database, etc.
    Storage {
        /// Storage features this provider supports
        features: Vec<StorageFeature>,
        /// Minimum throughput in Mbps (optional)
        min_throughput_mbps: Option<u64>,
    },

    /// Coordination capabilities (service mesh, discovery, load balancing)
    ///
    /// Could be provided by: songbird, k8s, consul, etcd, etc.
    Coordination {
        /// Coordination features this provider supports
        features: Vec<CoordinationFeature>,
        /// Maximum acceptable latency in milliseconds
        max_latency_ms: Option<u64>,
    },

    /// Intelligence capabilities (AI, ML, analysis, natural language)
    ///
    /// Could be provided by: any routing/intelligence capability provider, `OpenAI`, local models, etc.
    Intelligence {
        /// Intelligence features this provider supports
        features: Vec<IntelligenceFeature>,
        /// Supported model types
        model_types: Vec<ModelType>,
    },

    /// Compute capabilities (GPU, CPU, specialized hardware)
    ///
    /// Could be provided by: local, cloud, edge devices, etc.
    Compute {
        /// Compute features this provider supports
        features: Vec<ComputeFeature>,
        /// Minimum memory in GB
        min_memory_gb: Option<f64>,
    },

    /// Network capabilities (routing, mesh, tunneling)
    Network {
        /// Network features this provider supports
        features: Vec<NetworkFeature>,
        /// Minimum bandwidth in Mbps
        min_bandwidth_mbps: Option<u64>,
    },

    /// Monitoring capabilities (metrics, logging, tracing)
    Monitoring {
        /// Monitoring features this provider supports
        features: Vec<MonitoringFeature>,
        /// Data retention period in days
        retention_days: Option<u32>,
    },
}

/// Security capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityFeature {
    /// Symmetric encryption
    Encryption,
    /// Asymmetric signing
    Signing,
    /// Audit logging
    Audit,
    /// Key management
    KeyManagement,
    /// Certificate authority
    CertificateAuthority,
    /// Access control
    AccessControl,
    /// Two-factor authentication
    TwoFactor,
    /// Biometric authentication
    Biometric,
    /// Hardware security module
    HSM,
    /// Quantum-resistant crypto
    QuantumResistant,
}

/// Storage capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageFeature {
    /// Data compression
    Compression,
    /// Encryption at rest
    Encryption,
    /// Versioning/history
    Versioning,
    /// Deduplication
    Deduplication,
    /// Replication
    Replication,
    /// Snapshots
    Snapshots,
    /// Transaction support
    Transactions,
    /// Search/indexing
    Search,
    /// Content-addressable storage
    ContentAddressable,
    /// Streaming support
    Streaming,
}

/// Coordination capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordinationFeature {
    /// Service discovery
    ServiceDiscovery,
    /// Load balancing
    LoadBalancing,
    /// Health checks
    HealthChecks,
    /// Service mesh
    ServiceMesh,
    /// Distributed locks
    DistributedLocks,
    /// Leader election
    LeaderElection,
    /// Configuration management
    ConfigManagement,
    /// Circuit breaking
    CircuitBreaking,
    /// Rate limiting
    RateLimiting,
    /// Traffic routing
    TrafficRouting,
}

/// Intelligence capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntelligenceFeature {
    /// Natural language processing
    NaturalLanguage,
    /// Code generation
    CodeGeneration,
    /// Data analysis
    Analysis,
    /// Image recognition
    ImageRecognition,
    /// Speech processing
    SpeechProcessing,
    /// Reasoning/inference
    Reasoning,
    /// Knowledge graph
    KnowledgeGraph,
    /// Recommendation engine
    Recommendations,
    /// Anomaly detection
    AnomalyDetection,
    /// Forecasting
    Forecasting,
}

/// Compute capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeFeature {
    /// GPU acceleration
    GPU,
    /// Multi-core CPU
    MultiCore,
    /// Vector processing (SIMD)
    VectorProcessing,
    /// Neuromorphic hardware
    Neuromorphic,
    /// FPGA acceleration
    FPGA,
    /// Container isolation
    Containers,
    /// VM isolation
    VMs,
    /// Serverless functions
    Serverless,
    /// Edge computing
    Edge,
    /// High memory
    HighMemory,
}

/// Network capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkFeature {
    /// TCP/IP networking
    TCPIP,
    /// UDP networking
    UDP,
    /// Unix sockets
    UnixSockets,
    /// `WebSocket` support
    WebSocket,
    /// gRPC support
    GRPC,
    /// HTTP/REST support
    HTTP,
    /// Message queues
    MessageQueue,
    /// Pub/sub messaging
    PubSub,
    /// VPN/tunneling
    VPN,
    /// NAT traversal
    NATTraversal,
}

/// Monitoring capability features
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonitoringFeature {
    /// Metrics collection
    Metrics,
    /// Log aggregation
    Logging,
    /// Distributed tracing
    Tracing,
    /// Alerting
    Alerting,
    /// Dashboards
    Dashboards,
    /// APM (Application Performance Monitoring)
    APM,
    /// Profiling
    Profiling,
    /// Error tracking
    ErrorTracking,
    /// Real-time streaming
    RealTimeStreaming,
    /// Historical analysis
    HistoricalAnalysis,
}

/// Trust level for security capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Minimal trust (testing, development)
    Low,
    /// Standard trust (production non-sensitive)
    Medium,
    /// High trust (production sensitive data)
    High,
    /// Maximum trust (compliance, regulated industries)
    Maximum,
}

/// AI/ML model types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelType {
    /// Large Language Model
    LLM,
    /// Computer vision model
    Vision,
    /// Audio processing model
    Audio,
    /// Time series model
    TimeSeries,
    /// Tabular data model
    Tabular,
    /// Graph neural network
    GraphNN,
    /// Reinforcement learning
    ReinforcementLearning,
    /// Generative model
    Generative,
}

/// Information about a capability provider
///
/// This is what gets returned from discovery - it tells you WHO can provide
/// a capability, but the code requesting the capability doesn't need to know.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    /// Unique provider ID (NOT a primal name! Random UUID)
    pub provider_id: String,

    /// What capability this provides
    pub capability: CapabilityType,

    /// Additional metadata (performance, cost, etc.)
    pub metadata: HashMap<String, String>,

    /// Service endpoint (URL, socket path, etc.)
    pub endpoint: ServiceEndpoint,

    /// Provider health status
    pub health: HealthStatus,
}

/// Service endpoint (agnostic representation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceEndpoint {
    /// HTTP/HTTPS endpoint
    Http(String),
    /// Unix domain socket
    UnixSocket(std::path::PathBuf),
    /// TCP socket
    Tcp {
        /// Hostname or IP address
        host: String,
        /// Port number
        port: u16,
    },
    /// In-process (same binary)
    InProcess,
    /// Custom protocol
    Custom {
        /// Protocol identifier
        protocol: String,
        /// Network address
        address: String,
    },
}

/// Health status of a capability provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Healthy and ready
    Healthy,
    /// Degraded but functional
    Degraded,
    /// Unhealthy (should not use)
    Unhealthy,
    /// Unknown health
    Unknown,
}

/// Handle to an acquired capability
///
/// This is what you get back when you request a capability.
/// It provides access to the capability WITHOUT revealing who provides it.
pub struct CapabilityHandle {
    provider: CapabilityInfo,
    capability: CapabilityType,
}

impl CapabilityHandle {
    /// Create a new capability handle
    #[must_use]
    pub const fn new(provider: CapabilityInfo, capability: CapabilityType) -> Self {
        Self {
            provider,
            capability,
        }
    }

    /// Get the capability type
    #[must_use]
    pub const fn capability_type(&self) -> &CapabilityType {
        &self.capability
    }

    /// Get the service endpoint
    #[must_use]
    pub const fn endpoint(&self) -> &ServiceEndpoint {
        &self.provider.endpoint
    }

    /// Get provider metadata (for introspection only)
    #[must_use]
    pub const fn metadata(&self) -> &HashMap<String, String> {
        &self.provider.metadata
    }

    /// Check if provider is healthy
    #[must_use]
    pub const fn is_healthy(&self) -> bool {
        matches!(
            self.provider.health,
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// Get provider ID (for logging/debugging only, NOT for business logic!)
    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider.provider_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_type_creation() {
        let security = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
            min_trust_level: TrustLevel::High,
        };

        assert!(matches!(security, CapabilityType::Security { .. }));
        if let CapabilityType::Security { features, .. } = &security {
            assert_eq!(features.len(), 2);
        }
    }

    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::Low < TrustLevel::Medium);
        assert!(TrustLevel::Medium < TrustLevel::High);
        assert!(TrustLevel::High < TrustLevel::Maximum);
    }

    #[test]
    fn test_service_endpoint() {
        let http = ServiceEndpoint::Http("http://localhost:8080".to_string());
        assert!(matches!(http, ServiceEndpoint::Http(_)));

        let tcp = ServiceEndpoint::Tcp {
            host: "localhost".to_string(),
            port: 9000,
        };
        assert!(matches!(tcp, ServiceEndpoint::Tcp { .. }));
    }

    #[test]
    fn test_capability_handle() {
        let info = CapabilityInfo {
            provider_id: "test-provider".to_string(),
            capability: CapabilityType::Storage {
                features: vec![StorageFeature::Compression],
                min_throughput_mbps: Some(100),
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        };

        let handle = CapabilityHandle::new(
            info,
            CapabilityType::Storage {
                features: vec![StorageFeature::Compression],
                min_throughput_mbps: Some(100),
            },
        );

        assert!(handle.is_healthy());
        assert_eq!(handle.provider_id(), "test-provider");
    }

    #[test]
    fn test_health_status() {
        let healthy = HealthStatus::Healthy;
        let degraded = HealthStatus::Degraded;
        let unhealthy = HealthStatus::Unhealthy;

        assert_eq!(healthy, HealthStatus::Healthy);
        assert_ne!(healthy, unhealthy);
        assert_ne!(degraded, unhealthy);
    }
}
