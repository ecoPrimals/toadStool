// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal configuration and resource types for the integration trait.
//!
//! ## WateringHole Sovereignty: Capability-Based Types
//!
//! PrimalType represents **capability categories**, not primal names. ToadStool
//! only has self-knowledge; other primals are discovered at runtime by capability.
//! Use `SelfIdentity` for this primal; discover others via capability queries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toadstool_common::interned_strings::primals as legacy;

/// Generic configuration for any Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    /// Primal name
    pub name: String,
    /// Primal type (capability category)
    pub primal_type: PrimalType,
    /// Enable flag
    pub enabled: bool,
    /// Resource allocation
    pub resources: Option<PrimalResources>,
    /// Dependencies on other Primals (use "capability:storage" format)
    pub dependencies: Vec<String>,
    /// Custom configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Labels for metadata
    pub labels: HashMap<String, String>,
    /// Annotations for additional metadata
    pub annotations: HashMap<String, String>,
}

/// Capability categories for primal discovery (WateringHole sovereignty).
///
/// Represents WHAT a primal does, not WHO it is. ToadStool only knows itself
/// (SelfIdentity); other primals are discovered at runtime by capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimalType {
    /// Cryptographic operations (key derivation, encryption, signing)
    Crypto,
    /// Persistent storage and data management
    Storage,
    /// Service discovery and coordination
    Discovery,
    /// Orchestration and system monitoring
    Orchestration,
    /// Compute execution (native, WASM, GPU, ML)
    Compute,
    /// Self-identity (this primal only — ToadStool knows it IS ToadStool)
    SelfIdentity,
    /// Custom capability (discovered at runtime)
    Custom(String),
}

impl PrimalType {
    /// Convert `PrimalType` to its capability string representation
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "not const due to future evolution"
    )] // Custom(s) => s.as_str() is not const
    pub fn as_str(&self) -> &str {
        match self {
            Self::Crypto => "crypto",
            Self::Storage => "storage",
            Self::Discovery => "discovery",
            Self::Orchestration => "orchestration",
            Self::Compute => "compute",
            Self::SelfIdentity => "self",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Parse a string into a `PrimalType` by **capability category**.
    ///
    /// Canonical names: "crypto", "storage", "discovery", "orchestration", "compute", "self".
    /// Legacy primal names ("security", "storage", etc.) are accepted for manifest
    /// backward compatibility but should be migrated to capability strings.
    ///
    /// # Errors
    ///
    /// Reserved for future strict parsing; currently returns `Ok` for all inputs (unknown strings become [`Self::Custom`]).
    pub fn parse_type(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            // Canonical capability names
            "crypto" | "pki" | "security" => Ok(Self::Crypto),
            "storage" => Ok(Self::Storage),
            "discovery" | "coordination" => Ok(Self::Discovery),
            "orchestration" | "network" => Ok(Self::Orchestration),
            "compute" | "ai" | "ml" | "intelligence" => Ok(Self::Compute),
            "self" | "self_identity" => Ok(Self::SelfIdentity),
            s if s == legacy::TOADSTOOL => Ok(Self::SelfIdentity),
            // Legacy primal-name aliases (backward compat for existing manifests)
            legacy::LEGACY_COORDINATION_LABEL | "song-bird" => Ok(Self::Discovery),
            legacy::LEGACY_SECURITY_LABEL | "bear-dog" => Ok(Self::Crypto),
            legacy::LEGACY_STORAGE_LABEL | "nest-gate" => Ok(Self::Storage),
            legacy::LEGACY_INTELLIGENCE_LABEL => Ok(Self::Compute),
            "biomeos" => Ok(Self::Orchestration),
            other => Ok(Self::Custom(other.to_string())),
        }
    }

    /// Get all standard capability categories (excluding Custom)
    #[must_use]
    pub const fn standard_variants() -> &'static [Self] {
        &[
            Self::Crypto,
            Self::Storage,
            Self::Discovery,
            Self::Orchestration,
            Self::Compute,
            Self::SelfIdentity,
        ]
    }

    /// Check if this is a standard capability category
    #[must_use]
    pub const fn is_standard(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }
}

impl std::fmt::Display for PrimalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PrimalType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_type(s)
    }
}

/// Resource allocation for a Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResources {
    /// CPU cores allocation
    pub cpu_cores: Option<f64>,
    /// Memory allocation in GB
    pub memory_gb: Option<f64>,
    /// Storage allocation in GB
    pub storage_gb: Option<f64>,
    /// GPU allocation
    pub gpu: Option<GpuAllocation>,
    /// Network bandwidth limit
    pub network_bandwidth: Option<String>,
    /// Custom resource limits
    pub custom_limits: HashMap<String, serde_json::Value>,
}

impl PrimalConfig {
    /// Build integration config from a canonical manifest primal entry.
    ///
    /// The HashMap key in [`toadstool_core::manifest::BiomeManifest::primals`] is the
    /// primal name passed here.
    #[must_use]
    pub fn from_manifest(
        name: &str,
        entry: &toadstool_core::manifest::ManifestPrimalConfig,
    ) -> Self {
        Self {
            name: name.to_string(),
            primal_type: infer_primal_type(name, &entry.capabilities),
            enabled: entry.enabled,
            resources: entry.resources.as_ref().map(manifest_resources_to_primal),
            dependencies: entry.dependencies.clone(),
            config: entry.config.clone(),
            environment: HashMap::new(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }
}

fn infer_primal_type(name: &str, capabilities: &[String]) -> PrimalType {
    for cap in capabilities {
        let parsed = PrimalType::parse_type(cap).unwrap();
        if parsed.is_standard() {
            return parsed;
        }
    }
    PrimalType::parse_type(name).unwrap()
}

fn manifest_resources_to_primal(
    resources: &toadstool_core::manifest::ManifestResources,
) -> PrimalResources {
    let mut custom_limits = HashMap::new();
    if let Some(memory_limit) = &resources.memory_limit {
        custom_limits.insert(
            "memory_limit".to_string(),
            serde_json::Value::String(memory_limit.clone()),
        );
    }
    if let Some(storage_limit) = &resources.storage_limit {
        custom_limits.insert(
            "storage_limit".to_string(),
            serde_json::Value::String(storage_limit.clone()),
        );
    }
    PrimalResources {
        cpu_cores: resources.cpu_limit,
        memory_gb: None,
        storage_gb: None,
        gpu: resources.gpu_limit.map(|count| GpuAllocation {
            count,
            gpu_type: None,
            memory_gb: None,
            cuda_capability: None,
        }),
        network_bandwidth: None,
        custom_limits,
    }
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Number of GPUs
    pub count: u32,
    /// GPU type preference
    pub gpu_type: Option<String>,
    /// Memory per GPU in GB
    pub memory_gb: Option<f64>,
    /// CUDA compute capability
    pub cuda_capability: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_primal_type_as_str() {
        assert_eq!(PrimalType::Crypto.as_str(), "crypto");
        assert_eq!(PrimalType::Storage.as_str(), "storage");
        assert_eq!(PrimalType::Discovery.as_str(), "discovery");
        assert_eq!(PrimalType::Orchestration.as_str(), "orchestration");
        assert_eq!(PrimalType::Compute.as_str(), "compute");
        assert_eq!(PrimalType::SelfIdentity.as_str(), "self");
        assert_eq!(PrimalType::Custom("foo".to_string()).as_str(), "foo");
    }

    #[test]
    fn test_primal_type_parse_type() {
        assert_eq!(
            PrimalType::parse_type("toadstool").unwrap(),
            PrimalType::SelfIdentity
        );
        assert_eq!(
            PrimalType::parse_type("SONGBIRD").unwrap(),
            PrimalType::Discovery
        );
        assert_eq!(
            PrimalType::parse_type("Beardog").unwrap(),
            PrimalType::Crypto
        );
        assert_eq!(
            PrimalType::parse_type("storage").unwrap(),
            PrimalType::Storage
        );
        assert_eq!(
            PrimalType::parse_type("custom_type").unwrap(),
            PrimalType::Custom("custom_type".to_string())
        );
    }

    #[test]
    fn test_primal_type_standard_variants() {
        let variants = PrimalType::standard_variants();
        assert_eq!(variants.len(), 6);
        assert!(variants.contains(&PrimalType::Crypto));
        assert!(variants.contains(&PrimalType::Storage));
        assert!(variants.contains(&PrimalType::SelfIdentity));
    }

    #[test]
    fn test_primal_type_is_standard() {
        assert!(PrimalType::SelfIdentity.is_standard());
        assert!(PrimalType::Storage.is_standard());
        assert!(!PrimalType::Custom("x".to_string()).is_standard());
    }

    #[test]
    fn test_primal_type_display() {
        assert_eq!(format!("{}", PrimalType::SelfIdentity), "self");
        assert_eq!(format!("{}", PrimalType::Storage), "storage");
    }

    #[test]
    fn test_primal_type_from_str() {
        let p: PrimalType = "storage".parse().unwrap();
        assert_eq!(p, PrimalType::Storage);
    }

    #[test]
    fn test_primal_config_serialization() {
        let config = PrimalConfig {
            name: "test".to_string(),
            primal_type: PrimalType::SelfIdentity,
            enabled: true,
            resources: None,
            dependencies: vec![],
            config: HashMap::new(),
            environment: HashMap::new(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: PrimalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, config.name);
        assert_eq!(parsed.primal_type, config.primal_type);
    }

    #[test]
    fn test_primal_resources_serialization() {
        let resources = PrimalResources {
            cpu_cores: Some(4.0),
            memory_gb: Some(8.0),
            storage_gb: None,
            gpu: Some(GpuAllocation {
                count: 1,
                gpu_type: Some("nvidia".to_string()),
                memory_gb: Some(16.0),
                cuda_capability: None,
            }),
            network_bandwidth: None,
            custom_limits: HashMap::new(),
        };
        let json = serde_json::to_string(&resources).unwrap();
        let parsed: PrimalResources = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cpu_cores, resources.cpu_cores);
        assert_eq!(parsed.gpu.as_ref().unwrap().count, 1);
    }

    #[test]
    fn test_gpu_allocation_debug_clone() {
        let gpu = GpuAllocation {
            count: 2,
            gpu_type: Some("nvidia".to_string()),
            memory_gb: Some(24.0),
            cuda_capability: Some("8.0".to_string()),
        };
        let _ = format!("{gpu:?}");
        let cloned = gpu.clone();
        assert_eq!(cloned.count, gpu.count);
    }
}
