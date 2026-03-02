//! Biome manifest types for Primal integration trait.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::primal_types::PrimalConfig;

/// Biome manifest reference (simplified for this module)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// API version
    pub api_version: String,
    /// Manifest kind
    pub kind: String,
    /// Biome metadata
    pub metadata: BiomeMetadata,
    /// Primal configurations
    pub primals: HashMap<String, PrimalConfig>,
}

/// Biome metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Biome name
    pub name: String,
    /// Biome version
    pub version: String,
    /// Environment
    pub environment: Option<String>,
    /// Labels
    pub labels: HashMap<String, String>,
}
