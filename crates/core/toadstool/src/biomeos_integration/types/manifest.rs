// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core biome manifest and metadata structures
//!
//! [`BiomeManifest`] and [`BiomeMetadata`] are re-exported from the canonical
//! `toadstool_core::manifest` module. Legacy service configuration types remain
//! here for biomeOS integration code that predates the NUCLEUS manifest schema.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use toadstool_core::manifest::{BiomeManifest, BiomeMetadata};

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
