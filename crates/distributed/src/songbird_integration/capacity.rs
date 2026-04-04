// SPDX-License-Identifier: AGPL-3.0-only
//! Local capacity probing, reservation, and node capability reporting.

use std::sync::Arc;

use toadstool::error::ToadStoolResult;
use tokio::sync::RwLock;

use super::types::{
    CapacityConfig, CapacityInfo, LocalCapacityManager, NodeCapabilities, ResourceReservation,
};

impl LocalCapacityManager {
    /// Probe system capacity and initialize the local capacity manager.
    pub async fn new(_config: CapacityConfig) -> ToadStoolResult<Self> {
        // Probe real system capacity at construction so callers see accurate values
        // from the first call to get_available_capacity().
        Ok(Self {
            available_capacity: Arc::new(RwLock::new(CapacityInfo::from_system())),
        })
    }

    /// Return a snapshot of this node's available CPU, memory, and storage.
    pub async fn get_available_capacity(&self) -> ToadStoolResult<CapacityInfo> {
        Ok(self.available_capacity.read().await.clone())
    }

    /// Accept the job if this node has enough CPU, memory, and storage capacity.
    pub async fn can_accept_job(
        &self,
        requirements: &crate::ResourceRequirements,
    ) -> ToadStoolResult<bool> {
        let cap = self.available_capacity.read().await;
        Ok(requirements.cpu.min_cores <= cap.cpu_cores
            && requirements.memory.min_bytes <= cap.memory_bytes
            && requirements.storage.min_bytes <= cap.storage_bytes)
    }

    /// Reserve capacity for a job. Records a tentative deduction so that
    /// back-to-back `can_accept_job` calls don't double-count.
    pub async fn reserve_resources(
        &self,
        requirements: &crate::ResourceRequirements,
    ) -> ToadStoolResult<ResourceReservation> {
        {
            let mut cap = self.available_capacity.write().await;
            cap.cpu_cores = (cap.cpu_cores - requirements.cpu.min_cores).max(0.0);
            cap.memory_bytes = cap
                .memory_bytes
                .saturating_sub(requirements.memory.min_bytes);
            cap.storage_bytes = cap
                .storage_bytes
                .saturating_sub(requirements.storage.min_bytes);
        }
        Ok(ResourceReservation {
            reservation_id: uuid::Uuid::new_v4(),
            resources: requirements.clone(),
        })
    }

    /// Return reserved capacity to the available pool.
    pub async fn release_reservation(
        &self,
        reservation: ResourceReservation,
    ) -> ToadStoolResult<()> {
        {
            let mut cap = self.available_capacity.write().await;
            cap.cpu_cores += reservation.resources.cpu.min_cores;
            cap.memory_bytes += reservation.resources.memory.min_bytes;
            cap.storage_bytes += reservation.resources.storage.min_bytes;
            // Clamp to real system capacity so leaked reservations don't inflate values.
            let system = CapacityInfo::from_system();
            cap.cpu_cores = cap.cpu_cores.min(system.cpu_cores);
            cap.memory_bytes = cap.memory_bytes.min(system.memory_bytes);
            cap.storage_bytes = cap.storage_bytes.min(system.storage_bytes);
        }
        tracing::debug!("Released reservation: {:?}", reservation.reservation_id);
        Ok(())
    }

    /// Report current node capabilities sourced from the real system.
    pub async fn get_current_capabilities(&self) -> ToadStoolResult<NodeCapabilities> {
        let cap = self.available_capacity.read().await;
        let gb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        Ok(NodeCapabilities {
            cpu_cores: cap.cpu_cores,
            memory_gb: gb(cap.memory_bytes),
            storage_gb: gb(cap.storage_bytes),
            gpu_count: 0, // GPU detection handled by toadstool-runtime-gpu
            specialized_hardware: vec![],
            software_capabilities: vec!["rust".to_owned()],
        })
    }
}
