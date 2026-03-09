// SPDX-License-Identifier: AGPL-3.0-only
//! Safe volatile access abstractions for memory-mapped hardware
//!
//! Encapsulates unsafe volatile reads/writes and copy operations behind
//! bounds-checked safe methods. Unsafe is minimized to:
//! - Constructor (caller guarantees valid mmap'd region)
//! - The actual volatile/copy operation (minimal scope)

use crate::error::{AkidaError, Result};
use std::ptr::NonNull;

/// Safe wrapper for volatile access to a memory-mapped region
///
/// All bounds checking happens in safe code before entering unsafe blocks.
/// The unsafe scope is limited to the constructor and the actual
/// volatile/copy operations.
#[derive(Debug)]
pub struct VolatileSlice {
    ptr: NonNull<u8>,
    size: usize,
}

impl VolatileSlice {
    /// Create a volatile slice from a valid memory-mapped region
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - `ptr` is valid for reads and writes of `size` bytes
    /// - The memory is from a successful mmap (or equivalent)
    /// - The region is not deallocated while the slice is used
    #[must_use]
    pub unsafe fn from_raw_parts(ptr: NonNull<u8>, size: usize) -> Self {
        Self { ptr, size }
    }

    /// Read 32-bit value at offset (volatile)
    ///
    /// # Errors
    ///
    /// Returns error if `offset + 4` exceeds region size
    pub fn read_u32(&self, offset: usize) -> Result<u32> {
        if offset + 4 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds read: offset={offset:#x}, size=4, limit={:#x}",
                self.size
            )));
        }

        // SAFETY: Bounds validated above. ptr is valid (from constructor).
        // offset + 4 <= size ensures valid pointer arithmetic.
        // read_volatile required for MMIO (hardware may change value).
        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u32>();
            ptr.read_volatile()
        };
        Ok(value)
    }

    /// Write 32-bit value at offset (volatile)
    ///
    /// # Errors
    ///
    /// Returns error if `offset + 4` exceeds region size
    pub fn write_u32(&mut self, offset: usize, value: u32) -> Result<()> {
        if offset + 4 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds write: offset={offset:#x}, size=4, limit={:#x}",
                self.size
            )));
        }

        // SAFETY: Bounds validated above. ptr is valid (from constructor).
        // write_volatile required for MMIO (triggers hardware side effects).
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u32>();
            ptr.write_volatile(value);
        }
        Ok(())
    }

    /// Read 64-bit value at offset (volatile)
    ///
    /// # Errors
    ///
    /// Returns error if `offset + 8` exceeds region size
    pub fn read_u64(&self, offset: usize) -> Result<u64> {
        if offset + 8 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds read: offset={offset:#x}, size=8, limit={:#x}",
                self.size
            )));
        }

        // SAFETY: Bounds validated above. ptr is valid (from constructor).
        // read_volatile required for MMIO.
        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u64>();
            ptr.read_volatile()
        };
        Ok(value)
    }

    /// Write 64-bit value at offset (volatile)
    ///
    /// # Errors
    ///
    /// Returns error if `offset + 8` exceeds region size
    pub fn write_u64(&mut self, offset: usize, value: u64) -> Result<()> {
        if offset + 8 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds write: offset={offset:#x}, size=8, limit={:#x}",
                self.size
            )));
        }

        // SAFETY: Bounds validated above. ptr is valid (from constructor).
        // write_volatile required for MMIO.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u64>();
            ptr.write_volatile(value);
        }
        Ok(())
    }

    /// Read bytes from region into buffer
    ///
    /// # Errors
    ///
    /// Returns error if read would exceed region bounds
    pub fn read_region(&self, offset: usize, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds read: offset={offset:#x}, size={}, limit={:#x}",
                buf.len(),
                self.size
            )));
        }

        // SAFETY: Bounds validated above. src valid for buf.len() bytes.
        // dst from buffer.as_mut_ptr(). No overlap (different regions).
        // u8 has alignment 1.
        unsafe {
            let src = self.ptr.as_ptr().add(offset);
            std::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), buf.len());
        }
        Ok(())
    }

    /// Write bytes from buffer into region
    ///
    /// # Errors
    ///
    /// Returns error if write would exceed region bounds
    pub fn write_region(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds write: offset={offset:#x}, size={}, limit={:#x}",
                data.len(),
                self.size
            )));
        }

        // SAFETY: Bounds validated above. dst valid for data.len() bytes.
        // src from data.as_ptr(). No overlap. u8 has alignment 1.
        unsafe {
            let dst = self.ptr.as_ptr().add(offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
        Ok(())
    }
}
