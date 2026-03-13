// SPDX-License-Identifier: AGPL-3.0-only
//! VFIO-based BAR0 access for NVIDIA GPUs.
//!
//! Alternative to sysfs-based `Bar0Access` that works through VFIO's
//! IOMMU isolation layer. Used for the dual-use (gaming + science)
//! sovereign compute path where the GPU is bound to `vfio-pci`.
//!
//! # Prerequisites
//!
//! 1. GPU must be bound to `vfio-pci` (not nouveau/nvidia)
//! 2. IOMMU enabled in BIOS/kernel
//! 3. User in `vfio` group or root
//!
//! # Architecture
//!
//! ```text
//! VfioBar0Access
//!   ├─ /dev/vfio/vfio          (container)
//!   ├─ /dev/vfio/{group}       (IOMMU group)
//!   ├─ VFIO_GET_DEVICE_FD      (device fd)
//!   ├─ VFIO_DEVICE_GET_REGION_INFO (BAR0 offset+size)
//!   └─ mmap(device_fd, offset) (BAR0 MMIO)
//! ```

use crate::error::{NvPmuError, Result};
use rustix::ioctl::{opcode, Ioctl, IoctlOutput, Opcode};
use std::fs::OpenOptions;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};

const VFIO_TYPE: u8 = b';';
const VFIO_BASE: u8 = 100;

const OP_GET_API_VERSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE);
const OP_CHECK_EXTENSION: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 1);
const OP_SET_IOMMU: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 2);
const OP_GROUP_GET_STATUS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 3);
const OP_GROUP_SET_CONTAINER: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 4);
const OP_GROUP_GET_DEVICE_FD: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 6);
const OP_DEVICE_GET_REGION_INFO: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 8);

const VFIO_API_VERSION: i32 = 0;
const VFIO_TYPE1V2_IOMMU: u32 = 3;
const VFIO_GROUP_FLAGS_VIABLE: u32 = 1;
const BAR0_REGION_INDEX: u32 = 0;

#[repr(C)]
#[derive(Debug, Default)]
struct VfioGroupStatus {
    argsz: u32,
    flags: u32,
}

#[repr(C)]
#[derive(Debug, Default)]
struct VfioRegionInfo {
    argsz: u32,
    flags: u32,
    index: u32,
    cap_offset: u32,
    size: u64,
    offset: u64,
}

struct VfioReturnIoctl<const OP: Opcode> {
    arg: usize,
}

// SAFETY: VFIO no-arg or integer-arg ioctl; opcode is compile-time constant.
unsafe impl<const OP: Opcode> Ioctl for VfioReturnIoctl<OP> {
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
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(out)
    }
}

struct VfioPtrIoctl<const OP: Opcode, T> {
    ptr: *mut T,
}

// SAFETY: VFIO struct ioctl; T is repr(C) matching kernel ABI.
unsafe impl<const OP: Opcode, T> Ioctl for VfioPtrIoctl<OP, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        OP
    }
    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.cast()
    }
    unsafe fn output_from_ptr(
        _: IoctlOutput,
        _: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

fn ioctl_err(op: &str, e: rustix::io::Errno) -> NvPmuError {
    NvPmuError::Hardware(format!("VFIO {op}: {e}"))
}

/// VFIO-based BAR0 MMIO access for NVIDIA GPUs.
///
/// This is the sovereign compute path: the GPU is bound to `vfio-pci`,
/// giving userspace full BAR0 MMIO access through IOMMU isolation.
pub struct VfioBar0Access {
    bdf: String,
    base_ptr: *mut u8,
    region_size: usize,
    _container: std::fs::File,
    _group: std::fs::File,
    _device: OwnedFd,
}

// SAFETY: All mutable access is via &mut self methods; mmap region is process-private.
unsafe impl Send for VfioBar0Access {}

impl VfioBar0Access {
    /// Open BAR0 for a VFIO-bound NVIDIA GPU.
    ///
    /// # Errors
    ///
    /// Returns error if VFIO setup fails (GPU not bound, IOMMU disabled, etc.).
    #[allow(clippy::cast_possible_truncation)]
    pub fn open(bdf: &str) -> Result<Self> {
        let iommu_group = find_iommu_group(bdf)?;
        tracing::info!(bdf, iommu_group, "opening VFIO BAR0");

        let container = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/vfio/vfio")
            .map_err(|e| NvPmuError::Hardware(format!("/dev/vfio/vfio: {e}")))?;

        let api_version = {
            let ioctl = VfioReturnIoctl::<OP_GET_API_VERSION> { arg: 0 };
            // SAFETY: container fd from valid open; no-arg ioctl.
            unsafe { rustix::ioctl::ioctl(container.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("GET_API_VERSION", e))?
        };
        if api_version != VFIO_API_VERSION {
            return Err(NvPmuError::Hardware(format!(
                "VFIO API version mismatch: got {api_version}, expected {VFIO_API_VERSION}"
            )));
        }

        let has_type1 = {
            let ioctl = VfioReturnIoctl::<OP_CHECK_EXTENSION> {
                arg: VFIO_TYPE1V2_IOMMU as usize,
            };
            // SAFETY: container fd valid; arg is extension id.
            unsafe { rustix::ioctl::ioctl(container.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("CHECK_EXTENSION", e))?
        };
        if has_type1 != 1 {
            return Err(NvPmuError::Hardware(
                "VFIO Type1v2 IOMMU not supported".into(),
            ));
        }

        let group_path = format!("/dev/vfio/{iommu_group}");
        let group = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&group_path)
            .map_err(|e| NvPmuError::Hardware(format!("{group_path}: {e}")))?;

        let mut group_status = VfioGroupStatus {
            argsz: std::mem::size_of::<VfioGroupStatus>() as u32,
            flags: 0,
        };
        {
            let ioctl = VfioPtrIoctl::<OP_GROUP_GET_STATUS, _> {
                ptr: std::ptr::from_mut(&mut group_status),
            };
            // SAFETY: group fd from valid open; struct has correct argsz.
            unsafe { rustix::ioctl::ioctl(group.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("GROUP_GET_STATUS", e))?;
        }

        if (group_status.flags & VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(NvPmuError::Hardware(
                "VFIO group not viable — all devices must be bound to vfio-pci".into(),
            ));
        }

        let container_fd = container.as_raw_fd();
        {
            let ioctl = VfioReturnIoctl::<OP_GROUP_SET_CONTAINER> {
                arg: std::ptr::from_ref(&container_fd) as usize,
            };
            // SAFETY: group fd valid; arg points to container fd.
            unsafe { rustix::ioctl::ioctl(group.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("GROUP_SET_CONTAINER", e))?;
        }

        {
            let ioctl = VfioReturnIoctl::<OP_SET_IOMMU> {
                arg: VFIO_TYPE1V2_IOMMU as usize,
            };
            // SAFETY: container fd valid; arg is IOMMU type.
            unsafe { rustix::ioctl::ioctl(container.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("SET_IOMMU", e))?;
        }

        let bdf_cstr = std::ffi::CString::new(bdf)
            .map_err(|e| NvPmuError::Hardware(format!("Invalid BDF: {e}")))?;
        let device_fd = {
            let ioctl = VfioReturnIoctl::<OP_GROUP_GET_DEVICE_FD> {
                arg: bdf_cstr.as_ptr() as usize,
            };
            // SAFETY: group fd valid; arg is C string BDF address.
            unsafe { rustix::ioctl::ioctl(group.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("GROUP_GET_DEVICE_FD", e))?
        };
        // SAFETY: kernel returns a valid fd on success.
        let device = unsafe { OwnedFd::from_raw_fd(device_fd) };

        let mut region_info = VfioRegionInfo {
            argsz: std::mem::size_of::<VfioRegionInfo>() as u32,
            index: BAR0_REGION_INDEX,
            ..Default::default()
        };
        {
            let ioctl = VfioPtrIoctl::<OP_DEVICE_GET_REGION_INFO, _> {
                ptr: std::ptr::from_mut(&mut region_info),
            };
            // SAFETY: device fd valid; struct has correct argsz and index.
            unsafe { rustix::ioctl::ioctl(device.as_fd(), ioctl) }
                .map_err(|e| ioctl_err("DEVICE_GET_REGION_INFO", e))?;
        }

        if region_info.size == 0 {
            return Err(NvPmuError::Hardware("BAR0 region has size 0".into()));
        }

        let region_size = region_info.size as usize;

        // SAFETY: device fd valid; region offset from kernel; size verified non-zero;
        // MAP_SHARED for MMIO semantics; ProtFlags R|W for register access.
        let base_ptr = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                region_size,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                &device,
                region_info.offset,
            )
            .map_err(|e| NvPmuError::Hardware(format!("BAR0 mmap failed: {e}")))?
        }
        .cast::<u8>();

        tracing::info!(
            bdf,
            region_size,
            "VFIO BAR0 mapped ({region_size:#x} bytes)"
        );

        Ok(Self {
            bdf: bdf.to_string(),
            base_ptr,
            region_size,
            _container: container,
            _group: group,
            _device: device,
        })
    }

    /// PCI BDF address of the mapped GPU.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Size of the BAR0 MMIO region in bytes.
    #[must_use]
    pub const fn region_size(&self) -> usize {
        self.region_size
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "BAR0 offsets and sizes are always within usize on 64-bit targets"
    )]
    fn check_offset(&self, offset: u64) -> Result<()> {
        if (offset as usize) + 4 > self.region_size {
            return Err(NvPmuError::Hardware(format!(
                "BAR0 offset {offset:#x} out of range (size {:#x})",
                self.region_size
            )));
        }
        Ok(())
    }

    /// Read a 32-bit register at a BAR0-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of range.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_ptr_alignment,
        reason = "BAR0 offsets are u32-aligned by hardware spec; truncation safe on 64-bit"
    )]
    pub fn read_u32(&self, offset: u64) -> Result<u32> {
        self.check_offset(offset)?;
        // SAFETY: base_ptr valid from mmap; offset bounds-checked above; volatile for MMIO.
        let val =
            unsafe { std::ptr::read_volatile(self.base_ptr.add(offset as usize).cast::<u32>()) };
        Ok(val)
    }

    /// Write a 32-bit register at a BAR0-relative offset.
    ///
    /// # Errors
    /// Returns error if offset is out of range.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_ptr_alignment,
        reason = "BAR0 offsets are u32-aligned by hardware spec; truncation safe on 64-bit"
    )]
    pub fn write_u32(&mut self, offset: u64, value: u32) -> Result<()> {
        self.check_offset(offset)?;
        // SAFETY: base_ptr valid from mmap; offset bounds-checked; volatile for MMIO; &mut self.
        unsafe {
            std::ptr::write_volatile(self.base_ptr.add(offset as usize).cast::<u32>(), value);
        }
        Ok(())
    }
}

impl hw_learn::applicator::RegisterAccess for VfioBar0Access {
    fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
        Self::read_u32(self, offset).map_err(|e| e.to_string())
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
        Self::write_u32(self, offset, value).map_err(|e| e.to_string())
    }
}

impl Drop for VfioBar0Access {
    fn drop(&mut self) {
        // SAFETY: base_ptr from mmap in open(); region_size unchanged.
        unsafe {
            let _ = rustix::mm::munmap(self.base_ptr.cast(), self.region_size);
        }
        tracing::debug!(bdf = %self.bdf, "VFIO BAR0 unmapped");
    }
}

fn find_iommu_group(bdf: &str) -> Result<u32> {
    let path = format!("/sys/bus/pci/devices/{bdf}/iommu_group");
    let link = std::fs::read_link(&path).map_err(|e| {
        NvPmuError::Hardware(format!(
            "Cannot read IOMMU group for {bdf}: {e}. Is IOMMU enabled and GPU bound to vfio-pci?"
        ))
    })?;

    let group_str = link
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| NvPmuError::Hardware("Invalid IOMMU group path".into()))?;

    group_str
        .parse::<u32>()
        .map_err(|e| NvPmuError::Hardware(format!("Invalid IOMMU group number: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_iommu_group_nonexistent() {
        let result = find_iommu_group("9999:99:99.9");
        assert!(result.is_err());
    }

    #[test]
    fn vfio_constants_match_kernel() {
        assert_eq!(VFIO_API_VERSION, 0);
        assert_eq!(VFIO_TYPE1V2_IOMMU, 3);
        assert_eq!(VFIO_GROUP_FLAGS_VIABLE, 1);
        assert_eq!(BAR0_REGION_INDEX, 0);
    }

    #[test]
    fn region_info_layout_is_repr_c() {
        let info = VfioRegionInfo::default();
        assert_eq!(info.size, 0);
        assert_eq!(info.offset, 0);
        assert!(std::mem::size_of::<VfioRegionInfo>() >= 32);
    }
}
