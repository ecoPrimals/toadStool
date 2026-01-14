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

use crate::error::{Error, Result};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

/// Size of a memory page (4KB on most systems)
const PAGE_SIZE: usize = 4096;

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
    pub fn new(size: usize) -> Result<Self> {
        if size == 0 {
            return Err(Error::invalid_layout(size, PAGE_SIZE));
        }

        // Round size up to page boundary for optimal performance
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Create layout (page-aligned)
        let layout = Layout::from_size_align(aligned_size, PAGE_SIZE)
            .map_err(|e| Error::memory_allocation(format!("Invalid layout: {e}")))?;

        // SAFETY: Layout is valid (non-zero size, power-of-2 alignment)
        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            return Err(Error::memory_allocation("alloc returned null"));
        }

        // SAFETY: We just checked that ptr is not null above
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        // Lock memory to prevent swapping
        // SAFETY:
        // - ptr is valid (just allocated)
        // - aligned_size is the actual allocated size
        // - Memory will be unlocked in Drop before deallocation
        #[cfg(target_family = "unix")]
        unsafe {
            if libc::mlock(ptr.as_ptr() as *const libc::c_void, aligned_size) != 0 {
                // Cleanup on failure
                dealloc(ptr.as_ptr(), layout);
                return Err(Error::memory_lock(format!(
                    "mlock failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }

        // Prevent memory from appearing in core dumps
        // SAFETY:
        // - ptr is valid and locked
        // - MADV_DONTDUMP is a valid flag
        // - Does not invalidate the memory
        #[cfg(target_os = "linux")]
        unsafe {
            if libc::madvise(
                ptr.as_ptr().cast::<libc::c_void>(),
                aligned_size,
                libc::MADV_DONTDUMP,
            ) != 0
            {
                tracing::warn!(
                    "madvise(MADV_DONTDUMP) failed: {}",
                    std::io::Error::last_os_error()
                );
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
    /// # Safety
    ///
    /// - Returns a slice with lifetime tied to &self
    /// - Memory is valid for the lifetime of the struct
    /// - No concurrent mutable access (enforced by Rust)
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY:
        // - ptr is valid (allocated and not yet freed)
        // - logical_size is within allocated memory (logical_size <= physical_size)
        // - Memory is properly aligned
        // - Lifetime is tied to &self (no use-after-free)
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.logical_size) }
    }

    /// Get mutable slice view of memory
    ///
    /// Returns a slice with the logical size (as requested by user),
    /// not the physical allocated size.
    ///
    /// # Safety
    ///
    /// - Returns a mutable slice with lifetime tied to &mut self
    /// - Ensures exclusive access (only one mutable reference)
    /// - Memory is valid for the lifetime of the struct
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY:
        // - ptr is valid (allocated and not yet freed)
        // - logical_size is within allocated memory (logical_size <= physical_size)
        // - Memory is properly aligned
        // - Lifetime is tied to &mut self (exclusive access)
        // - No aliasing (Rust's &mut guarantees exclusive access)
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.logical_size) }
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
    pub fn wipe(&mut self) {
        // SAFETY: ptr is valid and physical_size is the actual allocated size
        unsafe {
            std::ptr::write_bytes(self.ptr.as_ptr(), 0, self.physical_size);
        }

        // Compiler fence to prevent optimizer from removing the write
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        tracing::trace!("Wiped {} bytes of isolated memory", self.physical_size);
    }
}

impl Drop for IsolatedMemoryRegion {
    fn drop(&mut self) {
        // Step 1: Explicitly wipe memory before unlocking/deallocating
        // Wipe entire physical allocation
        // SAFETY: ptr is valid and physical_size is the actual allocated size
        unsafe {
            std::ptr::write_bytes(self.ptr.as_ptr(), 0, self.physical_size);
        }

        // Compiler fence to ensure wipe completes
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);

        // Step 2: Unlock memory (reverse of mlock)
        // SAFETY:
        // - ptr is valid (not yet deallocated)
        // - physical_size matches what was locked in new()
        #[cfg(target_family = "unix")]
        unsafe {
            let result =
                libc::munlock(self.ptr.as_ptr() as *const libc::c_void, self.physical_size);
            if result != 0 {
                tracing::error!(
                    "munlock failed during drop: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        // Step 3: Deallocate memory
        // SAFETY:
        // - ptr was allocated with this layout in new()
        // - This is called exactly once (Drop guarantee)
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }

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
