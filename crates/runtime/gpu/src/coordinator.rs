// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compute Resource Coordinator Implementation

use super::config::ResourceConfig;
use super::traits::LoadBalancer;
use super::types::{
    DeviceId, DeviceRequirements, DeviceUsage, ResourceAllocation, ResourcePool,
    UniversalComputeDevice,
};
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
    _allocations: Arc<RwLock<HashMap<Uuid, ResourceAllocation>>>,
    /// Load balancer
    load_balancer: Arc<Mutex<WeightedRoundRobinBalancer>>,
    /// Configuration
    _config: ResourceConfig,
}

impl ComputeResourceCoordinator {
    /// Creates a new compute resource coordinator.
    #[must_use]
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            resource_pools: Arc::new(RwLock::new(HashMap::new())),
            _allocations: Arc::new(RwLock::new(HashMap::new())),
            load_balancer: Arc::new(Mutex::new(WeightedRoundRobinBalancer::new())),
            _config: config,
        }
    }

    /// Initialize resource pool for device
    ///
    /// # Errors
    ///
    /// Currently always succeeds; reserved for future validation failures.
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

        self.resource_pools
            .write()
            .await
            .insert(device.id.clone(), pool);
        Ok(())
    }

    /// Select optimal device for workload
    ///
    /// # Errors
    ///
    /// Returns when the load balancer cannot select a device (e.g. empty list).
    pub async fn select_device(
        &self,
        available_devices: &[DeviceId],
        requirements: &DeviceRequirements,
    ) -> ToadStoolResult<DeviceId> {
        let load_balancer = self.load_balancer.lock().await;
        load_balancer.select_device(available_devices, requirements)
    }

    /// Allocate resources for session
    ///
    /// # Errors
    ///
    /// Returns when the device pool is missing or lacks memory or compute units.
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

        drop(pools);
        Ok(allocation)
    }

    /// Release resources for session
    ///
    /// # Errors
    ///
    /// Returns when the device pool is not found for `device_id`.
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

        drop(pools);
        Ok(())
    }

    /// Get resource pool statistics
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // utilization ratios for display
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
            compute_utilization_percent: (f64::from(pool.allocated_compute_units)
                / f64::from(pool.total_compute_units))
                * 100.0,
        })
    }

    /// Update device load information
    pub async fn update_device_load(&self, device_id: &DeviceId, usage: &DeviceUsage) {
        let mut load_balancer = self.load_balancer.lock().await;
        load_balancer.update_device_load(device_id, usage);
    }
}

/// Resource pool statistics.
#[derive(Debug, Clone)]
pub struct ResourcePoolStats {
    /// Total memory in bytes.
    pub total_memory: u64,
    /// Allocated memory in bytes.
    pub allocated_memory: u64,
    /// Available memory in bytes.
    pub available_memory: u64,
    /// Total compute units.
    pub total_compute_units: u32,
    /// Allocated compute units.
    pub allocated_compute_units: u32,
    /// Available compute units.
    pub available_compute_units: u32,
    /// Memory utilization (0–100).
    pub memory_utilization_percent: f64,
    /// Compute utilization (0–100).
    pub compute_utilization_percent: f64,
}

/// Weighted round-robin load balancer for device selection.
pub struct WeightedRoundRobinBalancer {
    /// Device weights for weighted selection.
    device_weights: HashMap<DeviceId, f64>,
    /// Current round-robin index.
    current_index: usize,
}

impl Default for WeightedRoundRobinBalancer {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightedRoundRobinBalancer {
    /// Creates a new weighted round-robin balancer.
    #[must_use]
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
        let weight = 1.0 - f64::from(usage.gpu_utilization_percent / 100.0);
        self.device_weights.insert(device_id.clone(), weight);
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // stats map uses f64 for JSON-friendly values
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResourceConfig;
    use crate::types::{DeviceCapabilities, DeviceId, DeviceInfo, DeviceType, GpuFramework};

    fn make_device(id: &str, total_memory: u64, compute_units: u32) -> UniversalComputeDevice {
        UniversalComputeDevice {
            id: DeviceId {
                framework: GpuFramework::Cuda,
                device_index: 0,
                uuid: id.to_string(),
            },
            info: DeviceInfo {
                name: format!("Device {id}"),
                vendor: "Test".to_string(),
                device_type: DeviceType::DiscreteGpu,
                driver_version: "1.0".to_string(),
                architecture: "test".to_string(),
                physical_location: None,
            },
            capabilities: DeviceCapabilities {
                compute_capability: "7.0".to_string(),
                total_memory_bytes: total_memory,
                memory_bandwidth_gbps: 100.0,
                compute_units,
                max_work_group_size: (256, 256, 256),
                supported_data_types: vec![],
                extensions: std::collections::HashMap::new(),
                performance: crate::types::PerformanceCharacteristics {
                    peak_gflops_fp32: 1000.0,
                    peak_gflops_fp64: Some(500.0),
                    peak_gflops_fp16: Some(2000.0),
                    peak_memory_bandwidth_utilization: 0.8,
                    typical_power_watts: 100.0,
                    max_power_watts: 200.0,
                },
            },
            usage: Arc::new(RwLock::new(crate::types::DeviceUsage::default())),
            framework_handle: None,
        }
    }

    #[tokio::test]
    async fn test_initialize_device_pool() {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
        let result = coordinator.initialize_device_pool(&device).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_allocate_resources_success() -> ToadStoolResult<()> {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
        coordinator.initialize_device_pool(&device).await?;

        let requirements = DeviceRequirements::minimal();
        let alloc = coordinator
            .allocate_resources(&device.id, &requirements)
            .await?;
        assert_eq!(alloc.memory_bytes, 64 * 1024 * 1024);
        assert_eq!(alloc.compute_units, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_allocate_resources_insufficient_memory() -> ToadStoolResult<()> {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 32 * 1024 * 1024, 16); // 32MB only
        coordinator.initialize_device_pool(&device).await?;

        let requirements = DeviceRequirements::minimal();
        let err = coordinator
            .allocate_resources(&device.id, &requirements)
            .await
            .expect_err("allocation should fail with insufficient memory");
        assert!(err.to_string().contains("Insufficient memory"));
        Ok(())
    }

    #[tokio::test]
    async fn test_allocate_resources_insufficient_compute() -> ToadStoolResult<()> {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 8 * 1024 * 1024 * 1024, 1);
        coordinator.initialize_device_pool(&device).await?;

        let mut requirements = DeviceRequirements::minimal();
        requirements.min_compute_units = Some(4);
        let err = coordinator
            .allocate_resources(&device.id, &requirements)
            .await
            .expect_err("allocation should fail with insufficient compute units");
        assert!(err.to_string().contains("Insufficient compute"));
        Ok(())
    }

    #[tokio::test]
    async fn test_release_resources() -> ToadStoolResult<()> {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
        coordinator.initialize_device_pool(&device).await?;

        let requirements = DeviceRequirements::minimal();
        let alloc = coordinator
            .allocate_resources(&device.id, &requirements)
            .await?;
        coordinator.release_resources(&device.id, &alloc).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_pool_stats() -> ToadStoolResult<()> {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let device = make_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
        coordinator.initialize_device_pool(&device).await?;

        let s = coordinator
            .get_pool_stats(&device.id)
            .await
            .expect("pool stats should exist after initialization");
        assert_eq!(s.total_memory, 8 * 1024 * 1024 * 1024);
        assert_eq!(s.total_compute_units, 16);
        assert_eq!(s.allocated_memory, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_select_device_empty_fails() {
        let config = ResourceConfig::default();
        let coordinator = ComputeResourceCoordinator::new(config);
        let requirements = DeviceRequirements::minimal();
        let result = coordinator.select_device(&[], &requirements).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_weighted_round_robin_balancer_default() {
        let _ = WeightedRoundRobinBalancer::default();
    }
}
