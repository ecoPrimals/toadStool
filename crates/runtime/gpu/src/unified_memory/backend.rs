// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)]
// `unsafe impl Send/Sync` for `GpuPtr` — raw pointers are not `Send`/`Sync` by default (see rust-lang/rust#48214)
//! Backend trait for unified memory implementations

use crate::unified_memory::types::{
    AccessPattern, BackendType, MemoryFlags, UnifiedMemoryCapabilities,
};
use std::future::Future;
use std::pin::Pin;
use toadstool::error::ToadStoolResult;

/// Opaque CPU/GPU address token for unified-memory allocations.
///
/// `repr(transparent)` wrapper around `*mut u8`. Raw pointers do not implement [`Send`] or
/// [`Sync`] in Rust’s type system (see rust-lang/rust#48214), so this crate supplies
/// explicit `unsafe impl`s with documented invariants.
///
/// # Semantic invariant (API contract)
///
/// Each [`GpuPtr`] must correspond to **one** backend-owned allocation (no aliasing between
/// unrelated handles). Concurrent **access** to the memory (CPU/GPU coherence, queues) is
/// enforced by [`UnifiedMemoryBackend`] and buffer synchronization—not by [`GpuPtr`] alone.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct GpuPtr(*mut u8);

// SAFETY (`Send`):
// - `GpuPtr` is a `repr(transparent)` newtype around `*mut u8` used only as an opaque
//   device/CPU address token for backend-owned unified memory (see type docs).
// - `Send` is sound if copying/moving the handle to another thread does not violate the GPU
//   API’s rules for that allocation. The value itself is a plain address; it does not carry
//   `&mut` or Rust borrow semantics.
// - Callers/constructors ([`GpuPtr::from_raw`], backend mapping routines) must only store
//   addresses that the active backend considers valid to pass across threads for that allocation
//   (per Vulkan/wgpu/OpenCL threading rules). The pointer must not denote Rust stack memory or TLS.
// - Aliasing: multiple `GpuPtr` values may refer to the same allocation only if the backend
//   contract allows it; coherence and queue synchronization are enforced by
//   [`UnifiedMemoryBackend`], not by this type.
unsafe impl Send for GpuPtr {}

// SAFETY (`Sync`):
// - `Sync` allows `&GpuPtr` to be shared across threads (e.g. in `&BackendAllocation` behind
//   trait objects). Sharing is read-only at the Rust type level: only the bit pattern of the
//   address is observed unless code dereferences the raw pointer elsewhere.
// - Same constructor/backend guarantees as `Send`: the address must remain valid and
//   legal to reference from any thread that may read `&GpuPtr`, per the graphics/compute API.
// - Concurrent use of the *memory* still requires backend synchronization; `GpuPtr` does not
//   implement any locking.
unsafe impl Sync for GpuPtr {}

impl GpuPtr {
    /// A null GPU pointer.
    #[must_use]
    pub const fn null() -> Self {
        Self(std::ptr::null_mut())
    }

    /// Create from a raw pointer.
    #[must_use]
    pub const fn from_raw(ptr: *mut u8) -> Self {
        Self(ptr)
    }

    /// Extract the raw pointer.
    #[must_use]
    pub const fn as_ptr(self) -> *mut u8 {
        self.0
    }

    /// Whether the pointer is null.
    #[must_use]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

/// Backend-specific allocation handle
///
/// This is an opaque type that backends use to track their allocations.
/// Each backend implementation wraps its native allocation type in this enum.
#[derive(Debug)]
pub enum BackendAllocation {
    /// Vulkan memory allocation
    Vulkan(VulkanAllocation),

    /// `WebGPU` buffer allocation
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
    pub cpu_ptr: GpuPtr,
}

/// `WebGPU` allocation details
pub struct WebGpuAllocation {
    /// Actual wgpu buffer (kept alive)
    pub buffer: Option<wgpu::Buffer>,

    /// Size in bytes
    pub size: usize,

    /// Mapped pointer (when mapped)
    pub mapped_ptr: Option<GpuPtr>,
}

impl std::fmt::Debug for WebGpuAllocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebGpuAllocation")
            .field("buffer", &self.buffer.as_ref().map(|_| "<wgpu::Buffer>"))
            .field("size", &self.size)
            .field("mapped_ptr", &self.mapped_ptr)
            .finish()
    }
}

/// CPU allocation details.
///
/// Backed by [`toadstool_hw_safe::AlignedAlloc`] for RAII-managed aligned
/// allocation. No unsafe needed — slice access delegates to `AlignedAlloc`.
#[derive(Debug)]
pub struct CpuAllocation {
    /// RAII aligned allocation (zero-initialized, cache-line aligned).
    pub alloc: toadstool_hw_safe::AlignedAlloc,
}

impl CpuAllocation {
    /// Return a mutable slice over the allocation.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.alloc.as_mut_slice()
    }

    /// Return an immutable slice over the allocation.
    pub fn as_slice(&self) -> &[u8] {
        self.alloc.as_slice()
    }

    /// Raw pointer for GPU interop.
    pub fn ptr(&self) -> *mut u8 {
        self.alloc.as_ptr().as_ptr()
    }

    /// Allocation size in bytes.
    pub fn size(&self) -> usize {
        self.alloc.size()
    }
}

/// Unified memory backend trait
///
/// Implementations provide vendor-specific unified memory allocation
/// and management via open standards (Vulkan, `OpenCL`, `WebGPU`).
pub trait UnifiedMemoryBackend: Send + Sync {
    /// Backend name (e.g., "Vulkan", "`OpenCL`", "`WebGPU`", "CPU")
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
    fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<BackendAllocation>> + Send + '_>>;

    /// Free unified memory
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to free
    ///
    /// # Safety
    ///
    /// Caller must ensure no outstanding references to the allocation exist.
    fn free_unified(
        &self,
        allocation: BackendAllocation,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

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
    fn map_cpu_ptr<'a>(
        &'a self,
        allocation: &'a BackendAllocation,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<*mut u8>> + Send + 'a>>;

    /// Unmap CPU pointer (if needed)
    ///
    /// Some backends (`WebGPU`) require explicit unmapping.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to unmap
    fn unmap_cpu_ptr<'a>(
        &'a self,
        allocation: &'a BackendAllocation,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Default: no-op (most backends use persistent mapping)
            let _ = allocation;
            Ok(())
        })
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
    fn sync_cpu_to_device<'a>(
        &'a self,
        allocation: &'a BackendAllocation,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Default: no-op (assume coherent memory)
            let _ = allocation;
            Ok(())
        })
    }

    /// Synchronize GPU → CPU
    ///
    /// Ensures GPU writes are visible to CPU.
    /// No-op for coherent memory.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to sync
    fn sync_device_to_cpu<'a>(
        &'a self,
        allocation: &'a BackendAllocation,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Default: no-op (assume coherent memory)
            let _ = allocation;
            Ok(())
        })
    }

    /// Optimize for specific access pattern
    ///
    /// Hint to backend about expected usage pattern.
    ///
    /// # Arguments
    ///
    /// * `allocation` - The allocation to optimize
    /// * `pattern` - Expected access pattern
    fn optimize_for_pattern<'a>(
        &'a self,
        allocation: &'a BackendAllocation,
        pattern: AccessPattern,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move {
            // Default: no-op
            let _ = (allocation, pattern);
            Ok(())
        })
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
        assert_send_sync::<WebGpuAllocation>();
        assert_send_sync::<CpuAllocation>();
        assert_send_sync::<BackendAllocation>();
    }
}
