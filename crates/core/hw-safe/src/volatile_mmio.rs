// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // Volatile reads/writes require unsafe — this is the containment zone

//! Bounds-checked volatile MMIO register access.
//!
//! [`VolatileMmio`] provides safe `read_u32`/`write_u32`/`read_u64`/`write_u64` over a memory-mapped
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
/// `read_u32`/`write_u32` methods assume 4-byte alignment; `read_u64`/
/// `write_u64` assume 8-byte alignment. This is guaranteed by PCI BAR
/// mappings and NPU MMIO regions on Linux.
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

    /// Bounds-checked volatile read of a `T`-sized register.
    ///
    /// Single unsafe primitive for all register-width reads. The public
    /// `read_u32`/`read_u64` methods delegate here.
    fn read_reg<T: Copy>(&self, offset: usize) -> Result<T, MmioError> {
        let width = std::mem::size_of::<T>();
        if offset + width > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width,
                region_size: self.size,
            });
        }
        // SAFETY: bounds checked above; ptr valid for size bytes (constructor
        // invariant); T is naturally aligned for MMIO; volatile prevents
        // compiler reordering/elision.
        Ok(unsafe {
            let p = self.ptr.as_ptr().add(offset).cast::<T>();
            std::ptr::read_volatile(p)
        })
    }

    /// Bounds-checked volatile write of a `T`-sized register.
    ///
    /// Single unsafe primitive for all register-width writes. The public
    /// `write_u32`/`write_u64` methods delegate here.
    fn write_reg<T: Copy>(&self, offset: usize, value: T) -> Result<(), MmioError> {
        let width = std::mem::size_of::<T>();
        if offset + width > self.size {
            return Err(MmioError::OutOfBounds {
                offset,
                width,
                region_size: self.size,
            });
        }
        // SAFETY: bounds checked above; ptr valid and mapped (constructor
        // invariant); T is naturally aligned for MMIO.
        unsafe {
            let p = self.ptr.as_ptr().add(offset).cast::<T>();
            std::ptr::write_volatile(p, value);
        }
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
