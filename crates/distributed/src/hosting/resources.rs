use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Resource management for hosting
///
/// Manages resource allocation, tracking, and quotas for hosted workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostingResourceManager {
    /// Resource configuration
    pub config: HostingResourceConfig,
    /// Available resources by resource type
    pub available_resources: HashMap<String, u64>,
}

/// Configuration for hosting resources
///
/// Defines resource management policies including limits and quotas.
/// Supports flexible resource types through HashMap-based configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostingResourceConfig {
    /// Enable resource management
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Resource limits by resource type (e.g., "cpu_cores" -> 16, "memory_gb" -> 64)
    #[serde(default)]
    pub limits: HashMap<String, u64>,
    
    /// Resource quotas by resource type (e.g., "storage_gb" -> 1000)
    #[serde(default)]
    pub quotas: HashMap<String, u64>,
}

fn default_true() -> bool {
    true
}

impl Default for HostingResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            limits: HashMap::new(),
            quotas: HashMap::new(),
        }
    }
}

impl HostingResourceManager {
    /// Create a new resource manager
    #[must_use]
    pub fn new(config: HostingResourceConfig) -> Self {
        Self {
            config,
            available_resources: HashMap::new(),
        }
    }

    /// Allocate resources
    pub fn allocate_resources(
        &mut self,
        requirements: &HashMap<String, u64>,
    ) -> toadstool::ToadStoolResult<()> {
        // Stub implementation - allocate resources
        for (resource, amount) in requirements {
            self.available_resources.insert(resource.clone(), *amount);
        }
        Ok(())
    }

    /// Deallocate resources
    pub fn deallocate_resources(
        &mut self,
        resources: &HashMap<String, u64>,
    ) -> toadstool::ToadStoolResult<()> {
        // Stub implementation - deallocate resources
        for resource in resources.keys() {
            self.available_resources.remove(resource);
        }
        Ok(())
    }
}
