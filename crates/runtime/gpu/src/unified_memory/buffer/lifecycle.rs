// SPDX-License-Identifier: AGPL-3.0-or-later
//! Construction, accessors, and teardown for [`super::UnifiedBuffer`].

use super::UnifiedBuffer;
use crate::unified_memory::{
    backend::{BackendAllocation, UnifiedMemoryBackend},
    backend_dispatch::UnifiedMemoryBackendDispatch,
    types::{BufferId, SyncState, UnifiedBufferMetadata, UnifiedMemoryStats},
};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};

impl UnifiedBuffer {
    /// Create new unified buffer (internal use only)
    #[expect(
        clippy::too_many_arguments,
        reason = "parameters map directly to hardware/protocol fields"
    )]
    pub(crate) fn new(
        id: BufferId,
        size: usize,
        cpu_ptr: *mut u8,
        device_ptr: *const u8,
        allocation: BackendAllocation,
        backend: Arc<UnifiedMemoryBackendDispatch>,
        allocations: Arc<RwLock<HashMap<BufferId, UnifiedBufferMetadata>>>,
        total_allocated: Arc<AtomicU64>,
        metrics: Arc<RwLock<UnifiedMemoryStats>>,
    ) -> Self {
        tracing::debug!(
            "Creating UnifiedBuffer {} with size={}, cpu_ptr={:#x}, device_ptr={:#x}",
            id,
            size,
            cpu_ptr as usize,
            device_ptr as usize
        );

        assert!(
            !cpu_ptr.is_null(),
            "CPU pointer cannot be null at buffer creation"
        );
        assert!(
            cpu_ptr as usize >= 4096,
            "CPU pointer in NULL page at buffer creation"
        );
        assert!(size > 0, "Buffer size cannot be zero");

        #[expect(
            clippy::expect_used,
            reason = "infallible: assert above guarantees cpu_ptr >= 4096"
        )]
        let cpu_ptr_nonnull =
            NonNull::new(cpu_ptr).expect("CPU pointer cannot be null at buffer creation");

        Self {
            id,
            size,
            cpu_ptr: cpu_ptr_nonnull,
            device_ptr,
            allocation: Some(allocation),
            backend,
            sync_state: Arc::new(RwLock::new(SyncState::Synced)),
            allocations,
            total_allocated,
            metrics,
        }
    }

    /// Get buffer ID
    pub const fn id(&self) -> BufferId {
        self.id
    }

    /// Get buffer size in bytes
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Get device pointer (for GPU kernel execution)
    ///
    /// This pointer is opaque and backend-specific. Pass it to GPU kernels.
    pub const fn device_ptr(&self) -> *const u8 {
        self.device_ptr
    }

    /// Get current synchronization state
    pub fn sync_state(&self) -> SyncState {
        *self
            .sync_state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

// Implement Drop to clean up allocation
impl Drop for UnifiedBuffer {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            // Remove from tracking
            self.allocations
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&self.id);

            // Update total allocated atomically
            let new_total = self
                .total_allocated
                .fetch_sub(self.size as u64, Ordering::Relaxed)
                .saturating_sub(self.size as u64);

            // Update metrics in a single lock acquisition so stats() is always
            // consistent: active_allocations, total_allocated, and
            // deallocation_count are all updated atomically under the write lock.
            {
                let mut metrics = self
                    .metrics
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                metrics.deallocation_count += 1;
                metrics.active_allocations = metrics.active_allocations.saturating_sub(1);
                metrics.total_allocated = new_total;
            }

            let backend = Arc::clone(&self.backend);
            let size = self.size;
            let id = self.id;

            // Drop cannot await: schedule async `free_unified` on the current runtime when present,
            // otherwise run it on a one-shot runtime in a dedicated thread.
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    // We're in a tokio runtime, spawn a blocking task
                    handle.spawn(async move {
                        if let Err(e) = backend.free_unified(allocation).await {
                            tracing::error!(
                                "Failed to free buffer {} ({} bytes): {}. Memory leaked.",
                                id,
                                size,
                                e
                            );
                        } else {
                            tracing::debug!("Successfully freed buffer {} ({} bytes)", id, size);
                        }
                    });
                }
                Err(_) => {
                    // No runtime available, try to create one for cleanup
                    // This is expensive but better than leaking
                    tracing::warn!(
                        "No tokio runtime available for buffer {} cleanup, creating one-shot runtime",
                        id
                    );

                    std::thread::spawn(move || {
                        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        else {
                            tracing::error!(
                                "Failed to create runtime for buffer cleanup. Memory leaked."
                            );
                            return;
                        };

                        rt.block_on(async {
                            if let Err(e) = backend.free_unified(allocation).await {
                                tracing::error!(
                                    "Failed to free buffer {} ({} bytes): {}. Memory leaked.",
                                    id,
                                    size,
                                    e
                                );
                            } else {
                                tracing::debug!(
                                    "Successfully freed buffer {} ({} bytes) via one-shot runtime",
                                    id,
                                    size
                                );
                            }
                        });
                    });
                }
            }

            tracing::debug!(
                "Dropped buffer {} ({} bytes), cleanup scheduled",
                self.id,
                self.size
            );
        }
    }
}
