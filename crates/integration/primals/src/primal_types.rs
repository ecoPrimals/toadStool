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
    /// Convert PrimalType to its string representation
    ///
    /// # Examples
    /// ```
    /// use toadstool_integration_primals::PrimalType;
    ///
    /// assert_eq!(PrimalType::Songbird.as_str(), "songbird");
    /// assert_eq!(PrimalType::NestGate.as_str(), "nestgate");
    /// ```
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

    /// Parse a string into a PrimalType
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
