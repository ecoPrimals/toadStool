//! Resource management types

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

use crate::{resources::ResourceRequirements, ToadStoolResult};

/// Universal system resources (used by universal scheduler/coordinator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalSystemResources {
    /// CPU cores
    pub cpu_cores: f64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// Storage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth
    pub network_bandwidth: u64,
    /// GPU units
    pub gpu_units: u32,
    /// Special hardware
    pub special_hardware: HashMap<String, u32>,
}

/// Resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Job ID
    pub job_id: Uuid,
    /// Allocated resources
    pub allocated_resources: ResourceRequirements,
    /// Allocation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub allocated_at: SystemTime,
    /// Release timestamp
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub released_at: Option<SystemTime>,
}

/// Resource coordinator
pub struct ResourceCoordinator {
    /// Available resources
    available_resources: Arc<RwLock<UniversalSystemResources>>,
    /// Allocation history
    allocation_history: Arc<RwLock<Vec<ResourceAllocation>>>,
}

impl ResourceCoordinator {
    /// Create new resource coordinator
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// if system resource detection fails.
    #[must_use = "ResourceCoordinator creation should be checked"]
    #[allow(clippy::unused_async)] // API consistency with async resource discovery
    pub async fn new() -> ToadStoolResult<Self> {
        let available_resources = UniversalSystemResources {
            cpu_cores: 8.0,                          // Default to 8 cores
            memory_bytes: 8 * 1024 * 1024 * 1024,    // 8GB default
            storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB default
            network_bandwidth: 1000 * 1024 * 1024,   // 1Gbps default
            gpu_units: 0,
            special_hardware: HashMap::new(),
        };

        Ok(Self {
            available_resources: Arc::new(RwLock::new(available_resources)),
            allocation_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Allocate resources
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// if resource allocation exceeds available capacity.
    #[must_use = "Resource allocation result should be checked"]
    pub async fn allocate_resources(
        &self,
        requirements: &ResourceRequirements,
    ) -> ToadStoolResult<ResourceAllocation> {
        let allocation = ResourceAllocation {
            job_id: Uuid::new_v4(),
            allocated_resources: requirements.clone(),
            allocated_at: SystemTime::now(),
            released_at: None,
        };

        self.allocation_history
            .write()
            .await
            .push(allocation.clone());
        debug!("Allocated resources for job: {}", allocation.job_id);
        Ok(allocation)
    }

    /// Release resources
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// if resource release encounters issues.
    #[must_use = "Resource release result should be checked"]
    pub async fn release_resources(
        &self,
        mut allocation: ResourceAllocation,
    ) -> ToadStoolResult<()> {
        allocation.released_at = Some(SystemTime::now());

        // Add to history
        self.allocation_history.write().await.push(allocation);

        debug!("Released resources for job");
        Ok(())
    }

    /// Get available resources
    pub async fn get_available_resources(&self) -> UniversalSystemResources {
        self.available_resources.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_system_resources_struct() {
        let resources = UniversalSystemResources {
            cpu_cores: 16.0,
            memory_bytes: 32 * 1024 * 1024 * 1024,
            storage_bytes: 500 * 1024 * 1024 * 1024,
            network_bandwidth: 10 * 1024 * 1024 * 1024,
            gpu_units: 2,
            special_hardware: std::collections::HashMap::new(),
        };
        assert_eq!(resources.cpu_cores, 16.0);
        assert_eq!(resources.gpu_units, 2);
    }

    #[test]
    fn test_resource_allocation_struct() {
        let id = Uuid::new_v4();
        let allocation = ResourceAllocation {
            job_id: id,
            allocated_resources: ResourceRequirements::default(),
            allocated_at: SystemTime::now(),
            released_at: None,
        };
        assert_eq!(allocation.job_id, id);
        assert!(allocation.released_at.is_none());
    }

    #[tokio::test]
    async fn test_resource_coordinator_new() {
        let coordinator = ResourceCoordinator::new().await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_resource_coordinator_allocate_and_release() {
        let coordinator = ResourceCoordinator::new().await.unwrap();
        let requirements = ResourceRequirements::default();

        let allocation = coordinator
            .allocate_resources(&requirements)
            .await
            .expect("allocate should succeed");
        assert!(allocation.released_at.is_none());

        let release_result = coordinator.release_resources(allocation).await;
        assert!(release_result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_coordinator_get_available_resources() {
        let coordinator = ResourceCoordinator::new().await.unwrap();
        let resources = coordinator.get_available_resources().await;
        assert_eq!(resources.cpu_cores, 8.0);
        assert!(resources.memory_bytes > 0);
    }

    #[test]
    fn test_universal_system_resources_serde() {
        let resources = UniversalSystemResources {
            cpu_cores: 4.0,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            storage_bytes: 100 * 1024 * 1024 * 1024,
            network_bandwidth: 1000 * 1024 * 1024,
            gpu_units: 0,
            special_hardware: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&resources).unwrap();
        let decoded: UniversalSystemResources = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.cpu_cores, resources.cpu_cores);
    }
}
