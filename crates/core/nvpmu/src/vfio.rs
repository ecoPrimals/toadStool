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
    device: OwnedFd,
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
            device,
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

    /// The VFIO device file descriptor for MSI-X configuration.
    #[must_use]
    pub fn device_fd(&self) -> &OwnedFd {
        &self.device
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

// ═══════════════════════════════════════════════════════════
// MSI-X interrupt support for VFIO completion notification
// ═══════════════════════════════════════════════════════════

const VFIO_DEVICE_SET_IRQS: u8 = VFIO_BASE + 10;
const OP_DEVICE_SET_IRQS: Opcode = opcode::none(VFIO_TYPE, VFIO_DEVICE_SET_IRQS);

const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 1 << 2;
const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 1 << 5;
const VFIO_PCI_MSIX_IRQ_INDEX: u32 = 2;

#[repr(C)]
struct VfioIrqSet {
    argsz: u32,
    flags: u32,
    index: u32,
    start: u32,
    count: u32,
    // followed by eventfd data (i32)
}

/// MSI-X interrupt configuration for VFIO devices.
///
/// Uses eventfd for kernel-to-userspace completion notifications,
/// replacing busy-wait polling for dispatch completion.
pub struct VfioMsixInterrupt {
    eventfd: OwnedFd,
    irq_index: u32,
}

impl VfioMsixInterrupt {
    /// Configure MSI-X interrupt on a VFIO device.
    ///
    /// Creates an eventfd and wires it to the specified MSI-X vector
    /// on the VFIO device. The eventfd becomes readable when the GPU
    /// signals completion.
    pub fn configure(device_fd: &OwnedFd, vector: u32) -> Result<Self> {
        let eventfd = create_eventfd()?;

        let fd_val = eventfd.as_raw_fd();

        // Build the VFIO_DEVICE_SET_IRQS payload: VfioIrqSet header + eventfd i32
        let argsz = (std::mem::size_of::<VfioIrqSet>() + std::mem::size_of::<i32>()) as u32;
        let mut payload = Vec::with_capacity(argsz as usize);

        let irq_set = VfioIrqSet {
            argsz,
            flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
            index: VFIO_PCI_MSIX_IRQ_INDEX,
            start: vector,
            count: 1,
        };

        // SAFETY: VfioIrqSet is repr(C); we're copying its bytes.
        payload.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                std::ptr::from_ref(&irq_set).cast::<u8>(),
                std::mem::size_of::<VfioIrqSet>(),
            )
        });
        payload.extend_from_slice(&fd_val.to_ne_bytes());

        #[allow(
            clippy::cast_ptr_alignment,
            reason = "VFIO SET_IRQS reads argsz from the header to determine actual layout"
        )]
        let ioctl = VfioPtrIoctl::<OP_DEVICE_SET_IRQS, _> {
            ptr: payload.as_mut_ptr().cast::<VfioIrqSet>(),
        };

        // SAFETY: device_fd is a valid VFIO device; payload matches kernel ABI.
        unsafe { rustix::ioctl::ioctl(device_fd.as_fd(), ioctl) }
            .map_err(|e| NvPmuError::Hardware(format!("MSI-X configure vector {vector}: {e}")))?;

        tracing::info!(vector, "MSI-X interrupt configured via eventfd");

        Ok(Self {
            eventfd,
            irq_index: vector,
        })
    }

    /// Wait for the next interrupt (blocks until GPU signals completion).
    pub fn wait(&self) -> Result<u64> {
        let mut buf = [0u8; 8];
        rustix::io::read(&self.eventfd, &mut buf)
            .map_err(|e| NvPmuError::Hardware(format!("eventfd read: {e}")))?;
        Ok(u64::from_ne_bytes(buf))
    }

    /// Wait for interrupt with timeout.
    pub fn wait_timeout(&self, timeout: std::time::Duration) -> Result<Option<u64>> {
        use rustix::event::{poll, PollFlags, PollFd};
        use rustix::time::Timespec;

        let mut pollfd = [PollFd::new(&self.eventfd, PollFlags::IN)];
        let ts = Timespec::try_from(timeout)
            .map_err(|e| NvPmuError::Hardware(format!("timeout conversion: {e}")))?;

        match poll(&mut pollfd, Some(&ts)) {
            Ok(0) => Ok(None), // timeout
            Ok(_) => self.wait().map(Some),
            Err(e) => Err(NvPmuError::Hardware(format!("poll: {e}"))),
        }
    }

    /// The IRQ vector index this interrupt is configured for.
    pub const fn vector(&self) -> u32 {
        self.irq_index
    }
}

/// Create an eventfd for interrupt notification.
fn create_eventfd() -> Result<OwnedFd> {
    use rustix::event::{eventfd, EventfdFlags};
    eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)
        .map_err(|e| NvPmuError::Hardware(format!("eventfd create: {e}")))
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
