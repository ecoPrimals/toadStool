// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Resource management for hosting
///
/// Manages resource allocation, tracking, and quotas for hosted workloads.
/// **Deep Debt Evolution**: Complete implementation with proper tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostingResourceManager {
    /// Resource configuration
    pub config: HostingResourceConfig,
    /// Total available resources by resource type
    pub total_resources: HashMap<String, u64>,
    /// Currently allocated resources by resource type
    pub allocated_resources: HashMap<String, u64>,
    /// Active allocations by allocation ID
    #[serde(skip)]
    pub active_allocations: HashMap<String, HashMap<String, u64>>,
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

    /// Reservation buffer percentage (0.0-1.0) - keep some resources available
    #[serde(default = "default_buffer")]
    pub reservation_buffer: f64,
}

const fn default_true() -> bool {
    true
}

const fn default_buffer() -> f64 {
    0.1 // 10% buffer by default
}

impl Default for HostingResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            limits: HashMap::new(),
            quotas: HashMap::new(),
            reservation_buffer: 0.1,
        }
    }
}

impl HostingResourceManager {
    /// Create a new resource manager
    #[must_use]
    pub fn new(config: HostingResourceConfig) -> Self {
        Self {
            config,
            total_resources: HashMap::new(),
            allocated_resources: HashMap::new(),
            active_allocations: HashMap::new(),
        }
    }

    /// Create resource manager initialized from system resources
    ///
    /// **Deep Debt**: Self-knowledge - query actual system resources
    #[must_use]
    pub fn from_system(config: HostingResourceConfig) -> Self {
        let mut manager = Self::new(config);

        #[allow(clippy::cast_possible_truncation)]
        let cpu_cores = toadstool_sysmon::cpu_count() as u64;
        manager
            .total_resources
            .insert("cpu_cores".to_string(), cpu_cores);

        let memory_bytes = toadstool_sysmon::memory_info()
            .map(|m| m.total)
            .unwrap_or(0);
        manager
            .total_resources
            .insert("memory_bytes".to_string(), memory_bytes);

        // Initialize allocated as zero
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);
        manager
            .allocated_resources
            .insert("memory_bytes".to_string(), 0);

        debug!(
            "Initialized HostingResourceManager: {} CPU cores, {} GB memory",
            cpu_cores,
            memory_bytes / 1024 / 1024 / 1024
        );

        manager
    }

    /// Get available (unallocated) resources for a resource type
    #[must_use]
    pub fn available(&self, resource_type: &str) -> u64 {
        let total = self
            .total_resources
            .get(resource_type)
            .copied()
            .unwrap_or(0);
        let allocated = self
            .allocated_resources
            .get(resource_type)
            .copied()
            .unwrap_or(0);
        total.saturating_sub(allocated)
    }

    /// Check if requirements can be satisfied
    #[must_use]
    pub fn can_allocate(&self, requirements: &HashMap<String, u64>) -> bool {
        if !self.config.enabled {
            return true; // Resource management disabled, allow all
        }

        for (resource, amount) in requirements {
            // When no total is declared for a resource type, treat it as
            // unlimited — the manager has no basis to deny the allocation.
            let total = self.total_resources.get(resource).copied();
            if total.is_none() {
                // No capacity declared → skip limit check for this resource
                continue;
            }
            let available = self.available(resource);
            let buffer = (total.unwrap_or(0) as f64 * self.config.reservation_buffer) as u64;

            if *amount > available.saturating_sub(buffer) {
                debug!(
                    "Cannot allocate {} of {}: available={}, buffer={}, requested={}",
                    amount, resource, available, buffer, amount
                );
                return false;
            }

            // Check against limits
            if let Some(limit) = self.config.limits.get(resource)
                && *amount > *limit
            {
                debug!(
                    "Allocation {} of {} exceeds limit {}",
                    amount, resource, limit
                );
                return false;
            }
        }
        true
    }

    /// Allocate resources for a workload
    ///
    /// Returns an allocation ID that can be used to deallocate later.
    pub fn allocate_resources(
        &mut self,
        allocation_id: &str,
        requirements: &HashMap<String, u64>,
    ) -> toadstool::ToadStoolResult<()> {
        if !self.config.enabled {
            debug!("Resource management disabled, skipping allocation");
            return Ok(());
        }

        // Check if allocation is possible
        if !self.can_allocate(requirements) {
            return Err(toadstool::ToadStoolError::resource(
                "Insufficient resources for allocation".to_string(),
            ));
        }

        // Perform allocation
        for (resource, amount) in requirements {
            let current = self
                .allocated_resources
                .entry(resource.clone())
                .or_insert(0);
            *current = current.saturating_add(*amount);
        }

        // Track this allocation
        self.active_allocations
            .insert(allocation_id.to_string(), requirements.clone());

        debug!(
            "Allocated resources for {}: {:?}",
            allocation_id, requirements
        );
        Ok(())
    }

    /// Deallocate resources by allocation ID
    pub fn deallocate_resources(&mut self, allocation_id: &str) -> toadstool::ToadStoolResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if let Some(resources) = self.active_allocations.remove(allocation_id) {
            for (resource, amount) in &resources {
                if let Some(current) = self.allocated_resources.get_mut(resource) {
                    *current = current.saturating_sub(*amount);
                }
            }
            debug!(
                "Deallocated resources for {}: {:?}",
                allocation_id, resources
            );
        } else {
            warn!(
                "Attempted to deallocate unknown allocation: {}",
                allocation_id
            );
        }
        Ok(())
    }

    /// Get current resource utilization as a percentage (0.0-1.0)
    #[must_use]
    pub fn utilization(&self, resource_type: &str) -> f64 {
        let total = self
            .total_resources
            .get(resource_type)
            .copied()
            .unwrap_or(0);
        let allocated = self
            .allocated_resources
            .get(resource_type)
            .copied()
            .unwrap_or(0);

        if total == 0 {
            0.0
        } else {
            allocated as f64 / total as f64
        }
    }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;

    #[test]
    fn test_resource_manager_allocation() {
        let mut manager = HostingResourceManager::new(HostingResourceConfig::default());

        // Set up some resources
        manager.total_resources.insert("cpu_cores".to_string(), 8);
        manager
            .total_resources
            .insert("memory_bytes".to_string(), 16_000_000_000);
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);
        manager
            .allocated_resources
            .insert("memory_bytes".to_string(), 0);

        // Test allocation
        let mut requirements = HashMap::new();
        requirements.insert("cpu_cores".to_string(), 2);
        requirements.insert("memory_bytes".to_string(), 4_000_000_000);

        assert!(manager.can_allocate(&requirements));
        assert!(manager.allocate_resources("job-1", &requirements).is_ok());

        // Check utilization
        assert!((manager.utilization("cpu_cores") - 0.25).abs() < 0.01);
        assert!((manager.utilization("memory_bytes") - 0.25).abs() < 0.01);

        // Deallocate
        assert!(manager.deallocate_resources("job-1").is_ok());
        assert!((manager.utilization("cpu_cores") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_resources() {
        let mut manager = HostingResourceManager::new(HostingResourceConfig::default());

        manager.total_resources.insert("cpu_cores".to_string(), 4);
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);

        let mut requirements = HashMap::new();
        requirements.insert("cpu_cores".to_string(), 10); // More than available

        assert!(!manager.can_allocate(&requirements));
    }

    #[test]
    fn test_reservation_buffer_enforced() {
        let config = HostingResourceConfig {
            enabled: true,
            limits: HashMap::new(),
            quotas: HashMap::new(),
            reservation_buffer: 0.2, // 20% buffer
        };
        let mut manager = HostingResourceManager::new(config);
        manager.total_resources.insert("cpu_cores".to_string(), 10);
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);

        // 10 cores with 20% buffer = 8 effective; requesting 9 should fail
        let mut req = HashMap::new();
        req.insert("cpu_cores".to_string(), 9);
        assert!(!manager.can_allocate(&req));

        // Requesting 8 should succeed
        req.insert("cpu_cores".to_string(), 8);
        assert!(manager.can_allocate(&req));
    }

    #[test]
    fn test_limits_enforced() {
        let mut limits = HashMap::new();
        limits.insert("gpu_vram".to_string(), 4);
        let config = HostingResourceConfig {
            enabled: true,
            limits,
            quotas: HashMap::new(),
            reservation_buffer: 0.0,
        };
        let mut manager = HostingResourceManager::new(config);
        manager.total_resources.insert("gpu_vram".to_string(), 100);
        manager
            .allocated_resources
            .insert("gpu_vram".to_string(), 0);

        // Requesting 5 should fail (limit is 4)
        let mut req = HashMap::new();
        req.insert("gpu_vram".to_string(), 5);
        assert!(!manager.can_allocate(&req));

        // Requesting 4 should succeed
        req.insert("gpu_vram".to_string(), 4);
        assert!(manager.can_allocate(&req));
    }

    #[test]
    fn test_disabled_manager_allows_all() {
        let config = HostingResourceConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = HostingResourceManager::new(config);

        let mut req = HashMap::new();
        req.insert("cpu_cores".to_string(), 1_000_000);
        assert!(manager.can_allocate(&req));
    }

    #[test]
    fn test_multiple_allocations_accumulate() {
        let mut manager = HostingResourceManager::new(HostingResourceConfig::default());
        manager.total_resources.insert("cpu_cores".to_string(), 8);
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);

        let mut req = HashMap::new();
        req.insert("cpu_cores".to_string(), 2);

        assert!(manager.allocate_resources("j1", &req).is_ok());
        assert!(manager.allocate_resources("j2", &req).is_ok());
        assert!(manager.allocate_resources("j3", &req).is_ok());
        assert_eq!(manager.available("cpu_cores"), 2);

        // Fourth allocation should fail (2 remaining, buffer consumes some)
        req.insert("cpu_cores".to_string(), 3);
        assert!(!manager.can_allocate(&req));
    }

    #[test]
    fn test_deallocate_unknown_is_ok() {
        let mut manager = HostingResourceManager::new(HostingResourceConfig::default());
        // Should not error on unknown allocation
        assert!(manager.deallocate_resources("nonexistent").is_ok());
    }

    #[test]
    fn test_utilization_empty() {
        let manager = HostingResourceManager::new(HostingResourceConfig::default());
        assert_eq!(manager.utilization("anything"), 0.0);
    }

    #[test]
    fn test_available_unknown_resource() {
        let manager = HostingResourceManager::new(HostingResourceConfig::default());
        assert_eq!(manager.available("unknown_resource"), 0);
    }

    #[test]
    fn test_from_system_has_cpu_and_memory() {
        let manager = HostingResourceManager::from_system(HostingResourceConfig::default());
        assert!(
            manager
                .total_resources
                .get("cpu_cores")
                .copied()
                .unwrap_or(0)
                > 0
        );
        assert!(
            manager
                .total_resources
                .get("memory_bytes")
                .copied()
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn test_unknown_resource_type_allows_allocation() {
        let mut manager = HostingResourceManager::new(HostingResourceConfig::default());
        manager.total_resources.insert("cpu_cores".to_string(), 4);
        manager
            .allocated_resources
            .insert("cpu_cores".to_string(), 0);

        // Request an unknown resource type alongside a known one
        let mut req = HashMap::new();
        req.insert("cpu_cores".to_string(), 2);
        req.insert("fpga_units".to_string(), 10); // No total declared
        assert!(manager.can_allocate(&req));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = HostingResourceConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        let roundtrip: HostingResourceConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip.enabled, config.enabled);
        assert!((roundtrip.reservation_buffer - config.reservation_buffer).abs() < f64::EPSILON);
    }
}
