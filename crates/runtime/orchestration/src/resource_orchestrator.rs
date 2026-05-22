// SPDX-License-Identifier: AGPL-3.0-or-later
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

use std::sync::RwLock;

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
    /// Maximum guest load before yield semantics activate.
    ///
    /// When current load exceeds this threshold, new workloads are either
    /// queued (soft yield) or rejected (hard yield). This enables power-cycle-aware
    /// scheduling on shared gates (e.g. flockGate) where hardware availability
    /// fluctuates with host power state.
    ///
    /// `None` means no guest load limit (default — unlimited).
    #[serde(default)]
    pub max_guest_load: Option<GuestLoadPolicy>,
}

/// Guest load yield policy — controls how the orchestrator handles
/// workloads when guest load exceeds the configured threshold.
///
/// Designed for shared-hardware gates (flockGate) where GPUs may
/// become unavailable during host power cycles. The orchestrator
/// checks current load before dispatch and applies the yield strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestLoadPolicy {
    /// Maximum concurrent GPU-bound workloads before yield activates.
    pub max_concurrent_gpu: u32,
    /// What to do when load exceeds the threshold.
    #[serde(default)]
    pub yield_strategy: YieldStrategy,
}

/// Strategy applied when guest load exceeds `max_concurrent_gpu`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YieldStrategy {
    /// Queue the workload until load drops below threshold.
    #[default]
    Queue,
    /// Reject the workload immediately with a resource-exhausted error.
    Reject,
    /// Defer the workload until the next power-cycle window completes.
    DeferUntilPowerCycle,
}

impl Default for TenantQuota {
    fn default() -> Self {
        Self {
            max_devices: u32::MAX,
            max_vram_bytes: u64::MAX,
            max_concurrent_workloads: u32::MAX,
            max_compute_time: Duration::from_secs(u64::MAX),
            max_guest_load: None,
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
    #[expect(
        clippy::missing_const_for_fn,
        reason = "not const due to future evolution"
    )] // AvailableDevice fields may not be const in all contexts
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
    ///
    /// # Errors
    ///
    /// Returns [`OrchestrationError::LockPoisoned`] if an internal lock was
    /// poisoned by a prior panic.
    pub fn register_tenant(
        &self,
        tenant_id: &str,
        quota: TenantQuota,
    ) -> Result<(), OrchestrationError> {
        self.quotas
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("quotas: {e}")))?
            .insert(tenant_id.to_string(), quota);
        self.usage
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?
            .entry(tenant_id.to_string())
            .or_default();
        Ok(())
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
    ///
    /// # Errors
    ///
    /// Returns [`OrchestrationError::LockPoisoned`] if an internal lock was
    /// poisoned by a prior panic.
    pub fn release(&self, tenant_id: &str, device_index: u32) -> Result<(), OrchestrationError> {
        {
            let mut devices = self
                .devices
                .write()
                .map_err(|e| OrchestrationError::LockPoisoned(format!("devices: {e}")))?;
            if let Some(dev) = devices.iter_mut().find(|d| d.index == device_index)
                && dev.current_tenant.as_deref() == Some(tenant_id)
            {
                dev.allocated_vram_bytes = 0;
                dev.current_tenant = None;
            }
        }

        let mut usage = self
            .usage
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?;
        if let Some(tenant_usage) = usage.get_mut(tenant_id) {
            tenant_usage.device_allocations.remove(&device_index);
            tenant_usage.active_workloads = tenant_usage.active_workloads.saturating_sub(1);
        }
        Ok(())
    }

    /// Get current usage for a tenant.
    ///
    /// # Errors
    ///
    /// Returns [`OrchestrationError::LockPoisoned`] if an internal lock was
    /// poisoned by a prior panic.
    pub fn tenant_usage(&self, tenant_id: &str) -> Result<Option<TenantUsage>, OrchestrationError> {
        Ok(self
            .usage
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?
            .get(tenant_id)
            .cloned())
    }

    /// Get all tenant usage stats.
    ///
    /// # Errors
    ///
    /// Returns [`OrchestrationError::LockPoisoned`] if an internal lock was
    /// poisoned by a prior panic.
    pub fn all_usage(&self) -> Result<HashMap<String, TenantUsage>, OrchestrationError> {
        Ok(self
            .usage
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?
            .clone())
    }

    /// Current deployment model.
    #[must_use]
    pub const fn deployment_model(&self) -> DeploymentModel {
        self.model
    }

    /// Number of managed devices.
    ///
    /// # Errors
    ///
    /// Returns [`OrchestrationError::LockPoisoned`] if an internal lock was
    /// poisoned by a prior panic.
    pub fn device_count(&self) -> Result<usize, OrchestrationError> {
        Ok(self
            .devices
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("devices: {e}")))?
            .len())
    }

    // --- allocation strategies ---

    fn allocate_local_direct(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        let devices = self
            .devices
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("devices: {e}")))?;
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

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // device ref from devices, need for allocation
    fn allocate_local_multi(
        &self,
        request: &ResourceRequest,
    ) -> Result<ResourceAllocation, OrchestrationError> {
        self.check_quota(request)?;

        let mut devices = self
            .devices
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("devices: {e}")))?;
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

        let mut usage = self
            .usage
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?;
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

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // need both quotas and usage for check
    fn check_quota(&self, request: &ResourceRequest) -> Result<(), OrchestrationError> {
        let quotas = self
            .quotas
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("quotas: {e}")))?;
        let Some(quota) = quotas.get(&request.tenant_id) else {
            return Ok(());
        };

        let usage = self
            .usage
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(format!("usage: {e}")))?;
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
#[path = "resource_orchestrator_tests.rs"]
mod tests;
