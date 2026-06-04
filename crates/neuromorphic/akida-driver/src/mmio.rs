// SPDX-License-Identifier: AGPL-3.0-or-later
//! Memory-Mapped I/O for Akida NPU
//!
//! Provides safe abstractions for accessing Akida hardware registers.
//! Based on VFIO region mapping via `hw-safe` shared abstractions.
//!
//! # Deep Debt Evolution (ecoBin compliant, Apr 2026)
//!
//! Migrated to `hw-safe::DeviceMmap` + `hw-safe::vfio_setup` — zero local
//! `unsafe` blocks. All mmap/ioctl unsafety is contained in `hw-safe`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]

use crate::error::{AkidaError, Result};
use std::os::fd::AsFd;
use toadstool_hw_safe::volatile_mmio::MmioError;
use toadstool_hw_safe::{DeviceMmap, VolatileMmio, vfio_setup};

pub use toadstool_hw_safe::volatile_mmio::MmioError as MmioAccessError;

/// AKD1000 BAR regions
#[derive(Debug, Clone, Copy)]
pub enum Bar {
    /// Control/status registers (BAR0)
    Control = 0,
    /// Model memory (BAR1)
    Model = 1,
    /// Data buffers (BAR2)
    Data = 2,
}

/// AKD1000 register offsets (inferred from behavior)
pub mod regs {
    /// Device identification register
    pub const DEVICE_ID: usize = 0x0000;
    /// Device version register
    pub const VERSION: usize = 0x0004;
    /// Device status register
    pub const STATUS: usize = 0x0008;
    /// Control register
    pub const CONTROL: usize = 0x000C;
    /// NPU count register
    pub const NPU_COUNT: usize = 0x0010;
    /// SRAM size register (in KB)
    pub const SRAM_SIZE: usize = 0x0014;
    /// Interrupt status
    pub const IRQ_STATUS: usize = 0x0020;
    /// Interrupt enable
    pub const IRQ_ENABLE: usize = 0x0024;
    /// Model load address
    pub const MODEL_ADDR_LO: usize = 0x0100;
    /// Model load address high
    pub const MODEL_ADDR_HI: usize = 0x0104;
    /// Model size
    pub const MODEL_SIZE: usize = 0x0108;
    /// Model load trigger
    pub const MODEL_LOAD: usize = 0x010C;
    /// Input buffer address
    pub const INPUT_ADDR_LO: usize = 0x0200;
    /// Input buffer address high
    pub const INPUT_ADDR_HI: usize = 0x0204;
    /// Input size
    pub const INPUT_SIZE: usize = 0x0208;
    /// Output buffer address
    pub const OUTPUT_ADDR_LO: usize = 0x0300;
    /// Output buffer address high
    pub const OUTPUT_ADDR_HI: usize = 0x0304;
    /// Output size
    pub const OUTPUT_SIZE: usize = 0x0308;
    /// Inference trigger
    pub const INFER_START: usize = 0x0400;
    /// Inference status
    pub const INFER_STATUS: usize = 0x0404;

    /// Status register bit definitions
    pub mod status {
        /// Device is ready to accept commands
        pub const READY: u32 = 1 << 0;
        /// Device is currently processing
        pub const BUSY: u32 = 1 << 1;
        /// An error occurred during last operation
        pub const ERROR: u32 = 1 << 2;
        /// A model has been successfully loaded
        pub const MODEL_LOADED: u32 = 1 << 3;
    }

    /// Control register bit definitions
    pub mod control {
        /// Trigger a soft reset of the device
        pub const RESET: u32 = 1 << 0;
        /// Enable device operation
        pub const ENABLE: u32 = 1 << 1;
        /// Enable power-saving mode
        pub const POWER_SAVE: u32 = 1 << 2;
    }
}

/// Mapped BAR region for MMIO access.
///
/// Wraps [`DeviceMmap`] — all mmap/munmap unsafe is contained in `hw-safe`.
pub struct MappedRegion {
    mmap: DeviceMmap,
    bar: Bar,
}

impl std::fmt::Debug for MappedRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedRegion")
            .field("ptr", &format_args!("{:p}", self.mmap.as_ptr().as_ptr()))
            .field("size", &self.mmap.size())
            .field("bar", &self.bar)
            .finish()
    }
}

impl MappedRegion {
    /// Map a BAR region via VFIO.
    ///
    /// Uses `hw-safe::vfio_setup::device_get_region_info` and `DeviceMmap`
    /// — zero local unsafe blocks.
    ///
    /// # Errors
    ///
    /// Returns an error if the VFIO region info ioctl or the mmap fails.
    pub fn map(device_fd: &impl AsFd, bar: Bar) -> Result<Self> {
        let region_info = vfio_setup::device_get_region_info(device_fd.as_fd(), bar as u32)
            .map_err(|e| {
                AkidaError::capability_query_failed(format!(
                    "Failed to get BAR{} info: {e}",
                    bar as u32
                ))
            })?;

        tracing::debug!(
            "BAR{}: size={:#x}, offset={:#x}, flags={:#x}",
            bar as u32,
            region_info.size,
            region_info.offset,
            region_info.flags
        );

        let mmap =
            DeviceMmap::map_shared_rw(device_fd, region_info.offset, region_info.size as usize)
                .map_err(|e| {
                    AkidaError::capability_query_failed(format!(
                        "Failed to mmap BAR{}: {e}",
                        bar as u32
                    ))
                })?;

        tracing::info!(
            "Mapped BAR{} at {:p}, size={:#x}",
            bar as u32,
            mmap.as_ptr().as_ptr(),
            region_info.size
        );

        Ok(Self { mmap, bar })
    }

    /// Volatile MMIO view over the BAR mapping (borrows `self`).
    fn mmio(&self) -> VolatileMmio<'_> {
        self.mmap.as_volatile()
    }

    /// Read a 32-bit register
    ///
    /// # Panics
    ///
    /// Panics if `offset + 4` exceeds the mapped region size.
    #[deprecated(note = "use try_read32/try_write32 which return MmioError")]
    #[track_caller]
    pub fn read32(&self, offset: usize) -> u32 {
        self.try_read32(offset).expect("Register offset out of bounds")
    }

    /// Read a 32-bit register without panicking.
    ///
    /// # Errors
    ///
    /// Returns [`MmioAccessError`] if `offset + 4` exceeds the mapped region size.
    pub fn try_read32(&self, offset: usize) -> std::result::Result<u32, MmioError> {
        self.mmio().read_u32(offset)
    }

    /// Write a 32-bit register
    ///
    /// # Panics
    ///
    /// Panics if `offset + 4` exceeds the mapped region size.
    #[deprecated(note = "use try_read32/try_write32 which return MmioError")]
    #[track_caller]
    pub fn write32(&self, offset: usize, value: u32) {
        self.try_write32(offset, value)
            .expect("Register offset out of bounds");
    }

    /// Write a 32-bit register without panicking.
    ///
    /// # Errors
    ///
    /// Returns [`MmioAccessError`] if `offset + 4` exceeds the mapped region size.
    pub fn try_write32(&self, offset: usize, value: u32) -> std::result::Result<(), MmioError> {
        self.mmio().write_u32(offset, value)
    }

    /// Read a 64-bit register
    ///
    /// # Panics
    ///
    /// Panics if `offset + 8` exceeds the mapped region size.
    #[deprecated(note = "use try_read64/try_write64 which return MmioError")]
    #[track_caller]
    pub fn read64(&self, offset: usize) -> u64 {
        self.try_read64(offset).expect("Register offset out of bounds")
    }

    /// Read a 64-bit register without panicking.
    ///
    /// # Errors
    ///
    /// Returns [`MmioAccessError`] if `offset + 8` exceeds the mapped region size.
    pub fn try_read64(&self, offset: usize) -> std::result::Result<u64, MmioError> {
        self.mmio().read_u64(offset)
    }

    /// Write a 64-bit register
    ///
    /// # Panics
    ///
    /// Panics if `offset + 8` exceeds the mapped region size.
    #[deprecated(note = "use try_read64/try_write64 which return MmioError")]
    #[track_caller]
    pub fn write64(&self, offset: usize, value: u64) {
        self.try_write64(offset, value)
            .expect("Register offset out of bounds");
    }

    /// Write a 64-bit register without panicking.
    ///
    /// # Errors
    ///
    /// Returns [`MmioAccessError`] if `offset + 8` exceeds the mapped region size.
    pub fn try_write64(&self, offset: usize, value: u64) -> std::result::Result<(), MmioError> {
        self.mmio().write_u64(offset, value)
    }

    /// Get BAR type
    pub const fn bar(&self) -> Bar {
        self.bar
    }

    /// Get region size
    pub fn size(&self) -> usize {
        self.mmap.size()
    }
}

#[cfg(test)]
#[expect(
    clippy::assertions_on_constants,
    reason = "compile-time assertion by design"
)]
mod tests {
    use super::*;

    #[test]
    fn test_register_offsets() {
        assert_eq!(regs::DEVICE_ID, 0x0000);
        assert_eq!(regs::INFER_START, 0x0400);
        assert!(regs::status::READY != 0);
    }
}
