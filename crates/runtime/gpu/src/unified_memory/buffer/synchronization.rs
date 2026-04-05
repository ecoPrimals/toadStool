// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU/GPU synchronization and bulk fill helpers for [`super::UnifiedBuffer`].

use super::UnifiedBuffer;
use crate::unified_memory::types::{SyncState, SyncTarget};
use toadstool::error::ToadStoolResult;

impl UnifiedBuffer {
    /// Synchronize CPU → GPU
    ///
    /// Ensures CPU writes are visible to GPU.
    /// No-op if buffer is already synced or if using coherent memory.
    ///
    /// # Errors
    ///
    /// Returns when the backend synchronization operation fails.
    pub async fn sync_to_device(&self) -> ToadStoolResult<()> {
        let state = *self
            .sync_state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match state {
            SyncState::Synced | SyncState::GpuModified => Ok(()),
            SyncState::CpuModified => {
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_cpu_to_device(allocation).await?;
                    *self
                        .sync_state
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::Synced;
                    let mut metrics = self
                        .metrics
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    metrics.cpu_to_gpu_syncs += 1;
                    metrics.bytes_synced += self.size as u64;
                    drop(metrics);
                    tracing::trace!("Synced buffer {} to device", self.id);
                }
                Ok(())
            }
            SyncState::Conflict => {
                tracing::warn!("Sync conflict for buffer {}, CPU wins", self.id);
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_cpu_to_device(allocation).await?;
                    *self
                        .sync_state
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::Synced;
                }
                Ok(())
            }
        }
    }

    /// Synchronize GPU → CPU
    ///
    /// Ensures GPU writes are visible to CPU.
    /// No-op if buffer is already synced or if using coherent memory.
    ///
    /// # Errors
    ///
    /// Returns when the backend synchronization operation fails.
    pub async fn sync_to_cpu(&self) -> ToadStoolResult<()> {
        let state = *self
            .sync_state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        match state {
            SyncState::Synced | SyncState::CpuModified => Ok(()),
            SyncState::GpuModified => {
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_device_to_cpu(allocation).await?;
                    *self
                        .sync_state
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::Synced;
                    let mut metrics = self
                        .metrics
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    metrics.gpu_to_cpu_syncs += 1;
                    metrics.bytes_synced += self.size as u64;
                    drop(metrics);
                    tracing::trace!("Synced buffer {} to CPU", self.id);
                }
                Ok(())
            }
            SyncState::Conflict => {
                tracing::warn!("Sync conflict for buffer {}, GPU wins", self.id);
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_device_to_cpu(allocation).await?;
                    *self
                        .sync_state
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::Synced;
                }
                Ok(())
            }
        }
    }

    /// Auto-sync to target (only if needed).
    ///
    /// # Errors
    ///
    /// Returns when the chosen sync path fails.
    pub async fn auto_sync(&self, target: SyncTarget) -> ToadStoolResult<()> {
        match target {
            SyncTarget::Cpu => self.sync_to_cpu().await,
            SyncTarget::Device => self.sync_to_device().await,
        }
    }

    /// Mark GPU as modified (call after GPU kernel execution).
    pub fn mark_gpu_modified(&self) {
        *self
            .sync_state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::GpuModified;
    }

    /// Fill buffer with value.
    ///
    /// # Errors
    ///
    /// Returns when the CPU slice cannot be obtained or validated.
    pub async fn fill(&mut self, value: u8) -> ToadStoolResult<()> {
        let buffer_slice = self.as_cpu_slice_mut()?;
        buffer_slice.fill(value);
        *self
            .sync_state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::CpuModified;

        Ok(())
    }

    /// Zero buffer contents
    ///
    /// # Errors
    ///
    /// Returns when `fill` fails.
    pub async fn zero(&mut self) -> ToadStoolResult<()> {
        self.fill(0).await
    }
}
