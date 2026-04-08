// SPDX-License-Identifier: AGPL-3.0-or-later
use super::manifest::ServiceSource;
use super::resources::PrimalResources;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Networking configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    /// Enable coordination service integration
    pub coordination: bool,
    /// Network mode
    pub mode: String,
    /// DNS settings
    pub dns: Option<DNSConfig>,
    /// Port mappings
    pub port_mappings: Vec<PortMapping>,
    /// Service mesh settings
    pub service_mesh: Option<ServiceMeshConfig>,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSConfig {
    /// DNS servers
    pub servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
}

/// Port mapping definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port
    pub host_port: u16,
    /// Protocol
    pub protocol: String,
}

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    /// Enable service mesh
    pub enabled: bool,
    /// Mesh provider
    pub provider: String,
    /// Mesh settings
    pub settings: HashMap<String, serde_json::Value>,
}

// BiomeResources moved to resources.rs module

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
    pub resources: Option<PrimalResources>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<String>,
}
