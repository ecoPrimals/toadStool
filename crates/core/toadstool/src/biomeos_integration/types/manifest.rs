//! Core biome manifest and metadata structures
//!
//! This module contains the primary [`BiomeManifest`] structure that represents
//! a complete BiomeOS deployment configuration, along with its metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::agent::AgentConfig;
use super::auth::BiomeSecurity;
use super::config::PrimalsConfig;
use super::networking::BiomeNetworking;
use super::resources::BiomeResources;
use super::storage::BiomeStorage;

/// Enhanced `BiomeManifest` structure for Phase 4 Universal Orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// API version for compatibility
    pub api_version: String,
    /// Manifest type (always "Biome")
    pub kind: String,
    /// Biome metadata
    pub metadata: BiomeMetadata,
    /// Primal-specific configurations
    pub primals: PrimalsConfig,
    /// Storage configuration and provisioning
    pub storage: Option<BiomeStorage>,
    /// AI agent deployment configuration
    pub agents: Option<Vec<AgentConfig>>,
    /// Security policies and authentication
    pub security: Option<BiomeSecurity>,
    /// Network configuration
    pub networking: Option<BiomeNetworking>,
    /// Resource allocation and limits
    pub resources: Option<BiomeResources>,
    /// Legacy services configuration (for backward compatibility)
    pub services: Option<Vec<ServiceConfig>>,
}

/// Biome metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Unique biome name
    pub name: String,
    /// Team or organization
    pub team: Option<String>,
    /// Environment type (dev, staging, prod)
    pub environment: Option<String>,
    /// Biome version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Labels for categorization
    pub labels: HashMap<String, String>,
    /// Annotations for additional metadata
    pub annotations: HashMap<String, String>,
}

/// Legacy service configuration (for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service source
    pub source: ServiceSource,
    /// Replicas
    pub replicas: Option<u32>,
    /// Resource requirements
    pub resources: Option<super::resources::PrimalResources>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<super::networking::PortMapping>,
    /// Volume mounts
    pub volumes: Vec<super::storage::VolumeMountSpec>,
    /// Service dependencies
    pub dependencies: Vec<String>,
    /// Health check configuration
    pub health_check: Option<super::resources::BiomeHealthCheckConfig>,
}

/// Service source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServiceSource {
    /// Container image
    Container {
        /// Image name
        image: String,
        /// Image tag
        tag: String,
        /// Registry
        registry: Option<String>,
    },
    /// WASM module
    Wasm {
        /// Module path
        module: String,
        /// Runtime
        runtime: String,
    },
    /// Native binary
    Native {
        /// Binary path
        path: String,
        /// Arguments
        args: Vec<String>,
    },
}
