// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // Volatile reads/writes require unsafe — this is the containment zone

//! Bounds-checked volatile MMIO register access.
//!
//! [`VolatileMmio`] provides safe `read_u32`/`write_u32` over a memory-mapped
//! hardware region. Construction is unsafe (caller proves pointer validity);
//! all subsequent access is safe with runtime bounds checking.
//!
//! Aligned with coralReef's `coral-driver::mmio::VolatilePtr<T>` pattern:
//! unsafe to create, safe to use.

use std::ptr::NonNull;

/// Error type for MMIO operations.
#[derive(Debug, thiserror::Error)]
pub enum MmioError {
    /// Access would exceed the mapped region.
    #[error("MMIO access at offset {offset:#x} + {width} exceeds region size {region_size:#x}")]
    OutOfBounds {
        /// Requested offset.
        offset: usize,
        /// Access width in bytes.
        width: usize,
        /// Total region size.
        region_size: usize,
    },
}

/// Bounds-checked volatile MMIO view over a memory-mapped region.
///
/// Construction is unsafe — the caller must guarantee the pointer is valid
/// for `size` bytes and remains valid for the lifetime `'a`. All read/write
/// methods are safe, performing bounds checks before every access.
///
/// # Alignment
///
/// Hardware MMIO registers are naturally aligned by specification. The
/// `read_u32`/`write_u32` methods assume 4-byte alignment of the base
/// pointer and that offsets are 4-byte aligned. This is guaranteed by
/// PCI BAR mappings and NPU MMIO regions on Linux.
pub struct VolatileMmio<'a> {
    ptr: NonNull<u8>,
    size: usize,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl VolatileMmio<'_> {
    /// Create a volatile MMIO view.
    ///
    /// # Safety
    ///
    /// - `ptr` must be valid for reads and writes of `size` bytes.
    /// - The memory must remain mapped for the lifetime `'a`.
    /// - The memory must be suitable for volatile access (device MMIO or
    ///   similar — not normal heap memory where the compiler may optimize
    ///   away accesses).
    #[must_use]
    pub unsafe fn new(ptr: NonNull<u8>, size: usize) -> Self {
        Self {
            ptr,
            size,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Size of the MMIO region in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Read a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 4 > size`.
    pub fn read_u32(&self, offset: usize) -> Result<u32, MmioError> {
        if offset + 4 > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width: 4,
                region_size: self.size,
            });
        }
        // SAFETY: bounds checked above. ptr is valid for size bytes
        // (caller invariant from constructor). Volatile read is correct
        // for MMIO registers — prevents compiler reordering/elision.
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "MMIO registers are naturally u32-aligned"
        )]
        let val = unsafe {
            let p = self.ptr.as_ptr().add(offset).cast::<u32>();
            std::ptr::read_volatile(p)
        };
        Ok(val)
    }

    /// Write a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 4 > size`.
    pub fn write_u32(&self, offset: usize, value: u32) -> Result<(), MmioError> {
        if offset + 4 > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width: 4,
                region_size: self.size,
            });
        }
        // SAFETY: bounds checked above. ptr is valid and mapped (caller
        // invariant). Volatile write is correct for MMIO.
        #[allow(
            clippy::cast_ptr_alignment,
            reason = "MMIO registers are naturally u32-aligned"
        )]
        unsafe {
            let p = self.ptr.as_ptr().add(offset).cast::<u32>();
            std::ptr::write_volatile(p, value);
        }
        Ok(())
    }

    /// Read a byte range from the MMIO region into a buffer.
    ///
    /// Uses volatile byte-by-byte reads to prevent compiler optimization.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if the read would exceed the region.
    pub fn read_bytes(&self, offset: usize, buf: &mut [u8]) -> Result<(), MmioError> {
        if offset + buf.len() > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width: buf.len(),
                region_size: self.size,
            });
        }
        for (i, byte) in buf.iter_mut().enumerate() {
            // SAFETY: bounds checked above. Each byte offset is within the region.
            *byte = unsafe { std::ptr::read_volatile(self.ptr.as_ptr().add(offset + i)) };
        }
        Ok(())
    }

    /// Write a byte range to the MMIO region.
    ///
    /// Uses volatile byte-by-byte writes to prevent compiler optimization.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if the write would exceed the region.
    pub fn write_bytes(&self, offset: usize, data: &[u8]) -> Result<(), MmioError> {
        if offset + data.len() > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width: data.len(),
                region_size: self.size,
            });
        }
        for (i, &byte) in data.iter().enumerate() {
            // SAFETY: bounds checked above. Each byte offset is within the region.
            unsafe {
                std::ptr::write_volatile(self.ptr.as_ptr().add(offset + i), byte);
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for VolatileMmio<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolatileMmio")
            .field("ptr", &self.ptr)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
