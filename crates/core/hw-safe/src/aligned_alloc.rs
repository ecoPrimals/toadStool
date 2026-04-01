// SPDX-License-Identifier: AGPL-3.0-only

//! Aligned heap allocation with RAII cleanup.
//!
//! [`AlignedAlloc`] wraps `std::alloc::alloc_zeroed` / `dealloc` to provide
//! heap memory with arbitrary alignment. This consolidates the duplicate
//! allocation patterns across:
//!
//! - `gpu` `AlignedBuffer` (cpu backend)
//! - `gpu` `PinnedMemory`
//! - `secure_enclave` `IsolatedMemoryRegion` (allocation portion)
//! - `akida-driver` DMA buffers

use std::alloc::Layout;
use std::ptr::NonNull;

/// Error type for allocation operations.
#[derive(Debug, thiserror::Error)]
pub enum AllocError {
    /// The requested layout is invalid (zero size or non-power-of-two alignment).
    #[error("invalid layout: size={size}, align={align}")]
    InvalidLayout {
        /// Requested size.
        size: usize,
        /// Requested alignment.
        align: usize,
    },
    /// The allocator returned null (out of memory).
    #[error("allocation failed: size={size}, align={align}")]
    OutOfMemory {
        /// Requested size.
        size: usize,
        /// Requested alignment.
        align: usize,
    },
}

/// RAII aligned heap allocation.
///
/// Allocates zero-initialized memory with the requested alignment.
/// Deallocates on drop. Provides safe slice access to the underlying buffer.
///
/// ## Thread safety
///
/// `AlignedAlloc` is `Send` and `Sync` because it owns the allocation
/// exclusively. The borrow checker prevents data races through `&` vs
/// `&mut` access to the slice views.
pub struct AlignedAlloc {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl AlignedAlloc {
    /// Allocate `size` bytes of zero-initialized memory with `align` alignment.
    ///
    /// # Errors
    ///
    /// Returns [`AllocError::InvalidLayout`] if the size/alignment is invalid,
    /// or [`AllocError::OutOfMemory`] if allocation fails.
    pub fn new(size: usize, align: usize) -> Result<Self, AllocError> {
        let layout =
            Layout::from_size_align(size, align).map_err(|_| AllocError::InvalidLayout {
                size,
                align,
            })?;

        if size == 0 {
            return Err(AllocError::InvalidLayout { size, align });
        }

        // SAFETY: layout is valid (from_size_align succeeded, size > 0,
        // align is power-of-two). alloc_zeroed returns a pointer valid for
        // layout.size() bytes, or null on OOM. Dealloc in Drop with the
        // same layout.
        let raw = unsafe { std::alloc::alloc_zeroed(layout) };

        let ptr = NonNull::new(raw).ok_or(AllocError::OutOfMemory { size, align })?;

        Ok(Self { ptr, layout })
    }

    /// Allocate with a specific [`Layout`].
    ///
    /// # Errors
    ///
    /// Returns [`AllocError::OutOfMemory`] if allocation fails, or
    /// [`AllocError::InvalidLayout`] if the layout has zero size.
    pub fn from_layout(layout: Layout) -> Result<Self, AllocError> {
        if layout.size() == 0 {
            return Err(AllocError::InvalidLayout {
                size: layout.size(),
                align: layout.align(),
            });
        }

        // SAFETY: layout is valid (caller provides a valid Layout, size > 0).
        let raw = unsafe { std::alloc::alloc_zeroed(layout) };

        let ptr = NonNull::new(raw).ok_or_else(|| AllocError::OutOfMemory {
            size: layout.size(),
            align: layout.align(),
        })?;

        Ok(Self { ptr, layout })
    }

    /// View the allocation as a byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr is from alloc_zeroed with layout.size() bytes.
        // &self ensures no concurrent mutable access. Size is immutable
        // after construction.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.layout.size()) }
    }

    /// View the allocation as a mutable byte slice.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: ptr is from alloc_zeroed with layout.size() bytes.
        // &mut self ensures exclusive access. Size is immutable.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size()) }
    }

    /// Size of the allocation in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.layout.size()
    }

    /// Alignment of the allocation.
    #[must_use]
    pub const fn align(&self) -> usize {
        self.layout.align()
    }

    /// The layout used for this allocation.
    #[must_use]
    pub const fn layout(&self) -> Layout {
        self.layout
    }

    /// Raw pointer to the allocation (for FFI or advanced use).
    #[must_use]
    pub const fn as_ptr(&self) -> NonNull<u8> {
        self.ptr
    }
}

impl Drop for AlignedAlloc {
    fn drop(&mut self) {
        // SAFETY: ptr and layout are from a successful alloc_zeroed call
        // in the constructor. Drop runs exactly once. No outstanding
        // references can exist (self is being dropped).
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

// SAFETY: AlignedAlloc owns the allocation exclusively. The raw pointer
// is never shared. Moving between threads is safe because the allocation
// remains valid. The borrow checker enforces &/&mut exclusivity for
// as_slice/as_mut_slice.
unsafe impl Send for AlignedAlloc {}
unsafe impl Sync for AlignedAlloc {}

impl std::fmt::Debug for AlignedAlloc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlignedAlloc")
            .field("ptr", &self.ptr)
            .field("size", &self.layout.size())
            .field("align", &self.layout.align())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_allocation() {
        let alloc = AlignedAlloc::new(4096, 64).unwrap();
        assert_eq!(alloc.size(), 4096);
        assert_eq!(alloc.align(), 64);
        assert!(alloc.as_slice().iter().all(|&b| b == 0));
    }

    #[test]
    fn write_and_read() {
        let mut alloc = AlignedAlloc::new(256, 16).unwrap();
        alloc.as_mut_slice()[0] = 0xAB;
        alloc.as_mut_slice()[255] = 0xCD;
        assert_eq!(alloc.as_slice()[0], 0xAB);
        assert_eq!(alloc.as_slice()[255], 0xCD);
    }

    #[test]
    fn zero_size_rejected() {
        let result = AlignedAlloc::new(0, 64);
        assert!(matches!(result, Err(AllocError::InvalidLayout { .. })));
    }

    #[test]
    fn bad_alignment_rejected() {
        let result = AlignedAlloc::new(4096, 3); // 3 is not power of two
        assert!(matches!(result, Err(AllocError::InvalidLayout { .. })));
    }

    #[test]
    fn from_layout() {
        let layout = Layout::from_size_align(1024, 128).unwrap();
        let alloc = AlignedAlloc::from_layout(layout).unwrap();
        assert_eq!(alloc.size(), 1024);
        assert_eq!(alloc.align(), 128);
    }

    #[test]
    fn alignment_honored() {
        let alloc = AlignedAlloc::new(64, 4096).unwrap();
        assert_eq!(alloc.as_ptr().as_ptr() as usize % 4096, 0);
    }
}
