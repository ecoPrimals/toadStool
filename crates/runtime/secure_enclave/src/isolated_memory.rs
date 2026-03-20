// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // Memory isolation requires mlock/madvise kernel FFI via rustix
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
//! This eliminates unsafe libc FFI while maintaining identical functionality.

use crate::error::{Error, Result};
use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;

#[cfg(target_family = "unix")]
use rustix::mm::{mlock, munlock};

#[cfg(target_os = "linux")]
use rustix::mm::{Advice, madvise};

use std::ffi::c_void;

/// Size of a memory page (4KB on most systems)
const PAGE_SIZE: usize = 4096;

/// Allocate page-aligned memory and lock it (mlock) to prevent swapping.
///
/// Encapsulates the alloc+mlock pattern to reduce repeated unsafe blocks.
/// On mlock failure, deallocates and returns Err.
///
/// # Safety
/// All unsafe operations are contained here. Caller receives valid `NonNull`
/// and must dealloc with the returned Layout (or use in RAII wrapper).
fn alloc_and_lock(size: usize) -> Result<(NonNull<u8>, Layout)> {
    let layout = Layout::from_size_align(size, PAGE_SIZE)
        .map_err(|e| Error::memory_allocation(format!("Invalid layout: {e}")))?;

    // SAFETY: Layout is valid (from_size_align succeeded, PAGE_SIZE power-of-two). alloc returns a
    // pointer valid for layout.size() bytes, or null on OOM.
    let raw = unsafe { alloc(layout) };
    let ptr = NonNull::new(raw).ok_or_else(|| Error::memory_allocation("alloc returned null"))?;

    #[cfg(target_family = "unix")]
    {
        // SAFETY: ptr from alloc(layout), size matches layout.size(), region is page-aligned.
        // mlock requires page-aligned address; our layout uses PAGE_SIZE alignment.
        let result = unsafe { mlock(ptr.as_ptr().cast::<c_void>(), size) };
        if let Err(e) = result {
            // SAFETY: ptr from alloc above, layout unchanged, no references exist. Cleanup on
            // mlock failure before returning Err.
            unsafe { dealloc(ptr.as_ptr(), layout) };
            return Err(Error::memory_lock(format!("mlock failed: {e}")));
        }
    }

    Ok((ptr, layout))
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
    /// Pointer to allocated memory (never null)
    ptr: NonNull<u8>,

    /// Logical size (as requested by user)
    logical_size: usize,

    /// Physical size (rounded up to page boundary)
    physical_size: usize,

    /// Memory layout (for deallocation)
    layout: Layout,
}

// SAFETY: IsolatedMemoryRegion can be sent between threads because:
// - ptr points to heap-allocated memory that we own exclusively
// - No shared mutable state
// - mlock ensures memory stays resident (thread-safe)
unsafe impl Send for IsolatedMemoryRegion {}

// SAFETY: IsolatedMemoryRegion can be shared between threads with &self because:
// - We only provide &[u8] access via as_slice(), which is thread-safe
// - mlock is thread-safe
// - No interior mutability
unsafe impl Sync for IsolatedMemoryRegion {}

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
    /// - Memory protection fails (madvise)
    ///
    /// # Security
    ///
    /// Memory is immediately locked and protected after allocation,
    /// before returning to caller.
    ///
    /// # Panics
    ///
    /// Never panics. The internal `expect()` is infallible because the null
    /// case is returned as `Err` above; it exists only to satisfy the
    /// `NonNull` API.
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            return Err(Error::invalid_layout(size, PAGE_SIZE));
        }

        // Round size up to page boundary for optimal performance
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Allocate and lock in one encapsulated helper (reduces repeated unsafe)
        let (ptr, layout) = alloc_and_lock(aligned_size)?;

        // Prevent memory from appearing in core dumps
        #[cfg(target_os = "linux")]
        {
            // SAFETY: ptr from alloc_and_lock; aligned_size matches physical allocation. Region is
            // page-aligned; [ptr, ptr+aligned_size) is valid and within the allocation.
            let result = unsafe {
                madvise(
                    ptr.as_ptr().cast::<c_void>(),
                    aligned_size,
                    Advice::LinuxDontDump,
                )
            };
            if let Err(e) = result {
                tracing::warn!("madvise(MADV_DONTDUMP) failed: {e}");
                // Non-fatal: continue but log warning
            }
        }

        tracing::debug!(
            "Allocated isolated memory: {} bytes (aligned to {} bytes)",
            size,
            aligned_size
        );

        Ok(Self {
            ptr,
            logical_size: size,
            physical_size: aligned_size,
            layout,
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
    ///
    /// # Safety
    ///
    /// - Returns a slice with lifetime tied to &self
    /// - Memory is valid for the lifetime of the struct
    /// - No concurrent mutable access (enforced by Rust)
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        debug_assert!(
            self.logical_size <= self.physical_size,
            "logical_size must be <= physical_size (invariant)"
        );
        // SAFETY: ptr from alloc_and_lock, valid for physical_size bytes. logical_size <=
        // physical_size by construction. Lifetime tied to &self; no concurrent mutable access.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.logical_size) }
    }

    /// Get mutable slice view of memory
    ///
    /// Returns a slice with the logical size (as requested by user),
    /// not the physical allocated size.
    ///
    /// # Bounds
    ///
    /// Slice covers `[0..logical_size]`. Use `write_at` for bounds-checked writes.
    ///
    /// # Safety
    ///
    /// - Returns a mutable slice with lifetime tied to &mut self
    /// - Ensures exclusive access (only one mutable reference)
    /// - Memory is valid for the lifetime of the struct
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(
            self.logical_size <= self.physical_size,
            "logical_size must be <= physical_size (invariant)"
        );
        // SAFETY: ptr valid for physical_size bytes. logical_size <= physical_size. &mut self
        // gives exclusive access (no aliasing).
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.logical_size) }
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
        self.physical_size
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
        // Safe Rust: Use slice fill instead of raw write_bytes
        // SAFETY: as_physical_slice_mut is safe (we own the memory)
        self.as_physical_slice_mut().fill(0);

        // Compiler fence to prevent optimizer from removing the write
        // This is critical for security - ensures memory is actually zeroed
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        tracing::trace!("Wiped {} bytes of isolated memory", self.physical_size);
    }

    /// Get mutable slice of the physical allocation (for internal use like wiping)
    fn as_physical_slice_mut(&mut self) -> &mut [u8] {
        debug_assert!(
            self.physical_size > 0,
            "physical_size must be > 0 (invariant)"
        );
        // SAFETY: ptr from alloc_and_lock, valid for physical_size bytes. &mut self gives
        // exclusive access. physical_size matches layout.size() from allocation.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.physical_size) }
    }
}

impl Drop for IsolatedMemoryRegion {
    fn drop(&mut self) {
        // Step 1: Wipe memory before unlocking/deallocating
        // Uses safe slice-based fill via self.wipe()
        self.wipe();

        // Step 2: Unlock memory (reverse of mlock)
        #[cfg(target_family = "unix")]
        {
            // SAFETY: ptr from alloc in new(); physical_size matches the mlocked region. Region was
            // mlocked in new(); munlock must be called with same ptr and size.
            let result = unsafe { munlock(self.ptr.as_ptr().cast::<c_void>(), self.physical_size) };
            if let Err(e) = result {
                tracing::error!("munlock failed during drop: {e}");
            }
        }

        // Step 3: Deallocate memory
        // SAFETY: ptr from alloc_and_lock in new(); self.layout matches allocation. Drop runs at
        // most once; no references exist (wipe/munlock complete, self is being dropped).
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }

        tracing::trace!(
            "Dropped isolated memory region of {} bytes (physical)",
            self.physical_size
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
        // 1. Memory is zeroed (write_bytes)
        // 2. Compiler fence prevents optimization
        // 3. Memory is unlocked (munlock)
        // 4. Memory is deallocated
    }
}
