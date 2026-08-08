// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/munmap require unsafe — containment zone")]

//! RAII memory-mapped file region.
//!
//! [`SafeMmapRegion`] wraps `rustix::mm::mmap`/`munmap` for device BAR files,
//! sysfs resource files, and similar file-backed hardware mappings. The mapping
//! lifetime (munmap on drop) is managed by the RAII struct.
//!
//! This replaces the duplicate mmap patterns in:
//! - `akida-driver` `MmapRegion`
//! - `nvpmu` `Bar0Access`
//! - `display` V4L2 device mappings

use std::fs::File;
use std::os::fd::AsFd;
use std::path::Path;
use std::ptr::NonNull;

use crate::ExclusivePtr;
use crate::volatile_mmio::VolatileMmio;

/// Error type for mmap operations.
#[derive(Debug, thiserror::Error)]
pub enum MmapError {
    /// Failed to open the backing file.
    #[error("failed to open {path}: {source}")]
    Open {
        /// Path that failed to open.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// File size is zero (device not enabled or not present).
    #[error("file size is 0 for {path} (device not enabled?)")]
    ZeroSize {
        /// Path with zero size.
        path: String,
    },
    /// The mmap syscall failed.
    #[error("mmap failed for {path}: {source}")]
    MmapFailed {
        /// Path that failed to map.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// mmap returned a null pointer.
    #[error("mmap returned null for {path}")]
    NullPointer {
        /// Path with null mmap result.
        path: String,
    },
}

/// RAII memory-mapped file region.
///
/// Maps a file (typically a PCI BAR resource, sysfs attribute, or device
/// node) into the process address space. Unmaps automatically on drop.
///
/// ## Volatile MMIO
///
/// For hardware register access, use [`as_volatile`](SafeMmapRegion::as_volatile)
/// to get a [`VolatileMmio`] view with bounds-checked volatile reads and writes.
pub struct SafeMmapRegion {
    ptr: ExclusivePtr,
    size: usize,
    _file: File,
}

impl SafeMmapRegion {
    /// Map a file as a shared read-write memory region.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, has zero size, or
    /// the mmap syscall fails.
    pub fn map_shared_rw(path: &Path) -> Result<Self, MmapError> {
        let (file, size) = Self::open_validated(path, true)?;
        let path_str = path.display().to_string();

        // SAFETY: file is a valid open descriptor; size > 0 validated above;
        // PROT_READ|PROT_WRITE + MAP_SHARED are correct for device BAR files.
        let raw = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                file.as_fd(),
                0,
            )
        }
        .map_err(|e| MmapError::MmapFailed {
            path: path_str.clone(),
            source: e.into(),
        })?;

        let ptr = NonNull::new(raw.cast()).ok_or(MmapError::NullPointer { path: path_str })?;
        tracing::debug!(path = %path.display(), size, "mmap region created (rw)");
        Ok(Self {
            ptr: ExclusivePtr::new(ptr),
            size,
            _file: file,
        })
    }

    /// Map a file as a shared read-only memory region.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, has zero size, or
    /// the mmap syscall fails.
    pub fn map_shared_ro(path: &Path) -> Result<Self, MmapError> {
        let (file, size) = Self::open_validated(path, false)?;
        let path_str = path.display().to_string();

        // SAFETY: file is a valid open descriptor; size > 0 validated above;
        // PROT_READ + MAP_SHARED is correct for read-only device mappings.
        let raw = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                rustix::mm::ProtFlags::READ,
                rustix::mm::MapFlags::SHARED,
                file.as_fd(),
                0,
            )
        }
        .map_err(|e| MmapError::MmapFailed {
            path: path_str.clone(),
            source: e.into(),
        })?;

        let ptr = NonNull::new(raw.cast()).ok_or(MmapError::NullPointer { path: path_str })?;
        tracing::debug!(path = %path.display(), size, "mmap region created (ro)");
        Ok(Self {
            ptr: ExclusivePtr::new(ptr),
            size,
            _file: file,
        })
    }

    /// Create from an anonymous mapping (no backing file).
    ///
    /// Used by [`super::platform_backends::LinuxMemoryMapper::map_anonymous`].
    pub(crate) fn from_anonymous(ptr: NonNull<u8>, size: usize) -> Self {
        let file = std::fs::File::open("/dev/null").expect("/dev/null must exist on Linux");
        Self {
            ptr: ExclusivePtr::new(ptr),
            size,
            _file: file,
        }
    }

    fn open_validated(path: &Path, writable: bool) -> Result<(File, usize), MmapError> {
        let path_str = path.display().to_string();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(writable)
            .open(path)
            .map_err(|source| MmapError::Open {
                path: path_str.clone(),
                source,
            })?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "file sizes for device resources fit in usize on 64-bit"
        )]
        let size = file
            .metadata()
            .map_err(|source| MmapError::Open {
                path: path_str.clone(),
                source,
            })?
            .len() as usize;
        if size == 0 {
            return Err(MmapError::ZeroSize { path: path_str });
        }
        Ok((file, size))
    }

    /// Size of the mapped region in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get a [`VolatileMmio`] view for bounds-checked volatile register access.
    ///
    /// The returned view borrows this region — the mapping stays alive.
    #[must_use]
    pub fn as_volatile(&self) -> VolatileMmio<'_> {
        // SAFETY: ptr is valid for `size` bytes from the successful mmap.
        // The VolatileMmio borrows self, preventing use-after-unmap.
        unsafe { VolatileMmio::new(self.ptr.as_non_null(), self.size) }
    }

    /// Raw pointer to the mapped region. Use [`as_volatile`](Self::as_volatile)
    /// for safe register access instead.
    #[must_use]
    pub fn as_ptr(&self) -> NonNull<u8> {
        self.ptr.as_non_null()
    }
}

impl Drop for SafeMmapRegion {
    fn drop(&mut self) {
        // SAFETY: ptr and size from a successful mmap in constructor;
        // Drop runs exactly once; no outstanding borrows possible.
        unsafe {
            let _ = rustix::mm::munmap(self.ptr.as_ptr().cast(), self.size);
        }
    }
}

impl std::fmt::Debug for SafeMmapRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafeMmapRegion")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl toadstool_common::platform::MappedMemory for SafeMmapRegion {
    fn as_ptr(&self) -> *const u8 {
        self.ptr.as_non_null().as_ptr()
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_non_null().as_ptr()
    }

    fn len(&self) -> usize {
        self.size
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

        let region = SafeMmapRegion::map_shared_rw(tmp.path()).unwrap();
        assert_eq!(region.size(), 4096);
    }

    #[test]
    fn map_nonexistent_file() {
        let result = SafeMmapRegion::map_shared_rw(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn map_empty_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = SafeMmapRegion::map_shared_rw(tmp.path());
        assert!(matches!(result, Err(MmapError::ZeroSize { .. })));
    }

    #[test]
    fn volatile_view() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 4096]).unwrap();
        tmp.flush().unwrap();

        let region = SafeMmapRegion::map_shared_rw(tmp.path()).unwrap();
        let mmio = region.as_volatile();
        assert_eq!(mmio.size(), 4096);

        mmio.write_u32(0, 0xDEAD_BEEF).unwrap();
        assert_eq!(mmio.read_u32(0).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn volatile_bounds_check() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 8]).unwrap();
        tmp.flush().unwrap();

        let region = SafeMmapRegion::map_shared_rw(tmp.path()).unwrap();
        let mmio = region.as_volatile();

        assert!(mmio.read_u32(0).is_ok());
        assert!(mmio.read_u32(4).is_ok());
        assert!(mmio.read_u32(8).is_err());
    }

    #[test]
    fn map_shared_ro_works() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&[0u8; 4096]).unwrap();
        tmp.flush().unwrap();

        let region = SafeMmapRegion::map_shared_ro(tmp.path()).unwrap();
        assert_eq!(region.size(), 4096);
    }
}
