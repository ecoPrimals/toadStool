// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // mlock/munlock require unsafe — this is the containment zone

//! Locked (pinned) memory for DMA and security-sensitive buffers.
//!
//! [`LockedMemory`] composes [`AlignedAlloc`] with
//! `mlock`/`munlock` into a single RAII type so callers never need to write
//! unsafe mlock/munlock code themselves.
//!
//! The buffer is:
//! - Heap-allocated with caller-specified alignment (via `AlignedAlloc`)
//! - Locked into physical RAM (`mlock`) so the kernel will not page it out
//! - Automatically unlocked (`munlock`) and freed on drop
//!
//! This eliminates the duplicate mlock/munlock patterns in `nvpmu::dma` and
//! `akida-driver::backends::vfio::dma`.

use super::aligned_alloc::{AlignedAlloc, AllocError};

/// Errors specific to locked memory operations.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// The underlying allocation failed.
    #[error("allocation failed: {0}")]
    Alloc(#[from] AllocError),
    /// `mlock` syscall failed (e.g. `RLIMIT_MEMLOCK` exceeded).
    #[error("mlock failed: {0}")]
    Mlock(std::io::Error),
}

/// RAII locked memory buffer — aligned, pinned in RAM, zeroed on drop.
///
/// # Thread safety
///
/// Same as [`AlignedAlloc`]: exclusively owned, `Send + Sync`.
pub struct LockedMemory {
    inner: AlignedAlloc,
}

impl LockedMemory {
    /// Allocate `size` bytes with `align` alignment and lock them into RAM.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::Alloc`] if the allocation fails, or
    /// [`LockError::Mlock`] if locking fails (e.g. `RLIMIT_MEMLOCK`).
    pub fn new(size: usize, align: usize) -> Result<Self, LockError> {
        let inner = AlignedAlloc::new(size, align)?;

        // SAFETY: ptr is valid for `size` bytes (from AlignedAlloc). mlock is
        // a no-mutation syscall that pins existing pages. We munlock in Drop
        // with the same pointer and size.
        unsafe {
            rustix::mm::mlock(inner.as_ptr().as_ptr().cast(), inner.size())
                .map_err(|e| LockError::Mlock(e.into()))?;
        }

        Ok(Self { inner })
    }

    /// Allocate with page alignment (4096) — the common case for DMA buffers.
    ///
    /// # Errors
    ///
    /// Same as [`LockedMemory::new`].
    pub fn page_aligned(size: usize) -> Result<Self, LockError> {
        Self::new(size, 4096)
    }

    /// View the locked buffer as a byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }

    /// View the locked buffer as a mutable byte slice.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner.as_mut_slice()
    }

    /// Size of the allocation in bytes.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.inner.size()
    }

    /// Raw pointer for FFI (e.g. passing to VFIO DMA map ioctls).
    #[must_use]
    pub fn as_ptr(&self) -> std::ptr::NonNull<u8> {
        self.inner.as_ptr()
    }
}

impl Drop for LockedMemory {
    fn drop(&mut self) {
        // SAFETY: ptr and size are from the successful mlock in new().
        // Drop runs exactly once; no outstanding borrows can exist.
        unsafe {
            let _ = rustix::mm::munlock(self.inner.as_ptr().as_ptr().cast(), self.inner.size());
        }
        // AlignedAlloc's own Drop handles deallocation.
    }
}

// SAFETY: Same justification as AlignedAlloc — exclusive ownership.
unsafe impl Send for LockedMemory {}
unsafe impl Sync for LockedMemory {}

impl std::fmt::Debug for LockedMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockedMemory")
            .field("size", &self.inner.size())
            .field("align", &self.inner.align())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lock_unlock() {
        // mlock may fail in CI with low RLIMIT_MEMLOCK — that's fine,
        // we still test the allocation path.
        match LockedMemory::new(4096, 4096) {
            Ok(mem) => {
                assert_eq!(mem.size(), 4096);
                assert!(mem.as_slice().iter().all(|&b| b == 0));
            }
            Err(LockError::Mlock(_)) => {
                // Resource limit too low — skip gracefully
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn page_aligned_convenience() {
        match LockedMemory::page_aligned(8192) {
            Ok(mem) => {
                assert_eq!(mem.size(), 8192);
                assert_eq!(mem.as_ptr().as_ptr() as usize % 4096, 0);
            }
            Err(LockError::Mlock(_)) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn write_and_read() {
        match LockedMemory::new(256, 64) {
            Ok(mut mem) => {
                mem.as_mut_slice()[0] = 0xDE;
                mem.as_mut_slice()[255] = 0xAD;
                assert_eq!(mem.as_slice()[0], 0xDE);
                assert_eq!(mem.as_slice()[255], 0xAD);
            }
            Err(LockError::Mlock(_)) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    #[test]
    fn zero_size_rejected() {
        let result = LockedMemory::new(0, 4096);
        assert!(matches!(result, Err(LockError::Alloc(_))));
    }

    #[test]
    fn debug_impl() {
        match LockedMemory::new(1024, 64) {
            Ok(mem) => {
                let dbg = format!("{mem:?}");
                assert!(dbg.contains("LockedMemory"));
                assert!(dbg.contains("1024"));
            }
            Err(LockError::Mlock(_)) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}
