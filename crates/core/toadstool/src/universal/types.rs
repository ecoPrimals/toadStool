// SPDX-License-Identifier: AGPL-3.0-or-later
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
    /// Security primal (crypto/PKI capability)
    Security,
    /// Storage primal (artifact/data capability)
    Storage,
    /// AI primal (intelligence capability)
    AI,
    /// Network primal (coordination capability)
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
            Self::Compute => "compute",
            Self::Security => "security",
            Self::Storage => "storage",
            Self::AI => "ai",
            Self::Network => "network",
            Self::OS => "os",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Parse from a string (case-insensitive).
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "compute" => Self::Compute,
            "security" => Self::Security,
            "storage" => Self::Storage,
            "ai" => Self::AI,
            "network" => Self::Network,
            "os" => Self::OS,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Primal capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalCapability {
    // Compute capabilities
    /// Container runtime support
    ContainerRuntime {
        /// Supported orchestrators (e.g. docker, k8s).
        orchestrators: Vec<String>,
    },
    /// Serverless execution
    ServerlessExecution {
        /// Supported languages.
        languages: Vec<String>,
    },
    /// GPU acceleration
    GpuAcceleration {
        /// CUDA support available.
        cuda_support: bool,
    },
    /// Load balancing
    LoadBalancing {
        /// Supported algorithms.
        algorithms: Vec<String>,
    },
    /// Auto-scaling
    AutoScaling {
        /// Metrics used for scaling.
        metrics: Vec<String>,
    },
    /// Native execution
    NativeExecution {
        /// Supported CPU architectures.
        architectures: Vec<String>,
    },
    /// WASM execution
    WasmExecution {
        /// WASI support available.
        wasi_support: bool,
    },

    // Security capabilities
    /// Authentication
    Authentication {
        /// Supported auth methods.
        methods: Vec<String>,
    },
    /// Encryption
    Encryption {
        /// Supported algorithms.
        algorithms: Vec<String>,
    },
    /// Key management
    KeyManagement {
        /// HSM support available.
        hsm_support: bool,
    },

    // Storage capabilities
    /// File system support
    FileSystem {
        /// ZFS support available.
        supports_zfs: bool,
    },
    /// Object storage
    ObjectStorage {
        /// Supported backends (e.g. s3, gcs).
        backends: Vec<String>,
    },
    /// Data replication
    DataReplication {
        /// Consistency model.
        consistency: String,
    },

    // AI capabilities
    /// Model inference
    ModelInference {
        /// Supported model types.
        models: Vec<String>,
    },
    /// Agent framework
    AgentFramework {
        /// MCP support available.
        mcp_support: bool,
    },
    /// Machine learning
    MachineLearning {
        /// Training support available.
        training_support: bool,
    },

    // Network capabilities
    /// Service discovery
    ServiceDiscovery {
        /// Supported protocols (e.g. mdns, dns-sd).
        protocols: Vec<String>,
    },
    /// Network routing
    NetworkRouting {
        /// Supported routing protocols.
        protocols: Vec<String>,
    },
    /// Proxy services
    ProxyServices {
        /// Proxy types (e.g. http, tcp).
        types: Vec<String>,
    },

    // OS capabilities
    /// Process management
    ProcessManagement {
        /// Container support available.
        container_support: bool,
    },
    /// Resource management
    ResourceManagement {
        /// Quota enforcement support.
        quota_support: bool,
    },
    /// Team isolation
    TeamIsolation {
        /// Multi-tenant support.
        multi_tenant: bool,
    },

    /// Custom capability with arbitrary name and attributes
    Custom {
        /// Capability name.
        name: String,
        /// Arbitrary attributes.
        attributes: HashMap<String, String>,
    },
}

/// Primal health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimalHealth {
    /// Healthy and operational
    Healthy,
    /// Degraded but operational
    Degraded {
        /// List of issues.
        issues: Vec<String>,
    },
    /// Unhealthy and not operational
    Unhealthy {
        /// Failure reason.
        reason: String,
    },
}

// Note: PrimalEndpoints, PrimalRequest, PrimalResponse, ResponseStatus
// are now in requests.rs to avoid duplication

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Basic < SecurityLevel::Standard);
        assert!(SecurityLevel::Standard < SecurityLevel::High);
        assert!(SecurityLevel::High < SecurityLevel::Maximum);
    }

    #[test]
    fn test_security_level_equality() {
        assert_eq!(SecurityLevel::Basic, SecurityLevel::Basic);
        assert_ne!(SecurityLevel::Basic, SecurityLevel::Maximum);
    }

    #[test]
    fn test_security_level_serde_roundtrip() {
        for level in [
            SecurityLevel::Basic,
            SecurityLevel::Standard,
            SecurityLevel::High,
            SecurityLevel::Maximum,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: SecurityLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_primal_type_as_str() {
        assert_eq!(PrimalType::Compute.as_str(), "compute");
        assert_eq!(PrimalType::Security.as_str(), "security");
        assert_eq!(PrimalType::Storage.as_str(), "storage");
        assert_eq!(PrimalType::AI.as_str(), "ai");
        assert_eq!(PrimalType::Network.as_str(), "network");
        assert_eq!(PrimalType::OS.as_str(), "os");
        assert_eq!(
            PrimalType::Custom("custom_primal".to_string()).as_str(),
            "custom_primal"
        );
    }

    #[test]
    fn test_primal_type_from_str_lossy() {
        assert_eq!(PrimalType::from_str_lossy("compute"), PrimalType::Compute);
        assert_eq!(PrimalType::from_str_lossy("COMPUTE"), PrimalType::Compute);
        assert_eq!(PrimalType::from_str_lossy("Compute"), PrimalType::Compute);
        assert_eq!(PrimalType::from_str_lossy("security"), PrimalType::Security);
        assert_eq!(PrimalType::from_str_lossy("SECURITY"), PrimalType::Security);
        assert_eq!(PrimalType::from_str_lossy("storage"), PrimalType::Storage);
        assert_eq!(PrimalType::from_str_lossy("ai"), PrimalType::AI);
        assert_eq!(PrimalType::from_str_lossy("AI"), PrimalType::AI);
        assert_eq!(PrimalType::from_str_lossy("network"), PrimalType::Network);
        assert_eq!(PrimalType::from_str_lossy("os"), PrimalType::OS);
        assert_eq!(
            PrimalType::from_str_lossy("unknown_type"),
            PrimalType::Custom("unknown_type".to_string())
        );
    }

    #[test]
    fn test_primal_type_roundtrip() {
        for primal_type in [
            PrimalType::Compute,
            PrimalType::Security,
            PrimalType::Storage,
            PrimalType::AI,
            PrimalType::Network,
            PrimalType::OS,
        ] {
            let str_repr = primal_type.as_str();
            let parsed = PrimalType::from_str_lossy(str_repr);
            assert_eq!(primal_type, parsed);
        }
    }

    #[test]
    fn test_primal_type_serde_roundtrip() {
        let types = vec![
            PrimalType::Compute,
            PrimalType::Security,
            PrimalType::Custom("my_primal".to_string()),
        ];
        for primal_type in types {
            let json = serde_json::to_string(&primal_type).unwrap();
            let deserialized: PrimalType = serde_json::from_str(&json).unwrap();
            assert_eq!(primal_type, deserialized);
        }
    }

    #[test]
    fn test_network_location() {
        let loc = NetworkLocation {
            ip_address: "192.168.1.100".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: Some("home-network".to_string()),
            geo_location: Some("US-West".to_string()),
        };

        let json = serde_json::to_string(&loc).unwrap();
        let deserialized: NetworkLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(loc, deserialized);
    }

    #[test]
    fn test_network_location_minimal() {
        let loc = NetworkLocation {
            ip_address: "10.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        };

        assert_eq!(loc.ip_address, "10.0.0.1");
        assert!(loc.subnet.is_none());
    }

    #[test]
    fn test_primal_context() {
        let context = PrimalContext {
            user_id: "user123".to_string(),
            device_id: "device456".to_string(),
            session_id: "session789".to_string(),
            network_location: NetworkLocation {
                ip_address: "192.168.1.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::High,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&context).unwrap();
        let deserialized: PrimalContext = serde_json::from_str(&json).unwrap();
        assert_eq!(context, deserialized);
    }

    #[test]
    fn test_primal_context_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let context = PrimalContext {
            user_id: "user".to_string(),
            device_id: "device".to_string(),
            session_id: "session".to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata,
        };

        assert_eq!(context.metadata.len(), 2);
        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_primal_health_variants() {
        let healthy = PrimalHealth::Healthy;
        let degraded = PrimalHealth::Degraded {
            issues: vec!["high latency".to_string(), "memory pressure".to_string()],
        };
        let unhealthy = PrimalHealth::Unhealthy {
            reason: "connection failed".to_string(),
        };

        assert_eq!(healthy, PrimalHealth::Healthy);
        if let PrimalHealth::Degraded { issues } = &degraded {
            assert_eq!(issues.len(), 2);
        } else {
            unreachable!("Expected Degraded variant");
        }
        if let PrimalHealth::Unhealthy { reason } = &unhealthy {
            assert_eq!(reason, "connection failed");
        } else {
            unreachable!("Expected Unhealthy variant");
        }
    }

    #[test]
    fn test_primal_health_serde_roundtrip() {
        let variants = vec![
            PrimalHealth::Healthy,
            PrimalHealth::Degraded {
                issues: vec!["issue1".to_string()],
            },
            PrimalHealth::Unhealthy {
                reason: "failure".to_string(),
            },
        ];

        for health in variants {
            let json = serde_json::to_string(&health).unwrap();
            let deserialized: PrimalHealth = serde_json::from_str(&json).unwrap();
            assert_eq!(health, deserialized);
        }
    }

    #[test]
    fn test_primal_capability_compute() {
        let cap = PrimalCapability::ContainerRuntime {
            orchestrators: vec!["docker".to_string(), "podman".to_string()],
        };

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: PrimalCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }

    #[test]
    fn test_primal_capability_gpu() {
        let cap = PrimalCapability::GpuAcceleration { cuda_support: true };

        if let PrimalCapability::GpuAcceleration { cuda_support } = cap {
            assert!(cuda_support);
        } else {
            unreachable!("Expected GpuAcceleration variant");
        }
    }

    #[test]
    fn test_primal_capability_custom() {
        let mut attributes = HashMap::new();
        attributes.insert("version".to_string(), "1.0".to_string());
        attributes.insert("feature".to_string(), "enabled".to_string());

        let cap = PrimalCapability::Custom {
            name: "custom_cap".to_string(),
            attributes,
        };

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: PrimalCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }
}
