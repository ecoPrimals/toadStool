// SPDX-License-Identifier: AGPL-3.0-only
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
pub(crate) struct VfioIoctlReturn<const OP: Opcode> {
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
pub(crate) struct VfioIoctlPtr<const OP: Opcode, T> {
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
pub(crate) fn get_api_version(fd: BorrowedFd<'_>) -> Result<i32> {
    // SAFETY: no-arg VFIO ioctl; fd is valid from caller.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GET_API_VERSION }> { arg: 0 };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn check_extension(fd: BorrowedFd<'_>, arg: u32) -> Result<i32> {
    // SAFETY: u32-arg VFIO ioctl; fd valid; arg is extension id.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_CHECK_EXTENSION }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn set_iommu(fd: BorrowedFd<'_>, arg: u32) -> Result<i32> {
    // SAFETY: u32-arg VFIO ioctl; fd valid; arg is IOMMU type.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_SET_IOMMU }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn group_status(fd: BorrowedFd<'_>, arg: &mut VfioGroupStatus) -> Result<()> {
    // SAFETY: struct ioctl; fd valid; arg has kernel layout.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_GROUP_GET_STATUS }, _> {
        ptr: std::ptr::from_mut(arg),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn device_info(fd: BorrowedFd<'_>, arg: &mut VfioDeviceInfo) -> Result<()> {
    // SAFETY: struct ioctl; fd valid; arg has kernel layout.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_DEVICE_GET_INFO }, _> {
        ptr: std::ptr::from_mut(arg),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn dma_map(fd: BorrowedFd<'_>, arg: &VfioDmaMap) -> Result<()> {
    // SAFETY: write-only struct ioctl; fd valid; arg has kernel layout.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_IOMMU_MAP_DMA }, VfioDmaMap> {
        ptr: std::ptr::from_ref(arg).cast_mut(),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn dma_unmap(fd: BorrowedFd<'_>, arg: &VfioDmaUnmap) -> Result<()> {
    // SAFETY: write-only struct ioctl; fd valid; arg has kernel layout.
    let ioctl = VfioIoctlPtr::<{ ioctls::OP_IOMMU_UNMAP_DMA }, VfioDmaUnmap> {
        ptr: std::ptr::from_ref(arg).cast_mut(),
    };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn group_set_container(fd: BorrowedFd<'_>, arg: *const std::ffi::c_void) -> Result<i32> {
    // SAFETY: pointer-arg ioctl; fd valid; arg points to container fd.
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GROUP_SET_CONTAINER }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}

#[inline]
pub(crate) fn group_get_device_fd(fd: BorrowedFd<'_>, arg: *const std::ffi::c_void) -> Result<i32> {
    // SAFETY: pointer-arg ioctl; fd valid; arg is C string (PCIe address).
    let ioctl = VfioIoctlReturn::<{ ioctls::OP_GROUP_GET_DEVICE_FD }> { arg: arg as usize };
    unsafe { rustix::ioctl::ioctl(fd, ioctl) }.map_err(ioctl_err)
}
