// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // mmap/munmap/mlock/munlock for huge pages — containment zone

//! Locked huge-page memory for high-performance DMA.
//!
//! [`HugePageMemory`] allocates memory via `mmap_anonymous` with `MAP_HUGETLB`,
//! locks it into RAM via `mlock`, and cleans up on drop. This removes the need
//! for consumers to write inline `mmap_anonymous`/`mlock`/`munmap` unsafe blocks.

use std::ptr::NonNull;

use rustix::mm::{MapFlags, ProtFlags, mlock, mmap_anonymous, munlock, munmap};

use crate::ExclusivePtr;
use crate::contiguous::ContiguousBytes;

/// Supported huge page sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageSize {
    /// 2 MiB huge pages (log2 = 21).
    Huge2M,
    /// 1 GiB huge pages (log2 = 30).
    Huge1G,
}

impl HugePageSize {
    /// Page size in bytes.
    #[must_use]
    pub const fn bytes(self) -> usize {
        match self {
            Self::Huge2M => 2 * 1024 * 1024,
            Self::Huge1G => 1024 * 1024 * 1024,
        }
    }

    fn log2(self) -> u32 {
        match self {
            Self::Huge2M => 21,
            Self::Huge1G => 30,
        }
    }
}

/// Errors from huge-page allocation.
#[derive(Debug, thiserror::Error)]
pub enum HugePageError {
    /// Zero-size allocation requested.
    #[error("cannot allocate 0 bytes")]
    ZeroSize,
    /// `MAP_HUGETLB` flags not supported on this platform.
    #[error("MAP_HUGETLB unsupported for {0:?}")]
    Unsupported(HugePageSize),
    /// `mmap_anonymous` failed.
    #[error("huge page mmap failed: {0}")]
    MmapFailed(std::io::Error),
    /// `mlock` failed (e.g. `RLIMIT_MEMLOCK` exceeded).
    #[error("mlock failed: {0}")]
    MlockFailed(std::io::Error),
}

/// RAII huge-page memory — `mmap_anonymous` + `MAP_HUGETLB` + `mlock`.
///
/// On drop: `munlock` then `munmap`. All unsafe is contained here.
pub struct HugePageMemory {
    ptr: ExclusivePtr,
    size: usize,
}

impl HugePageMemory {
    /// Allocate `size` bytes using huge pages and lock into RAM.
    ///
    /// `size` is rounded up to the next multiple of the huge page boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `size` is 0
    /// - `MAP_HUGETLB` flags are unavailable
    /// - `mmap_anonymous` or `mlock` fails
    pub fn new(size: usize, page_size: HugePageSize) -> Result<Self, HugePageError> {
        if size == 0 {
            return Err(HugePageError::ZeroSize);
        }

        let page_bytes = page_size.bytes();
        let aligned_size = size.div_ceil(page_bytes) * page_bytes;

        let map_flags = MapFlags::hugetlb_with_size_log2(page_size.log2())
            .ok_or(HugePageError::Unsupported(page_size))?;

        // SAFETY: rustix exposes `mmap_anonymous` as `unsafe fn` (like libc): the caller must
        // prove `addr`/`length` are valid for mapping. We pass `null` and `aligned_size` from
        // this function; the kernel returns the only pointer we treat as valid.
        let raw = unsafe {
            mmap_anonymous(
                std::ptr::null_mut(),
                aligned_size,
                ProtFlags::READ | ProtFlags::WRITE,
                map_flags,
            )
        }
        .map_err(|e| HugePageError::MmapFailed(e.into()))?;

        let ptr = NonNull::new(raw.cast::<u8>()).expect("mmap_anonymous returned null after Ok");

        // SAFETY: `ptr`/`aligned_size` describe the mapping returned above; rustix `mlock` requires
        // the range to be valid to read (it is).
        if let Err(e) = unsafe { mlock(ptr.as_ptr().cast(), aligned_size) } {
            // Cleanup: munmap the region we just mapped.
            // SAFETY: same mapping as the successful `mmap_anonymous`.
            unsafe {
                let _ = munmap(ptr.as_ptr().cast(), aligned_size);
            }
            return Err(HugePageError::MlockFailed(e.into()));
        }

        Ok(Self {
            ptr: ExclusivePtr::new(ptr),
            size: aligned_size,
        })
    }

    /// Allocation size in bytes (rounded up to page boundary).
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Raw pointer for FFI (e.g. VFIO DMA map).
    #[must_use]
    pub fn as_ptr(&self) -> NonNull<u8> {
        self.ptr.as_non_null()
    }

    /// Immutable byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.as_bytes()
    }

    /// Mutable byte slice.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_bytes_mut()
    }
}

impl Drop for HugePageMemory {
    fn drop(&mut self) {
        // SAFETY: `ptr`/`size` match the still-mapped region from `new()`; rustix `munlock`/`munmap`
        // require the same provenance as the original mapping syscalls.
        unsafe {
            let _ = munlock(self.ptr.as_ptr().cast(), self.size);
            let _ = munmap(self.ptr.as_ptr().cast(), self.size);
        }
    }
}

// Send+Sync auto-derived via ExclusivePtr — no manual unsafe impl needed.
#[expect(dead_code, reason = "compile-time trait bound assertion")]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<HugePageMemory>();
    }
};

// SAFETY: HugePageMemory owns the mmap'd region exclusively; ptr is valid
// for `size` bytes from construction until Drop munmaps it.
unsafe impl ContiguousBytes for HugePageMemory {
    fn raw_ptr(&self) -> NonNull<u8> {
        self.ptr.as_non_null()
    }
    fn raw_len(&self) -> usize {
        self.size
    }
}

impl std::fmt::Debug for HugePageMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HugePageMemory")
            .field("size", &self.size)
            .field("ptr", &self.ptr.as_non_null())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_size_rejected() {
        let result = HugePageMemory::new(0, HugePageSize::Huge2M);
        assert!(matches!(result, Err(HugePageError::ZeroSize)));
    }

    #[test]
    fn page_size_constants() {
        assert_eq!(HugePageSize::Huge2M.bytes(), 2 * 1024 * 1024);
        assert_eq!(HugePageSize::Huge1G.bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn alignment_rounding() {
        let page = HugePageSize::Huge2M.bytes();
        assert_eq!(1usize.div_ceil(page) * page, page);
        assert_eq!((page + 1).div_ceil(page) * page, 2 * page);
        assert_eq!(page.div_ceil(page) * page, page);
    }

    #[test]
    fn alloc_may_fail_without_huge_pages() {
        match HugePageMemory::new(HugePageSize::Huge2M.bytes(), HugePageSize::Huge2M) {
            Ok(mem) => {
                assert_eq!(mem.size(), HugePageSize::Huge2M.bytes());
                assert!(mem.as_slice().iter().all(|&b| b == 0));
            }
            Err(HugePageError::MmapFailed(_) | HugePageError::MlockFailed(_)) => {
                // No huge pages configured — expected in CI
            }
            Err(e) => unreachable!("unexpected error: {e}"),
        }
    }
}
