//! CPU fallback backend - Always available

use crate::unified_memory::{
    backend::{BackendAllocation, BackendInitializer, CpuAllocation, UnifiedMemoryBackend},
    types::*,
};
use async_trait::async_trait;
use std::alloc::{alloc, dealloc, Layout};
use toadstool::error::{ToadStoolError, ToadStoolResult};

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

    /// Allocate aligned memory
    fn allocate_aligned(size: usize, align: usize) -> ToadStoolResult<*mut u8> {
        // Validate alignment
        if !align.is_power_of_two() {
            return Err(ToadStoolError::runtime("Alignment must be power of 2"));
        }

        // Create layout
        let layout = Layout::from_size_align(size, align)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create layout: {}", e)))?;

        // Allocate
        // SAFETY: Layout is valid (from_size_align checked size and power-of-2 align above).
        // alloc returns null on OOM (we check); otherwise valid for layout.size() bytes.
        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            return Err(ToadStoolError::runtime("Out of memory"));
        }

        Ok(ptr)
    }

    /// Free aligned memory
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - ptr was allocated with allocate_aligned
    /// - size and align match the original allocation
    /// - ptr is not used after this call
    unsafe fn free_aligned(ptr: *mut u8, size: usize, align: usize) {
        if !ptr.is_null() {
            // EVOLVED: Use Layout; caller guarantees size/align match allocation.
            // SAFETY: size and align from allocate_aligned; always valid there.
            let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
            // SAFETY: ptr from alloc, layout matches; freed exactly once.
            dealloc(ptr, layout);
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
        // Allocate aligned memory
        let ptr = Self::allocate_aligned(size, self.capabilities.alignment_requirement)?;

        tracing::debug!(
            "CPU backend allocated {} bytes at address {:#x} (alignment {})",
            size,
            ptr as usize,
            self.capabilities.alignment_requirement
        );

        // Zero the memory for safety using slice-based approach
        // SAFETY: ptr from allocate_aligned; size matches allocation; exclusive access.
        {
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
            slice.fill(0);
        }

        tracing::debug!("CPU backend zeroed {} bytes at {:#x}", size, ptr as usize);

        Ok(BackendAllocation::Cpu(CpuAllocation { ptr, size }))
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match allocation {
            BackendAllocation::Cpu(alloc) => {
                // SAFETY: alloc from our allocate_unified; ptr/size/align match original allocation.
                // free_aligned requires matching layout (documented in its Safety section).
                unsafe {
                    Self::free_aligned(
                        alloc.ptr,
                        alloc.size,
                        self.capabilities.alignment_requirement,
                    );
                }
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

        // Write some data
        // SAFETY: cpu_ptr from allocate_unified(4096); verified non-null; 4096 bytes valid.
        unsafe {
            std::ptr::write_bytes(cpu_ptr, 42, 4096);
        }

        // Read it back
        // SAFETY: cpu_ptr valid; memory initialized by write_bytes above; single byte read.
        unsafe {
            let first_byte = *cpu_ptr;
            assert_eq!(first_byte, 42);
        }

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
