// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "VFIO ioctls are kernel FFI — containment zone")]

//! Shared VFIO setup operations — ioctl wrappers and kernel ABI types.
//!
//! Provides a single implementation of the VFIO container, group, and device
//! setup ioctls so that `nvpmu` and `akida-driver` share the same kernel ABI
//! structs and unsafe ioctl wrappers instead of duplicating them.
//!
//! Types and constants are available on all platforms. Ioctl functions are
//! Linux-only (gated internally).

#[cfg(target_os = "linux")]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

#[cfg(target_os = "linux")]
use rustix::ioctl::{self, Ioctl, IoctlOutput, Opcode, opcode};

// ── VFIO ioctl opcodes (from Linux UAPI) ──────────────────────────────

#[cfg(target_os = "linux")]
const VFIO_TYPE: u8 = b';';
#[cfg(target_os = "linux")]
const VFIO_BASE: u8 = 100;

#[cfg(target_os = "linux")]
const OP_GET_API_VERSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE);
#[cfg(target_os = "linux")]
const OP_CHECK_EXTENSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 1);
#[cfg(target_os = "linux")]
const OP_SET_IOMMU: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 2);
#[cfg(target_os = "linux")]
const OP_GROUP_GET_STATUS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 3);
#[cfg(target_os = "linux")]
const OP_GROUP_SET_CONTAINER: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 4);
#[cfg(target_os = "linux")]
const OP_GROUP_GET_DEVICE_FD: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 6);
#[cfg(target_os = "linux")]
const OP_DEVICE_GET_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 7);
#[cfg(target_os = "linux")]
const OP_DEVICE_GET_REGION_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 8);
#[cfg(target_os = "linux")]
const OP_DEVICE_RESET: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 11);

// ── Public ABI constants ──────────────────────────────────────────────

/// Expected VFIO API version (always 0 per the kernel).
pub const VFIO_API_VERSION: i32 = 0;

/// Type1v2 IOMMU — the standard IOMMU type for VFIO on x86/ARM.
pub const VFIO_TYPE1V2_IOMMU: u32 = 3;

/// Group is viable (all devices bound to vfio-pci).
pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1;

// ── Kernel ABI structs ────────────────────────────────────────────────

/// VFIO group status (from `VFIO_GROUP_GET_STATUS` ioctl).
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioGroupStatus {
    /// Struct size for kernel version negotiation.
    pub argsz: u32,
    /// Group status flags (check [`VFIO_GROUP_FLAGS_VIABLE`]).
    pub flags: u32,
}

/// VFIO device info (from `VFIO_DEVICE_GET_INFO` ioctl).
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioDeviceInfo {
    /// Struct size for kernel version negotiation.
    pub argsz: u32,
    /// Device flags.
    pub flags: u32,
    /// Number of regions (BARs).
    pub num_regions: u32,
    /// Number of IRQ types.
    pub num_irqs: u32,
}

/// VFIO region info (from `VFIO_DEVICE_GET_REGION_INFO` ioctl).
#[repr(C)]
#[derive(Debug, Default)]
pub struct VfioRegionInfo {
    /// Struct size for kernel version negotiation.
    pub argsz: u32,
    /// Region flags (capabilities, permissions).
    pub flags: u32,
    /// Region index (BAR number).
    pub index: u32,
    /// Offset to extended capabilities.
    pub cap_offset: u32,
    /// Size of the region in bytes.
    pub size: u64,
    /// Offset for mmap from the device fd.
    pub offset: u64,
}

// ── Generic ioctl adapters ────────────────────────────────────────────

#[cfg(target_os = "linux")]
/// Ioctl adapter for VFIO commands that return an i32.
struct VfioReturnIoctl<const OP: Opcode> {
    arg: usize,
}

#[cfg(target_os = "linux")]
// SAFETY: VFIO no-arg or integer-arg ioctl; opcode is compile-time constant.
// output_from_ptr wraps kernel return code without pointer dereference.
unsafe impl<const OP: Opcode> Ioctl for VfioReturnIoctl<OP> {
    type Output = i32;
    const IS_MUTATING: bool = false;

    fn opcode(&self) -> Opcode {
        OP
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.arg as *mut std::ffi::c_void
    }
    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        out: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        // SAFETY: `out` is rustix's ioctl syscall return; `ptr` is unused (no dereference).
        Ok(out)
    }
}

#[cfg(target_os = "linux")]
/// Ioctl adapter for VFIO commands that read/write a repr(C) struct.
struct VfioPtrIoctl<const OP: Opcode, T> {
    ptr: *mut T,
}

#[cfg(target_os = "linux")]
// SAFETY: opcode is compile-time VFIO constant; T is repr(C) matching kernel ABI.
// IS_MUTATING=true since the kernel writes back into the struct.
unsafe impl<const OP: Opcode, T> Ioctl for VfioPtrIoctl<OP, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }
    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        // SAFETY: No-op — body is intentionally empty (trait method stub for ioctl infrastructure).
        Ok(())
    }
}

// ── Public ioctl wrappers ─────────────────────────────────────────────
//
// Each function encapsulates one unsafe ioctl call. Callers deal only
// with safe Rust types. Errors are returned as `std::io::Result`.

#[cfg(target_os = "linux")]
fn io_err(e: rustix::io::Errno) -> std::io::Error {
    e.into()
}

#[cfg(target_os = "linux")]
fn do_ioctl<I: Ioctl>(fd: BorrowedFd<'_>, cmd: I) -> std::io::Result<I::Output> {
    // SAFETY: all callers in this module construct `cmd` from VFIO kernel-ABI
    // types with compile-time opcodes. `fd` comes from the caller's valid
    // open VFIO container/group/device file descriptor.
    unsafe { ioctl::ioctl(fd, cmd) }.map_err(io_err)
}

/// `VFIO_GET_API_VERSION` — returns the VFIO API version (should be 0).
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn get_api_version(container: BorrowedFd<'_>) -> std::io::Result<i32> {
    do_ioctl(container, VfioReturnIoctl::<OP_GET_API_VERSION> { arg: 0 })
}

/// `VFIO_CHECK_EXTENSION` — check if an IOMMU extension is supported.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn check_extension(container: BorrowedFd<'_>, extension: u32) -> std::io::Result<i32> {
    do_ioctl(
        container,
        VfioReturnIoctl::<OP_CHECK_EXTENSION> {
            arg: extension as usize,
        },
    )
}

/// `VFIO_SET_IOMMU` — attach an IOMMU type to the container.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn set_iommu(container: BorrowedFd<'_>, iommu_type: u32) -> std::io::Result<()> {
    do_ioctl(
        container,
        VfioReturnIoctl::<OP_SET_IOMMU> {
            arg: iommu_type as usize,
        },
    )?;
    Ok(())
}

/// `VFIO_GROUP_GET_STATUS` — query group viability.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
pub fn group_get_status(group: BorrowedFd<'_>) -> std::io::Result<VfioGroupStatus> {
    let mut status = VfioGroupStatus {
        argsz: std::mem::size_of::<VfioGroupStatus>() as u32,
        flags: 0,
    };
    do_ioctl(
        group,
        VfioPtrIoctl::<OP_GROUP_GET_STATUS, _> {
            ptr: std::ptr::from_mut(&mut status),
        },
    )?;
    Ok(status)
}

/// `VFIO_GROUP_SET_CONTAINER` — associate a group with a container.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn group_set_container(group: BorrowedFd<'_>, container: impl AsFd) -> std::io::Result<()> {
    let container_fd = container.as_fd().as_raw_fd();
    do_ioctl(
        group,
        VfioReturnIoctl::<OP_GROUP_SET_CONTAINER> {
            arg: std::ptr::from_ref(&container_fd) as usize,
        },
    )?;
    Ok(())
}

/// `VFIO_GROUP_GET_DEVICE_FD` — get an owned file descriptor for a device.
///
/// Returns an [`OwnedFd`] that the kernel allocated for the device.
/// The caller receives exclusive ownership.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn group_get_device_fd(
    group: BorrowedFd<'_>,
    bdf: &std::ffi::CStr,
) -> std::io::Result<OwnedFd> {
    let raw = do_ioctl(
        group,
        VfioReturnIoctl::<OP_GROUP_GET_DEVICE_FD> {
            arg: bdf.as_ptr() as usize,
        },
    )?;
    // SAFETY: kernel returns a valid fd on success.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

/// `VFIO_DEVICE_GET_INFO` — query device region/IRQ counts.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
pub fn device_get_info(device: BorrowedFd<'_>) -> std::io::Result<VfioDeviceInfo> {
    let mut info = VfioDeviceInfo {
        argsz: std::mem::size_of::<VfioDeviceInfo>() as u32,
        ..Default::default()
    };
    do_ioctl(
        device,
        VfioPtrIoctl::<OP_DEVICE_GET_INFO, _> {
            ptr: std::ptr::from_mut(&mut info),
        },
    )?;
    Ok(info)
}

/// `VFIO_DEVICE_GET_REGION_INFO` — query BAR size, offset, and flags.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
pub fn device_get_region_info(
    device: BorrowedFd<'_>,
    index: u32,
) -> std::io::Result<VfioRegionInfo> {
    let mut info = VfioRegionInfo {
        argsz: std::mem::size_of::<VfioRegionInfo>() as u32,
        index,
        ..Default::default()
    };
    do_ioctl(
        device,
        VfioPtrIoctl::<OP_DEVICE_GET_REGION_INFO, _> {
            ptr: std::ptr::from_mut(&mut info),
        },
    )?;
    Ok(info)
}

/// `VFIO_DEVICE_RESET` — reset a device through the VFIO subsystem.
///
/// Not all devices support this; the kernel returns an error for
/// devices without reset capability.
///
/// # Errors
///
/// Returns I/O error if the ioctl fails.
#[cfg(target_os = "linux")]
pub fn device_reset(device: BorrowedFd<'_>) -> std::io::Result<()> {
    do_ioctl(device, VfioReturnIoctl::<OP_DEVICE_RESET> { arg: 0 })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_constants() {
        assert_eq!(VFIO_API_VERSION, 0);
        assert_eq!(VFIO_TYPE1V2_IOMMU, 3);
        assert_eq!(VFIO_GROUP_FLAGS_VIABLE, 1);
    }

    #[test]
    fn group_status_layout() {
        assert_eq!(
            std::mem::size_of::<VfioGroupStatus>(),
            8,
            "VfioGroupStatus must be 8 bytes"
        );
    }

    #[test]
    fn device_info_layout() {
        assert!(
            std::mem::size_of::<VfioDeviceInfo>() >= 16,
            "VfioDeviceInfo must be at least 16 bytes"
        );
    }

    #[test]
    fn region_info_layout() {
        assert!(
            std::mem::size_of::<VfioRegionInfo>() >= 32,
            "VfioRegionInfo must be at least 32 bytes"
        );
    }

    #[test]
    fn region_info_default() {
        let info = VfioRegionInfo::default();
        assert_eq!(info.size, 0);
        assert_eq!(info.offset, 0);
    }
}
