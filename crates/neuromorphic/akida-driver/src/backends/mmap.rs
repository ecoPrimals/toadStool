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

        // SAFETY: mmap is unsafe but we validate all preconditions:
        // - File descriptor is valid (just opened via OpenOptions)
        // - Size is non-zero (checked above, prevents zero-sized mapping)
        // - Flags are valid: PROT_READ|PROT_WRITE for MMIO access, MAP_SHARED for device memory
        // - Offset is 0 (start of BAR)
        // - We check for MAP_FAILED and return error if mapping fails
        // - We store file in struct to keep fd open for lifetime of mapping
        // - We unmap in Drop impl to prevent memory leak
        // - The mapped memory is process-private (no other references exist)
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

            // SAFETY: We just checked addr != MAP_FAILED, so addr is a valid pointer.
            // NonNull::new_unchecked is safe because:
            // - mmap returns either MAP_FAILED (checked) or a valid pointer
            // - The pointer points to mapped memory of at least `size` bytes
            // - The cast to *mut u8 is valid (byte-aligned memory)
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

        // SAFETY: Volatile read from memory-mapped hardware register.
        // Invariants that must hold:
        // - Bounds validated above: offset + 4 <= self.size
        // - ptr is valid (from successful mmap, stored in NonNull)
        // - offset + 4 bytes are within mapped region
        // - Pointer arithmetic: self.ptr.as_ptr().add(offset) is valid (offset < size)
        // - Cast to *const u32: PCIe BAR registers are 4-byte aligned per spec
        // - read_volatile is required: MMIO registers have side effects and compiler
        //   must not reorder or optimize these reads (hardware may change value)
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

        // SAFETY: Volatile write to memory-mapped hardware register.
        // Invariants that must hold:
        // - Bounds validated above: offset + 4 <= self.size
        // - ptr is valid (from successful mmap, stored in NonNull)
        // - offset + 4 bytes are within mapped region
        // - Pointer arithmetic: self.ptr.as_ptr().add(offset) is valid (offset < size)
        // - Cast to *mut u32: PCIe BAR registers are 4-byte aligned per spec
        // - write_volatile is required: MMIO writes have side effects (trigger hardware
        //   operations) and compiler must not reorder or optimize these writes
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

        // SAFETY: copy_nonoverlapping requires:
        // - src is valid for reads of buffer.len() bytes
        // - dst is valid for writes of buffer.len() bytes
        // - src and dst do not overlap
        // - Both pointers are properly aligned
        // Invariants that hold:
        // - Bounds validated above: offset + buffer.len() <= self.size
        // - src = self.ptr.as_ptr().add(offset) is valid (offset < size, within mapped region)
        // - dst = buffer.as_mut_ptr() is valid (buffer is a valid mutable slice)
        // - No overlap: src points to mapped hardware memory, dst points to user buffer
        // - Alignment: u8 has alignment 1, so both pointers are properly aligned
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

        // SAFETY: copy_nonoverlapping requires:
        // - src is valid for reads of data.len() bytes
        // - dst is valid for writes of data.len() bytes
        // - src and dst do not overlap
        // - Both pointers are properly aligned
        // Invariants that hold:
        // - Bounds validated above: offset + data.len() <= self.size
        // - src = data.as_ptr() is valid (data is a valid slice)
        // - dst = self.ptr.as_ptr().add(offset) is valid (offset < size, within mapped region)
        // - No overlap: src points to user data, dst points to mapped hardware memory
        // - Alignment: u8 has alignment 1, so both pointers are properly aligned
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

        // SAFETY: munmap requires:
        // - addr must be a pointer returned by mmap (or MAP_FAILED, but we check that)
        // - length must match the length passed to mmap
        // Invariants that hold:
        // - self.ptr was created from successful mmap (checked MAP_FAILED in new())
        // - self.size matches the size passed to mmap in new()
        // - The mapping is still valid (we're in Drop, so no use-after-free)
        // - Cast to *mut c_void is valid (mmap returns void*, munmap expects void*)
        unsafe {
            libc::munmap(self.ptr.as_ptr().cast(), self.size);
        }
    }
}

// SAFETY: Send implementation is safe because:
// - MmapRegion owns the mapped memory exclusively (no other references exist)
// - The memory mapping is process-private (MAP_SHARED with device file, but no
//   other in-process references to the same mapping)
// - The mapped memory is valid for the lifetime of the MmapRegion (file kept open)
// - All pointer operations are bounds-checked and safe
// - Moving MmapRegion between threads doesn't invalidate the mapping
unsafe impl Send for MmapRegion {}

// SAFETY: Sync implementation is safe because:
// - MmapRegion API requires &mut self for writes (exclusive access enforced by borrow checker)
// - Read operations use &self but are safe because:
//   - All reads are bounds-checked
//   - Volatile reads prevent data races (hardware register reads are idempotent)
//   - Multiple concurrent reads from MMIO registers are safe (hardware handles it)
// - The underlying memory mapping is thread-safe (mmap'd memory can be accessed from any thread)
// - No internal mutable state without synchronization (size, ptr, _file are immutable)
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
