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
