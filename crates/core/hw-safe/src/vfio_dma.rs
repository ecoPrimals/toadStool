// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // VFIO ioctls are inherently unsafe — this is the containment zone

//! Shared VFIO DMA mapping operations.
//!
//! Provides a single implementation of VFIO IOMMU DMA map/unmap ioctls
//! so that `nvpmu` and `akida-driver` share the same kernel ABI structs
//! and unsafe ioctl wrappers instead of duplicating them.

use std::os::fd::BorrowedFd;

use rustix::ioctl::{self, Ioctl, IoctlOutput, Opcode, opcode};

/// VFIO ioctl type byte (Linux kernel ABI: `';'` = 0x3B).
pub const VFIO_TYPE: u8 = b';';
/// VFIO ioctl base number (kernel ABI: first VFIO opcode = 100).
pub const VFIO_BASE: u8 = 100;
const OP_IOMMU_MAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 13);
const OP_IOMMU_UNMAP_DMA: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 14);

/// VFIO DMA mapping request sent to the kernel via ioctl.
#[repr(C)]
pub struct VfioDmaMap {
    /// Struct size for kernel version negotiation.
    pub argsz: u32,
    /// Mapping flags (read/write).
    pub flags: u32,
    /// Virtual address of the host buffer.
    pub vaddr: u64,
    /// I/O virtual address visible to the device.
    pub iova: u64,
    /// Size of the mapping in bytes.
    pub size: u64,
}

/// VFIO DMA unmap request sent to the kernel via ioctl.
#[repr(C)]
pub struct VfioDmaUnmap {
    /// Struct size for kernel version negotiation.
    pub argsz: u32,
    /// Unmap flags.
    pub flags: u32,
    /// I/O virtual address to unmap.
    pub iova: u64,
    /// Size to unmap.
    pub size: u64,
}

/// VFIO DMA mapping flags.
pub mod flags {
    /// Allow device reads from this region.
    pub const READ: u32 = 1;
    /// Allow device writes to this region.
    pub const WRITE: u32 = 2;
    /// Allow device reads and writes.
    pub const READ_WRITE: u32 = READ | WRITE;
}

struct DmaMapIoctl<'a>(&'a VfioDmaMap);

// SAFETY: we provide a valid repr(C) struct pointer to the kernel and
// the ioctl number matches the VFIO spec (VFIO_IOMMU_MAP_DMA).
unsafe impl Ioctl for DmaMapIoctl<'_> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP_IOMMU_MAP_DMA
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        std::ptr::from_ref(self.0).cast_mut().cast()
    }

    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

struct DmaUnmapIoctl<'a>(&'a VfioDmaUnmap);

// SAFETY: same rationale as DmaMapIoctl (VFIO_IOMMU_UNMAP_DMA).
unsafe impl Ioctl for DmaUnmapIoctl<'_> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP_IOMMU_UNMAP_DMA
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        std::ptr::from_ref(self.0).cast_mut().cast()
    }

    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

/// Single ioctl dispatch for VFIO DMA operations.
fn do_ioctl<I: Ioctl>(fd: BorrowedFd<'_>, cmd: I) -> std::io::Result<I::Output> {
    // SAFETY: callers construct `cmd` from VFIO kernel-ABI types with
    // compile-time opcodes. `fd` is a valid VFIO container fd.
    unsafe { ioctl::ioctl(fd, cmd) }.map_err(Into::into)
}

/// Map a host buffer into the device IOMMU via VFIO.
///
/// # Errors
///
/// Returns an I/O error if the ioctl fails.
///
/// # Safety
///
/// The caller must ensure:
/// - `container_fd` is an open VFIO container with an attached IOMMU.
/// - `map.vaddr` points to allocated memory of at least `map.size` bytes.
/// - The IOVA range `[map.iova, map.iova + map.size)` is not already mapped.
pub unsafe fn dma_map(container_fd: BorrowedFd<'_>, map: &VfioDmaMap) -> std::io::Result<()> {
    do_ioctl(container_fd, DmaMapIoctl(map))
}

/// Remove a device IOMMU mapping via VFIO.
///
/// # Errors
///
/// Returns an I/O error if the ioctl fails.
///
/// # Safety
///
/// The caller must ensure `container_fd` is valid and `unmap.iova`/`size`
/// correspond to a previously mapped region.
pub unsafe fn dma_unmap(container_fd: BorrowedFd<'_>, unmap: &VfioDmaUnmap) -> std::io::Result<()> {
    do_ioctl(container_fd, DmaUnmapIoctl(unmap))
}

/// Align `size` up to the nearest multiple of `page`.
#[must_use]
pub const fn page_align_up(size: usize, page: usize) -> usize {
    size.div_ceil(page) * page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_align_up_basic() {
        assert_eq!(page_align_up(1, 4096), 4096);
        assert_eq!(page_align_up(4096, 4096), 4096);
        assert_eq!(page_align_up(4097, 4096), 8192);
        assert_eq!(page_align_up(0, 4096), 0);
    }

    #[test]
    fn vfio_dma_map_layout() {
        assert_eq!(
            std::mem::size_of::<VfioDmaMap>(),
            4 + 4 + 8 + 8 + 8,
            "VfioDmaMap must match VFIO kernel ABI"
        );
    }

    #[test]
    fn vfio_dma_unmap_layout() {
        assert_eq!(
            std::mem::size_of::<VfioDmaUnmap>(),
            4 + 4 + 8 + 8,
            "VfioDmaUnmap must match VFIO kernel ABI"
        );
    }

    #[test]
    fn flags_constants() {
        assert_eq!(flags::READ_WRITE, flags::READ | flags::WRITE);
    }
}
