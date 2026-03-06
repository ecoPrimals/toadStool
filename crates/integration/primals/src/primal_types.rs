// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal configuration and resource types for the integration trait.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Generic configuration for any Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    /// Primal name
    pub name: String,
    /// Primal type
    pub primal_type: PrimalType,
    /// Enable flag
    pub enabled: bool,
    /// Resource allocation
    pub resources: Option<PrimalResources>,
    /// Dependencies on other Primals
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

/// Types of Primals in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimalType {
    /// `ToadStool` - Universal Compute
    ToadStool,
    /// Songbird - Network Coordination
    Songbird,
    /// `BearDog` - Security
    BearDog,
    /// `NestGate` - Storage
    NestGate,
    /// Squirrel - AI
    Squirrel,
    /// biomeOS - Universal OS
    BiomeOS,
    /// Custom Primal
    Custom(String),
}

impl PrimalType {
    /// Convert `PrimalType` to its string representation
    ///
    /// # Examples
    /// ```
    /// use toadstool_integration_primals::PrimalType;
    ///
    /// assert_eq!(PrimalType::Songbird.as_str(), "songbird");
    /// assert_eq!(PrimalType::NestGate.as_str(), "nestgate");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            PrimalType::ToadStool => "toadstool",
            PrimalType::Songbird => "songbird",
            PrimalType::BearDog => "beardog",
            PrimalType::NestGate => "nestgate",
            PrimalType::Squirrel => "squirrel",
            PrimalType::BiomeOS => "biomeos",
            PrimalType::Custom(s) => s.as_str(),
        }
    }

    /// Parse a string into a `PrimalType`
    ///
    /// # Examples
    /// ```
    /// use toadstool_integration_primals::PrimalType;
    ///
    /// let primal = PrimalType::parse_type("songbird").unwrap();
    /// assert_eq!(primal, PrimalType::Songbird);
    /// ```
    pub fn parse_type(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "toadstool" => Ok(PrimalType::ToadStool),
            "songbird" => Ok(PrimalType::Songbird),
            "beardog" => Ok(PrimalType::BearDog),
            "nestgate" => Ok(PrimalType::NestGate),
            "squirrel" => Ok(PrimalType::Squirrel),
            "biomeos" => Ok(PrimalType::BiomeOS),
            other => Ok(PrimalType::Custom(other.to_string())),
        }
    }

    /// Get all standard primal types (excluding Custom)
    ///
    /// # Examples
    /// ```
    /// use toadstool_integration_primals::PrimalType;
    ///
    /// let primals = PrimalType::standard_variants();
    /// assert_eq!(primals.len(), 6);
    /// ```
    #[must_use]
    pub fn standard_variants() -> &'static [PrimalType] {
        &[
            PrimalType::ToadStool,
            PrimalType::Songbird,
            PrimalType::BearDog,
            PrimalType::NestGate,
            PrimalType::Squirrel,
            PrimalType::BiomeOS,
        ]
    }

    /// Check if this is a standard primal type
    #[must_use]
    pub fn is_standard(&self) -> bool {
        !matches!(self, PrimalType::Custom(_))
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
        PrimalType::parse_type(s)
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
        assert_eq!(PrimalType::ToadStool.as_str(), "toadstool");
        assert_eq!(PrimalType::Songbird.as_str(), "songbird");
        assert_eq!(PrimalType::BearDog.as_str(), "beardog");
        assert_eq!(PrimalType::NestGate.as_str(), "nestgate");
        assert_eq!(PrimalType::Squirrel.as_str(), "squirrel");
        assert_eq!(PrimalType::BiomeOS.as_str(), "biomeos");
        assert_eq!(PrimalType::Custom("foo".to_string()).as_str(), "foo");
    }

    #[test]
    fn test_primal_type_parse_type() {
        assert_eq!(
            PrimalType::parse_type("toadstool").unwrap(),
            PrimalType::ToadStool
        );
        assert_eq!(
            PrimalType::parse_type("SONGBIRD").unwrap(),
            PrimalType::Songbird
        );
        assert_eq!(
            PrimalType::parse_type("Beardog").unwrap(),
            PrimalType::BearDog
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
        assert!(variants.contains(&PrimalType::ToadStool));
        assert!(variants.contains(&PrimalType::Songbird));
    }

    #[test]
    fn test_primal_type_is_standard() {
        assert!(PrimalType::ToadStool.is_standard());
        assert!(PrimalType::Songbird.is_standard());
        assert!(!PrimalType::Custom("x".to_string()).is_standard());
    }

    #[test]
    fn test_primal_type_display() {
        assert_eq!(format!("{}", PrimalType::ToadStool), "toadstool");
        assert_eq!(format!("{}", PrimalType::NestGate), "nestgate");
    }

    #[test]
    fn test_primal_type_from_str() {
        let p: PrimalType = "songbird".parse().unwrap();
        assert_eq!(p, PrimalType::Songbird);
    }

    #[test]
    fn test_primal_config_serialization() {
        let config = PrimalConfig {
            name: "test".to_string(),
            primal_type: PrimalType::ToadStool,
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
        let _ = format!("{:?}", gpu);
        let cloned = gpu.clone();
        assert_eq!(cloned.count, gpu.count);
    }
}
