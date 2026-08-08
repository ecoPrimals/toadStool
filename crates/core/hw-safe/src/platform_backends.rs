// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/mlock require unsafe — containment zone")]

//! [`toadstool_common::platform`] trait implementations for Linux.
//!
//! Provides concrete `MemoryMapper` and `PinnedMemory` implementations using
//! `rustix` syscalls. These are the "L3 backend" for the G68 platform traits.

use std::path::Path;

use toadstool_common::platform;

use crate::safe_mmap::{MmapError, SafeMmapRegion};

/// Linux memory mapper — creates [`SafeMmapRegion`] handles from file paths.
///
/// Implements [`platform::MemoryMapper`] using `rustix::mm::mmap`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxMemoryMapper;

impl platform::MemoryMapper for LinuxMemoryMapper {
    type Mapping = SafeMmapRegion;
    type Error = MmapError;

    fn map_file(
        &self,
        path: &Path,
        _offset: u64,
        _length: usize,
        writable: bool,
    ) -> Result<Self::Mapping, Self::Error> {
        if writable {
            SafeMmapRegion::map_shared_rw(path)
        } else {
            SafeMmapRegion::map_shared_ro(path)
        }
    }

    fn map_anonymous(&self, length: usize) -> Result<Self::Mapping, Self::Error> {
        use std::ptr;

        if length == 0 {
            return Err(MmapError::ZeroSize {
                path: "<anonymous>".to_string(),
            });
        }

        // SAFETY: null hint, valid length, anonymous mapping (no fd).
        let raw = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                length,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
            )
        }
        .map_err(|e| MmapError::MmapFailed {
            path: "<anonymous>".to_string(),
            source: e.into(),
        })?;

        let ptr = std::ptr::NonNull::new(raw.cast()).ok_or(MmapError::NullPointer {
            path: "<anonymous>".to_string(),
        })?;

        Ok(SafeMmapRegion::from_anonymous(ptr, length))
    }
}

/// Linux memory pinner — mlock/munlock for DMA-safe page pinning.
///
/// Implements [`platform::PinnedMemory`] using `rustix::mm::mlock`/`munlock`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPinnedMemory;

impl platform::PinnedMemory for LinuxPinnedMemory {
    type Error = std::io::Error;

    fn pin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice, so ptr + len describe a valid memory range.
        // mlock requires *mut but only reads the pages (no modification).
        unsafe {
            rustix::mm::mlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }

    fn unpin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice from a previously locked mapping.
        unsafe {
            rustix::mm::munlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }
}
