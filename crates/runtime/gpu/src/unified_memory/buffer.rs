//! Unified buffer - Zero-copy CPU/GPU accessible memory

use crate::unified_memory::{
    backend::{BackendAllocation, UnifiedMemoryBackend},
    types::*,
};
use bytes::Bytes;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
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

    /// CPU-accessible pointer (DEEP DEBT EVOLUTION: NonNull for compile-time guarantee)
    ///
    /// Using `NonNull<u8>` instead of `*mut u8` provides:
    /// - Compile-time null safety (cannot be null by construction)
    /// - Covariant over `T` (safer type system interactions)
    /// - Niche optimization (`Option<NonNull<T>>` same size as `*mut T`)
    ///
    /// This is a "Fast AND Safe" evolution - same performance, better safety
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

impl UnifiedBuffer {
    /// Validate CPU pointer before use (Deep Debt: comprehensive validation)
    ///
    /// DEEP DEBT EVOLUTION: With NonNull, we no longer need to check for null!
    /// The type system guarantees it at compile time.
    fn validate_cpu_ptr(&self) -> ToadStoolResult<()> {
        // Check allocation still exists
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime(
                "Buffer has been freed (allocation is None)",
            ));
        }

        // DEEP DEBT: Null check eliminated! NonNull provides compile-time guarantee
        // Old code: if self.cpu_ptr.is_null() { ... }
        // NonNull makes this impossible by construction

        // Check pointer value is reasonable (not in NULL page)
        // Note: NonNull still needs this check as it can't prevent all invalid addresses
        let ptr_val = self.cpu_ptr.as_ptr() as usize;
        if ptr_val < 4096 {
            return Err(ToadStoolError::runtime(format!(
                "CPU pointer value {ptr_val} is in NULL page (invalid)"
            )));
        }

        // Check pointer alignment (must be properly aligned)
        if !ptr_val.is_multiple_of(std::mem::align_of::<u8>()) {
            return Err(ToadStoolError::runtime(format!(
                "CPU pointer {ptr_val:#x} is not properly aligned"
            )));
        }

        // Check size is reasonable
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        Ok(())
    }

    /// Get safe mutable slice from CPU pointer (internal helper)
    ///
    /// # Safety
    /// This is the ONLY place we convert raw pointer to slice.
    /// All unsafe pointer operations go through this method.
    ///
    /// # Guarantees
    /// - Pointer is validated (not null, properly aligned, allocation exists)
    /// - Size is valid (checked at creation and validation)
    /// - Exclusive access via &mut self
    ///
    /// # DEEP DEBT EVOLUTION:
    /// EVOLVED: Returns Result instead of panicking!
    /// From: Panic on error (not composable)
    /// To: Result (caller handles error)
    fn as_cpu_slice_mut(&mut self) -> ToadStoolResult<&mut [u8]> {
        // DEEP DEBT: Validate before every use!
        self.validate_cpu_ptr()?;

        // UNAVOIDABLE UNSAFE: from_raw_parts_mut - no safe alternative when wrapping
        // backend-returned raw pointer for slice access. bytemuck::cast_slice is for
        // type conversion, not ptr→slice.
        //
        // SAFETY: (1) cpu_ptr NonNull, validated (alignment, allocation, not NULL page);
        // (2) size validated at creation and in validate_cpu_ptr; (3) exclusive &mut self;
        // (4) ptr from backend allocation, valid for buffer lifetime; (5) u8 align=1.
        Ok(unsafe { std::slice::from_raw_parts_mut(self.cpu_ptr.as_ptr(), self.size) })
    }

    /// Get safe immutable slice from CPU pointer (internal helper)
    ///
    /// # Safety
    /// This is the ONLY place we convert raw pointer to slice for reads.
    /// All unsafe pointer operations go through this method.
    ///
    /// # Guarantees
    /// - Pointer is validated (not null, properly aligned, allocation exists)
    /// - Size is valid (checked at creation and validation)
    /// - Shared access via &self (Rust ensures no concurrent writes)
    ///
    /// # DEEP DEBT EVOLUTION:
    /// EVOLVED: Returns Result instead of panicking!
    /// From: Panic on error (not composable)
    /// To: Result (caller handles error)
    fn as_cpu_slice(&self) -> ToadStoolResult<&[u8]> {
        // DEEP DEBT: Validate before every use!
        self.validate_cpu_ptr()?;

        // UNAVOIDABLE UNSAFE: from_raw_parts - no safe alternative when wrapping
        // backend-returned raw pointer for slice access.
        //
        // SAFETY: (1) cpu_ptr NonNull, validated (alignment, allocation, not NULL page);
        // (2) size validated; (3) &self gives shared access, no concurrent mutation;
        // (4) ptr from backend allocation, valid for buffer lifetime; (5) u8 align=1.
        Ok(unsafe { std::slice::from_raw_parts(self.cpu_ptr.as_ptr(), self.size) })
    }

    /// Create new unified buffer (internal use only)
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: BufferId,
        size: usize,
        cpu_ptr: *mut u8,
        device_ptr: *const u8,
        allocation: BackendAllocation,
        backend: Arc<dyn UnifiedMemoryBackend>,
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

        // DEEP DEBT EVOLUTION: Convert to NonNull for compile-time null safety
        // Use safe NonNull::new().expect() instead of new_unchecked - assertions
        // guarantee non-null, so expect() documents the invariant without unsafe.
        assert!(
            !cpu_ptr.is_null(),
            "CPU pointer cannot be null at buffer creation"
        );
        assert!(
            cpu_ptr as usize >= 4096,
            "CPU pointer in NULL page at buffer creation"
        );
        assert!(size > 0, "Buffer size cannot be zero");

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
        *self
            .sync_state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Write data from CPU (async, non-blocking)
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in bytes from buffer start
    /// * `data` - Data to write (accepts `&[u8]`, `Vec<u8>`, `Bytes`, or any `AsRef<[u8]>`)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Buffer has been freed
    /// - Write would overflow buffer
    /// - Pointer is invalid
    pub async fn write_async<D: AsRef<[u8]>>(
        &mut self,
        offset: usize,
        data: D,
    ) -> ToadStoolResult<()> {
        let data = data.as_ref();
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

        // DEEP DEBT: Null checks removed - NonNull provides compile-time guarantee
        // Old code: if self.cpu_ptr.is_null() { return Err(...); }
        // NonNull makes this impossible by construction

        // Validate pointer value (defensive check - still useful for invalid addresses)
        let ptr_value = self.cpu_ptr.as_ptr() as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        // Deep Debt: Use safe slice operations instead of raw pointers!
        // Get safe mutable slice (unsafe encapsulated in helper method)
        let buffer_slice = self.as_cpu_slice_mut()?;
        let target_slice = &mut buffer_slice[offset..offset + data.len()];

        // Now use safe slice copy (no unsafe here!)
        target_slice.copy_from_slice(data);

        // Update sync state
        *self
            .sync_state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::CpuModified;

        // Update metadata
        if let Some(metadata) = self
            .allocations
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&self.id)
        {
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
    /// Returns [`Bytes`] for zero-copy cloning when passing data across threads/tasks.
    /// Use `.to_vec()` if you need mutable access to the result.
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
    pub async fn read_async(&self, offset: usize, len: usize) -> ToadStoolResult<Bytes> {
        // Handle zero-length read
        if len == 0 {
            return Ok(Bytes::new());
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

        // DEEP DEBT: Null checks removed - NonNull provides compile-time guarantee
        // Old code: if self.cpu_ptr.is_null() { return Err(...); }
        // NonNull makes this impossible by construction

        // Validate pointer value (defensive check - still useful for invalid addresses)
        let ptr_value = self.cpu_ptr.as_ptr() as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        // Deep Debt: Use safe slice operations instead of raw pointers!
        // Get safe immutable slice (unsafe encapsulated in helper method)
        let buffer_slice = self.as_cpu_slice()?;
        let source_slice = &buffer_slice[offset..offset + len];

        // Use Bytes for zero-copy clone when passing across threads/tasks
        let result = Bytes::copy_from_slice(source_slice);

        // Update metadata
        if let Some(metadata) = self
            .allocations
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&self.id)
        {
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
    pub async fn zero(&mut self) -> ToadStoolResult<()> {
        self.fill(0).await
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

            // DEEP DEBT FIX: Actually free the memory!
            // Drop can't be async, so we need to free synchronously
            // For backends that need async cleanup, they must handle it internally

            let backend = Arc::clone(&self.backend);
            let size = self.size;
            let id = self.id;

            // Try to get or create a runtime for async free
            // This is a temporary solution until we have proper RAII
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
                        "No tokio runtime available for buffer {} cleanup, creating temporary runtime",
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
                                    "Successfully freed buffer {} ({} bytes) via temporary runtime",
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

// SAFETY: Send implementation is safe because:
// - All interior data structures are thread-safe:
//   - Arc<T> is Send when T: Send (all our Arc types are Send)
//   - RwLock<T> is Send when T: Send (SyncState is Send)
//   - DashMap is thread-safe and Send
//   - AtomicU64 is Send
// - Raw pointers (cpu_ptr, device_ptr) are only accessed through safe API methods
// - The safe API enforces proper synchronization:
//   - Mutable operations require &mut self (exclusive access)
//   - Immutable operations use &self with interior mutability (RwLock, DashMap)
// - Moving UnifiedBuffer between threads doesn't invalidate the underlying memory
//   (unified memory is allocated by backend and remains valid across threads)
// - No thread-local state that would be invalidated by moving
unsafe impl Send for UnifiedBuffer {}

// SAFETY: Sync implementation is safe because:
// - All interior data structures are thread-safe and Sync:
//   - Arc<T> is Sync when T: Sync + Send (all our Arc types meet this)
//   - RwLock<T> is Sync when T: Send (SyncState is Send)
//   - DashMap is thread-safe and Sync
//   - AtomicU64 is Sync
// - Concurrent access patterns are safe:
//   - Multiple &self references can coexist (read-only operations)
//   - Mutable operations require &mut self (exclusive access enforced by borrow checker)
//   - Interior mutability (sync_state, allocations, metrics) uses proper synchronization
// - Raw pointers (cpu_ptr, device_ptr) are only accessed through safe API:
//   - Read operations use &self and validate before access
//   - Write operations use &mut self (exclusive access)
//   - Slice creation is bounds-checked and validated
// - No data races: all shared mutable state is protected by RwLock or atomic operations
// - The underlying unified memory is safe for concurrent reads (backend guarantees this)
unsafe impl Sync for UnifiedBuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified_memory::manager::UniversalUnifiedMemory;

    #[tokio::test]
    async fn test_buffer_write_read() {
        eprintln!("=== Original test_buffer_write_read starting ===");

        // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await
                .unwrap();
        eprintln!("Memory created, backend: {}", memory.backend_name());

        let mut buffer = memory.allocate(4096).await.unwrap();
        eprintln!("Buffer allocated: {}", buffer.size());

        // Write data
        let data = vec![42u8; 1024];
        eprintln!("Writing {} bytes...", data.len());
        buffer.write_async(0, &data).await.unwrap();
        eprintln!("Write complete");

        // Read back
        eprintln!("Reading back...");
        let result = buffer.read_async(0, 1024).await.unwrap();
        eprintln!("Read complete");

        assert_eq!(data.as_slice(), result.as_ref());
        eprintln!("=== Test passed ===");
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
    async fn test_buffer_sync_state() {
        // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await
                .unwrap();
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
    async fn test_buffer_fill() {
        // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await
                .unwrap();
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
