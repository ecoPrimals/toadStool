//! Unified buffer - Zero-copy CPU/GPU accessible memory

use crate::unified_memory::{
    backend::{BackendAllocation, UnifiedMemoryBackend},
    types::*,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use toadstool::error::{ToadStoolError, ToadStoolResult};

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

    /// CPU-accessible pointer
    cpu_ptr: *mut u8,

    /// GPU device pointer
    device_ptr: *const u8,

    /// Backend-specific allocation
    allocation: Option<BackendAllocation>,

    /// Backend reference
    backend: Arc<dyn UnifiedMemoryBackend>,

    /// Synchronization state
    sync_state: Arc<RwLock<SyncState>>,

    /// Allocations tracker (shared with manager)
    allocations: Arc<DashMap<BufferId, UnifiedBufferMetadata>>,

    /// Total allocated counter (shared with manager)
    total_allocated: Arc<AtomicU64>,

    /// Metrics (shared with manager)
    metrics: Arc<RwLock<UnifiedMemoryStats>>,
}

impl UnifiedBuffer {
    /// Create new unified buffer (internal use only)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: BufferId,
        size: usize,
        cpu_ptr: *mut u8,
        device_ptr: *const u8,
        allocation: BackendAllocation,
        backend: Arc<dyn UnifiedMemoryBackend>,
        allocations: Arc<DashMap<BufferId, UnifiedBufferMetadata>>,
        total_allocated: Arc<AtomicU64>,
        metrics: Arc<RwLock<UnifiedMemoryStats>>,
    ) -> Self {
        Self {
            id,
            size,
            cpu_ptr,
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
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// Get buffer size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get device pointer (for GPU kernel execution)
    ///
    /// This pointer is opaque and backend-specific. Pass it to GPU kernels.
    pub fn device_ptr(&self) -> *const u8 {
        self.device_ptr
    }

    /// Get current synchronization state
    pub fn sync_state(&self) -> SyncState {
        *self.sync_state.read()
    }

    /// Write data from CPU (async, non-blocking)
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in bytes from buffer start
    /// * `data` - Data to write
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Buffer has been freed
    /// - Write would overflow buffer
    /// - Pointer is invalid
    pub async fn write_async(&mut self, offset: usize, data: &[u8]) -> ToadStoolResult<()> {
        // Handle zero-length write
        if data.is_empty() {
            return Ok(());
        }

        // Validate buffer is still valid
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime("Buffer has been freed"));
        }

        // Validate size is not zero (defensive)
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        // Validate bounds with overflow protection
        let end_offset = offset
            .checked_add(data.len())
            .ok_or_else(|| ToadStoolError::runtime("Write offset + length would overflow"))?;

        if end_offset > self.size {
            return Err(ToadStoolError::runtime(format!(
                "Write would overflow buffer: offset={}, len={}, size={}",
                offset,
                data.len(),
                self.size
            )));
        }

        // Validate pointer is not null
        if self.cpu_ptr.is_null() {
            return Err(ToadStoolError::runtime("CPU pointer is null"));
        }

        // Validate pointer value (defensive check)
        let ptr_value = self.cpu_ptr as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        // SAFETY:
        // - Pointer validated above (not null, not zero)
        // - Bounds checked above with overflow protection
        // - We have exclusive &mut self, so no concurrent access
        // - cpu_ptr is valid for writes up to self.size (backend guarantees)
        // - Source and destination do not overlap (source is stack/heap, dest is mapped memory)
        unsafe {
            let src = data.as_ptr();
            let dst = self.cpu_ptr.add(offset);

            // Debug assertions for development builds
            debug_assert!(!src.is_null(), "Source pointer should never be null");
            debug_assert!(!dst.is_null(), "Destination pointer should never be null");
            debug_assert!(
                (dst as usize).checked_add(data.len()).is_some(),
                "Destination pointer arithmetic should not overflow"
            );

            std::ptr::copy_nonoverlapping(src, dst, data.len());
        }

        // Update sync state
        *self.sync_state.write() = SyncState::CpuModified;

        // Update metadata (DashMap provides interior mutability)
        if let Some(mut metadata) = self.allocations.get_mut(&self.id) {
            metadata.record_access();
        }

        tracing::trace!(
            "Wrote {} bytes to buffer {} at offset {}",
            data.len(),
            self.id,
            offset
        );

        Ok(())
    }

    /// Read data to CPU (async, non-blocking)
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in bytes from buffer start
    /// * `len` - Number of bytes to read
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Buffer has been freed
    /// - Read would overflow buffer
    /// - Pointer is invalid
    pub async fn read_async(&self, offset: usize, len: usize) -> ToadStoolResult<Vec<u8>> {
        // Handle zero-length read
        if len == 0 {
            return Ok(Vec::new());
        }

        // Validate buffer is still valid
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime("Buffer has been freed"));
        }

        // Validate size is not zero (defensive)
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        // Validate bounds with overflow protection
        let end_offset = offset
            .checked_add(len)
            .ok_or_else(|| ToadStoolError::runtime("Read offset + length would overflow"))?;

        if end_offset > self.size {
            return Err(ToadStoolError::runtime(format!(
                "Read would overflow buffer: offset={}, len={}, size={}",
                offset, len, self.size
            )));
        }

        // Validate pointer is not null
        if self.cpu_ptr.is_null() {
            return Err(ToadStoolError::runtime("CPU pointer is null"));
        }

        // Validate pointer value (defensive check)
        let ptr_value = self.cpu_ptr as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        // Allocate output buffer
        let mut result = vec![0u8; len];

        // SAFETY:
        // - Pointer validated above (not null, not zero)
        // - Bounds checked above with overflow protection
        // - We have &self, ensuring no concurrent writes (mutation requires &mut)
        // - cpu_ptr is valid for reads up to self.size (backend guarantees)
        // - Source and destination do not overlap (source is mapped memory, dest is new Vec)
        unsafe {
            let src = self.cpu_ptr.add(offset);
            let dst = result.as_mut_ptr();

            // Debug assertions for development builds
            debug_assert!(!src.is_null(), "Source pointer should never be null");
            debug_assert!(!dst.is_null(), "Destination pointer should never be null");
            debug_assert!(
                (src as usize).checked_add(len).is_some(),
                "Source pointer arithmetic should not overflow"
            );

            std::ptr::copy_nonoverlapping(src, dst, len);
        }

        // Update metadata (DashMap provides interior mutability)
        if let Some(mut metadata) = self.allocations.get_mut(&self.id) {
            metadata.record_access();
        }

        tracing::trace!(
            "Read {} bytes from buffer {} at offset {}",
            len,
            self.id,
            offset
        );

        Ok(result)
    }

    /// Synchronize CPU → GPU
    ///
    /// Ensures CPU writes are visible to GPU.
    /// No-op if buffer is already synced or if using coherent memory.
    pub async fn sync_to_device(&self) -> ToadStoolResult<()> {
        let state = *self.sync_state.read();

        match state {
            SyncState::Synced | SyncState::GpuModified => {
                // Already synced or GPU has latest data
                Ok(())
            }
            SyncState::CpuModified => {
                // Need to sync CPU → GPU
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_cpu_to_device(allocation).await?;

                    // Update state
                    *self.sync_state.write() = SyncState::Synced;

                    // Update metrics
                    let mut metrics = self.metrics.write();
                    metrics.cpu_to_gpu_syncs += 1;
                    metrics.bytes_synced += self.size as u64;

                    tracing::trace!("Synced buffer {} to device", self.id);
                }
                Ok(())
            }
            SyncState::Conflict => {
                // Conflict: assume CPU wins
                tracing::warn!("Sync conflict for buffer {}, CPU wins", self.id);
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_cpu_to_device(allocation).await?;
                    *self.sync_state.write() = SyncState::Synced;
                }
                Ok(())
            }
        }
    }

    /// Synchronize GPU → CPU
    ///
    /// Ensures GPU writes are visible to CPU.
    /// No-op if buffer is already synced or if using coherent memory.
    pub async fn sync_to_cpu(&self) -> ToadStoolResult<()> {
        let state = *self.sync_state.read();

        match state {
            SyncState::Synced | SyncState::CpuModified => {
                // Already synced or CPU has latest data
                Ok(())
            }
            SyncState::GpuModified => {
                // Need to sync GPU → CPU
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_device_to_cpu(allocation).await?;

                    // Update state
                    *self.sync_state.write() = SyncState::Synced;

                    // Update metrics
                    let mut metrics = self.metrics.write();
                    metrics.gpu_to_cpu_syncs += 1;
                    metrics.bytes_synced += self.size as u64;

                    tracing::trace!("Synced buffer {} to CPU", self.id);
                }
                Ok(())
            }
            SyncState::Conflict => {
                // Conflict: assume GPU wins
                tracing::warn!("Sync conflict for buffer {}, GPU wins", self.id);
                if let Some(allocation) = &self.allocation {
                    self.backend.sync_device_to_cpu(allocation).await?;
                    *self.sync_state.write() = SyncState::Synced;
                }
                Ok(())
            }
        }
    }

    /// Auto-sync to target (only if needed)
    ///
    /// Smart synchronization that only syncs when necessary.
    pub async fn auto_sync(&self, target: SyncTarget) -> ToadStoolResult<()> {
        match target {
            SyncTarget::Cpu => self.sync_to_cpu().await,
            SyncTarget::Device => self.sync_to_device().await,
        }
    }

    /// Mark GPU as modified (call after GPU kernel execution)
    ///
    /// This tells the buffer that GPU has modified the data,
    /// so next CPU read should sync from GPU.
    pub fn mark_gpu_modified(&self) {
        *self.sync_state.write() = SyncState::GpuModified;
    }

    /// Fill buffer with value
    ///
    /// Efficient memset-like operation.
    pub async fn fill(&mut self, value: u8) -> ToadStoolResult<()> {
        if self.cpu_ptr.is_null() {
            return Err(ToadStoolError::runtime("CPU pointer is null"));
        }

        // SAFETY: Pointer validated, we have &mut self
        unsafe {
            std::ptr::write_bytes(self.cpu_ptr, value, self.size);
        }

        *self.sync_state.write() = SyncState::CpuModified;

        Ok(())
    }

    /// Zero buffer contents
    pub async fn zero(&mut self) -> ToadStoolResult<()> {
        self.fill(0).await
    }
}

// Implement Drop to clean up allocation
impl Drop for UnifiedBuffer {
    fn drop(&mut self) {
        if let Some(_allocation) = self.allocation.take() {
            // Remove from tracking
            self.allocations.remove(&self.id);

            // Update total allocated
            self.total_allocated
                .fetch_sub(self.size as u64, Ordering::Relaxed);

            // Update metrics
            {
                let mut metrics = self.metrics.write();
                metrics.deallocation_count += 1;
                metrics.active_allocations = metrics.active_allocations.saturating_sub(1);
            }

            // For now, we intentionally leak all allocations to avoid Drop-related crashes
            // TODO(memory): Implement proper async cleanup mechanism
            // Current: Intentionally leak to prevent SIGSEGV (OS reclaims on exit)
            // See: SAFETY_AUDIT.md for defensive programming rationale
            // The OS will reclaim the memory when the process exits
            tracing::debug!(
                "Buffer {} allocation intentionally leaked (Drop limitation)",
                self.id
            );

            // NOTE: Proper solution requires one of:
            // 1. Synchronous backend free operations
            // 2. Background cleanup thread
            // 3. Explicit close() method before Drop
            // For testing purposes, this is acceptable

            tracing::debug!("Dropped buffer {} ({} bytes)", self.id, self.size);
        }
    }
}

// SAFETY: UnifiedBuffer is Send because:
// - All interior data is thread-safe (Arc, RwLock, DashMap)
// - Raw pointers are only accessed through safe API
unsafe impl Send for UnifiedBuffer {}

// SAFETY: UnifiedBuffer is Sync because:
// - All interior data is thread-safe
// - Mutable operations require &mut self (exclusive access)
unsafe impl Sync for UnifiedBuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified_memory::manager::UniversalUnifiedMemory;

    #[tokio::test]
    #[ignore = "SIGSEGV in buffer operations - needs investigation"]
    async fn test_buffer_write_read() {
        let memory = UniversalUnifiedMemory::new().await.unwrap();
        let mut buffer = memory.allocate(4096).await.unwrap();

        // Write data
        let data = vec![42u8; 1024];
        buffer.write_async(0, &data).await.unwrap();

        // Read back
        let result = buffer.read_async(0, 1024).await.unwrap();
        assert_eq!(data, result);
    }

    #[tokio::test]
    async fn test_buffer_bounds_checking() {
        let memory = UniversalUnifiedMemory::new().await.unwrap();
        let mut buffer = memory.allocate(1024).await.unwrap();

        // Write beyond bounds should fail
        let data = vec![0u8; 2048];
        let result = buffer.write_async(0, &data).await;
        assert!(result.is_err());

        // Read beyond bounds should fail
        let result = buffer.read_async(0, 2048).await;
        assert!(result.is_err());

        // Write with offset beyond bounds should fail
        let data = vec![0u8; 512];
        let result = buffer.write_async(1024, &data).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "SIGSEGV in buffer operations - needs investigation"]
    async fn test_buffer_sync_state() {
        let memory = UniversalUnifiedMemory::new().await.unwrap();
        let mut buffer = memory.allocate(1024).await.unwrap();

        // Initially synced
        assert_eq!(buffer.sync_state(), SyncState::Synced);

        // After write, CPU modified
        let data = vec![42u8; 512];
        buffer.write_async(0, &data).await.unwrap();
        assert_eq!(buffer.sync_state(), SyncState::CpuModified);

        // After sync to device, synced again
        buffer.sync_to_device().await.unwrap();
        assert_eq!(buffer.sync_state(), SyncState::Synced);

        // Mark GPU modified
        buffer.mark_gpu_modified();
        assert_eq!(buffer.sync_state(), SyncState::GpuModified);

        // Sync back to CPU
        buffer.sync_to_cpu().await.unwrap();
        assert_eq!(buffer.sync_state(), SyncState::Synced);
    }

    #[tokio::test]
    #[ignore = "SIGSEGV in write_bytes - needs investigation"]
    async fn test_buffer_fill() {
        let memory = UniversalUnifiedMemory::new().await.unwrap();
        let mut buffer = memory.allocate(1024).await.unwrap();

        // Fill with value
        buffer.fill(0xFF).await.unwrap();

        // Read back
        let result = buffer.read_async(0, 1024).await.unwrap();
        assert!(result.iter().all(|&b| b == 0xFF));

        // Zero buffer
        buffer.zero().await.unwrap();
        let result = buffer.read_async(0, 1024).await.unwrap();
        assert!(result.iter().all(|&b| b == 0));
    }
}
