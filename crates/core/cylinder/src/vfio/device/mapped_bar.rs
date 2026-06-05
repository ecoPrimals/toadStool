// SPDX-License-Identifier: AGPL-3.0-or-later
//! MMIO BAR mapping for VFIO devices.

use crate::error::DriverError;
use crate::mmio_region::MmioRegion;

/// Trait for BAR0 register access.
///
/// Matches the GSP applicator's interface so firmware init sequences can
/// write BAR0 registers through a VFIO-mapped BAR. Defined locally to
/// avoid coupling to the `gsp` module (which stays in coralReef).
pub trait RegisterAccess {
    /// Read a 32-bit register at a BAR0-relative offset.
    fn read_u32(&self, offset: u32) -> Result<u32, ApplyError>;

    /// Write a 32-bit register at a BAR0-relative offset.
    fn write_u32(&mut self, offset: u32, value: u32) -> Result<(), ApplyError>;
}

/// Errors during BAR0 register application.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// BAR0 MMIO access failed.
    #[error("MMIO access failed at offset {offset:#010x}: {detail}")]
    MmioFailed {
        /// Register offset.
        offset: u32,
        /// Error detail.
        detail: String,
    },
    /// Verification read returned unexpected value.
    #[error("Verification failed at {offset:#010x}: got {actual:#010x}, expected {expected:#010x}")]
    VerifyFailed {
        /// Register offset.
        offset: u32,
        /// Value read from hardware.
        actual: u32,
        /// Expected value.
        expected: u32,
    },
    /// Thermal safety limit exceeded.
    #[error("Thermal safety: temperature {temp_c}C exceeds limit {limit_c}C")]
    ThermalLimit {
        /// Current temperature.
        temp_c: f64,
        /// Safety threshold.
        limit_c: f64,
    },
}

use std::borrow::Cow;

/// A mapped BAR region from a VFIO device.
///
/// ## Thread safety (`Send` / `Sync`)
///
/// The region wraps a `MmioRegion` whose pointer refers to a `MAP_SHARED` MMIO
/// mapping tied to the VFIO device fd lifetime. Access is performed only through
/// volatile operations (`read_u32` / `write_u32`), which are safe to use from
/// multiple threads for aligned 32-bit MMIO on supported architectures when the
/// mapping is shared read-only or callers coordinate writes. The owning struct is
/// therefore `Send` + `Sync` for the same reasons as other mmap-backed BAR
/// wrappers in this crate.
pub struct MappedBar {
    pub(crate) region: MmioRegion,
}

impl MappedBar {
    /// Read a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of range or not 4-byte aligned.
    pub fn read_u32(&self, offset: usize) -> Result<u32, DriverError> {
        if !offset.is_multiple_of(4) {
            return Err(DriverError::MmapFailed(Cow::Owned(format!(
                "BAR offset {offset:#x} is not 4-byte aligned"
            ))));
        }
        self.region.read_u32(offset)
    }

    /// Write a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of range or not 4-byte aligned.
    pub fn write_u32(&self, offset: usize, value: u32) -> Result<(), DriverError> {
        if !offset.is_multiple_of(4) {
            return Err(DriverError::MmapFailed(Cow::Owned(format!(
                "BAR offset {offset:#x} is not 4-byte aligned"
            ))));
        }
        self.region.write_u32(offset, value)
    }

    /// Size of this BAR region in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.region.len()
    }

    /// Apply a GR init sequence's BAR0 writes.
    ///
    /// Implements the `RegisterAccess` trait bridge so the GSP applicator
    /// can write directly through the VFIO-mapped BAR0.
    pub fn apply_gr_bar0_writes(&self, writes: &[(u32, u32)]) -> (usize, usize) {
        let mut applied = 0;
        let mut failed = 0;
        for &(offset, value) in writes {
            if self.write_u32(offset as usize, value).is_ok() {
                applied += 1;
            } else {
                failed += 1;
            }
        }
        (applied, failed)
    }

    /// Raw pointer to the BAR base (for callers that need ptr arithmetic).
    #[must_use]
    pub fn base_ptr(&self) -> *mut u8 {
        self.region.as_ptr()
    }

    /// Fork-isolated 32-bit MMIO read.  See [`crate::vfio::isolation`].
    pub fn isolated_read_u32(
        &self,
        offset: u32,
        timeout: std::time::Duration,
    ) -> crate::vfio::isolation::IsolationResult<u32> {
        // SAFETY: MmioRegion invariant guarantees base_ptr() is a valid BAR0 mmap.
        unsafe { crate::vfio::isolation::fork_isolated_mmio_read(self.base_ptr(), offset, timeout) }
    }

    /// Fork-isolated 32-bit MMIO write.  See [`crate::vfio::isolation`].
    pub fn isolated_write_u32(
        &self,
        offset: u32,
        value: u32,
        timeout: std::time::Duration,
    ) -> crate::vfio::isolation::IsolationResult<()> {
        // SAFETY: MmioRegion invariant guarantees base_ptr() is a valid BAR0 mmap.
        unsafe {
            crate::vfio::isolation::fork_isolated_mmio_write(
                self.base_ptr(),
                offset,
                value,
                timeout,
            )
        }
    }

    /// Fork-isolated batch of reads/writes.  See [`crate::vfio::isolation`].
    pub fn isolated_batch(
        &self,
        ops: &[(u32, Option<u32>)],
        timeout: std::time::Duration,
    ) -> crate::vfio::isolation::IsolationResult<Vec<u32>> {
        // SAFETY: MmioRegion invariant guarantees base_ptr() is a valid BAR0 mmap.
        unsafe { crate::vfio::isolation::fork_isolated_mmio_batch(self.base_ptr(), ops, timeout) }
    }
}

impl RegisterAccess for MappedBar {
    fn read_u32(&self, offset: u32) -> Result<u32, ApplyError> {
        self.read_u32(offset as usize)
            .map_err(|e| ApplyError::MmioFailed {
                offset,
                detail: e.to_string(),
            })
    }

    fn write_u32(&mut self, offset: u32, value: u32) -> Result<(), ApplyError> {
        MappedBar::write_u32(self, offset as usize, value).map_err(|e| ApplyError::MmioFailed {
            offset,
            detail: e.to_string(),
        })
    }
}

impl MappedBar {
    /// Create a `MappedBar` from a sysfs PCI BAR0 resource file (read-write).
    ///
    /// Opens `/sys/bus/pci/devices/{bdf}/resource0` with `O_RDWR` and mmaps it.
    /// The file descriptor is leaked intentionally — the mapping lives for the
    /// duration of the `MappedBar` lifetime and the kernel reclaims on drop
    /// via `MmioRegion`'s unmap.
    pub fn from_sysfs_rw(bdf: &str, size: usize) -> Result<Self, DriverError> {
        let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                DriverError::MmapFailed(Cow::Owned(format!(
                    "sysfs BAR0 open failed for {bdf}: {e}"
                )))
            })?;

        let mmap = toadstool_hw_safe::DeviceMmap::map_shared_rw(&file, 0, size).map_err(|e| {
            DriverError::MmapFailed(Cow::Owned(format!(
                "sysfs BAR0 mmap failed for {bdf}: {e}"
            )))
        })?;

        // Leak the file descriptor — the mmap keeps the mapping alive.
        std::mem::forget(file);
        Ok(Self {
            region: MmioRegion::from_device_mmap(mmap),
        })
    }
}

/// Test-only constructor backed by heap memory.
#[cfg(test)]
impl MappedBar {
    /// Create a `MappedBar` backed by heap memory for unit tests.
    pub fn from_test_heap(data: Box<[u8]>) -> Self {
        let region = MmioRegion::from_heap_slice_for_test(data);
        Self { region }
    }
}

// SAFETY: `MappedBar` wraps `MmioRegion` (`NonNull<u8>`), which is not
// `Send` alone. The VFIO BAR0 `MAP_SHARED` mapping remains valid when moved
// across threads; access is only through volatile methods on `&self`/`&mut self`.
unsafe impl Send for MappedBar {}

// SAFETY: Volatile MMIO reads/writes on `&self` do not expose aliased Rust
// `&mut` to the mapped bytes. The mapping is process-global once established;
// concurrent write ordering is enforced by callers.
unsafe impl Sync for MappedBar {}
