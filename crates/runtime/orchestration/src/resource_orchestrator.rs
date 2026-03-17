// SPDX-License-Identifier: AGPL-3.0-only
//! Multi-tenant resource orchestrator.
//!
//! Maps `{tenant, priority, resource_request}` → `{device, time_slot}`.
//! Supports four deployment models:
//!
//! - **Local direct**: Single tenant, full access. Trivially gives everything.
//! - **Local multi**: Multiple springs share GPUs. Priority-based allocation.
//! - **Cloud rental**: External tenants with strict quotas and isolation.
//! - **Cloud consumer**: We rent external GPUs. Checkpointing for preemption.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::OrchestrationError;

/// Deployment model — determines isolation and allocation behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentModel {
    /// Single tenant, full hardware access. No quotas enforced.
    #[default]
    LocalDirect,
    /// Multiple springs share local hardware. Priority-based allocation.
    LocalMulti,
    /// External tenants rent our GPUs. Full isolation + quotas.
    CloudRental,
    /// We consume external GPU resources. Checkpoint on preemption.
    CloudConsumer,
}

/// A request for compute resources from a tenant.
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Tenant identifier.
    pub tenant_id: String,
    /// Priority level (0 = emergency, 5 = background).
    pub priority: u8,
    /// Requested GPU device indices (empty = any available).
    pub preferred_devices: Vec<u32>,
    /// Minimum VRAM required in bytes.
    pub min_vram_bytes: u64,
    /// Estimated execution duration.
    pub estimated_duration: Duration,
}

/// A granted resource allocation.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceAllocation {
    /// Allocated device index.
    pub device_index: u32,
    /// Allocated VRAM in bytes (may be less than total if shared).
    pub vram_bytes: u64,
    /// Time slot start (relative to allocation time).
    pub start_offset: Duration,
    /// Allocated duration (may be less than requested if time-sliced).
    pub granted_duration: Duration,
    /// Whether this is exclusive access or shared.
    pub exclusive: bool,
}

/// Tracks what resources a tenant is currently using.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TenantUsage {
    /// Active device allocations: `device_index` → vram bytes.
    pub device_allocations: HashMap<u32, u64>,
    /// Total compute time used in this accounting period.
    pub compute_time_used: Duration,
    /// Number of active workloads.
    pub active_workloads: u32,
}

/// Resource quota for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuota {
    /// Maximum number of GPU devices.
    pub max_devices: u32,
    /// Maximum total VRAM across all devices.
    pub max_vram_bytes: u64,
    /// Maximum concurrent workloads.
    pub max_concurrent_workloads: u32,
    /// Maximum compute time per accounting period.
    pub max_compute_time: Duration,
}

impl Default for TenantQuota {
    fn default() -> Self {
        Self {
            max_devices: u32::MAX,
            max_vram_bytes: u64::MAX,
            max_concurrent_workloads: u32::MAX,
            max_compute_time: Duration::from_secs(u64::MAX),
        }
    }
}

/// A GPU device available for allocation.
#[derive(Debug, Clone)]
pub struct AvailableDevice {
    /// Device index (DRM card index).
    pub index: u32,
    /// Total VRAM in bytes.
    pub total_vram_bytes: u64,
    /// Currently allocated VRAM in bytes.
    pub allocated_vram_bytes: u64,
    /// Current tenant using the device (if any).
    pub current_tenant: Option<String>,
}

impl AvailableDevice {
    /// VRAM remaining for new allocations.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // AvailableDevice fields may not be const in all contexts
    pub fn free_vram_bytes(&self) -> u64 {
        self.total_vram_bytes
            .saturating_sub(self.allocated_vram_bytes)
    }
}

/// Resource orchestrator for multi-tenant GPU scheduling.
///
/// Thread-safe. All state is behind `RwLock`.
pub struct ResourceOrchestrator {
    model: DeploymentModel,
    devices: Arc<RwLock<Vec<AvailableDevice>>>,
    quotas: Arc<RwLock<HashMap<String, TenantQuota>>>,
    usage: Arc<RwLock<HashMap<String, TenantUsage>>>,
}

impl ResourceOrchestrator {
    /// Create a new orchestrator with the given deployment model and devices.
    #[must_use]
    pub fn new(model: DeploymentModel, devices: Vec<AvailableDevice>) -> Self {
        Self {
            model,
            devices: Arc::new(RwLock::new(devices)),
            quotas: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tenant with resource quotas.
    pub fn register_tenant(&self, tenant_id: &str, quota: TenantQuota) {
        self.quotas.write().insert(tenant_id.to_string(), quota);
        self.usage.write().entry(tenant_id.to_string()).or_default();
    }

    /// Request resource allocation for a tenant.
    ///
    /// # Errors
    ///
    /// Returns an error if resources are unavailable or the tenant exceeds quotas.
    pub fn allocate(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        match self.model {
            DeploymentModel::LocalDirect => self.allocate_local_direct(request),
            DeploymentModel::LocalMulti => self.allocate_local_multi(request),
            DeploymentModel::CloudRental => self.allocate_cloud_rental(request),
            DeploymentModel::CloudConsumer => self.allocate_local_direct(request),
        }
    }

    /// Release a device allocation for a tenant.
    pub fn release(&self, tenant_id: &str, device_index: u32) {
        {
            let mut devices = self.devices.write();
            if let Some(dev) = devices.iter_mut().find(|d| d.index == device_index)
                && dev.current_tenant.as_deref() == Some(tenant_id)
            {
                dev.allocated_vram_bytes = 0;
                dev.current_tenant = None;
            }
        }

        let mut usage = self.usage.write();
        if let Some(tenant_usage) = usage.get_mut(tenant_id) {
            tenant_usage.device_allocations.remove(&device_index);
            tenant_usage.active_workloads = tenant_usage.active_workloads.saturating_sub(1);
        }
    }

    /// Get current usage for a tenant.
    #[must_use]
    pub fn tenant_usage(&self, tenant_id: &str) -> Option<TenantUsage> {
        self.usage.read().get(tenant_id).cloned()
    }

    /// Get all tenant usage stats.
    #[must_use]
    pub fn all_usage(&self) -> HashMap<String, TenantUsage> {
        self.usage.read().clone()
    }

    /// Current deployment model.
    #[must_use]
    pub const fn deployment_model(&self) -> DeploymentModel {
        self.model
    }

    /// Number of managed devices.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Uses RwLock
    pub fn device_count(&self) -> usize {
        self.devices.read().len()
    }

    // --- allocation strategies ---

    #[allow(clippy::significant_drop_tightening)] // device ref from devices lock
    fn allocate_local_direct(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        let devices = self.devices.read();
        let device = if request.preferred_devices.is_empty() {
            devices.iter().max_by_key(|d| d.free_vram_bytes())
        } else {
            devices
                .iter()
                .find(|d| request.preferred_devices.contains(&d.index))
        };

        let device = device.ok_or_else(|| {
            OrchestrationError::ResourceUnavailable(
                "No GPU device available for allocation".to_string(),
            )
        })?;

        Ok(ResourceAllocation {
            device_index: device.index,
            vram_bytes: device.total_vram_bytes,
            start_offset: Duration::ZERO,
            granted_duration: request.estimated_duration,
            exclusive: true,
        })
    }

    #[allow(clippy::significant_drop_tightening)] // device ref from devices, need for allocation
    fn allocate_local_multi(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        self.check_quota(request)?;

        let mut devices = self.devices.write();
        let device = devices
            .iter_mut()
            .filter(|d| {
                request.preferred_devices.is_empty() || request.preferred_devices.contains(&d.index)
            })
            .filter(|d| d.free_vram_bytes() >= request.min_vram_bytes)
            .max_by_key(|d| d.free_vram_bytes());

        let device = device.ok_or_else(|| {
            OrchestrationError::ResourceUnavailable(format!(
                "No device with {} bytes free VRAM",
                request.min_vram_bytes
            ))
        })?;

        let allocated = request.min_vram_bytes.min(device.free_vram_bytes());
        device.allocated_vram_bytes += allocated;
        device.current_tenant = Some(request.tenant_id.clone());
        let idx = device.index;

        drop(devices);

        let mut usage = self.usage.write();
        let tenant_usage = usage.entry(request.tenant_id.clone()).or_default();
        tenant_usage.device_allocations.insert(idx, allocated);
        tenant_usage.active_workloads += 1;

        Ok(ResourceAllocation {
            device_index: idx,
            vram_bytes: allocated,
            start_offset: Duration::ZERO,
            granted_duration: request.estimated_duration,
            exclusive: false,
        })
    }

    fn allocate_cloud_rental(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        self.check_quota(request)?;
        self.allocate_local_multi(request)
    }

    #[allow(clippy::significant_drop_tightening)] // need both quotas and usage for check
    fn check_quota(&self, request: &ResourceRequest) -> Result<(), OrchestrationError> {
        let quotas = self.quotas.read();
        let Some(quota) = quotas.get(&request.tenant_id) else {
            return Ok(());
        };

        let usage = self.usage.read();
        let current = usage.get(&request.tenant_id);

        if let Some(current) = current {
            if current.active_workloads >= quota.max_concurrent_workloads {
                return Err(OrchestrationError::QuotaExceeded(format!(
                    "Tenant {} exceeds max concurrent workloads ({})",
                    request.tenant_id, quota.max_concurrent_workloads
                )));
            }

            let total_allocated: u64 = current.device_allocations.values().sum();
            if total_allocated + request.min_vram_bytes > quota.max_vram_bytes {
                return Err(OrchestrationError::QuotaExceeded(format!(
                    "Tenant {} exceeds VRAM quota ({} + {} > {})",
                    request.tenant_id,
                    total_allocated,
                    request.min_vram_bytes,
                    quota.max_vram_bytes
                )));
            }

            if u32::try_from(current.device_allocations.len()).unwrap_or(u32::MAX)
                >= quota.max_devices
            {
                return Err(OrchestrationError::QuotaExceeded(format!(
                    "Tenant {} exceeds max devices ({})",
                    request.tenant_id, quota.max_devices
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_gpu_devices() -> Vec<AvailableDevice> {
        vec![
            AvailableDevice {
                index: 0,
                total_vram_bytes: 16_000_000_000,
                allocated_vram_bytes: 0,
                current_tenant: None,
            },
            AvailableDevice {
                index: 1,
                total_vram_bytes: 24_000_000_000,
                allocated_vram_bytes: 0,
                current_tenant: None,
            },
        ]
    }

    fn test_request(tenant: &str, priority: u8) -> ResourceRequest {
        ResourceRequest {
            tenant_id: tenant.into(),
            priority,
            preferred_devices: vec![],
            min_vram_bytes: 1_000_000_000,
            estimated_duration: Duration::from_secs(60),
        }
    }

    #[test]
    fn test_local_direct_gives_largest_device() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
        let alloc = orch.allocate(&test_request("hotspring", 3)).unwrap();
        assert_eq!(alloc.device_index, 1);
        assert_eq!(alloc.vram_bytes, 24_000_000_000);
        assert!(alloc.exclusive);
    }

    #[test]
    fn test_local_direct_preferred_device() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
        let mut req = test_request("hotspring", 3);
        req.preferred_devices = vec![0];
        let alloc = orch.allocate(&req).unwrap();
        assert_eq!(alloc.device_index, 0);
    }

    #[test]
    fn test_local_multi_shared_allocation() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
        let alloc1 = orch.allocate(&test_request("hotspring", 3)).unwrap();
        let alloc2 = orch.allocate(&test_request("wetspring", 3)).unwrap();
        assert!(!alloc1.exclusive);
        assert!(!alloc2.exclusive);
    }

    #[test]
    fn test_quota_enforcement_max_workloads() {
        let orch = ResourceOrchestrator::new(DeploymentModel::CloudRental, two_gpu_devices());
        orch.register_tenant(
            "tenant-a",
            TenantQuota {
                max_concurrent_workloads: 1,
                ..Default::default()
            },
        );

        let req = test_request("tenant-a", 3);
        let _alloc1 = orch.allocate(&req).unwrap();
        let result = orch.allocate(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_quota_enforcement_max_vram() {
        let orch = ResourceOrchestrator::new(DeploymentModel::CloudRental, two_gpu_devices());
        orch.register_tenant(
            "tenant-a",
            TenantQuota {
                max_vram_bytes: 500_000_000,
                ..Default::default()
            },
        );

        let req = test_request("tenant-a", 3);
        let result = orch.allocate(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_release_frees_resources() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
        let alloc = orch.allocate(&test_request("hotspring", 3)).unwrap();

        let usage = orch.tenant_usage("hotspring").unwrap();
        assert_eq!(usage.active_workloads, 1);

        orch.release("hotspring", alloc.device_index);

        let usage = orch.tenant_usage("hotspring").unwrap();
        assert_eq!(usage.active_workloads, 0);
    }

    #[test]
    fn test_deployment_model_default() {
        assert_eq!(DeploymentModel::default(), DeploymentModel::LocalDirect);
    }

    #[test]
    fn test_device_count() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
        assert_eq!(orch.device_count(), 2);
    }

    #[test]
    fn test_all_usage_empty() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalDirect, two_gpu_devices());
        assert!(orch.all_usage().is_empty());
    }

    #[test]
    fn test_unregistered_tenant_no_quota_check() {
        let orch = ResourceOrchestrator::new(DeploymentModel::LocalMulti, two_gpu_devices());
        let result = orch.allocate(&test_request("unknown-tenant", 3));
        assert!(result.is_ok());
    }

    #[test]
    fn test_free_vram_calculation() {
        let dev = AvailableDevice {
            index: 0,
            total_vram_bytes: 16_000_000_000,
            allocated_vram_bytes: 4_000_000_000,
            current_tenant: Some("test".into()),
        };
        assert_eq!(dev.free_vram_bytes(), 12_000_000_000);
    }

    #[test]
    fn test_free_vram_saturates() {
        let dev = AvailableDevice {
            index: 0,
            total_vram_bytes: 0,
            allocated_vram_bytes: 100,
            current_tenant: None,
        };
        assert_eq!(dev.free_vram_bytes(), 0);
    }
}
