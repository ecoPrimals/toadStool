// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    unsafe_code,
    reason = "volatile reads/writes require unsafe — containment zone"
)]

//! Bounds-checked volatile MMIO register access.
//!
//! [`VolatileMmio`] provides safe `read_u32`/`write_u32`/`read_u64`/`write_u64` over a memory-mapped
//! hardware region. Construction is unsafe (caller proves pointer validity);
//! all subsequent access is safe with runtime bounds checking.
//!
//! Aligned with the visualization service's `coral-driver::mmio::VolatilePtr<T>` pattern:
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
    /// Access address is not naturally aligned for the register width.
    #[error("MMIO access at effective address {address:#x} is not {alignment}-byte aligned")]
    Misaligned {
        /// Effective address (base + offset).
        address: usize,
        /// Required alignment in bytes.
        alignment: usize,
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
/// `read_u32`/`write_u32` methods assume 4-byte alignment; `read_u64`/
/// `write_u64` assume 8-byte alignment. This is guaranteed by PCI BAR
/// mappings and NPU MMIO regions on Linux.
pub struct VolatileMmio<'a> {
    ptr: NonNull<u8>,
    size: usize,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl VolatileMmio<'_> {
    /// Reject accesses whose end offset overflows `usize` or exceeds the region.
    #[inline]
    fn check_bounds(&self, offset: usize, width: usize) -> Result<(), MmioError> {
        let end = offset.checked_add(width).ok_or(MmioError::OutOfBounds {
            offset,
            width,
            region_size: self.size,
        })?;
        if end > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width,
                region_size: self.size,
            });
        }
        Ok(())
    }

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

    /// Bounds-checked volatile read of a `T`-sized register.
    ///
    /// Single unsafe primitive for all register-width reads. The public
    /// `read_u32`/`read_u64` methods delegate here.
    fn check_alignment<T>(&self, offset: usize) -> Result<(), MmioError> {
        let align = std::mem::align_of::<T>();
        let addr = (self.ptr.as_ptr() as usize).wrapping_add(offset);
        if !addr.is_multiple_of(align) {
            return Err(MmioError::Misaligned {
                address: addr,
                alignment: align,
            });
        }
        Ok(())
    }

    fn read_reg<T: Copy>(&self, offset: usize) -> Result<T, MmioError> {
        let width = std::mem::size_of::<T>();
        self.check_bounds(offset, width)?;
        self.check_alignment::<T>(offset)?;
        let p = self.ptr.as_ptr().wrapping_add(offset).cast::<T>();
        // SAFETY: `check_bounds` ensures `offset+width <= self.size` so `p` lies in the
        // mapped region; `check_alignment` ensures natural alignment for `T`.
        // Constructor invariant: region is valid for volatile reads.
        Ok(unsafe { std::ptr::read_volatile(p) })
    }

    /// Bounds-checked volatile write of a `T`-sized register.
    ///
    /// Single unsafe primitive for all register-width writes. The public
    /// `write_u32`/`write_u64` methods delegate here.
    fn write_reg<T: Copy>(&self, offset: usize, value: T) -> Result<(), MmioError> {
        let width = std::mem::size_of::<T>();
        self.check_bounds(offset, width)?;
        self.check_alignment::<T>(offset)?;
        let p = self.ptr.as_ptr().wrapping_add(offset).cast::<T>();
        // SAFETY: `check_bounds` + `check_alignment` ensure valid, aligned access
        // within the mapped region. Constructor invariant: suitable for volatile writes.
        unsafe { std::ptr::write_volatile(p, value) }
        Ok(())
    }

    /// Read a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 4 > size`.
    pub fn read_u32(&self, offset: usize) -> Result<u32, MmioError> {
        self.read_reg::<u32>(offset)
    }

    /// Write a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 4 > size`.
    pub fn write_u32(&self, offset: usize, value: u32) -> Result<(), MmioError> {
        self.write_reg::<u32>(offset, value)
    }

    /// Read a 64-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 8 > size`.
    pub fn read_u64(&self, offset: usize) -> Result<u64, MmioError> {
        self.read_reg::<u64>(offset)
    }

    /// Write a 64-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if `offset + 8 > size`.
    pub fn write_u64(&self, offset: usize, value: u64) -> Result<(), MmioError> {
        self.write_reg::<u64>(offset, value)
    }

    /// Read a byte range from the MMIO region into a buffer.
    ///
    /// Uses volatile byte-by-byte reads to prevent compiler optimization.
    ///
    /// # Errors
    ///
    /// Returns [`MmioError::OutOfBounds`] if the read would exceed the region.
    pub fn read_bytes(&self, offset: usize, buf: &mut [u8]) -> Result<(), MmioError> {
        self.check_bounds(offset, buf.len())?;
        for (i, byte) in buf.iter_mut().enumerate() {
            let p = self.ptr.as_ptr().wrapping_add(offset + i);
            // SAFETY: `check_bounds` ensures every `offset + i` is within the mapped region.
            *byte = unsafe { std::ptr::read_volatile(p) };
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
        self.check_bounds(offset, data.len())?;
        for (i, &byte) in data.iter().enumerate() {
            let p = self.ptr.as_ptr().wrapping_add(offset + i);
            // SAFETY: `check_bounds` ensures every `offset + i` is within the mapped region.
            unsafe { std::ptr::write_volatile(p, byte) }
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
