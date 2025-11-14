//! Resource management types

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
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
    pub allocated_at: chrono::DateTime<chrono::Utc>,
    /// Release timestamp
    pub released_at: Option<chrono::DateTime<chrono::Utc>>,
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
    pub async fn allocate_resources(
        &self,
        requirements: &ResourceRequirements,
    ) -> ToadStoolResult<ResourceAllocation> {
        let allocation = ResourceAllocation {
            job_id: Uuid::new_v4(),
            allocated_resources: requirements.clone(),
            allocated_at: chrono::Utc::now(),
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
    pub async fn release_resources(
        &self,
        mut allocation: ResourceAllocation,
    ) -> ToadStoolResult<()> {
        allocation.released_at = Some(chrono::Utc::now());

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
