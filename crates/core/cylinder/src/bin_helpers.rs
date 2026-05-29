// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)]
//! Shared BAR0 mmap helper for diagnostic/boot binaries.
//!
//! Deduplicates the `Bar0` struct that was previously copy-pasted across
//! `sovereign_acr_boot`, `sovereign_pmu_boot`, and `capture_pmu_falcon`.

use std::io;

/// RAII BAR0 mmap handle with volatile register access.
///
/// Maps a PCI resource0 file (or any suitable fd) into the process address
/// space via `rustix::mm::mmap`. Automatically unmaps on drop.
pub struct Bar0 {
    ptr: *mut u32,
    len: usize,
}

// SAFETY: Bar0 is a raw pointer wrapper for memory-mapped I/O. The mapped
// region is process-local and the pointer is not shared across threads.
// The bins that use this are single-threaded CLI tools.
unsafe impl Send for Bar0 {}

impl Bar0 {
    /// Map `size` bytes of a PCI BAR via the given fd (read-write).
    ///
    /// # Safety
    /// Caller must ensure `fd` refers to a valid PCI resource file and that
    /// `size` does not exceed the BAR region.
    pub unsafe fn map(fd: std::os::fd::BorrowedFd, size: usize) -> io::Result<Self> {
        let ptr = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                fd,
                0,
            )
        }
        .map_err(|e| io::Error::from_raw_os_error(e.raw_os_error()))?;
        Ok(Self {
            ptr: ptr.cast(),
            len: size,
        })
    }

    /// Map `size` bytes with configurable write permission.
    ///
    /// # Safety
    /// Same requirements as [`Bar0::map`].
    pub unsafe fn map_with_prot(
        fd: std::os::fd::BorrowedFd,
        size: usize,
        write: bool,
    ) -> io::Result<Self> {
        let prot = if write {
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE
        } else {
            rustix::mm::ProtFlags::READ
        };
        let ptr = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                prot,
                rustix::mm::MapFlags::SHARED,
                fd,
                0,
            )
        }
        .map_err(|e| io::Error::from_raw_os_error(e.raw_os_error()))?;
        Ok(Self {
            ptr: ptr.cast(),
            len: size,
        })
    }

    /// Volatile 32-bit read at `offset` (byte offset, must be 4-byte aligned).
    pub fn r32(&self, offset: u32) -> u32 {
        unsafe { std::ptr::read_volatile(self.ptr.add(offset as usize / 4)) }
    }

    /// Volatile 32-bit write at `offset` (byte offset, must be 4-byte aligned).
    pub fn w32(&self, offset: u32, val: u32) {
        unsafe { std::ptr::write_volatile(self.ptr.add(offset as usize / 4), val) }
    }
}

impl Drop for Bar0 {
    fn drop(&mut self) {
        unsafe {
            let _ = rustix::mm::munmap(self.ptr.cast(), self.len);
        }
    }
}
