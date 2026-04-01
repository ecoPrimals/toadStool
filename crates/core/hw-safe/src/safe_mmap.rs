// SPDX-License-Identifier: AGPL-3.0-only

//! RAII memory-mapped file region.
//!
//! [`SafeMmapRegion`] wraps `mmap`/`munmap` for device BAR files, sysfs
//! resource files, and similar file-backed hardware mappings. It owns the
//! mapping lifetime and unmaps on drop.
//!
//! This replaces the duplicate mmap patterns in:
//! - `akida-driver` `MmapRegion`
//! - `nvpmu` `Bar0Access`
//! - `display` V4L2 device mappings

use std::fs::File;
use std::os::unix::io::AsFd;
use std::path::Path;
use std::ptr::NonNull;

use rustix::mm::{MapFlags, ProtFlags};

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
        /// Underlying errno.
        source: rustix::io::Errno,
    },
    /// The mmap syscall returned a null pointer.
    #[error("mmap returned null for {path}")]
    NullPointer {
        /// Path that returned null.
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
#[derive(Debug)]
pub struct SafeMmapRegion {
    ptr: NonNull<u8>,
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
        Self::map_with_flags(path, ProtFlags::READ | ProtFlags::WRITE, MapFlags::SHARED)
    }

    /// Map a file as a shared read-only memory region.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, has zero size, or
    /// the mmap syscall fails.
    pub fn map_shared_ro(path: &Path) -> Result<Self, MmapError> {
        Self::map_with_flags(path, ProtFlags::READ, MapFlags::SHARED)
    }

    /// Map a file with custom protection and mapping flags.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, has zero size, or
    /// the mmap syscall fails.
    pub fn map_with_flags(
        path: &Path,
        prot: ProtFlags,
        flags: MapFlags,
    ) -> Result<Self, MmapError> {
        let path_str = path.display().to_string();

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(prot.contains(ProtFlags::WRITE))
            .open(path)
            .map_err(|source| MmapError::Open {
                path: path_str.clone(),
                source,
            })?;

        #[allow(
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
            return Err(MmapError::ZeroSize {
                path: path_str,
            });
        }

        Self::map_file(file, size, prot, flags, &path_str)
    }

    /// Map an already-opened file with known size.
    ///
    /// # Errors
    ///
    /// Returns an error if the mmap syscall fails or returns null.
    pub fn map_file(
        file: File,
        size: usize,
        prot: ProtFlags,
        flags: MapFlags,
        label: &str,
    ) -> Result<Self, MmapError> {
        // SAFETY: Invariants for mmap:
        // - `file` is a valid, open fd (just opened or passed by caller)
        // - `size` > 0 (caller responsibility for this entry point)
        // - `prot` and `flags` are valid rustix enum values
        // - offset 0 is within the file
        // - The returned pointer is valid for `size` bytes until munmap
        // - File is kept alive in the struct; munmap in Drop
        let ptr = unsafe {
            let addr = rustix::mm::mmap(
                std::ptr::null_mut(),
                size,
                prot,
                flags,
                file.as_fd(),
                0,
            )
            .map_err(|source| MmapError::MmapFailed {
                path: label.to_string(),
                source,
            })?;

            NonNull::new(addr.cast::<u8>()).ok_or_else(|| MmapError::NullPointer {
                path: label.to_string(),
            })?
        };

        tracing::debug!(label, size, "mmap region created");

        Ok(Self {
            ptr,
            size,
            _file: file,
        })
    }

    /// Size of the mapped region in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Get a [`VolatileMmio`] view for bounds-checked volatile register access.
    ///
    /// The returned view borrows this region — the mapping stays alive.
    #[must_use]
    pub fn as_volatile(&self) -> VolatileMmio<'_> {
        // SAFETY: ptr is valid for `size` bytes (from mmap), mapping is alive
        // (self borrows the File and pointer). The VolatileMmio borrows self,
        // preventing use-after-unmap.
        unsafe { VolatileMmio::new(self.ptr, self.size) }
    }

    /// Raw pointer to the mapped region. Use [`as_volatile`](Self::as_volatile)
    /// for safe register access instead.
    #[must_use]
    pub const fn as_ptr(&self) -> NonNull<u8> {
        self.ptr
    }
}

impl Drop for SafeMmapRegion {
    fn drop(&mut self) {
        // SAFETY: ptr and size are from a successful mmap call in the
        // constructor. Drop runs exactly once. No outstanding VolatileMmio
        // borrows can exist (they borrow &self, which is being dropped).
        unsafe {
            if let Err(e) = rustix::mm::munmap(self.ptr.as_ptr().cast(), self.size) {
                tracing::error!("munmap failed: {e}");
            }
        }
    }
}

// SAFETY: SafeMmapRegion owns the mapped memory exclusively. The memory
// mapping is process-private. Moving between threads is safe because the
// mapping and file descriptor remain valid regardless of which thread
// accesses them. Writes go through VolatileMmio which requires &self
// (hardware registers tolerate concurrent volatile writes).
unsafe impl Send for SafeMmapRegion {}
unsafe impl Sync for SafeMmapRegion {}

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
        assert!(mmio.read_u32(8).is_err()); // out of bounds
    }
}
