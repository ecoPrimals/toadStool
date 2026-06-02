// SPDX-License-Identifier: AGPL-3.0-or-later
//! Safe wrapper for sysfs BAR0 mmap reads (and optional writes).
//!
//! Consolidates the mmap → volatile-read → munmap pattern used by
//! multiple oracle modules into a single safe API with bounds checking.

use crate::error::ChannelError;
use crate::mmio_region::MmioRegion;
use toadstool_hw_safe::DeviceMmap;

/// Map [`DeviceMmapError`] to a `rustix` errno for [`ChannelError::Bar0Mmap`].
pub(crate) fn device_mmap_err_to_errno(
    e: toadstool_hw_safe::device_mmap::DeviceMmapError,
) -> rustix::io::Errno {
    match e {
        toadstool_hw_safe::device_mmap::DeviceMmapError::ZeroSize => rustix::io::Errno::INVAL,
        toadstool_hw_safe::device_mmap::DeviceMmapError::MmapFailed(io) => io
            .raw_os_error()
            .map(rustix::io::Errno::from_raw_os_error)
            .unwrap_or(rustix::io::Errno::IO),
        toadstool_hw_safe::device_mmap::DeviceMmapError::NullPointer => rustix::io::Errno::NOMEM,
    }
}

/// Read-only mmap of a PCI BAR0 resource via sysfs.
///
/// Provides safe, bounds-checked volatile reads for register probing.
/// The mapping is automatically unmapped on drop.
///
/// ## Thread safety (`Send` / `Sync`)
///
/// The [`std::fs::File`] keeps the sysfs mapping alive; [`MmioRegion`]
/// holds the hw-safe mapping and length. Read-only volatile `u32` loads are safe to
/// share across threads for aligned MMIO access on the supported platforms, in line
/// with other BAR0 readers in this crate.
pub struct SysfsBar0 {
    _file: std::fs::File,
    region: MmioRegion,
}

/// 16 MiB — standard BAR0 size for NVIDIA Volta-class GPUs.
pub const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

// SAFETY: Matches the `Send` / `Sync` rationale in the [`SysfsBar0`] docs.
unsafe impl Send for SysfsBar0 {}

// SAFETY: Matches the `Send` / `Sync` rationale in the [`SysfsBar0`] docs.
unsafe impl Sync for SysfsBar0 {}

impl SysfsBar0 {
    /// Open and mmap a PCI device's BAR0 via sysfs `resource0`.
    ///
    /// # Errors
    ///
    /// Returns an error if the sysfs path cannot be opened or mmap fails.
    pub fn open(bdf: &str, size: usize) -> Result<Self, ChannelError> {
        crate::vfio::ember_gate::check_channel(bdf)?;
        let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|e| ChannelError::resource_io("open", path.clone(), e))?;

        let mmap = DeviceMmap::map_shared_ro(&file, 0, size).map_err(|e| {
            ChannelError::Bar0Mmap {
                path: path.clone(),
                source: device_mmap_err_to_errno(e),
            }
        })?;

        Ok(Self {
            _file: file,
            region: MmioRegion::from_device_mmap(mmap),
        })
    }

    /// Read a 32-bit register at the given byte offset.
    ///
    /// Returns `0` if the offset is out of bounds.
    #[must_use]
    pub fn read_u32(&self, offset: usize) -> u32 {
        if offset
            .checked_add(4)
            .is_none_or(|end| end > self.region.len())
        {
            return 0;
        }
        self.region.read_u32(offset).unwrap_or_default()
    }

    /// The size of the mapped BAR0 region in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.region.len()
    }
}

/// Read-write mmap of a PCI BAR0 resource via sysfs.
///
/// Like [`SysfsBar0`] but opened with `O_RDWR` and `PROT_READ | PROT_WRITE`
/// for register writes (e.g. MMIO RPC handlers).
pub struct SysfsBar0Rw {
    _file: std::fs::File,
    region: MmioRegion,
}

// SAFETY: Same rationale as `SysfsBar0` — volatile MMIO at aligned offsets.
unsafe impl Send for SysfsBar0Rw {}
// SAFETY: Same rationale as `SysfsBar0`.
unsafe impl Sync for SysfsBar0Rw {}

impl SysfsBar0Rw {
    /// Open and mmap a PCI device's BAR0 via sysfs `resource0` with read-write access.
    ///
    /// # Errors
    ///
    /// Returns an error if the sysfs path cannot be opened or mmap fails.
    pub fn open(bdf: &str, size: usize) -> Result<Self, ChannelError> {
        crate::vfio::ember_gate::check_channel(bdf)?;
        let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| ChannelError::resource_io("open (rw)", path.clone(), e))?;

        let mmap = DeviceMmap::map_shared_rw(&file, 0, size).map_err(|e| {
            ChannelError::Bar0Mmap {
                path: path.clone(),
                source: device_mmap_err_to_errno(e),
            }
        })?;

        Ok(Self {
            _file: file,
            region: MmioRegion::from_device_mmap(mmap),
        })
    }

    /// Read a 32-bit register at the given byte offset.
    #[must_use]
    pub fn read_u32(&self, offset: usize) -> u32 {
        if offset
            .checked_add(4)
            .is_none_or(|end| end > self.region.len())
        {
            return 0;
        }
        self.region.read_u32(offset).unwrap_or_default()
    }

    /// Write a 32-bit register at the given byte offset.
    ///
    /// # Errors
    ///
    /// Returns error if offset is out of range.
    pub fn write_u32(&self, offset: usize, value: u32) -> Result<(), ChannelError> {
        if offset
            .checked_add(4)
            .is_none_or(|end| end > self.region.len())
        {
            return Err(ChannelError::Bar0WriteOutOfBounds {
                offset,
                map_size: self.region.len(),
            });
        }
        self.region.write_u32(offset, value).map_err(|e| {
            ChannelError::resource_io(
                "write_u32",
                format!("BAR0+{offset:#x}"),
                std::io::Error::other(e.to_string()),
            )
        })
    }

    /// The size of the mapped BAR0 region in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.region.len()
    }
}

/// Best-effort read-only BAR0 probe without ember gate (crash forensics, watchdog).
///
/// Opens sysfs `resource0`, maps `map_size` bytes read-only, and reads one
/// register. Returns `None` on any failure.
#[must_use]
pub fn read_u32_best_effort(bdf: &str, offset: usize, map_size: usize) -> Option<u32> {
    let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let file = std::fs::OpenOptions::new().read(true).open(&path).ok()?;
    let mmap = DeviceMmap::map_shared_ro(&file, 0, map_size).ok()?;
    mmap.as_volatile().read_u32(offset).ok()
}

/// Best-effort multi-register BAR0 snapshot without ember gate.
///
/// Returns `None` if open/mmap fails. Individual out-of-bounds reads are skipped.
#[must_use]
pub fn read_registers_best_effort(
    bdf: &str,
    map_size: usize,
    offsets: &[(&'static str, usize)],
) -> Option<Vec<(&'static str, u32)>> {
    let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let file = std::fs::OpenOptions::new().read(true).open(&path).ok()?;
    let mmap = DeviceMmap::map_shared_ro(&file, 0, map_size).ok()?;
    let mmio = mmap.as_volatile();
    let mut regs = Vec::with_capacity(offsets.len());
    for &(name, offset) in offsets {
        if let Ok(val) = mmio.read_u32(offset) {
            regs.push((name, val));
        }
    }
    Some(regs)
}
