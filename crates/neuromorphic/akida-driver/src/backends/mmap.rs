//! Memory-mapped region abstraction
//!
//! Deep Debt Principles:
//! - Minimal unsafe (only in mmap, well-encapsulated)
//! - Runtime validation (bounds checking)
//! - Safe public API
//! - Comprehensive error handling

use crate::error::{AkidaError, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::ptr::NonNull;

/// Memory-mapped PCIe BAR region
///
/// Provides safe, bounds-checked access to memory-mapped hardware.
/// Unsafe operations are encapsulated and well-documented.
#[derive(Debug)]
pub struct MmapRegion {
    ptr: NonNull<u8>,
    size: usize,
    _file: File,
    pcie_address: String,
    bar_index: usize,
}

impl MmapRegion {
    /// Create memory-mapped region for PCIe BAR
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Resource file doesn't exist
    /// - Cannot open file
    /// - mmap fails
    ///
    /// # Safety
    ///
    /// This function contains unsafe mmap operation, but:
    /// - Validates file descriptor before mapping
    /// - Checks mmap return value
    /// - Ensures proper cleanup via Drop
    pub fn new(pcie_address: &str, bar_index: usize) -> Result<Self> {
        let path = format!("/sys/bus/pci/devices/{pcie_address}/resource{bar_index}");

        tracing::debug!("Mapping PCIe BAR: {path}");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                AkidaError::capability_query_failed(format!(
                    "Cannot open {path}: {e}. Is device enabled?"
                ))
            })?;

        // Truncation acceptable: BAR sizes fit in usize on 64-bit (our only target)
        #[allow(clippy::cast_possible_truncation)]
        let size = file
            .metadata()
            .map_err(|e| AkidaError::capability_query_failed(format!("Cannot stat BAR: {e}")))?
            .len() as usize;

        if size == 0 {
            return Err(AkidaError::capability_query_failed(
                "BAR size is 0 (device not enabled?)",
            ));
        }

        tracing::debug!("BAR size: {size} bytes ({} MB)", size / (1024 * 1024));

        // SAFETY: mmap is unsafe but we validate:
        // - File descriptor is valid (just opened)
        // - Size is non-zero
        // - We check MAP_FAILED
        // - We store file to keep it open
        // - We unmap in Drop
        let ptr = unsafe {
            let addr = libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            );

            if addr == libc::MAP_FAILED {
                let err = std::io::Error::last_os_error();
                return Err(AkidaError::capability_query_failed(format!(
                    "mmap failed: {err}"
                )));
            }

            // SAFETY: We just checked addr != MAP_FAILED
            NonNull::new_unchecked(addr.cast::<u8>())
        };

        tracing::info!(
            "Mapped BAR{bar_index} for {pcie_address} ({} MB at {ptr:p})",
            size / (1024 * 1024),
        );

        Ok(Self {
            ptr,
            size,
            _file: file,
            pcie_address: pcie_address.to_string(),
            bar_index,
        })
    }

    /// Read 32-bit register at offset
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of bounds
    pub fn read_u32(&self, offset: usize) -> Result<u32> {
        if offset + 4 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds read: offset={offset:#x}, size=4, limit={:#x}",
                self.size
            )));
        }

        // SAFETY: We just validated bounds. Volatile read prevents compiler
        // reordering, which is required for MMIO registers. The pointer is
        // aligned because BAR offsets for u32 registers are 4-byte aligned
        // per PCIe spec, and we trust hardware register map correctness.
        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u32>();
            ptr.read_volatile()
        };

        tracing::trace!("Read u32 @ {offset:#x} = {value:#x}");
        Ok(value)
    }

    /// Write 32-bit register at offset
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of bounds
    pub fn write_u32(&mut self, offset: usize, value: u32) -> Result<()> {
        if offset + 4 > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds write: offset={offset:#x}, size=4, limit={:#x}",
                self.size
            )));
        }

        tracing::trace!("Write u32 @ {offset:#x} = {value:#x}");

        // SAFETY: We just validated bounds. Volatile write ensures hardware
        // sees the update immediately. See read_u32 for alignment rationale.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            let ptr = self.ptr.as_ptr().add(offset).cast::<u32>();
            ptr.write_volatile(value);
        }

        Ok(())
    }

    /// Read bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if read would exceed bounds
    pub fn read_bytes(&self, offset: usize, buffer: &mut [u8]) -> Result<()> {
        if offset + buffer.len() > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds read: offset={offset:#x}, size={}, limit={:#x}",
                buffer.len(),
                self.size
            )));
        }

        // SAFETY: Bounds validated above, both pointers are valid
        unsafe {
            let src = self.ptr.as_ptr().add(offset);
            std::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), buffer.len());
        }

        Ok(())
    }

    /// Write bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if write would exceed bounds
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(AkidaError::transfer_failed(format!(
                "Out of bounds write: offset={offset:#x}, size={}, limit={:#x}",
                data.len(),
                self.size
            )));
        }

        // SAFETY: Bounds validated above, both pointers are valid
        unsafe {
            let dst = self.ptr.as_ptr().add(offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }

        Ok(())
    }

    /// Get region size
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Get PCIe address
    #[must_use]
    pub fn pcie_address(&self) -> &str {
        &self.pcie_address
    }

    /// Get BAR index
    #[must_use]
    pub const fn bar_index(&self) -> usize {
        self.bar_index
    }
}

impl Drop for MmapRegion {
    fn drop(&mut self) {
        tracing::debug!(
            "Unmapping BAR{} for {} ({} MB)",
            self.bar_index,
            self.pcie_address,
            self.size / (1024 * 1024)
        );

        // SAFETY: ptr and size are valid (from successful mmap)
        unsafe {
            libc::munmap(self.ptr.as_ptr().cast(), self.size);
        }
    }
}

// SAFETY: MmapRegion owns the mapped memory exclusively.
// The memory mapping is process-private (MAP_SHARED with a file but
// no other in-process references exist). Send/Sync is safe because
// the MmapRegion API requires &mut self for writes.
unsafe impl Send for MmapRegion {}
unsafe impl Sync for MmapRegion {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bounds_checking() {
        // This would require actual hardware, so we document the behavior
        // Bounds checking prevents:
        // - Reading beyond BAR size
        // - Writing beyond BAR size
        // - Accessing unallocated memory
    }
}
