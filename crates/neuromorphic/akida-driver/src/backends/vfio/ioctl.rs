// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO ioctl wrappers — safe Rust interfaces over kernel ioctls
//!
//! Each wrapper function encapsulates one `unsafe` ioctl call with documented
//! safety invariants. All callers pass valid `BorrowedFd` from VFIO opens.

use crate::error::{AkidaError, Result};
use rustix::io::Result as IoResult;
use rustix::ioctl::{Ioctl, IoctlOutput, Opcode};
use std::os::fd::BorrowedFd;

use super::types::ioctls;
use super::types::{VfioDeviceInfo, VfioDmaMap, VfioDmaUnmap, VfioGroupStatus};

/// Ioctl adapter for VFIO commands that return an i32 (no-arg or integer-arg).
pub struct VfioIoctlReturn<const OP: Opcode> {
    arg: usize,
}

// SAFETY: opcode is a compile-time VFIO constant; as_ptr returns arg cast to *mut c_void
// (no-arg or integer-arg ioctls); output_from_ptr wraps the kernel return value.
unsafe impl<const OP: Opcode> Ioctl for VfioIoctlReturn<OP> {
    type Output = i32;
    const IS_MUTATING: bool = false;

    fn opcode(&self) -> Opcode {
        OP
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.arg as *mut std::ffi::c_void
    }

    unsafe fn output_from_ptr(
        out: IoctlOutput,
        _extract_output: *mut std::ffi::c_void,
    ) -> IoResult<Self::Output> {
        Ok(out)
    }
}

/// Ioctl adapter for VFIO commands that read/write a kernel ABI struct.
pub struct VfioIoctlPtr<const OP: Opcode, T> {
    ptr: *mut T,
}

// SAFETY: opcode is compile-time VFIO constant; as_ptr casts caller-supplied *mut T to
// *mut c_void (T is a repr(C) VFIO struct matching kernel ABI); IS_MUTATING=true since
// the kernel writes back. All callers construct T on the stack with correct argsz.
unsafe impl<const OP: Opcode, T> Ioctl for VfioIoctlPtr<OP, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }

    unsafe fn output_from_ptr(
        _out: IoctlOutput,
        _extract_output: *mut std::ffi::c_void,
    ) -> IoResult<Self::Output> {
        Ok(())
    }
}

fn ioctl_err(e: rustix::io::Errno) -> AkidaError {
    AkidaError::capability_query_failed(format!("ioctl failed: {e}"))
}

#[inline]
pub fn get_api_version(fd: BorrowedFd<'_>) -> Result<i32> {
    // SAFETY: Invariants: fd must be valid VFIO fd; ioctl opcode matches kernel ABI.
    // Satisfied: fd from caller (VFIO container/device open); opcode is VFIO constant.
    // Violation: invalid fd → kernel error/UB; wrong opcode → wrong syscall.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GET_API_VERSION }> { arg: 0 };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn check_extension(fd: BorrowedFd<'_>, arg: u32) -> Result<i32> {
    // SAFETY: Invariants: fd valid; arg is extension ID (kernel expects u32).
    // Satisfied: fd from caller; arg is VFIO extension constant. Violation: invalid fd → UB.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_CHECK_EXTENSION }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn set_iommu(fd: BorrowedFd<'_>, arg: u32) -> Result<i32> {
    // SAFETY: Invariants: fd valid VFIO container; arg is IOMMU type (e.g. TYPE1V2).
    // Satisfied: fd from container open; arg is ioctls constant. Violation: invalid fd → UB.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_SET_IOMMU }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn group_status(fd: BorrowedFd<'_>, arg: &mut VfioGroupStatus) -> Result<()> {
    // SAFETY: Invariants: fd valid; arg must be repr(C) matching kernel VfioGroupStatus ABI.
    // Satisfied: fd from VFIO group open; VfioGroupStatus is #[repr(C)]. Violation: layout mismatch → kernel corruption.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_GROUP_GET_STATUS }, _> {
        ptr: std::ptr::from_mut(arg),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn device_info(fd: BorrowedFd<'_>, arg: &mut VfioDeviceInfo) -> Result<()> {
    // SAFETY: Invariants: fd valid VFIO device; arg repr(C) matching kernel ABI.
    // Satisfied: fd from device open; VfioDeviceInfo is #[repr(C)]. Violation: layout mismatch → kernel corruption.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_DEVICE_GET_INFO }, _> {
        ptr: std::ptr::from_mut(arg),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn dma_map(fd: BorrowedFd<'_>, arg: &VfioDmaMap) -> Result<()> {
    // SAFETY: Invariants: fd valid VFIO container; arg repr(C) matching kernel VfioDmaMap.
    // Satisfied: fd from container; VfioDmaMap is #[repr(C)]; vaddr/iova/size from alloc.
    // Violation: layout mismatch → kernel corruption; invalid vaddr → DMA to wrong memory.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_IOMMU_MAP_DMA }, VfioDmaMap> {
        ptr: std::ptr::from_ref(arg).cast_mut(),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn dma_unmap(fd: BorrowedFd<'_>, arg: &VfioDmaUnmap) -> Result<()> {
    // SAFETY: Invariants: fd valid; arg repr(C) matching kernel VfioDmaUnmap; iova/size must match prior map.
    // Satisfied: fd from container; VfioDmaUnmap is #[repr(C)]; iova/size from DmaBuffer.
    // Violation: layout mismatch → kernel corruption; wrong iova → unmapping wrong region.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_IOMMU_UNMAP_DMA }, VfioDmaUnmap> {
        ptr: std::ptr::from_ref(arg).cast_mut(),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn group_set_container(fd: BorrowedFd<'_>, arg: *const std::ffi::c_void) -> Result<i32> {
    // SAFETY: Invariants: fd valid VFIO group; arg points to int (container fd) valid for ioctl duration.
    // Satisfied: fd from group open; arg from &container_fd. Violation: invalid ptr → kernel read of garbage.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GROUP_SET_CONTAINER }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub fn group_get_device_fd(fd: BorrowedFd<'_>, arg: *const std::ffi::c_void) -> Result<i32> {
    // SAFETY: Invariants: fd valid VFIO group; arg is valid C string (null-terminated PCIe address).
    // Satisfied: fd from group open; arg from CString::as_ptr(). Violation: invalid string → kernel crash.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GROUP_GET_DEVICE_FD }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}
