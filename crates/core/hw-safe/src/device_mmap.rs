// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // Device mmap/munmap require unsafe — this is the containment zone

//! RAII device memory mapping for file-descriptor-based hardware regions.
//!
//! [`DeviceMmap`] wraps `rustix::mm::mmap`/`munmap` for device BAR files,
//! V4L2 video buffers, and similar fd-based hardware mappings where the
//! mapping offset comes from a kernel ioctl rather than a file-system path.
//!
//! This replaces the duplicate mmap/munmap + `unsafe impl Send` patterns in:
//! - `nvpmu::vfio` BAR0 mapping
//! - `akida-driver::mmio` BAR mapping
//! - `display::v4l2` capture buffer mapping

use std::os::fd::{AsFd, AsRawFd};
use std::ptr::NonNull;

use crate::ExclusivePtr;
use crate::contiguous::ContiguousBytes;
use crate::volatile_mmio::VolatileMmio;

/// Error type for device mmap operations.
#[derive(Debug, thiserror::Error)]
pub enum DeviceMmapError {
    /// The requested size is zero.
    #[error("cannot mmap 0 bytes")]
    ZeroSize,
    /// The mmap syscall failed.
    #[error("device mmap failed: {0}")]
    MmapFailed(std::io::Error),
}

/// RAII memory-mapped device region.
///
/// Maps a device file descriptor at a kernel-supplied offset with
/// `PROT_READ | PROT_WRITE` and `MAP_SHARED`. Unmaps automatically on drop.
///
/// ## Volatile MMIO (hardware registers)
///
/// Use [`as_volatile`](Self::as_volatile) for bounds-checked volatile access.
///
/// ## Data buffers (V4L2 frames, DMA regions)
///
/// Use [`as_slice`](Self::as_slice) / [`as_mut_slice`](Self::as_mut_slice).
pub struct DeviceMmap {
    ptr: ExclusivePtr,
    size: usize,
}

impl DeviceMmap {
    /// Map a device fd at `offset` as a shared read-write region of `size` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DeviceMmapError::ZeroSize`] if `size == 0`, or
    /// [`DeviceMmapError::MmapFailed`] if the kernel rejects the mapping.
    pub fn map_shared_rw(fd: impl AsFd, offset: u64, size: usize) -> Result<Self, DeviceMmapError> {
        if size == 0 {
            return Err(DeviceMmapError::ZeroSize);
        }

        if isize::try_from(size).is_err() {
            return Err(DeviceMmapError::MmapFailed(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "mmap length exceeds isize::MAX",
            )));
        }

        #[cfg(debug_assertions)]
        {
            let raw = fd.as_fd().as_raw_fd();
            debug_assert!(raw >= 0, "AsFd: negative fd");
        }

        // SAFETY: fd is a valid open descriptor (AsFd contract); offset and
        // size describe a mappable region of the device (caller invariant);
        // PROT_READ|PROT_WRITE + MAP_SHARED are correct for device MMIO and
        // V4L2 buffers.
        let raw = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                fd,
                offset,
            )
        }
        .map_err(|e| DeviceMmapError::MmapFailed(e.into()))?;

        let ptr = NonNull::new(raw.cast()).expect("mmap returned non-null on success");
        tracing::debug!(size, offset, "device mmap region created");
        Ok(Self {
            ptr: ExclusivePtr::new(ptr),
            size,
        })
    }

    /// Size of the mapped region in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Raw pointer to the mapped region.
    #[must_use]
    pub fn as_ptr(&self) -> NonNull<u8> {
        self.ptr.as_non_null()
    }

    /// Bounds-checked volatile MMIO view (borrows `self`).
    ///
    /// Use this for PCI BAR register access. The returned [`VolatileMmio`]
    /// prevents use-after-unmap because it borrows `self`.
    #[must_use]
    pub fn as_volatile(&self) -> VolatileMmio<'_> {
        // SAFETY: ptr is valid for `size` bytes from the successful mmap.
        // The VolatileMmio borrows self, preventing use-after-unmap.
        unsafe { VolatileMmio::new(self.ptr.as_non_null(), self.size) }
    }

    /// View the mapped region as a byte slice.
    ///
    /// For non-MMIO mappings (V4L2 video frames, data buffers).
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.as_bytes()
    }

    /// View the mapped region as a mutable byte slice.
    ///
    /// For non-MMIO mappings (V4L2 video frames, data buffers).
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_bytes_mut()
    }
}

impl Drop for DeviceMmap {
    fn drop(&mut self) {
        // SAFETY: ptr and size from successful mmap in constructor;
        // Drop runs exactly once; no outstanding slice borrows possible.
        unsafe {
            let _ = rustix::mm::munmap(self.ptr.as_ptr().cast(), self.size);
        }
    }
}

// Send+Sync auto-derived via ExclusivePtr — no manual unsafe impl needed.
#[expect(dead_code, reason = "compile-time trait bound assertion")]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<DeviceMmap>();
    }
};

// SAFETY: DeviceMmap owns the mmap'd region exclusively; ptr is valid for
// `size` bytes from construction until Drop munmaps it.
unsafe impl ContiguousBytes for DeviceMmap {
    fn raw_ptr(&self) -> NonNull<u8> {
        self.ptr.as_non_null()
    }
    fn raw_len(&self) -> usize {
        debug_assert!(self.size > 0, "zero-size maps are rejected in constructor");
        debug_assert!(
            isize::try_from(self.size).is_ok(),
            "ContiguousBytes: raw_len must fit isize (slice precondition)"
        );
        self.size
    }
}

impl std::fmt::Debug for DeviceMmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceMmap")
            .field(
                "ptr",
                &format_args!("{:p}", self.ptr.as_non_null().as_ptr()),
            )
            .field("size", &self.size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn map_real_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 4096]).unwrap();
        tmp.flush().unwrap();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.path())
            .unwrap();
        let region = DeviceMmap::map_shared_rw(&file, 0, 4096).unwrap();
        assert_eq!(region.size(), 4096);
    }

    #[test]
    fn zero_size_rejected() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = DeviceMmap::map_shared_rw(tmp.as_file(), 0, 0);
        assert!(matches!(result, Err(DeviceMmapError::ZeroSize)));
    }

    #[test]
    fn volatile_view() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 4096]).unwrap();
        tmp.flush().unwrap();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.path())
            .unwrap();
        let region = DeviceMmap::map_shared_rw(&file, 0, 4096).unwrap();
        let mmio = region.as_volatile();
        mmio.write_u32(0, 0xDEAD_BEEF).unwrap();
        assert_eq!(mmio.read_u32(0).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn slice_view() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[42u8; 256]).unwrap();
        tmp.flush().unwrap();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.path())
            .unwrap();
        let region = DeviceMmap::map_shared_rw(&file, 0, 256).unwrap();
        assert_eq!(region.as_slice().len(), 256);
        assert_eq!(region.as_slice()[0], 42);
    }

    #[test]
    fn debug_impl() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 4096]).unwrap();
        tmp.flush().unwrap();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(tmp.path())
            .unwrap();
        let region = DeviceMmap::map_shared_rw(&file, 0, 4096).unwrap();
        let dbg = format!("{region:?}");
        assert!(dbg.contains("DeviceMmap"));
        assert!(dbg.contains("4096"));
    }
}
