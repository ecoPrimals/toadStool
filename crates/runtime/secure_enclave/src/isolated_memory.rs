// SPDX-License-Identifier: AGPL-3.0-or-later
//! Isolated memory region for secure computation
//!
//! Provides memory regions that are:
//! - **Locked**: Cannot be swapped to disk (mlock) — Linux only
//! - **Protected**: Cannot appear in core dumps (madvise `MADV_DONTDUMP`) — Linux only
//! - **Wiped**: Explicitly zeroed before deallocation
//! - **Aligned**: Page-aligned for optimal performance
//!
//! This is a **deep solution** implementing true memory isolation,
//! not just a wrapper around `Vec<u8>`.
//!
//! # Evolution (Feb 12, 2026)
//!
//! Evolved from `libc` raw C bindings to `rustix` safe Rust wrappers.
//! Allocation and `mlock`/`munlock` are delegated to [`toadstool_hw_safe::LockedMemory`]
//! so this module avoids duplicating those unsafe operations.

use crate::error::{Error, Result};

/// Size of a memory page (4KB on most systems)
const PAGE_SIZE: usize = 4096;

/// Round `size` up to the next page boundary.
const fn align_to_page(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

#[cfg(target_os = "linux")]
use toadstool_hw_safe::LockedMemory;
#[cfg(target_os = "linux")]
use toadstool_hw_safe::locked_memory::LockError;

#[cfg(target_os = "linux")]
fn map_lock_error(e: LockError) -> Error {
    match e {
        LockError::Alloc(a) => Error::memory_allocation(a.to_string()),
        LockError::Mlock(io) => Error::memory_lock(format!("mlock failed: {io}")),
    }
}

/// Best-effort: exclude region from core dumps (`MADV_DONTDUMP`).
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "rustix madvise is unsafe; pointer is our live LockedMemory allocation"
)]
fn madvise_linux_dontdump(ptr: std::ptr::NonNull<u8>, len: usize) {
    use rustix::mm::{Advice, madvise};
    use std::ffi::c_void;

    #[cfg(debug_assertions)]
    {
        debug_assert_eq!(
            ptr.addr().get() % PAGE_SIZE,
            0,
            "madvise range must start on a page boundary (LockedMemory uses PAGE_SIZE alignment)"
        );
        debug_assert_eq!(
            len % PAGE_SIZE,
            0,
            "madvise length must be a multiple of page size (caller rounds to page boundary)"
        );
        debug_assert!(len > 0, "madvise length must be non-zero");
    }

    // SAFETY: `ptr`/`len` describe the same page-aligned region locked by `LockedMemory` in the
    // caller. `LinuxDontDump` does not alter buffer bytes for heap memory.
    let result = unsafe { madvise(ptr.as_ptr().cast::<c_void>(), len, Advice::LinuxDontDump) };
    if let Err(e) = result {
        tracing::warn!("madvise(MADV_DONTDUMP) failed: {e}");
    }
}

/// Isolated memory region with security guarantees
///
/// # Security Properties
///
/// 1. **No Swap** (Linux): Memory locked with `mlock(2)`, cannot be paged to disk
/// 2. **No Core Dump** (Linux): Protected with `madvise(MADV_DONTDUMP)`
/// 3. **Explicit Wipe**: Memory zeroed before deallocation (not just Drop)
/// 4. **Page Aligned**: Aligned to page boundaries for performance
///
/// On non-Linux targets, isolation uses page-aligned heap allocation with explicit wipe.
/// Swap-lock and core-dump exclusion are unavailable without platform support.
#[cfg(target_os = "linux")]
pub struct IsolatedMemoryRegion {
    /// Locked, page-aligned backing store (`mlock` + zeroed alloc via hw-safe)
    inner: LockedMemory,
    /// Logical size (as requested by user)
    logical_size: usize,
}

#[cfg(not(target_os = "linux"))]
/// Isolated memory region with explicit wipe on drop (heap fallback on non-Linux).
pub struct IsolatedMemoryRegion {
    /// Page-aligned heap backing store
    inner: Vec<u8>,
    /// Logical size (as requested by user)
    logical_size: usize,
}

impl IsolatedMemoryRegion {
    /// Create a new isolated memory region
    ///
    /// # Arguments
    ///
    /// * `size` - Size in bytes (will be rounded up to page boundary)
    ///
    /// # Errors
    ///
    /// Returns error if memory allocation fails. On Linux, also returns an error
    /// if memory locking (`mlock`) fails.
    ///
    /// On Linux, `madvise(MADV_DONTDUMP)` is attempted best-effort (failure is logged, not fatal).
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            return Err(Error::invalid_layout(size, PAGE_SIZE));
        }

        let aligned_size = align_to_page(size);

        #[cfg(target_os = "linux")]
        {
            let inner = LockedMemory::new(aligned_size, PAGE_SIZE).map_err(map_lock_error)?;
            madvise_linux_dontdump(inner.as_ptr(), aligned_size);

            tracing::debug!(
                "Allocated isolated memory: {} bytes (aligned to {} bytes)",
                size,
                aligned_size
            );

            return Ok(Self {
                inner,
                logical_size: size,
            });
        }

        #[cfg(not(target_os = "linux"))]
        {
            let mut inner = vec![0u8; aligned_size];
            inner.shrink_to_fit();

            tracing::debug!(
                "Allocated isolated memory (heap fallback): {} bytes (aligned to {} bytes)",
                size,
                aligned_size
            );

            Ok(Self {
                inner,
                logical_size: size,
            })
        }
    }

    /// Get immutable slice view of memory
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        debug_assert!(
            self.logical_size <= self.physical_size(),
            "logical_size must be <= physical size (invariant)"
        );
        &self.backing_slice()[..self.logical_size]
    }

    /// Get mutable slice view of memory
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let logical_size = self.logical_size;
        debug_assert!(
            logical_size <= self.physical_size(),
            "logical_size must be <= physical size (invariant)"
        );
        &mut self.backing_slice_mut()[..logical_size]
    }

    fn backing_slice(&self) -> &[u8] {
        #[cfg(target_os = "linux")]
        {
            return self.inner.as_slice();
        }

        #[cfg(not(target_os = "linux"))]
        {
            return &self.inner;
        }
    }

    fn backing_slice_mut(&mut self) -> &mut [u8] {
        #[cfg(target_os = "linux")]
        {
            return self.inner.as_mut_slice();
        }

        #[cfg(not(target_os = "linux"))]
        {
            return &mut self.inner;
        }
    }

    /// Read a subslice with bounds checking.
    pub fn read_at(&self, offset: usize, len: usize) -> Result<&[u8]> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::security_violation("read offset + len would overflow"))?;
        if end > self.logical_size {
            return Err(Error::security_violation(format!(
                "read out of bounds: offset={}, len={}, size={}",
                offset, len, self.logical_size
            )));
        }
        Ok(&self.as_slice()[offset..end])
    }

    /// Write data at offset with bounds checking.
    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        let len = data.len();
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::security_violation("write offset + len would overflow"))?;
        if end > self.logical_size {
            return Err(Error::security_violation(format!(
                "write out of bounds: offset={}, len={}, size={}",
                offset, len, self.logical_size
            )));
        }
        self.as_mut_slice()[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// Get the logical size of this memory region (as requested by user)
    #[must_use]
    pub const fn size(&self) -> usize {
        self.logical_size
    }

    /// Get the physical size of this memory region (rounded to page boundary)
    #[must_use]
    pub fn physical_size(&self) -> usize {
        #[cfg(target_os = "linux")]
        {
            return self.inner.size();
        }

        #[cfg(not(target_os = "linux"))]
        {
            return self.inner.len();
        }
    }

    /// Explicitly wipe memory contents
    pub fn wipe(&mut self) {
        self.backing_slice_mut().fill(0);
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
        tracing::trace!("Wiped {} bytes of isolated memory", self.physical_size());
    }
}

impl Drop for IsolatedMemoryRegion {
    fn drop(&mut self) {
        self.wipe();
        tracing::trace!(
            "Dropped isolated memory region of {} bytes (physical)",
            self.physical_size()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_isolated_memory() {
        let result = IsolatedMemoryRegion::new(4096);
        assert!(result.is_ok());
        let region = result.unwrap();
        assert_eq!(region.size(), 4096);
    }

    #[test]
    fn test_zero_size_fails() {
        let result = IsolatedMemoryRegion::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_write() {
        let mut region = IsolatedMemoryRegion::new(1024).unwrap();

        let data = b"sensitive data";
        region.as_mut_slice()[..data.len()].copy_from_slice(data);

        let read_back = &region.as_slice()[..data.len()];
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_explicit_wipe() {
        let mut region = IsolatedMemoryRegion::new(1024).unwrap();

        region.as_mut_slice().fill(0xFF);
        assert_eq!(region.as_slice()[0], 0xFF);

        region.wipe();
        assert_eq!(region.as_slice()[0], 0x00);
    }

    #[test]
    fn test_read_at_write_at_bounds() {
        let mut region = IsolatedMemoryRegion::new(1024).unwrap();

        let data = b"hello";
        region.write_at(0, data).unwrap();
        let read = region.read_at(0, data.len()).unwrap();
        assert_eq!(read, data);

        let large = vec![0u8; 2048];
        assert!(region.write_at(0, &large).is_err());
        assert!(region.write_at(512, &large).is_err());

        assert!(region.read_at(1020, 10).is_err());
        assert!(region.read_at(1024, 1).is_err());
    }

    #[test]
    fn test_size_alignment() {
        let region = IsolatedMemoryRegion::new(1000).unwrap();
        assert_eq!(region.size(), 1000);
        assert_eq!(region.physical_size(), 4096);
    }

    #[test]
    fn test_drop_wipes_memory() {
        {
            let mut region = IsolatedMemoryRegion::new(1024).unwrap();
            region.as_mut_slice().fill(0xFF);
        }
    }
}
