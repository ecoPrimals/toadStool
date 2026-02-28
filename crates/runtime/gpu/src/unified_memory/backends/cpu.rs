//! CPU fallback backend - Always available
//!
//! # Safety Evolution
//!
//! This module uses safe Rust patterns where possible:
//! - `AlignedBuffer`: RAII wrapper for aligned memory with automatic cleanup
//! - `NonNull`: Null-checked pointer type for better safety guarantees
//! - Encapsulated unsafe: All raw pointer operations in a single, audited location

use crate::unified_memory::{
    backend::{BackendAllocation, BackendInitializer, CpuAllocation, UnifiedMemoryBackend},
    types::*,
};
use async_trait::async_trait;
use std::alloc::{dealloc, Layout};
use std::ptr::NonNull;
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// RAII wrapper for aligned memory allocation (Safe Rust pattern)
///
/// This struct encapsulates unsafe pointer operations and ensures memory is
/// properly deallocated via Drop. The `ManuallyDrop` on the inner buffer
/// prevents double-free when converting to raw pointer for backend use.
///
/// # Safety Invariants
///
/// - `ptr` is always valid and non-null (guaranteed by NonNull)
/// - `size` and `align` exactly match the allocation parameters
/// - Memory is zeroed on allocation
/// - Memory is freed exactly once in Drop (unless taken via `into_raw`)
struct AlignedBuffer {
    ptr: NonNull<u8>,
    size: usize,
    align: usize,
}

impl AlignedBuffer {
    /// Allocate aligned, zeroed memory
    ///
    /// # Errors
    /// Returns error if alignment is not power of 2 or allocation fails (OOM)
    fn new(size: usize, align: usize) -> ToadStoolResult<Self> {
        if !align.is_power_of_two() {
            return Err(ToadStoolError::runtime("Alignment must be power of 2"));
        }

        let layout = Layout::from_size_align(size, align)
            .map_err(|e| ToadStoolError::runtime(format!("Invalid layout: {e}")))?;

        // SAFETY: Layout is valid (from_size_align succeeded, align is power-of-two). alloc_zeroed
        // returns a pointer valid for layout.size() bytes, or null on OOM. Memory is
        // zero-initialized. The pointer is used for dealloc with the same layout in Drop.
        let raw = unsafe { std::alloc::alloc_zeroed(layout) };
        let ptr = NonNull::new(raw).ok_or_else(|| ToadStoolError::runtime("Out of memory"))?;

        Ok(Self { ptr, size, align })
    }

    /// Get the raw pointer (for use in allocations)
    #[allow(dead_code)] // API for future use
    fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Consume the buffer and return the raw pointer
    /// Caller takes ownership and responsibility for deallocation
    fn into_raw(self) -> *mut u8 {
        let ptr = self.ptr.as_ptr();
        // Prevent Drop from running (caller now owns memory)
        std::mem::forget(self);
        ptr
    }

    /// Create from raw pointer (takes ownership)
    ///
    /// # Safety
    /// - `ptr` must be non-null and point to valid memory allocated with the given layout
    /// - `size` and `align` must exactly match the original allocation parameters
    /// - Caller must transfer ownership (must not use ptr after this, must not dealloc)
    /// - If Some is returned, caller must not dealloc; Drop will handle it
    unsafe fn from_raw(ptr: *mut u8, size: usize, align: usize) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self { ptr, size, align })
    }
}

impl Drop for AlignedBuffer {
    #[allow(clippy::expect_used)]
    fn drop(&mut self) {
        // size/align are immutable after construction and were validated
        // at alloc time, so from_size_align cannot fail here.
        // Drop cannot propagate errors, so expect is the correct choice.
        let layout = Layout::from_size_align(self.size, self.align)
            .expect("layout valid: matches original allocation");
        // SAFETY: ptr comes from alloc_zeroed(layout) in new(); layout matches the allocation.
        // Drop runs exactly once; no references exist (self is being dropped).
        unsafe { dealloc(self.ptr.as_ptr(), layout) };
    }
}

// SAFETY: AlignedBuffer owns its allocation exclusively. No interior mutability;
// ptr/size/align are immutable after construction. Drop deallocates with the
// same Layout used at allocation time.
unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

/// CPU shared memory backend
///
/// This backend uses standard heap allocation via Rust's allocator.
/// It provides a fallback when no GPU unified memory is available.
///
/// # Characteristics
///
/// - **Always available**: Works on any system
/// - **Coherent**: No synchronization needed
/// - **Fast CPU access**: Direct memory access
/// - **No GPU acceleration**: CPU-only, no actual GPU access
///
/// # Safety Evolution
///
/// Uses `AlignedBuffer` RAII wrapper to minimize unsafe code and ensure
/// proper memory management with automatic cleanup.
///
/// # Use Cases
///
/// - Development and testing
/// - Systems without GPU
/// - Graceful degradation
pub struct CpuBackend {
    capabilities: UnifiedMemoryCapabilities,
}

impl CpuBackend {
    /// Create new CPU backend
    pub fn new() -> ToadStoolResult<Self> {
        Ok(Self {
            capabilities: UnifiedMemoryCapabilities {
                backend_type: BackendType::Cpu,
                max_allocation_size: 1024 * 1024 * 1024 * 4, // 4GB
                zero_copy: true,                             // Technically true (no copies)
                coherent: true,                              // Always coherent
                cpu_fast_access: true,                       // Direct CPU access
                gpu_fast_access: false,                      // No actual GPU
                alignment_requirement: 64,                   // Cache line alignment
            },
        })
    }

    /// Allocate aligned memory using safe RAII wrapper
    ///
    /// **EVOLVED**: Uses `AlignedBuffer` for safe memory management
    fn allocate_aligned_safe(size: usize, align: usize) -> ToadStoolResult<AlignedBuffer> {
        AlignedBuffer::new(size, align)
    }

    /// Free aligned memory safely
    ///
    /// **EVOLVED**: Uses `AlignedBuffer` RAII for automatic cleanup.
    /// This function reconstructs the buffer and lets Drop handle deallocation.
    fn free_aligned_safe(ptr: *mut u8, size: usize, align: usize) {
        if !ptr.is_null() {
            // SAFETY: ptr, size, and align come from allocate_aligned_safe; caller transfers
            // ownership. from_raw takes ownership; Drop deallocates with matching layout.
            if let Some(buffer) = unsafe { AlignedBuffer::from_raw(ptr, size, align) } {
                drop(buffer); // Explicit drop for clarity (would happen anyway)
            }
        }
    }
}

#[async_trait]
impl BackendInitializer for CpuBackend {
    async fn try_init() -> ToadStoolResult<Self> {
        Self::new()
    }

    fn is_available() -> bool {
        true // Always available
    }
}

#[async_trait]
impl UnifiedMemoryBackend for CpuBackend {
    fn name(&self) -> &'static str {
        "CPU"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Cpu
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        &self.capabilities
    }

    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        // EVOLVED: Use safe RAII wrapper for allocation
        let buffer = Self::allocate_aligned_safe(size, self.capabilities.alignment_requirement)?;
        let ptr = buffer.into_raw(); // Transfer ownership to CpuAllocation

        tracing::debug!(
            "CPU backend allocated {} bytes at address {:#x} (alignment {}, zeroed)",
            size,
            ptr as usize,
            self.capabilities.alignment_requirement
        );

        Ok(BackendAllocation::Cpu(CpuAllocation { ptr, size }))
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match allocation {
            BackendAllocation::Cpu(alloc) => {
                // EVOLVED: Use safe deallocation via RAII wrapper
                Self::free_aligned_safe(
                    alloc.ptr,
                    alloc.size,
                    self.capabilities.alignment_requirement,
                );
                Ok(())
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for CPU backend",
            )),
        }
    }

    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        match allocation {
            BackendAllocation::Cpu(alloc) => Ok(alloc.ptr),
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for CPU backend",
            )),
        }
    }

    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8 {
        match allocation {
            BackendAllocation::Cpu(alloc) => alloc.ptr as *const u8,
            _ => std::ptr::null(),
        }
    }

    // CPU backend doesn't need sync (always coherent)
    async fn sync_cpu_to_device(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        Ok(()) // No-op
    }

    async fn sync_device_to_cpu(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        Ok(()) // No-op
    }

    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        matches!(allocation, BackendAllocation::Cpu(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cpu_backend_initialization() {
        let backend = CpuBackend::try_init().await;
        assert!(backend.is_ok());

        let backend = backend.unwrap();
        assert_eq!(backend.name(), "CPU");
        assert_eq!(backend.backend_type(), BackendType::Cpu);
        assert!(backend.capabilities().coherent);
    }

    #[tokio::test]
    async fn test_cpu_backend_allocation() {
        let backend = CpuBackend::new().unwrap();

        // Allocate memory
        let allocation = backend
            .allocate_unified(4096, MemoryFlags::default())
            .await
            .unwrap();

        // Should be CPU allocation
        assert!(matches!(allocation, BackendAllocation::Cpu(_)));

        // Get pointers
        let cpu_ptr = backend.map_cpu_ptr(&allocation).await.unwrap();
        let device_ptr = backend.get_device_ptr(&allocation);

        assert!(!cpu_ptr.is_null());
        assert!(!device_ptr.is_null());
        assert_eq!(cpu_ptr as *const u8, device_ptr); // Same pointer!

        let BackendAllocation::Cpu(ref alloc) = allocation else {
            panic!("expected CPU allocation");
        };
        let size = alloc.size;
        // Use safe slice creation via NonNull for test - encapsulates the unsafe.
        let ptr = NonNull::new(cpu_ptr).expect("cpu_ptr from allocate_unified is non-null");
        // SAFETY: ptr from allocate_unified, valid for alloc.size bytes. Allocation not freed (we
        // hold it). Exclusive access in test. u8 has alignment 1.
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), size) };
        slice.fill(42);
        assert_eq!(slice[0], 42);

        // Free
        backend.free_unified(allocation).await.unwrap();
    }

    #[tokio::test]
    async fn test_cpu_backend_sync() {
        let backend = CpuBackend::new().unwrap();
        let allocation = backend
            .allocate_unified(1024, MemoryFlags::default())
            .await
            .unwrap();

        // Sync should be no-op (always coherent)
        let result = backend.sync_cpu_to_device(&allocation).await;
        assert!(result.is_ok());

        let result = backend.sync_device_to_cpu(&allocation).await;
        assert!(result.is_ok());

        backend.free_unified(allocation).await.unwrap();
    }

    #[test]
    fn test_cpu_backend_always_available() {
        assert!(CpuBackend::is_available());
    }
}
