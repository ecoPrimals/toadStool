// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capacity management types

use std::sync::Arc;

use std::sync::RwLock;

use crate::UniversalJob;

// ============================================================================
// Capacity Management
// ============================================================================

/// Thread-safe view of resources available on this node for scheduling.
pub struct LocalCapacityManager {
    pub(crate) available_capacity: Arc<RwLock<CapacityInfo>>,
}

/// Snapshot of CPU, memory, and storage available for accepting work.
#[derive(Debug, Clone)]
pub struct CapacityInfo {
    /// Logical CPU cores available.
    pub cpu_cores: f64,
    /// RAM available in bytes.
    pub memory_bytes: u64,
    /// Aggregate free storage in bytes.
    pub storage_bytes: u64,
}

impl CapacityInfo {
    /// Whether this snapshot satisfies `job` resource requirements.
    #[must_use]
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;
        requirements.cpu.min_cores <= self.cpu_cores
            && requirements.memory.min_bytes <= self.memory_bytes
            && requirements.storage.min_bytes <= self.storage_bytes
    }

    /// Build capacity info from local sysmon readings (best effort).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )]
    pub fn from_system() -> Self {
        let cpu_cores = toadstool_sysmon::cpu_count() as f64;
        let memory_bytes = toadstool_sysmon::memory_info()
            .map(|m| m.available)
            .unwrap_or(0);
        let disks = toadstool_sysmon::disk_usage().unwrap_or_default();
        let storage_bytes: u64 = disks.iter().map(|d| d.available_space).sum();
        Self {
            cpu_cores,
            memory_bytes,
            storage_bytes,
        }
    }
}
