// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // Raw pointer arithmetic for zero-copy GPU buffer access
//! Unified buffer — zero-copy CPU/GPU accessible memory.
//!
//! Submodules group pointer validation, I/O, synchronization, lifecycle, and
//! thread-safety markers for [`UnifiedBuffer`].

mod access;
mod lifecycle;
mod read_write;
mod synchronization;
mod threading;

#[cfg(test)]
mod tests;

use crate::unified_memory::{
    backend::{BackendAllocation, UnifiedMemoryBackend},
    types::*,
};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{Arc, RwLock, atomic::AtomicU64};

/// Zero-copy buffer accessible from both CPU and GPU
///
/// This is the main type users interact with. It provides safe access
/// to unified memory via async methods.
///
/// # Safety
///
/// All unsafe pointer operations are encapsulated within this type.
/// Public API is completely safe.
///
/// # Example
///
/// ```no_run
/// # use toadstool_runtime_gpu::unified_memory::*;
/// # async fn example(buffer: &mut UnifiedBuffer) -> toadstool::error::ToadStoolResult<()> {
/// // Write from CPU
/// let data = vec![42u8; 1024];
/// buffer.write_async(0, &data).await?;
///
/// // Sync to GPU
/// buffer.sync_to_device().await?;
///
/// // Get device pointer for kernel
/// let device_ptr = buffer.device_ptr();
///
/// // Sync back from GPU
/// buffer.sync_to_cpu().await?;
///
/// // Read from CPU
/// let result = buffer.read_async(0, 1024).await?;
/// # Ok(())
/// # }
/// ```
pub struct UnifiedBuffer {
    /// Buffer ID
    id: BufferId,

    /// Size in bytes
    size: usize,

    /// CPU-accessible pointer.
    ///
    /// `NonNull<u8>` instead of `*mut u8` provides compile-time null safety,
    /// covariance, and niche optimization (`Option<NonNull<T>>` is pointer-sized).
    cpu_ptr: NonNull<u8>,

    /// GPU device pointer
    device_ptr: *const u8,

    /// Backend-specific allocation
    allocation: Option<BackendAllocation>,

    /// Backend reference
    backend: Arc<dyn UnifiedMemoryBackend>,

    /// Synchronization state
    sync_state: Arc<RwLock<SyncState>>,

    /// Allocations tracker (shared with manager)
    allocations: Arc<RwLock<HashMap<BufferId, UnifiedBufferMetadata>>>,

    /// Total allocated counter (shared with manager)
    total_allocated: Arc<AtomicU64>,

    /// Metrics (shared with manager)
    metrics: Arc<RwLock<UnifiedMemoryStats>>,
}
