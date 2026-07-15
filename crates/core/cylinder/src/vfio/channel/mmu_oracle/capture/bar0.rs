// SPDX-License-Identifier: AGPL-3.0-or-later
//! BAR0 MMIO accessor for oracle page table walking (sysfs or borrowed VFIO mapping).

use std::ptr::NonNull;

use toadstool_hw_safe::{DeviceMmap, MmioError, VolatileMmio};

use crate::error::ChannelError;
use crate::mmio_region::MmioRegion;
use crate::vfio::sysfs_bar0::{self, DEFAULT_BAR0_SIZE};

use super::super::super::registers::misc;

const PRAMIN_OFFSET: usize = 0x0070_0000;

enum Bar0Backing {
    /// sysfs `resource0` mapping via [`DeviceMmap`] / [`MmioRegion`].
    Owned {
        _file: std::fs::File,
        region: MmioRegion,
    },
    /// Borrowed VFIO [`MappedBar`](crate::vfio::device::MappedBar); not unmapped on drop.
    Borrowed { ptr: NonNull<u8>, len: usize },
}

fn mmio_err_read(offset: usize, map_size: usize, e: MmioError) -> ChannelError {
    match e {
        MmioError::OutOfBounds { .. } => ChannelError::Bar0ReadOutOfBounds { offset, map_size },
        MmioError::Misaligned { address, alignment } => ChannelError::resource_io(
            "read_u32",
            format!("BAR0+{offset:#x} misaligned at {address:#x} (need {alignment})"),
            std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()),
        ),
    }
}

fn mmio_err_write(offset: usize, map_size: usize, e: MmioError) -> ChannelError {
    match e {
        MmioError::OutOfBounds { .. } => ChannelError::Bar0WriteOutOfBounds { offset, map_size },
        MmioError::Misaligned { address, alignment } => ChannelError::resource_io(
            "write_u32",
            format!("BAR0+{offset:#x} misaligned at {address:#x} (need {alignment})"),
            std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()),
        ),
    }
}

/// Read-write mmap of BAR0 for oracle page table walking.
pub(crate) struct Bar0Rw {
    backing: Bar0Backing,
}

impl Bar0Rw {
    fn map_len(&self) -> usize {
        match &self.backing {
            Bar0Backing::Owned { region, .. } => region.len(),
            Bar0Backing::Borrowed { len, .. } => *len,
        }
    }

    pub fn open(bdf: &str) -> Result<Self, ChannelError> {
        crate::vfio::ember_gate::check_channel(bdf)?;
        let path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| ChannelError::resource_io("open", path.clone(), e))?;

        let mmap = DeviceMmap::map_shared_rw(&file, 0, DEFAULT_BAR0_SIZE).map_err(|e| {
            ChannelError::Bar0Mmap {
                path: path.clone(),
                source: sysfs_bar0::device_mmap_err_to_io(e),
            }
        })?;

        Ok(Self {
            backing: Bar0Backing::Owned {
                _file: file,
                region: MmioRegion::from_device_mmap(mmap),
            },
        })
    }

    /// Wrap an existing VFIO MappedBar pointer for oracle capture.
    ///
    /// The resulting `Bar0Rw` does NOT unmap on drop — the caller owns the mapping.
    ///
    /// # Safety
    /// The caller must ensure the pointer remains valid for the lifetime of
    /// this `Bar0Rw` and the underlying mapping is at least `size` bytes.
    pub(crate) unsafe fn from_raw(ptr: *mut u8, size: usize) -> Result<Self, ChannelError> {
        let ptr = NonNull::new(ptr).ok_or(ChannelError::Bar0ExternalNull)?;
        Ok(Self {
            backing: Bar0Backing::Borrowed { ptr, len: size },
        })
    }

    pub fn read_u32(&self, offset: usize) -> u32 {
        self.try_read_u32(offset).unwrap_or(0xDEAD_DEAD)
    }

    /// Read a 32-bit MMIO register, returning an error for out-of-bounds access.
    ///
    /// Prefer this over [`read_u32`] in new code (PMU probing, etc.) where
    /// sentinel ambiguity is unacceptable.
    pub fn try_read_u32(&self, offset: usize) -> Result<u32, ChannelError> {
        let map_size = self.map_len();
        if offset.checked_add(4).is_none_or(|end| end > map_size) {
            return Err(ChannelError::Bar0ReadOutOfBounds { offset, map_size });
        }
        match &self.backing {
            Bar0Backing::Owned { region, .. } => region.read_u32(offset).map_err(|e| {
                ChannelError::resource_io(
                    "read_u32",
                    format!("BAR0+{offset:#x}"),
                    std::io::Error::other(e.to_string()),
                )
            }),
            Bar0Backing::Borrowed { ptr, len } => {
                // SAFETY: Borrowed arm guarantees ptr/len valid for lifetime of Bar0Rw (from_raw contract).
                let volatile = unsafe { VolatileMmio::new(*ptr, *len) };
                volatile
                    .read_u32(offset)
                    .map_err(|e| mmio_err_read(offset, map_size, e))
            }
        }
    }

    pub fn write_u32(&self, offset: usize, val: u32) {
        let _ = self.try_write_u32(offset, val);
    }

    /// Write a 32-bit MMIO register, returning an error for out-of-bounds access.
    pub fn try_write_u32(&self, offset: usize, val: u32) -> Result<(), ChannelError> {
        let map_size = self.map_len();
        if offset.checked_add(4).is_none_or(|end| end > map_size) {
            return Err(ChannelError::Bar0WriteOutOfBounds { offset, map_size });
        }
        match &self.backing {
            Bar0Backing::Owned { region, .. } => region.write_u32(offset, val).map_err(|e| {
                ChannelError::resource_io(
                    "write_u32",
                    format!("BAR0+{offset:#x}"),
                    std::io::Error::other(e.to_string()),
                )
            }),
            Bar0Backing::Borrowed { ptr, len } => {
                // SAFETY: Borrowed arm guarantees ptr/len valid for lifetime of Bar0Rw (from_raw contract).
                let volatile = unsafe { VolatileMmio::new(*ptr, *len) };
                volatile
                    .write_u32(offset, val)
                    .map_err(|e| mmio_err_write(offset, map_size, e))
            }
        }
    }

    fn read_pramin_u64(&self, offset_in_window: usize) -> u64 {
        let lo = self.read_u32(PRAMIN_OFFSET + offset_in_window) as u64;
        let hi = self.read_u32(PRAMIN_OFFSET + offset_in_window + 4) as u64;
        lo | (hi << 32)
    }

    fn read_pramin_u32(&self, offset_in_window: usize) -> u32 {
        self.read_u32(PRAMIN_OFFSET + offset_in_window)
    }

    pub(super) fn set_window(&self, vram_page: u64) {
        let window_val = (vram_page >> 16) as u32;
        self.write_u32(misc::BAR0_WINDOW, window_val);
        let _ = self.read_u32(misc::BAR0_WINDOW);
    }

    pub fn read_vram_u32(&self, vram_addr: u64) -> u32 {
        let page = vram_addr & !0xF_FFFF;
        let offset = (vram_addr & 0xF_FFFF) as usize;
        self.set_window(page);
        self.read_pramin_u32(offset)
    }

    pub fn read_vram_u64(&self, vram_addr: u64) -> u64 {
        let page = vram_addr & !0xF_FFFF;
        let offset = (vram_addr & 0xF_FFFF) as usize;
        self.set_window(page);
        self.read_pramin_u64(offset)
    }
}
