// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared BAR0 mmap helper for diagnostic/boot binaries.
//!
//! Deduplicates the `Bar0` struct that was previously copy-pasted across
//! `sovereign_acr_boot`, `sovereign_pmu_boot`, and `capture_pmu_falcon`.

use std::io;

use toadstool_hw_safe::DeviceMmap;

/// RAII BAR0 mmap handle with volatile register access.
///
/// Maps a PCI resource0 file (or any suitable fd) into the process address
/// space via [`DeviceMmap`]. Automatically unmaps on drop.
pub struct Bar0 {
    mmap: DeviceMmap,
}

impl Bar0 {
    /// Map `size` bytes of a PCI BAR via the given fd (read-write).
    ///
    /// # Safety
    ///
    /// Caller must ensure `fd` refers to a valid PCI resource file, that
    /// `size` does not exceed the BAR region, and that the fd outlives this mapping.
    pub unsafe fn map(fd: std::os::fd::BorrowedFd<'_>, size: usize) -> io::Result<Self> {
        let mmap =
            DeviceMmap::map_shared_rw(fd, 0, size).map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self { mmap })
    }

    /// Map `size` bytes with configurable write permission.
    ///
    /// # Safety
    ///
    /// Same requirements as [`Bar0::map`].
    pub unsafe fn map_with_prot(
        fd: std::os::fd::BorrowedFd<'_>,
        size: usize,
        write: bool,
    ) -> io::Result<Self> {
        let mmap = if write {
            DeviceMmap::map_shared_rw(fd, 0, size)
        } else {
            DeviceMmap::map_shared_ro(fd, 0, size)
        }
        .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self { mmap })
    }

    /// Volatile 32-bit read at `offset` (byte offset, must be 4-byte aligned).
    pub fn r32(&self, offset: u32) -> u32 {
        self.mmap
            .as_volatile()
            .read_u32(offset as usize)
            .unwrap_or(0)
    }

    /// Volatile 32-bit write at `offset` (byte offset, must be 4-byte aligned).
    pub fn w32(&self, offset: u32, val: u32) {
        let _ = self.mmap.as_volatile().write_u32(offset as usize, val);
    }
}
