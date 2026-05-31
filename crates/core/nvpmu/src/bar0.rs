// SPDX-License-Identifier: AGPL-3.0-or-later
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
//! # Evolution
//!
//! Migrated from hand-rolled mmap/munmap to [`toadstool_hw_safe::SafeMmapRegion`],
//! which owns the mapping lifetime and provides [`toadstool_hw_safe::VolatileMmio`] for
//! bounds-checked volatile reads and writes.

use crate::error::{NvPmuError, Result};
use std::path::Path;
use toadstool_hw_safe::SafeMmapRegion;

/// BAR0 MMIO region for a specific GPU.
///
/// Maps the GPU's BAR0 register space via PCI sysfs for direct
/// register access. Drop unmaps the region automatically via the
/// inner [`SafeMmapRegion`].
pub struct Bar0Access {
    inner: SafeMmapRegion,
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
        let path = toadstool_common::sysfs_paths::sysfs_pci_device_file(bdf, "resource0");
        Self::open_path(bdf, Path::new(&path))
    }

    fn open_path(bdf: &str, path: &Path) -> Result<Self> {
        let inner = SafeMmapRegion::map_shared_rw(path)
            .map_err(|e| NvPmuError::SensorNotFound(format!("BAR0 mmap for {bdf}: {e}")))?;

        tracing::info!(
            bdf,
            size = inner.size(),
            "BAR0 mapped ({} MB)",
            inner.size() / (1024 * 1024)
        );

        Ok(Self {
            inner,
            bdf: bdf.to_string(),
        })
    }

    /// Read a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of bounds.
    pub fn read_u32(&self, offset: u64) -> Result<u32> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "BAR offsets never exceed usize on 64-bit Linux"
        )]
        let off = offset as usize;
        self.inner
            .as_volatile()
            .read_u32(off)
            .map_err(|e| NvPmuError::SensorNotFound(format!("BAR0 read @ {offset:#x}: {e}")))
    }

    /// Write a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of bounds.
    pub fn write_u32(&mut self, offset: u64, value: u32) -> Result<()> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "BAR offsets never exceed usize on 64-bit Linux"
        )]
        let off = offset as usize;
        self.inner
            .as_volatile()
            .write_u32(off, value)
            .map_err(|e| NvPmuError::SensorNotFound(format!("BAR0 write @ {offset:#x}: {e}")))
    }

    /// BAR0 region size in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.inner.size()
    }

    /// PCI BDF address of the GPU.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }
}

// Send auto-derived: Bar0Access contains SafeMmapRegion (Send via memmap2's
// MmapInner: Send + Sync) and String (Send). No raw pointers.
#[expect(dead_code, reason = "compile-time trait bound assertion")]
const _: () = {
    fn assert_send<T: Send>() {}
    fn check() {
        assert_send::<Bar0Access>();
    }
};

impl hw_learn::applicator::RegisterAccess for Bar0Access {
    fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
        Self::read_u32(self, offset).map_err(|e| e.to_string())
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
        Self::write_u32(self, offset, value).map_err(|e| e.to_string())
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
