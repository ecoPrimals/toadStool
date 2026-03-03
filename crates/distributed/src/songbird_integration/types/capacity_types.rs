// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capacity management types

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::UniversalJob;

// ============================================================================
// Capacity Management
// ============================================================================

pub struct LocalCapacityManager {
    pub(crate) available_capacity: Arc<RwLock<CapacityInfo>>,
}

#[derive(Debug, Clone)]
pub struct CapacityInfo {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
}

impl CapacityInfo {
    #[must_use]
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;
        requirements.cpu.min_cores <= self.cpu_cores
            && requirements.memory.min_bytes <= self.memory_bytes
            && requirements.storage.min_bytes <= self.storage_bytes
    }

    #[must_use]
    pub fn from_system() -> Self {
        use sysinfo::Disks;
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let memory_bytes = sys.available_memory();
        let disks = Disks::new_with_refreshed_list();
        let storage_bytes: u64 = disks
            .iter()
            .filter(|disk| {
                let fs = disk.file_system().to_string_lossy();
                !fs.contains("tmpfs")
                    && !fs.contains("devtmpfs")
                    && !fs.contains("squashfs")
                    && !fs.contains("overlay")
            })
            .map(sysinfo::Disk::available_space)
            .sum();
        Self {
            cpu_cores,
            memory_bytes,
            storage_bytes,
        }
    }
}
