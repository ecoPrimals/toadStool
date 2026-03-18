// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::*;

/// Biome manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// Biome name
    pub name: String,

    /// Biome version
    pub version: String,

    /// Description of the biome
    pub description: Option<String>,

    /// Primal configurations
    pub primals: HashMap<String, PrimalConfig>,

    /// Storage configuration
    pub storage: Option<BiomeStorage>,

    /// Agent configurations
    pub agents: Option<Vec<AgentConfig>>,

    /// Security configuration
    pub security: Option<BiomeSecurity>,

    /// Service configurations
    pub services: Vec<ServiceConfig>,

    /// Networking configuration
    pub networking: Option<BiomeNetworking>,

    /// Resource configuration
    pub resources: Option<BiomeResources>,

    /// Federation configuration
    pub federation: Option<FederationConfig>,

    /// Health check configurations
    pub health_checks: Vec<HealthCheckConfig>,
}
