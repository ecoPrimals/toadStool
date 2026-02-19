//! Core universal types for primal operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security level for primal operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Basic security
    Basic,
    /// Standard security
    Standard,
    /// High security
    High,
    /// Maximum security
    Maximum,
}

/// Network location information
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkLocation {
    /// IP address
    pub ip_address: String,
    /// Subnet
    pub subnet: Option<String>,
    /// Network identifier
    pub network_id: Option<String>,
    /// Geographic location
    pub geo_location: Option<String>,
}

/// Context for user/device-specific primal routing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimalContext {
    /// User identifier
    pub user_id: String,
    /// Device identifier
    pub device_id: String,
    /// Session identifier
    pub session_id: String,
    /// Network location
    pub network_location: NetworkLocation,
    /// Security level required
    pub security_level: SecurityLevel,
    /// Additional context metadata
    pub metadata: HashMap<String, String>,
}

/// Primal type categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimalType {
    /// Compute primal (`ToadStool`)
    Compute,
    /// Security primal (`BearDog`)
    Security,
    /// Storage primal (`NestGate`)
    Storage,
    /// AI primal (Squirrel)
    AI,
    /// Network primal (Songbird)
    Network,
    /// OS primal (`BiomeOS`)
    OS,
    /// Custom primal type
    Custom(String),
}

impl PrimalType {
    /// Canonical lowercase name for routing and discovery.
    ///
    /// Use this instead of `format!("{:?}", …)` to avoid coupling to Debug output.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            PrimalType::Compute => "compute",
            PrimalType::Security => "security",
            PrimalType::Storage => "storage",
            PrimalType::AI => "ai",
            PrimalType::Network => "network",
            PrimalType::OS => "os",
            PrimalType::Custom(name) => name.as_str(),
        }
    }

    /// Parse from a string (case-insensitive).
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "compute" => PrimalType::Compute,
            "security" => PrimalType::Security,
            "storage" => PrimalType::Storage,
            "ai" => PrimalType::AI,
            "network" => PrimalType::Network,
            "os" => PrimalType::OS,
            other => PrimalType::Custom(other.to_string()),
        }
    }
}

/// Primal capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalCapability {
    // Compute capabilities
    /// Container runtime support
    ContainerRuntime { orchestrators: Vec<String> },
    /// Serverless execution
    ServerlessExecution { languages: Vec<String> },
    /// GPU acceleration
    GpuAcceleration { cuda_support: bool },
    /// Load balancing
    LoadBalancing { algorithms: Vec<String> },
    /// Auto-scaling
    AutoScaling { metrics: Vec<String> },
    /// Native execution
    NativeExecution { architectures: Vec<String> },
    /// WASM execution
    WasmExecution { wasi_support: bool },

    // Security capabilities
    /// Authentication
    Authentication { methods: Vec<String> },
    /// Encryption
    Encryption { algorithms: Vec<String> },
    /// Key management
    KeyManagement { hsm_support: bool },

    // Storage capabilities
    /// File system support
    FileSystem { supports_zfs: bool },
    /// Object storage
    ObjectStorage { backends: Vec<String> },
    /// Data replication
    DataReplication { consistency: String },

    // AI capabilities
    /// Model inference
    ModelInference { models: Vec<String> },
    /// Agent framework
    AgentFramework { mcp_support: bool },
    /// Machine learning
    MachineLearning { training_support: bool },

    // Network capabilities
    /// Service discovery
    ServiceDiscovery { protocols: Vec<String> },
    /// Network routing
    NetworkRouting { protocols: Vec<String> },
    /// Proxy services
    ProxyServices { types: Vec<String> },

    // OS capabilities
    /// Process management
    ProcessManagement { container_support: bool },
    /// Resource management
    ResourceManagement { quota_support: bool },
    /// Team isolation
    TeamIsolation { multi_tenant: bool },

    // Custom capability
    Custom {
        name: String,
        attributes: HashMap<String, String>,
    },
}

/// Primal health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalHealth {
    /// Healthy and operational
    Healthy,
    /// Degraded but operational
    Degraded { issues: Vec<String> },
    /// Unhealthy and not operational
    Unhealthy { reason: String },
}

// Note: PrimalEndpoints, PrimalRequest, PrimalResponse, ResponseStatus
// are now in requests.rs to avoid duplication
