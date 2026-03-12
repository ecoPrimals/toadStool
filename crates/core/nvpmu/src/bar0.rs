// SPDX-License-Identifier: AGPL-3.0-only
//! BAR0 MMIO register access for NVIDIA GPUs.
//!
//! Maps GPU BAR0 via `/sys/bus/pci/devices/{BDF}/resource0` for direct
//! register read/write. This is the mechanism for software PMU
//! initialization — replaying captured register sequences to bring up
//! the compute engine on GPUs without PMU firmware.
//!
//! # Safety
//!
//! BAR0 access requires appropriate permissions (root or udev rules).
//! Wrong register writes can hang the GPU or cause hardware damage.
//! Always verify thermal safety before and after applying recipes.
//!
//! # Phase 3 (this module)
//!
//! Provides the `Bar0Access` struct that can be used as a
//! `hw_learn::applicator::RegisterAccess` backend.

use crate::error::{NvPmuError, Result};
use std::fs::{File, OpenOptions};
use std::path::Path;

/// BAR0 MMIO region for a specific GPU.
///
/// Maps the GPU's BAR0 register space via PCI sysfs for direct
/// register access. Drop unmaps the region.
pub struct Bar0Access {
    /// Memory-mapped region (raw pointer + size).
    ptr: std::ptr::NonNull<u8>,
    size: usize,
    _file: File,
    bdf: String,
}

impl Bar0Access {
    /// Open BAR0 for a GPU identified by its PCI BDF address.
    ///
    /// # Errors
    ///
    /// Returns error if the resource file cannot be opened or mmap fails.
    /// Requires read+write permission on `/sys/bus/pci/devices/{bdf}/resource0`.
    pub fn open(bdf: &str) -> Result<Self> {
        let path = format!("/sys/bus/pci/devices/{bdf}/resource0");
        Self::open_path(bdf, Path::new(&path))
    }

    fn open_path(bdf: &str, path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        #[allow(clippy::cast_possible_truncation)]
        let size = file.metadata()?.len() as usize;
        if size == 0 {
            return Err(NvPmuError::SensorNotFound(format!(
                "BAR0 size is 0 for {bdf} (device not enabled?)"
            )));
        }

        // SAFETY: mmap of a PCI BAR resource file is standard Linux practice.
        // We validate the file descriptor (just opened), size (non-zero),
        // and use MAP_SHARED for device memory. The mapping is valid for the
        // lifetime of the File (held in struct). Unmapped in Drop.
        let ptr = unsafe {
            use std::os::unix::io::AsFd;
            let addr = rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                file.as_fd(),
                0,
            )
            .map_err(|e| {
                std::io::Error::other(format!("mmap BAR0: {e}"))
            })?;
            std::ptr::NonNull::new(addr.cast::<u8>())
                .ok_or_else(|| std::io::Error::other("mmap returned null"))?
        };

        tracing::info!(bdf, size, "BAR0 mapped ({} MB)", size / (1024 * 1024));

        Ok(Self {
            ptr,
            size,
            _file: file,
            bdf: bdf.to_string(),
        })
    }

    /// Read a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of bounds.
    pub fn read_u32(&self, offset: u64) -> Result<u32> {
        // Truncation is acceptable: BAR offsets never exceed usize on 64-bit (our only target)
        #[allow(clippy::cast_possible_truncation)]
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(NvPmuError::SensorNotFound(format!(
                "BAR0 read out of bounds: offset {offset:#x}, size {:#x}",
                self.size
            )));
        }
        // SAFETY: bounds checked above, ptr is from valid mmap, volatile
        // read is correct for MMIO registers. Alignment: BAR0 registers are
        // naturally u32-aligned; offsets must be 4-byte aligned by hardware spec.
        #[allow(clippy::cast_ptr_alignment)]
        let val = unsafe {
            let p = self.ptr.as_ptr().add(off).cast::<u32>();
            std::ptr::read_volatile(p)
        };
        Ok(val)
    }

    /// Write a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of bounds.
    pub fn write_u32(&mut self, offset: u64, value: u32) -> Result<()> {
        #[allow(clippy::cast_possible_truncation)]
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(NvPmuError::SensorNotFound(format!(
                "BAR0 write out of bounds: offset {offset:#x}, size {:#x}",
                self.size
            )));
        }
        // SAFETY: bounds checked above, ptr is from valid mmap, volatile
        // write is correct for MMIO registers. Alignment: see read_u32.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            let p = self.ptr.as_ptr().add(off).cast::<u32>();
            std::ptr::write_volatile(p, value);
        }
        Ok(())
    }

    /// BAR0 region size in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// PCI BDF address of the GPU.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }
}

impl Drop for Bar0Access {
    fn drop(&mut self) {
        // SAFETY: ptr/size from successful mmap in constructor.
        unsafe {
            if let Err(e) = rustix::mm::munmap(self.ptr.as_ptr().cast(), self.size) {
                tracing::error!(bdf = %self.bdf, "munmap BAR0 failed: {e}");
            }
        }
        tracing::debug!(bdf = %self.bdf, "BAR0 unmapped");
    }
}

// SAFETY: Bar0Access owns the mapped memory exclusively. Moving between
// threads doesn't invalidate the mapping. Writes require &mut self.
unsafe impl Send for Bar0Access {}

impl hw_learn::applicator::RegisterAccess for Bar0Access {
    fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
        Bar0Access::read_u32(self, offset).map_err(|e| e.to_string())
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
        Bar0Access::write_u32(self, offset, value).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_nonexistent_device() {
        let result = Bar0Access::open("0000:nonexistent:00.0");
        assert!(result.is_err());
    }
}
