//! Config-file discovery types and capability parsing
//!
//! Config files are JSON, searched in order:
//! 1. `$TOADSTOOL_DISCOVERY_CONFIG` env var (full path)
//! 2. `$BIOMEOS_RUNTIME_DIR/discovery.json` (biomeOS runtime dir)
//! 3. `/etc/biomeos/discovery.json` (system-wide)

use std::collections::HashMap;

use serde::Deserialize;

use crate::primal_identity::{
    AuthCapability, Capability, CoordinationCapability, CryptoCapability, StorageCapability,
};

/// A single service entry in a discovery config file.
#[derive(Debug, Deserialize)]
pub struct ConfigFileService {
    pub id: Option<String>,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub endpoints: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Root structure of a discovery config file.
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub services: Vec<ConfigFileService>,
}

/// Default version when not specified in config.
pub fn default_version() -> String {
    "unknown".to_string()
}

/// Map capability string (from config/mDNS TXT records) to typed `Capability`.
#[must_use]
pub fn capability_from_str(s: &str) -> Capability {
    match s.trim().to_lowercase().as_str() {
        "coordination" | "orchestration" => {
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        }
        "storage" | "object_storage" | "object-storage" => {
            Capability::Storage(StorageCapability::ObjectStorage)
        }
        "security" | "crypto" | "cryptography" => {
            Capability::Crypto(CryptoCapability::KeyManagement)
        }
        "authentication" | "auth" => Capability::Authentication(AuthCapability::TokenManagement),
        "compute" | "native" | "execution" => {
            Capability::Compute(crate::primal_identity::ComputeCapability::NativeExecution)
        }
        "gpu" | "gpu_compute" | "gpu-compute" => {
            Capability::Compute(crate::primal_identity::ComputeCapability::GpuCompute)
        }
        other => Capability::Custom {
            name: other.to_string(),
            version: "0".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primal_identity::{ComputeCapability, CoordinationCapability, StorageCapability};

    #[test]
    fn test_capability_from_str_coordination() {
        let cap = capability_from_str("coordination");
        assert!(matches!(
            cap,
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        ));
    }

    #[test]
    fn test_capability_from_str_orchestration() {
        let cap = capability_from_str("orchestration");
        assert!(matches!(
            cap,
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        ));
    }

    #[test]
    fn test_capability_from_str_storage() {
        let cap = capability_from_str("storage");
        assert!(matches!(
            cap,
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_capability_from_str_object_storage() {
        let cap = capability_from_str("object_storage");
        assert!(matches!(
            cap,
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_capability_from_str_object_storage_hyphen() {
        let cap = capability_from_str("object-storage");
        assert!(matches!(
            cap,
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_capability_from_str_security() {
        let cap = capability_from_str("security");
        assert!(matches!(cap, Capability::Crypto(_)));
    }

    #[test]
    fn test_capability_from_str_crypto() {
        let cap = capability_from_str("crypto");
        assert!(matches!(cap, Capability::Crypto(_)));
    }

    #[test]
    fn test_capability_from_str_cryptography() {
        let cap = capability_from_str("cryptography");
        assert!(matches!(cap, Capability::Crypto(_)));
    }

    #[test]
    fn test_capability_from_str_authentication() {
        let cap = capability_from_str("authentication");
        assert!(matches!(cap, Capability::Authentication(_)));
    }

    #[test]
    fn test_capability_from_str_auth() {
        let cap = capability_from_str("auth");
        assert!(matches!(cap, Capability::Authentication(_)));
    }

    #[test]
    fn test_capability_from_str_compute() {
        let cap = capability_from_str("compute");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_capability_from_str_native() {
        let cap = capability_from_str("native");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_capability_from_str_execution() {
        let cap = capability_from_str("execution");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_capability_from_str_gpu() {
        let cap = capability_from_str("gpu");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::GpuCompute)
        ));
    }

    #[test]
    fn test_capability_from_str_gpu_compute() {
        let cap = capability_from_str("gpu_compute");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::GpuCompute)
        ));
    }

    #[test]
    fn test_capability_from_str_gpu_compute_hyphen() {
        let cap = capability_from_str("gpu-compute");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::GpuCompute)
        ));
    }

    #[test]
    fn test_capability_from_str_custom() {
        let cap = capability_from_str("custom-service");
        assert!(matches!(
            cap,
            Capability::Custom { name, version } if name == "custom-service" && version == "0"
        ));
    }

    #[test]
    fn test_capability_from_str_trim_whitespace() {
        let cap = capability_from_str("  compute  ");
        assert!(matches!(
            cap,
            Capability::Compute(ComputeCapability::NativeExecution)
        ));
    }

    #[test]
    fn test_capability_from_str_case_insensitive() {
        let cap = capability_from_str("STORAGE");
        assert!(matches!(
            cap,
            Capability::Storage(StorageCapability::ObjectStorage)
        ));
    }

    #[test]
    fn test_default_version() {
        assert_eq!(default_version(), "unknown");
    }

    #[test]
    fn test_config_file_deserialize() {
        let json = r#"{"services":[{"name":"svc1","capabilities":["compute"],"endpoints":["http://localhost:8080"]}]}"#;
        let config: ConfigFile = serde_json::from_str(json).expect("parse");
        assert_eq!(config.services.len(), 1);
        assert_eq!(config.services[0].name, "svc1");
        assert_eq!(config.services[0].capabilities, vec!["compute"]);
    }
}
