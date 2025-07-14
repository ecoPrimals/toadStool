//! Compute Resource Coordinator Implementation

use super::config::ResourceConfig;
use super::traits::LoadBalancer;
use super::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Compute resource coordinator
pub struct ComputeResourceCoordinator {
    /// Global resource pools
    resource_pools: Arc<RwLock<HashMap<DeviceId, ResourcePool>>>,
    /// Allocation tracking
    allocations: Arc<RwLock<HashMap<Uuid, ResourceAllocation>>>,
    /// Load balancer
    load_balancer: Arc<Mutex<Box<dyn LoadBalancer>>>,
    /// Configuration
    config: ResourceConfig,
}

impl ComputeResourceCoordinator {
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            resource_pools: Arc::new(RwLock::new(HashMap::new())),
            allocations: Arc::new(RwLock::new(HashMap::new())),
            load_balancer: Arc::new(Mutex::new(Box::new(WeightedRoundRobinBalancer::new()))),
            config,
        }
    }

    /// Initialize resource pool for device
    pub async fn initialize_device_pool(
        &self,
        device: &UniversalComputeDevice,
    ) -> ToadStoolResult<()> {
        let pool = ResourcePool {
            total_memory: device.capabilities.total_memory_bytes,
            allocated_memory: 0,
            total_compute_units: device.capabilities.compute_units,
            allocated_compute_units: 0,
            allocation_queue: Vec::new(),
        };

        let mut pools = self.resource_pools.write().await;
        pools.insert(device.id.clone(), pool);
        Ok(())
    }

    /// Select optimal device for workload
    pub async fn select_device(
        &self,
        available_devices: &[DeviceId],
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId> {
        let load_balancer = self.load_balancer.lock().await;
        load_balancer.select_device(available_devices, requirements)
    }

    /// Allocate resources for session
    pub async fn allocate_resources(
        &self,
        device_id: &DeviceId,
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<ResourceAllocation> {
        let mut pools = self.resource_pools.write().await;
        let pool = pools
            .get_mut(device_id)
            .ok_or_else(|| ToadStoolError::runtime("Device pool not found"))?;

        // Calculate required resources
        let required_memory = requirements.min_memory_bytes.unwrap_or(64 * 1024 * 1024); // 64MB default
        let required_compute_units = requirements.min_compute_units.unwrap_or(1);

        // Check availability
        let available_memory = pool.total_memory - pool.allocated_memory;
        let available_compute_units = pool.total_compute_units - pool.allocated_compute_units;

        if required_memory > available_memory {
            return Err(ToadStoolError::runtime("Insufficient memory available"));
        }

        if required_compute_units > available_compute_units {
            return Err(ToadStoolError::runtime(
                "Insufficient compute units available",
            ));
        }

        // Allocate resources
        pool.allocated_memory += required_memory;
        pool.allocated_compute_units += required_compute_units;

        let allocation = ResourceAllocation {
            memory_bytes: required_memory,
            compute_units: required_compute_units,
            priority: 1,
        };

        Ok(allocation)
    }

    /// Release resources for session
    pub async fn release_resources(
        &self,
        device_id: &DeviceId,
        allocation: &ResourceAllocation,
    ) -> ToadStoolResult<()> {
        let mut pools = self.resource_pools.write().await;
        let pool = pools
            .get_mut(device_id)
            .ok_or_else(|| ToadStoolError::runtime("Device pool not found"))?;

        pool.allocated_memory = pool
            .allocated_memory
            .saturating_sub(allocation.memory_bytes);
        pool.allocated_compute_units = pool
            .allocated_compute_units
            .saturating_sub(allocation.compute_units);

        Ok(())
    }

    /// Get resource pool statistics
    pub async fn get_pool_stats(&self, device_id: &DeviceId) -> Option<ResourcePoolStats> {
        let pools = self.resource_pools.read().await;
        pools.get(device_id).map(|pool| ResourcePoolStats {
            total_memory: pool.total_memory,
            allocated_memory: pool.allocated_memory,
            available_memory: pool.total_memory - pool.allocated_memory,
            total_compute_units: pool.total_compute_units,
            allocated_compute_units: pool.allocated_compute_units,
            available_compute_units: pool.total_compute_units - pool.allocated_compute_units,
            memory_utilization_percent: (pool.allocated_memory as f64 / pool.total_memory as f64)
                * 100.0,
            compute_utilization_percent: (pool.allocated_compute_units as f64
                / pool.total_compute_units as f64)
                * 100.0,
        })
    }

    /// Update device load information
    pub async fn update_device_load(&self, device_id: &DeviceId, usage: &DeviceUsage) {
        let mut load_balancer = self.load_balancer.lock().await;
        load_balancer.update_device_load(device_id, usage);
    }
}

/// Resource pool statistics
#[derive(Debug, Clone)]
pub struct ResourcePoolStats {
    pub total_memory: u64,
    pub allocated_memory: u64,
    pub available_memory: u64,
    pub total_compute_units: u32,
    pub allocated_compute_units: u32,
    pub available_compute_units: u32,
    pub memory_utilization_percent: f64,
    pub compute_utilization_percent: f64,
}

/// Weighted round-robin load balancer
pub struct WeightedRoundRobinBalancer {
    device_weights: HashMap<DeviceId, f64>,
    current_index: usize,
}

impl Default for WeightedRoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightedRoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            device_weights: HashMap::new(),
            current_index: 0,
        }
    }
}

impl LoadBalancer for WeightedRoundRobinBalancer {
    fn select_device(
        &self,
        devices: &[DeviceId],
        _requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId> {
        if devices.is_empty() {
            return Err(ToadStoolError::runtime("No devices available"));
        }

        // Simple round-robin for now
        let index = self.current_index % devices.len();
        Ok(devices[index].clone())
    }

    fn update_device_load(&mut self, device_id: &DeviceId, usage: &DeviceUsage) {
        // Update device weight based on utilization (lower utilization = higher weight)
        let weight = 1.0 - (usage.gpu_utilization_percent / 100.0) as f64;
        self.device_weights.insert(device_id.clone(), weight);
    }

    fn get_statistics(&self) -> HashMap<String, f64> {
        let mut stats = HashMap::new();
        stats.insert(
            "total_devices".to_string(),
            self.device_weights.len() as f64,
        );

        if !self.device_weights.is_empty() {
            let avg_weight =
                self.device_weights.values().sum::<f64>() / self.device_weights.len() as f64;
            stats.insert("average_weight".to_string(), avg_weight);
        }

        stats
    }
}
