// SPDX-License-Identifier: AGPL-3.0-or-later
//! RAII wrapper for mmap-backed MMIO with bounds-checked volatile access.
//!
//! Volatile register access delegates to [`toadstool_hw_safe::VolatileMmio`];
//! fd-backed mappings use [`toadstool_hw_safe::DeviceMmap`].

use std::borrow::Cow;
use std::ptr::NonNull;

use toadstool_hw_safe::{DeviceMmap, MmioError, VolatileMmio};

use crate::error::DriverError;

/// Owns a byte range of MMIO address space and releases it on drop (`munmap` for
/// kernel mappings, or heap deallocation for test-only heap backings).
///
/// This type does **not** implement [`Send`] or [`Sync`]. Wrappers that prove
/// additional invariants (for example VFIO-mapped BARs) may provide those impls.
pub(crate) struct MmioRegion {
    backing: Backing,
}

enum Backing {
    /// Region mapped via [`DeviceMmap`]; unmapped on drop by hw-safe.
    Device(DeviceMmap),
    /// Adopted `mmap` result; this struct calls `munmap` on drop.
    Adopted { ptr: NonNull<u8>, len: usize },
    /// Test-only: do not `munmap`.
    #[cfg(test)]
    Heap(Box<[u8]>),
}

impl MmioRegion {
    /// Wrap an fd-backed mapping from [`DeviceMmap`].
    #[must_use]
    pub(crate) fn from_device_mmap(mmap: DeviceMmap) -> Self {
        Self {
            backing: Backing::Device(mmap),
        }
    }

    /// Take ownership of an existing `mmap` result; the region is unmapped on [`Drop`].
    ///
    /// Prefer [`Self::from_device_mmap`] when mapping from an fd via [`DeviceMmap`].
    /// Use this only when another subsystem (for example VFIO runtime) already
    /// performed `mmap` and cylinder must adopt the pointer without remapping.
    ///
    /// # Safety
    ///
    /// - `ptr` must be non-null and reference exactly `len` bytes that were
    ///   mapped with `mmap` (or equivalent) in this process.
    /// - The mapping must not be unmapped elsewhere before this value is dropped.
    /// - `len` must match the length passed to `mmap`.
    #[must_use]
    pub(crate) unsafe fn new(ptr: *mut u8, len: usize) -> Self {
        assert!(
            !ptr.is_null(),
            "MmioRegion::new: mmap pointer must be non-null (caller broke safety contract)"
        );
        Self {
            backing: Backing::Adopted {
                // SAFETY: asserted non-null above
                ptr: unsafe { NonNull::new_unchecked(ptr) },
                len,
            },
        }
    }

    /// Byte length of the mapped region.
    #[must_use]
    pub(crate) const fn len(&self) -> usize {
        match &self.backing {
            Backing::Device(m) => m.size(),
            Backing::Adopted { len, .. } => *len,
            #[cfg(test)]
            Backing::Heap(h) => h.len(),
        }
    }

    /// Raw base pointer (for legacy callers that perform their own arithmetic).
    #[must_use]
    pub(crate) fn as_ptr(&self) -> *mut u8 {
        match &self.backing {
            Backing::Device(m) => m.as_ptr().as_ptr(),
            Backing::Adopted { ptr, .. } => ptr.as_ptr(),
            #[cfg(test)]
            Backing::Heap(h) => h.as_ptr().cast_mut(),
        }
    }

    fn volatile(&self) -> VolatileMmio<'_> {
        match &self.backing {
            Backing::Device(m) => m.as_volatile(),
            Backing::Adopted { ptr, len } => {
                // SAFETY: Adopted pointers satisfy VolatileMmio construction invariants.
                unsafe { VolatileMmio::new(*ptr, *len) }
            }
            #[cfg(test)]
            Backing::Heap(h) => {
                let ptr = NonNull::new(h.as_ptr().cast_mut())
                    .expect("non-empty heap backing has non-null ptr");
                // SAFETY: heap backing is owned by this struct for its lifetime.
                unsafe { VolatileMmio::new(ptr, h.len()) }
            }
        }
    }

    fn map_hw_err(e: MmioError) -> DriverError {
        match e {
            MmioError::OutOfBounds { offset, .. } => DriverError::MmapFailed(Cow::Owned(format!(
                "MMIO read: offset {offset:#x} + 4 out of range"
            ))),
            MmioError::Misaligned { address, alignment } => DriverError::MmapFailed(Cow::Owned(
                format!("MMIO access at {address:#x} is not {alignment}-byte aligned"),
            )),
        }
    }

    /// Volatile 32-bit read at `offset` bytes from the region base.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::MmapFailed`] if `offset + 4` exceeds the region
    /// or the offset is misaligned.
    pub(crate) fn read_u32(&self, offset: usize) -> Result<u32, DriverError> {
        self.volatile().read_u32(offset).map_err(Self::map_hw_err)
    }

    /// Volatile 32-bit write at `offset` bytes from the region base.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::MmapFailed`] if `offset + 4` exceeds the region
    /// or the offset is misaligned.
    pub(crate) fn write_u32(&self, offset: usize, value: u32) -> Result<(), DriverError> {
        self.volatile()
            .write_u32(offset, value)
            .map_err(Self::map_hw_err)
    }

    /// Heap-backed region for unit tests (no `munmap`; frees the buffer on drop).
    #[cfg(test)]
    pub(crate) fn from_heap_slice_for_test(backing: Box<[u8]>) -> Self {
        assert!(
            !backing.is_empty(),
            "from_heap_slice_for_test: empty slice not supported"
        );
        Self {
            backing: Backing::Heap(backing),
        }
    }
}

impl Drop for MmioRegion {
    fn drop(&mut self) {
        if let Backing::Adopted { ptr, len } = self.backing {
            // SAFETY: Adopted backing came from `new`, whose safety contract
            // requires that `ptr`/`len` came from `mmap` and were not freed elsewhere.
            unsafe {
                let _ = rustix::mm::munmap(ptr.as_ptr().cast::<std::ffi::c_void>(), len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_roundtrip_read_write() {
        let data = vec![0u8; 64].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(data);
        region.write_u32(0, 0x1122_3344).expect("write");
        assert_eq!(region.read_u32(0).expect("read"), 0x1122_3344);
    }

    #[test]
    fn heap_offset_read_write() {
        let data = vec![0u8; 128].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(data);
        region.write_u32(16, 0xAAAA_BBBB).expect("write");
        assert_eq!(region.read_u32(16).expect("read"), 0xAAAA_BBBB);
    }

    #[test]
    fn read_oob_returns_error() {
        let data = vec![0u8; 8].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(data);
        assert!(region.read_u32(8).is_err());
        assert!(region.read_u32(5).is_err());
    }

    #[test]
    fn write_oob_returns_error() {
        let data = vec![0u8; 8].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(data);
        assert!(region.write_u32(8, 0).is_err());
    }

    #[test]
    fn len_matches_backing() {
        let data = vec![0u8; 256].into_boxed_slice();
        let region = MmioRegion::from_heap_slice_for_test(data);
        assert_eq!(region.len(), 256);
    }
}
