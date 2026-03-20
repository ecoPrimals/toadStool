// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // Unsafe Send/Sync impls for GPU allocation handles
//! Backend trait for unified memory implementations

use crate::unified_memory::types::*;
use async_trait::async_trait;
use toadstool::error::ToadStoolResult;

/// Backend-specific allocation handle
///
/// This is an opaque type that backends use to track their allocations.
/// Each backend implementation wraps its native allocation type in this enum.
#[derive(Debug)]
pub enum BackendAllocation {
    /// Vulkan memory allocation
    Vulkan(VulkanAllocation),

    /// OpenCL SVM allocation
    OpenCL(OpenClAllocation),

    /// WebGPU buffer allocation
    WebGpu(WebGpuAllocation),

    /// CPU shared memory allocation
    Cpu(CpuAllocation),
}

/// Vulkan allocation details
#[derive(Debug)]
pub struct VulkanAllocation {
    /// Vulkan device memory handle
    pub memory: u64, // vk::DeviceMemory as u64

    /// Size in bytes
    pub size: usize,

    /// Mapped CPU pointer
    pub cpu_ptr: *mut u8,
}

// SAFETY: VulkanAllocation is Send/Sync because: (1) The inner allocation (memory, cpu_ptr)
// is heap-allocated and owned exclusively; (2) Vulkan device/context handles are thread-safe
// per Vulkan spec; (3) No interior mutability; shared access requires external sync.
// Violation: moving while mapped or concurrent unsynchronized access could cause UB.
unsafe impl Send for VulkanAllocation {}
unsafe impl Sync for VulkanAllocation {}

/// OpenCL SVM allocation details
#[derive(Debug)]
pub struct OpenClAllocation {
    /// SVM pointer (unified CPU/GPU address)
    pub ptr: *mut u8,

    /// Size in bytes
    pub size: usize,

    /// OpenCL context handle (for cleanup)
    pub context_handle: u64,
}

// SAFETY: OpenClAllocation is Send/Sync because: (1) The SVM pointer is heap-allocated and
// owned exclusively; not shared without synchronization; (2) OpenCL context handles are
// thread-safe per OpenCL spec; (3) No interior mutability.
// Violation: concurrent unsynchronized access to SVM region could cause data races.
unsafe impl Send for OpenClAllocation {}
unsafe impl Sync for OpenClAllocation {}

/// WebGPU allocation details
pub struct WebGpuAllocation {
    /// Actual wgpu buffer (kept alive)
    pub buffer: Option<wgpu::Buffer>,

    /// Size in bytes
    pub size: usize,

    /// Mapped pointer (when mapped)
    pub mapped_ptr: Option<*mut u8>,
}

// Manual Debug impl since wgpu::Buffer doesn't implement Debug
impl std::fmt::Debug for WebGpuAllocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebGpuAllocation")
            .field("buffer", &self.buffer.as_ref().map(|_| "<wgpu::Buffer>"))
            .field("size", &self.size)
            .field("mapped_ptr", &self.mapped_ptr)
            .finish()
    }
}

// SAFETY: WebGpuAllocation is Send/Sync because: (1) wgpu::Buffer is Send+Sync; mapped_ptr
// is only set when buffer is mapped and used under proper sync (map_async + get_mapped_range);
// (2) Inner allocation is owned exclusively; not shared without synchronization.
// Violation: accessing mapped_ptr after unmap or during concurrent map would cause UB.
unsafe impl Send for WebGpuAllocation {}
unsafe impl Sync for WebGpuAllocation {}

/// CPU allocation details
#[derive(Debug)]
pub struct CpuAllocation {
    /// Allocated pointer
    pub ptr: *mut u8,

    /// Size in bytes
    pub size: usize,
}

impl CpuAllocation {
    /// Return a mutable slice over the allocation.
    ///
    /// The allocation must be constructed by a backend (e.g. `CpuBackend`) which
    /// guarantees `ptr` is valid for `size` bytes and properly aligned.
    ///
    /// No safe alternative: `from_raw_parts_mut` is required to create a slice from
    /// the raw ptr+size returned by the allocator. CpuBackend uses AlignedBuffer
    /// which guarantees valid, aligned, exclusive allocation.
    pub const fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: Invariants: ptr valid for size bytes; properly aligned; exclusive access.
        // Satisfied: ptr from CpuBackend (AlignedBuffer::into_raw); size matches; &mut self
        // guarantees no aliasing. Violation: invalid ptr/size → UB; aliasing → data race.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

// SAFETY: CpuAllocation is Send/Sync because: (1) ptr points to heap-allocated memory
// owned exclusively by this allocation; (2) No interior mutability; not shared without
// synchronization; (3) Raw pointer is not dereferenced across threads without exclusive
// access. Violation: use-after-free if allocation freed while in use; data races if
// accessed concurrently without sync.
unsafe impl Send for CpuAllocation {}
unsafe impl Sync for CpuAllocation {}

/// Unified memory backend trait
///
/// Implementations provide vendor-specific unified memory allocation
/// and management via open standards (Vulkan, OpenCL, WebGPU).
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait UnifiedMemoryBackend: Send + Sync {
    /// Backend name (e.g., "Vulkan", "OpenCL", "WebGPU", "CPU")
    fn name(&self) -> &'static str;

    /// Backend type
    fn backend_type(&self) -> BackendType;

    /// Query backend capabilities
    fn capabilities(&self) -> &UnifiedMemoryCapabilities;

    /// Allocate unified memory
    ///
    /// Returns an allocation that is accessible from both CPU and GPU.
    ///
    /// # Arguments
    ///
    /// * `size` - Size in bytes (will be aligned to backend requirements)
    /// * `flags` - Memory allocation flags
    ///
    /// # Returns
    ///
    /// Backend-specific allocation handle on success, or error if allocation fails.
    async fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation>;

    /// Free unified memory
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to free
    ///
    /// # Safety
    ///
    /// Caller must ensure no outstanding references to the allocation exist.
    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()>;

    /// Get CPU-accessible pointer
    ///
    /// Returns a pointer that can be used for CPU reads/writes.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to map
    ///
    /// # Safety
    ///
    /// The returned pointer is valid only for the lifetime of the allocation.
    /// Caller must ensure proper synchronization before accessing.
    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8>;

    /// Unmap CPU pointer (if needed)
    ///
    /// Some backends (WebGPU) require explicit unmapping.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to unmap
    async fn unmap_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // Default: no-op (most backends use persistent mapping)
        let _ = allocation;
        Ok(())
    }

    /// Get GPU device pointer
    ///
    /// Returns a pointer/handle that can be used for GPU kernel execution.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation
    ///
    /// # Returns
    ///
    /// Device pointer (opaque, backend-specific)
    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8;

    /// Synchronize CPU → GPU
    ///
    /// Ensures CPU writes are visible to GPU.
    /// No-op for coherent memory.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to sync
    async fn sync_cpu_to_device(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // Default: no-op (assume coherent memory)
        let _ = allocation;
        Ok(())
    }

    /// Synchronize GPU → CPU
    ///
    /// Ensures GPU writes are visible to CPU.
    /// No-op for coherent memory.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to sync
    async fn sync_device_to_cpu(&self, allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // Default: no-op (assume coherent memory)
        let _ = allocation;
        Ok(())
    }

    /// Optimize for specific access pattern
    ///
    /// Hint to backend about expected usage pattern.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to optimize
    /// * `pattern` - Expected access pattern
    async fn optimize_for_pattern(
        &self,
        allocation: &BackendAllocation,
        pattern: AccessPattern,
    ) -> ToadStoolResult<()> {
        // Default: no-op
        let _ = (allocation, pattern);
        Ok(())
    }

    /// Validate allocation is still valid
    ///
    /// Checks if the allocation hasn't been freed.
    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        // Default: assume valid
        let _ = allocation;
        true
    }
}

/// Helper trait for backend initialization
pub trait BackendInitializer: Sized {
    /// Try to initialize the backend
    ///
    /// Returns Ok(backend) if initialization succeeds, Err otherwise.
    async fn try_init() -> ToadStoolResult<Self>;

    /// Check if backend is available on this system
    fn is_available() -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_allocation_sizes() {
        // Ensure allocation types are reasonable size
        use std::mem::size_of;

        // Note: Sizes are approximate and may vary by platform/compiler
        // WebGpuAllocation contains wgpu::Buffer which is larger
        assert!(size_of::<BackendAllocation>() <= 512); // Increased for WebGpu
        assert!(size_of::<VulkanAllocation>() <= 32);
        assert!(size_of::<OpenClAllocation>() <= 32);
        // WebGpuAllocation is larger due to containing wgpu::Buffer (complex type)
        assert!(size_of::<WebGpuAllocation>() <= 256);
        assert!(size_of::<CpuAllocation>() <= 24);

        // Log actual sizes for reference
        println!(
            "BackendAllocation: {} bytes",
            size_of::<BackendAllocation>()
        );
        println!("WebGpuAllocation: {} bytes", size_of::<WebGpuAllocation>());
    }

    #[test]
    fn test_allocation_send_sync() {
        // Compile-time check that allocations are Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<VulkanAllocation>();
        assert_send_sync::<OpenClAllocation>();
        assert_send_sync::<WebGpuAllocation>();
        assert_send_sync::<CpuAllocation>();
        assert_send_sync::<BackendAllocation>();
    }
}
