// SPDX-License-Identifier: AGPL-3.0-or-later
//! Isolated memory region for secure computation
//!
//! Provides memory regions that are:
//! - **Locked**: Cannot be swapped to disk (mlock)
//! - **Protected**: Cannot appear in core dumps (madvise `MADV_DONTDUMP`)
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

use toadstool_hw_safe::LockedMemory;
use toadstool_hw_safe::locked_memory::LockError;

/// Size of a memory page (4KB on most systems)
const PAGE_SIZE: usize = 4096;

fn map_lock_error(e: LockError) -> Error {
    match e {
        LockError::Alloc(a) => Error::memory_allocation(a.to_string()),
        LockError::Mlock(io) => Error::memory_lock(format!("mlock failed: {io}")),
    }
}

/// Best-effort: exclude region from core dumps (`MADV_DONTDUMP`).
#[cfg(target_os = "linux")]
#[expect(unsafe_code)] // rustix `madvise` is `unsafe` — pointer is our live `LockedMemory` allocation
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
/// 1. **No Swap**: Memory locked with `mlock(2)`, cannot be paged to disk
/// 2. **No Core Dump**: Protected with `madvise(MADV_DONTDUMP)`
/// 3. **Explicit Wipe**: Memory zeroed before deallocation (not just Drop)
/// 4. **Page Aligned**: Aligned to page boundaries for performance
///
/// # Implementation Note
///
/// Memory is allocated in page-aligned chunks for optimal performance and
/// to satisfy mlock requirements, but the exposed size is the logical size
/// requested by the user.
///
/// # Example
///
/// ```rust,ignore
/// use secure_enclave::IsolatedMemoryRegion;
///
/// // Allocate 1MB of isolated memory
/// let mut region = IsolatedMemoryRegion::new(1024 * 1024)?;
///
/// // Use it for sensitive data
/// let buffer = region.as_mut_slice();
/// buffer.copy_from_slice(&sensitive_data);
///
/// // Process...
/// process(buffer)?;
///
/// // Memory automatically wiped on drop
/// ```
pub struct IsolatedMemoryRegion {
    /// Locked, page-aligned backing store (`mlock` + zeroed alloc via hw-safe)
    inner: LockedMemory,

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
    /// Returns error if:
    /// - Memory allocation fails
    /// - Memory locking fails (mlock)
    ///
    /// On Linux, `madvise(MADV_DONTDUMP)` is attempted best-effort (failure is logged, not fatal).
    ///
    /// # Security
    ///
    /// Memory is immediately locked and protected after allocation,
    /// before returning to caller.
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            return Err(Error::invalid_layout(size, PAGE_SIZE));
        }

        // Round size up to page boundary for optimal performance
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        let inner = LockedMemory::new(aligned_size, PAGE_SIZE).map_err(map_lock_error)?;

        // Prevent memory from appearing in core dumps
        #[cfg(target_os = "linux")]
        madvise_linux_dontdump(inner.as_ptr(), aligned_size);

        tracing::debug!(
            "Allocated isolated memory: {} bytes (aligned to {} bytes)",
            size,
            aligned_size
        );

        Ok(Self {
            inner,
            logical_size: size,
        })
    }

    /// Get immutable slice view of memory
    ///
    /// Returns a slice with the logical size (as requested by user),
    /// not the physical allocated size.
    ///
    /// # Bounds
    ///
    /// Slice covers `[0..logical_size]`. Use `read_at` for bounds-checked subslice access.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        debug_assert!(
            self.logical_size <= self.inner.size(),
            "logical_size must be <= physical size (invariant)"
        );
        &self.inner.as_slice()[..self.logical_size]
    }

    /// Get mutable slice view of memory
    ///
    /// Returns a slice with the logical size (as requested by user),
    /// not the physical allocated size.
    ///
    /// # Bounds
    ///
    /// Slice covers `[0..logical_size]`. Use `write_at` for bounds-checked writes.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(
            self.logical_size <= self.inner.size(),
            "logical_size must be <= physical size (invariant)"
        );
        &mut self.inner.as_mut_slice()[..self.logical_size]
    }

    /// Read a subslice with bounds checking.
    ///
    /// Returns `Ok(&[u8])` for the range `[offset..offset+len]` if in bounds.
    /// Returns `Err` if offset+len would overflow or exceed logical size.
    ///
    /// # Errors
    ///
    /// Returns an error if `offset + len` would overflow or exceed the logical size.
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
    ///
    /// Returns `Ok(())` if the full `data` fits at `offset`.
    /// Returns `Err` if `offset+data.len()` would overflow or exceed logical size.
    ///
    /// # Errors
    ///
    /// Returns an error if `offset + data.len()` would overflow or exceed the logical size.
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
    pub const fn physical_size(&self) -> usize {
        self.inner.size()
    }

    /// Explicitly wipe memory contents
    ///
    /// This is also called automatically in Drop, but can be called
    /// explicitly for additional security.
    ///
    /// Wipes the entire physical allocation, not just the logical size.
    ///
    /// # Evolution Note
    ///
    /// Uses slice-based `fill(0)` instead of raw `write_bytes` for safer code.
    /// The compiler fence ensures the optimizer cannot remove the zeroing.
    pub fn wipe(&mut self) {
        self.inner.as_mut_slice().fill(0);

        // Compiler fence to prevent optimizer from removing the write
        // This is critical for security - ensures memory is actually zeroed
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        tracing::trace!("Wiped {} bytes of isolated memory", self.inner.size());
    }
}

impl Drop for IsolatedMemoryRegion {
    fn drop(&mut self) {
        // Step 1: Wipe memory before unlock/dealloc (LockedMemory::drop does munlock + dealloc)
        self.wipe();

        tracing::trace!(
            "Dropped isolated memory region of {} bytes (physical)",
            self.inner.size()
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

        // Write data
        let data = b"sensitive data";
        region.as_mut_slice()[..data.len()].copy_from_slice(data);

        // Read back
        let read_back = &region.as_slice()[..data.len()];
        assert_eq!(read_back, data);
    }

    #[test]
    fn test_explicit_wipe() {
        let mut region = IsolatedMemoryRegion::new(1024).unwrap();

        // Write data
        region.as_mut_slice().fill(0xFF);
        assert_eq!(region.as_slice()[0], 0xFF);

        // Explicit wipe
        region.wipe();
        assert_eq!(region.as_slice()[0], 0x00);
    }

    #[test]
    fn test_read_at_write_at_bounds() {
        let mut region = IsolatedMemoryRegion::new(1024).unwrap();

        // Valid write and read
        let data = b"hello";
        region.write_at(0, data).unwrap();
        let read = region.read_at(0, data.len()).unwrap();
        assert_eq!(read, data);

        // Out of bounds write
        let large = vec![0u8; 2048];
        assert!(region.write_at(0, &large).is_err());
        assert!(region.write_at(512, &large).is_err());

        // Out of bounds read
        assert!(region.read_at(1020, 10).is_err());
        assert!(region.read_at(1024, 1).is_err());
    }

    #[test]
    fn test_size_alignment() {
        // Request 1000 bytes
        let region = IsolatedMemoryRegion::new(1000).unwrap();
        // Logical size should be 1000 (as requested)
        assert_eq!(region.size(), 1000);
        // Physical size should be rounded up to 4096 (page size)
        assert_eq!(region.physical_size(), 4096);
    }

    #[test]
    fn test_drop_wipes_memory() {
        // This test documents the drop behavior
        // Memory is wiped before deallocation (verified by Drop implementation)
        {
            let mut region = IsolatedMemoryRegion::new(1024).unwrap();
            region.as_mut_slice().fill(0xFF);
            // region dropped here - memory wiped then deallocated
        }
        // After drop, memory is deallocated and cannot be inspected
        // But Drop implementation guarantees:
        // 1. Memory is zeroed (fill)
        // 2. Compiler fence prevents optimization
        // 3. Memory is unlocked (munlock)
        // 4. Memory is deallocated
    }
}
