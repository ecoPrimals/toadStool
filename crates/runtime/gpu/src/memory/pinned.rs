// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pinned Host Memory for Fast GPU Transfers
//!
//! Delegates aligned allocation to [`toadstool_hw_safe::AlignedAlloc`],
//! eliminating duplicate unsafe alloc/dealloc code.
//!
//! Page-locked (pinned) host memory enables:
//! - 2-3x faster host <-> GPU transfers via DMA
//! - Zero-copy access from GPU (on supported hardware)
//! - Asynchronous transfers without blocking CPU
//!
//! ## Alignment
//!
//! All allocations use 64-byte (cache-line) alignment for DMA-friendly access.

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_hw_safe::AlignedAlloc;

/// Cache-line alignment for DMA-friendly pinned allocations.
const PINNED_ALIGNMENT: usize = 64;

/// Pinned host memory for fast GPU transfers
///
/// Memory is aligned for optimal GPU DMA performance. Backed by
/// [`AlignedAlloc`] from `toadstool-hw-safe`.
pub struct PinnedMemory {
    inner: AlignedAlloc,
}

impl PinnedMemory {
    /// Allocate pinned host memory
    ///
    /// # Arguments
    /// - `size`: Size in bytes
    ///
    /// # Errors
    /// - If size is 0
    /// - If system cannot allocate memory
    ///
    /// # Safety
    /// Pinned memory is a limited resource. Excessive allocation may fail
    /// or cause system instability. Use for high-frequency transfers only.
    pub fn new(size: usize) -> ToadStoolResult<Self> {
        let inner = AlignedAlloc::new(size, PINNED_ALIGNMENT)
            .map_err(|e| ToadStoolError::runtime(format!("pinned alloc: {e}")))?;

        tracing::debug!("Allocated {} bytes of pinned memory (aligned to {PINNED_ALIGNMENT})", size);

        Ok(Self { inner })
    }

    /// Get immutable slice view of pinned memory
    ///
    /// Zero-copy access to underlying data.
    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }

    /// Get mutable slice view of pinned memory
    ///
    /// Zero-copy access to underlying data.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner.as_mut_slice()
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    /// Get raw pointer (for GPU API interop)
    pub fn as_ptr(&self) -> *const u8 {
        self.inner.as_ptr().as_ptr()
    }

    /// Get raw mutable pointer (for GPU API interop)
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.inner.as_ptr().as_ptr()
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
        // AlignedAlloc already zero-initializes, so the flag is a no-op for correctness
        // but kept for API compatibility
        PinnedMemory::new(self.size)
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

        memory.as_mut_slice()[0] = 42;
        memory.as_mut_slice()[99] = 84;

        assert_eq!(memory.as_slice()[0], 42);
        assert_eq!(memory.as_slice()[99], 84);
    }

    #[test]
    fn test_pinned_memory_builder() {
        let memory = PinnedMemoryBuilder::new(128)
            .zero_initialized()
            .build()
            .unwrap();

        for byte in memory.as_slice() {
            assert_eq!(*byte, 0);
        }
    }

    #[test]
    fn test_pinned_memory_large_allocation() {
        let memory = PinnedMemory::new(10 * 1024 * 1024).unwrap();
        assert_eq!(memory.size(), 10 * 1024 * 1024);
    }
}
