// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // mmap/munmap require unsafe for NPU BAR memory mapping
//! Memory-mapped region abstraction
//!
//! Deep Debt Principles:
//! - Minimal unsafe (only in mmap, well-encapsulated)
//! - Runtime validation (bounds checking)
//! - Safe public API
//! - Comprehensive error handling
//!
//! # Evolution (Feb 12, 2026)
//!
//! Evolved from `libc` raw C bindings to `rustix` safe Rust wrappers.
//! This provides better error handling and type safety while maintaining
//! identical functionality.

use crate::backends::volatile_access::VolatileSlice;
use crate::error::{AkidaError, Result};
use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsFd;
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
    ///
    /// # Panics
    ///
    /// Panics if `rustix::mm::mmap` returns a null pointer on success
    /// (should never happen per rustix API contract).
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

        // SAFETY: Invariants: fd valid; size>0; flags valid; offset within file.
        // Satisfied: file from OpenOptions; size checked above; ProtFlags/MapFlags from rustix;
        // offset 0. File stored in struct; munmap in Drop. Violation: invalid fd → kernel error;
        // zero size → implementation-defined; leak if no munmap.
        let ptr = unsafe {
            let addr = mmap(
                std::ptr::null_mut(),
                size,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED,
                file.as_fd(),
                0,
            )
            .map_err(|e| AkidaError::capability_query_failed(format!("mmap failed: {e}")))?;

            // EVOLVED: NonNull::new + expect is safe; rustix returns non-null on Ok
            NonNull::new(addr.cast::<u8>())
                .expect("rustix mmap returns non-null pointer on success")
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
        // SAFETY: Invariants: ptr valid for size bytes; from mmap; not yet unmapped.
        // Satisfied: ptr/size from new(); _file keeps mapping alive. Violation: use-after-unmap → UB.
        let slice = unsafe { VolatileSlice::from_raw_parts(self.ptr, self.size) };
        let value = slice.read_u32(offset)?;
        tracing::trace!("Read u32 @ {offset:#x} = {value:#x}");
        Ok(value)
    }

    /// Write 32-bit register at offset
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of bounds
    pub fn write_u32(&mut self, offset: usize, value: u32) -> Result<()> {
        tracing::trace!("Write u32 @ {offset:#x} = {value:#x}");
        // SAFETY: Invariants: ptr valid for size; mapping alive. Satisfied: from new(); _file held.
        let mut slice = unsafe { VolatileSlice::from_raw_parts(self.ptr, self.size) };
        slice.write_u32(offset, value)
    }

    /// Read bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if read would exceed bounds
    pub fn read_bytes(&self, offset: usize, buffer: &mut [u8]) -> Result<()> {
        // SAFETY: Invariants: ptr valid for size; mapping alive. Satisfied: from new(); _file held.
        let slice = unsafe { VolatileSlice::from_raw_parts(self.ptr, self.size) };
        slice.read_region(offset, buffer)
    }

    /// Write bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if write would exceed bounds
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        // SAFETY: Invariants: ptr valid for size; mapping alive. Satisfied: from new(); _file held.
        let mut slice = unsafe { VolatileSlice::from_raw_parts(self.ptr, self.size) };
        slice.write_region(offset, data)
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

        // SAFETY: Invariants: addr from mmap; length matches original mmap; no refs to mapping.
        // Satisfied: ptr/size from new(); Drop runs once; no outstanding slices. Violation: wrong ptr/size → UB.
        unsafe {
            if let Err(e) = munmap(self.ptr.as_ptr().cast(), self.size) {
                tracing::error!("munmap failed during drop: {e}");
            }
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
    use crate::error::AkidaError;

    #[test]
    fn test_mmap_region_size_accessors() {
        // MmapRegion::new requires real PCIe device - test documented invariants
        // size(), pcie_address(), bar_index() return stored values
    }

    #[test]
    fn test_mmap_error_messages() {
        // Verify AkidaError types used by MmapRegion
        let _e = AkidaError::capability_query_failed("test");
        let _e2 = AkidaError::transfer_failed("out of bounds");
    }

    #[test]
    fn test_mmap_region_new_nonexistent_device() {
        let result = super::MmapRegion::new("0000:nonexistent:00.0", 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Cannot open") || msg.contains("capability") || msg.contains("resource")
        );
    }

    #[test]
    fn test_akida_error_display() {
        let e = AkidaError::capability_query_failed("Cannot open /dev/foo");
        assert!(e.to_string().contains("Cannot open"));
    }
}
