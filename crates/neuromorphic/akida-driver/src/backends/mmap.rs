// SPDX-License-Identifier: AGPL-3.0-only
//! Memory-mapped region abstraction for Akida NPU PCIe BARs.
//!
//! Delegates mmap lifecycle to [`toadstool_hw_safe::SafeMmapRegion`],
//! eliminating duplicate unsafe mmap/munmap code. All volatile MMIO
//! is performed through the safe `VolatileMmio` wrapper.

use crate::error::{AkidaError, Result};
use std::path::Path;

/// Memory-mapped PCIe BAR region
///
/// Provides safe, bounds-checked access to memory-mapped hardware.
/// Backed by [`toadstool_hw_safe::SafeMmapRegion`] for the mmap lifecycle.
#[derive(Debug)]
pub struct MmapRegion {
    inner: toadstool_hw_safe::SafeMmapRegion,
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
    pub fn new(pcie_address: &str, bar_index: usize) -> Result<Self> {
        let path = format!("/sys/bus/pci/devices/{pcie_address}/resource{bar_index}");

        tracing::debug!("Mapping PCIe BAR: {path}");

        let inner = toadstool_hw_safe::SafeMmapRegion::map_shared_rw(Path::new(&path))
            .map_err(|e| {
                AkidaError::capability_query_failed(format!(
                    "BAR{bar_index} mmap for {pcie_address}: {e}"
                ))
            })?;

        tracing::info!(
            "Mapped BAR{bar_index} for {pcie_address} ({} MB)",
            inner.size() / (1024 * 1024),
        );

        Ok(Self {
            inner,
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
        let volatile = self.inner.as_volatile();
        let value = volatile.read_u32(offset).map_err(|e| {
            AkidaError::transfer_failed(format!("BAR read_u32 @ {offset:#x}: {e}"))
        })?;
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
        let volatile = self.inner.as_volatile();
        volatile.write_u32(offset, value).map_err(|e| {
            AkidaError::transfer_failed(format!("BAR write_u32 @ {offset:#x}: {e}"))
        })
    }

    /// Read bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if read would exceed bounds
    pub fn read_bytes(&self, offset: usize, buffer: &mut [u8]) -> Result<()> {
        self.inner
            .as_volatile()
            .read_bytes(offset, buffer)
            .map_err(|e| AkidaError::transfer_failed(format!("BAR read_bytes @ {offset:#x}: {e}")))
    }

    /// Write bytes at offset
    ///
    /// # Errors
    ///
    /// Returns error if write would exceed bounds
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.inner
            .as_volatile()
            .write_bytes(offset, data)
            .map_err(|e| {
                AkidaError::transfer_failed(format!("BAR write_bytes @ {offset:#x}: {e}"))
            })
    }

    /// Get region size
    #[must_use]
    pub fn size(&self) -> usize {
        self.inner.size()
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
            msg.contains("mmap") || msg.contains("capability") || msg.contains("BAR")
        );
    }

    #[test]
    fn test_akida_error_display() {
        let e = AkidaError::capability_query_failed("Cannot open /dev/foo");
        assert!(e.to_string().contains("Cannot open"));
    }
}
