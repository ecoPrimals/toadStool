// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/mlock require unsafe — containment zone")]

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use toadstool_common::platform;

#[cfg(target_os = "linux")]
use crate::safe_mmap::{MmapError, SafeMmapRegion};

/// Linux memory mapper — creates [`SafeMmapRegion`] handles from file paths.
///
/// Implements [`platform::MemoryMapper`] using `rustix::mm::mmap`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxMemoryMapper;

#[cfg(target_os = "linux")]
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

        let ptr = std::ptr::NonNull::new(raw.cast()).ok_or_else(|| MmapError::NullPointer {
            path: "<anonymous>".to_string(),
        })?;

        Ok(SafeMmapRegion::from_anonymous(ptr, length))
    }
}

/// Linux memory pinner — mlock/munlock for DMA-safe page pinning.
///
/// Implements [`platform::PinnedMemory`] using `rustix::mm::mlock`/`munlock`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPinnedMemory;

#[cfg(target_os = "linux")]
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

/// Maps a VFIO device BAR region into the process address space.
///
/// Returns a raw pointer to the mapped region. Caller is responsible for
/// unmapping via [`vfio_bar_unmap`] and ensuring volatile access semantics.
///
/// # Safety
///
/// The caller must ensure:
/// - `device_fd` is a valid VFIO device file descriptor
/// - `size` and `offset` were obtained from a valid VFIO_DEVICE_GET_REGION_INFO ioctl
/// - The returned pointer is not used after [`vfio_bar_unmap`] is called
#[cfg(target_os = "linux")]
pub unsafe fn vfio_bar_map(
    device_fd: std::os::fd::BorrowedFd<'_>,
    size: usize,
    offset: u64,
) -> std::io::Result<*mut u8> {
    // SAFETY: caller guarantees fd, size, offset are valid from VFIO region info.
    let ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            size,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            device_fd,
            offset,
        )
        .map_err(std::io::Error::from)?
    };
    Ok(ptr.cast())
}

/// Unmaps a previously mapped VFIO BAR region.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` was obtained from a previous call to [`vfio_bar_map`]
/// - `size` matches the size used in the original mapping
/// - No references to the mapped memory exist after this call
#[cfg(target_os = "linux")]
pub unsafe fn vfio_bar_unmap(ptr: *mut u8, size: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr and size are from a previous vfio_bar_map.
    unsafe { rustix::mm::munmap(ptr.cast(), size).map_err(std::io::Error::from) }
}

/// Lock a raw memory region into physical RAM (prevents swap).
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` is valid for `len` bytes
/// - The memory region was properly allocated
/// - `unlock_memory` is called before freeing the allocation
#[cfg(target_os = "linux")]
pub unsafe fn lock_memory(ptr: *mut u8, len: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr is valid for len bytes.
    unsafe { rustix::mm::mlock(ptr.cast(), len).map_err(std::io::Error::from) }
}

/// Unlock a previously locked memory region, allowing it to be swapped.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` and `len` match a previous call to [`lock_memory`]
#[cfg(target_os = "linux")]
pub unsafe fn unlock_memory(ptr: *mut u8, len: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr/len match a prior lock_memory call.
    unsafe { rustix::mm::munlock(ptr.cast(), len).map_err(std::io::Error::from) }
}

/// Map a device file region into memory.
///
/// # Safety
///
/// The caller must ensure `fd` is valid and `size`/`offset` describe a valid region.
#[cfg(target_os = "linux")]
pub unsafe fn mmap_device(
    fd: std::os::fd::BorrowedFd<'_>,
    size: usize,
    offset: u64,
    writable: bool,
) -> std::io::Result<*mut u8> {
    let prot = if writable {
        rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE
    } else {
        rustix::mm::ProtFlags::READ
    };
    // SAFETY: caller guarantees fd, size, offset are valid.
    let ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            size,
            prot,
            rustix::mm::MapFlags::SHARED,
            fd,
            offset,
        )
        .map_err(std::io::Error::from)?
    };
    Ok(ptr.cast())
}

/// Unmap a previously mapped device memory region.
///
/// # Safety
///
/// The caller must ensure `ptr` and `size` match a previous `mmap_device` call.
#[cfg(target_os = "linux")]
pub unsafe fn munmap_device(ptr: *mut u8, size: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr/size from prior mmap.
    unsafe { rustix::mm::munmap(ptr.cast(), size).map_err(std::io::Error::from) }
}
