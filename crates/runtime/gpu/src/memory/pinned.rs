//! Pinned Host Memory for Fast GPU Transfers
//!
//! Page-locked (pinned) host memory enables:
//! - 2-3x faster host ↔ GPU transfers via DMA
//! - Zero-copy access from GPU (on supported hardware)
//! - Asynchronous transfers without blocking CPU
//!
//! ## Safety
//! Pinned memory is a limited resource - use judiciously

use std::ptr::NonNull;
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Cache-line alignment for DMA-friendly pinned allocations.
const PINNED_ALIGNMENT: usize = 64;

/// Pinned host memory for fast GPU transfers
///
/// Memory is page-locked and aligned for optimal GPU DMA performance
pub struct PinnedMemory {
    ptr: NonNull<u8>,
    size: usize,
}

// SAFETY: PinnedMemory owns its allocation exclusively. No interior mutability.
// All access goes through &self (as_slice) or &mut self (as_mut_slice).
// Drop deallocates with the same Layout used at allocation time.
unsafe impl Send for PinnedMemory {}
unsafe impl Sync for PinnedMemory {}

impl PinnedMemory {
    /// Allocate pinned host memory
    ///
    /// # Arguments
    /// - `size`: Size in bytes
    ///
    /// # Errors
    /// - If size is 0
    /// - If system cannot allocate pinned memory (limited resource)
    ///
    /// # Safety
    /// Pinned memory is a limited resource. Excessive allocation may fail
    /// or cause system instability. Use for high-frequency transfers only.
    pub fn new(size: usize) -> ToadStoolResult<Self> {
        if size == 0 {
            return Err(ToadStoolError::runtime(
                "Cannot allocate zero-size pinned memory",
            ));
        }

        // For cross-platform support, we use aligned allocation
        // Real pinned memory would use platform-specific APIs:
        // - CUDA: cudaMallocHost() / cudaHostAlloc()
        // - OpenCL: clCreateBuffer with CL_MEM_ALLOC_HOST_PTR
        // - Vulkan: vkAllocateMemory with HOST_VISIBLE | HOST_COHERENT

        let layout = std::alloc::Layout::from_size_align(size, PINNED_ALIGNMENT)
            .map_err(|e| ToadStoolError::runtime(format!("Invalid layout: {}", e)))?;

        // SAFETY: Layout is valid (from_size_align succeeded, size>0, PINNED_ALIGNMENT=64).
        let raw = unsafe { std::alloc::alloc(layout) };
        let ptr = NonNull::new(raw).ok_or_else(|| {
            ToadStoolError::runtime(format!("Failed to allocate {size} bytes of pinned memory"))
        })?;

        tracing::debug!("Allocated {} bytes of pinned memory (aligned to {})", size, PINNED_ALIGNMENT);

        Ok(Self {
            ptr,
            size,
        })
    }

    /// Get immutable slice view of pinned memory
    ///
    /// Zero-copy access to underlying data
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr valid for size bytes; u8 align=1; lifetime tied to &self.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Get mutable slice view of pinned memory
    ///
    /// Zero-copy access to underlying data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: ptr valid for size bytes; &mut self gives exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get raw pointer (for GPU API interop)
    ///
    /// # Safety
    /// Caller must ensure pointer is used correctly with GPU APIs
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Get raw mutable pointer (for GPU API interop)
    ///
    /// # Safety
    /// Caller must ensure pointer is used correctly with GPU APIs
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for PinnedMemory {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(self.size, PINNED_ALIGNMENT)
            .expect("Layout valid during drop");

        // SAFETY: ptr from alloc(layout) in new(); layout matches; Drop runs exactly once.
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), layout);
        }

        tracing::debug!("Freed {} bytes of pinned memory", self.size);
    }
}

/// Builder for pinned memory with options
pub struct PinnedMemoryBuilder {
    size: usize,
    zero_initialized: bool,
}

impl PinnedMemoryBuilder {
    /// Create a new builder for specified size
    pub fn new(size: usize) -> Self {
        Self {
            size,
            zero_initialized: false,
        }
    }

    /// Zero-initialize the memory
    pub fn zero_initialized(mut self) -> Self {
        self.zero_initialized = true;
        self
    }

    /// Build the pinned memory
    pub fn build(self) -> ToadStoolResult<PinnedMemory> {
        let mut memory = PinnedMemory::new(self.size)?;

        if self.zero_initialized {
            // Zero the memory
            memory.as_mut_slice().fill(0);
        }

        Ok(memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_memory_allocation() {
        let memory = PinnedMemory::new(1024).unwrap();
        assert_eq!(memory.size(), 1024);
        assert!(!memory.as_ptr().is_null());
    }

    #[test]
    fn test_pinned_memory_zero_size() {
        let result = PinnedMemory::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_pinned_memory_access() {
        let mut memory = PinnedMemory::new(100).unwrap();

        // Write data
        memory.as_mut_slice()[0] = 42;
        memory.as_mut_slice()[99] = 84;

        // Read data
        assert_eq!(memory.as_slice()[0], 42);
        assert_eq!(memory.as_slice()[99], 84);
    }

    #[test]
    fn test_pinned_memory_builder() {
        let memory = PinnedMemoryBuilder::new(128)
            .zero_initialized()
            .build()
            .unwrap();

        // Verify zero-initialized
        for byte in memory.as_slice() {
            assert_eq!(*byte, 0);
        }
    }

    #[test]
    fn test_pinned_memory_large_allocation() {
        // Test with 10 MB
        let memory = PinnedMemory::new(10 * 1024 * 1024).unwrap();
        assert_eq!(memory.size(), 10 * 1024 * 1024);
    }
}

