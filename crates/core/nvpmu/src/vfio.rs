// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::doc_markdown,
    reason = "VFIO ioctl names (VFIO_GET_API_VERSION, etc.) are kernel ABI identifiers"
)]
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
use rustix::ioctl::{Ioctl, IoctlOutput, Opcode, opcode};
use std::fs::OpenOptions;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use toadstool_common::constants::platform_paths::devfs;
use toadstool_hw_safe::vfio_setup::{
    self, VFIO_API_VERSION, VFIO_GROUP_FLAGS_VIABLE, VFIO_TYPE1V2_IOMMU,
};
use toadstool_hw_safe::{DeviceMmap, VolatileMmio};

const BAR0_REGION_INDEX: u32 = 0;

fn vfio_err(op: &str, e: &std::io::Error) -> NvPmuError {
    NvPmuError::Hardware(format!("VFIO {op}: {e}"))
}

/// VFIO-based BAR0 MMIO access for NVIDIA GPUs.
///
/// This is the sovereign compute path: the GPU is bound to `vfio-pci`,
/// giving userspace full BAR0 MMIO access through IOMMU isolation.
pub struct VfioBar0Access {
    bdf: String,
    bar0: DeviceMmap,
    _container: std::fs::File,
    _group: std::fs::File,
    device: OwnedFd,
}

impl VfioBar0Access {
    /// Open BAR0 for a VFIO-bound NVIDIA GPU.
    ///
    /// # Errors
    ///
    /// Returns error if VFIO setup fails (GPU not bound, IOMMU disabled, etc.).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )]
    pub fn open(bdf: &str) -> Result<Self> {
        let iommu_group = find_iommu_group(bdf)?;
        tracing::info!(bdf, iommu_group, "opening VFIO BAR0");

        let container = OpenOptions::new()
            .read(true)
            .write(true)
            .open(devfs::VFIO_CONTAINER)
            .map_err(|e| NvPmuError::Hardware(format!("{}: {e}", devfs::VFIO_CONTAINER)))?;

        let api_version = vfio_setup::get_api_version(container.as_fd())
            .map_err(|e| vfio_err("GET_API_VERSION", &e))?;
        if api_version != VFIO_API_VERSION {
            return Err(NvPmuError::Hardware(format!(
                "VFIO API version mismatch: got {api_version}, expected {VFIO_API_VERSION}"
            )));
        }

        if vfio_setup::check_extension(container.as_fd(), VFIO_TYPE1V2_IOMMU)
            .map_err(|e| vfio_err("CHECK_EXTENSION", &e))?
            != 1
        {
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

        let group_status = vfio_setup::group_get_status(group.as_fd())
            .map_err(|e| vfio_err("GROUP_GET_STATUS", &e))?;
        if (group_status.flags & VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(NvPmuError::Hardware(
                "VFIO group not viable — all devices must be bound to vfio-pci".into(),
            ));
        }

        vfio_setup::group_set_container(group.as_fd(), &container)
            .map_err(|e| vfio_err("GROUP_SET_CONTAINER", &e))?;
        vfio_setup::set_iommu(container.as_fd(), VFIO_TYPE1V2_IOMMU)
            .map_err(|e| vfio_err("SET_IOMMU", &e))?;

        let bdf_cstr = std::ffi::CString::new(bdf)
            .map_err(|e| NvPmuError::Hardware(format!("Invalid BDF: {e}")))?;
        let device = vfio_setup::group_get_device_fd(group.as_fd(), &bdf_cstr)
            .map_err(|e| vfio_err("GROUP_GET_DEVICE_FD", &e))?;

        let region_info = vfio_setup::device_get_region_info(device.as_fd(), BAR0_REGION_INDEX)
            .map_err(|e| vfio_err("DEVICE_GET_REGION_INFO", &e))?;

        if region_info.size == 0 {
            return Err(NvPmuError::Hardware("BAR0 region has size 0".into()));
        }

        let region_size = region_info.size as usize;

        let bar0 = DeviceMmap::map_shared_rw(&device, region_info.offset, region_size)
            .map_err(|e| NvPmuError::Hardware(format!("BAR0 mmap failed: {e}")))?;

        tracing::info!(
            bdf,
            region_size,
            "VFIO BAR0 mapped ({region_size:#x} bytes)"
        );

        Ok(Self {
            bdf: bdf.to_string(),
            bar0,
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
    pub fn region_size(&self) -> usize {
        self.bar0.size()
    }

    /// The VFIO device file descriptor for MSI-X configuration.
    #[must_use]
    pub const fn device_fd(&self) -> &OwnedFd {
        &self.device
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "BAR0 offsets and sizes are always within usize on 64-bit targets"
    )]
    fn check_offset(&self, offset: u64) -> Result<()> {
        if (offset as usize) + 4 > self.bar0.size() {
            return Err(NvPmuError::Hardware(format!(
                "BAR0 offset {offset:#x} out of range (size {:#x})",
                self.bar0.size()
            )));
        }
        Ok(())
    }

    /// Volatile MMIO view over the BAR0 mapping (same lifetime as `self`).
    fn mmio(&self) -> VolatileMmio<'_> {
        self.bar0.as_volatile()
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
        self.mmio()
            .read_u32(offset as usize)
            .map_err(|e| NvPmuError::Hardware(e.to_string()))
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
        self.mmio()
            .write_u32(offset as usize, value)
            .map_err(|e| NvPmuError::Hardware(e.to_string()))
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

// ═══════════════════════════════════════════════════════════
// MSI-X interrupt support for VFIO completion notification
// ═══════════════════════════════════════════════════════════

use toadstool_hw_safe::vfio_dma::{VFIO_BASE, VFIO_TYPE};
const OP_DEVICE_SET_IRQS: Opcode = opcode::none(VFIO_TYPE, VFIO_BASE + 10);

struct VfioPtrIoctl<const OP: Opcode, T> {
    ptr: *mut T,
}

// SAFETY:
// - `T` is `#[repr(C)]` and matches the kernel’s userspace layout for this `OP`
//   (here: VFIO’s `struct vfio_irq_set` plus trailing `i32` eventfd per `argsz`).
// - `as_ptr` passes the buffer the kernel reads for this ioctl; `opcode()` matches
//   VFIO’s registered command.
// - `IS_MUTATING = true`: the kernel may update userspace-visible state for this
//   VFIO command path (conservative; avoids mis-optimization in rustix).
// - `output_from_ptr`: for this ioctl the Rust [`Ioctl::Output`] is `()`; the kernel
//   does not marshal a separate return value through `extract_output` beyond the
//   standard ioctl result. After a successful syscall, callers do not read `T` back
//   through this hook (VFIO consumed the buffer per `argsz`).
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
        _ioctl_ret: IoctlOutput,
        extract_output: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        // `extract_output` is the same address passed to `ioctl` (see rustix `Ioctl`).
        debug_assert!(
            !extract_output.is_null(),
            "rustix always passes the ioctl buffer pointer"
        );
        Ok(())
    }
}

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
}

/// Kernel `vfio_irq_set` plus trailing `i32` eventfd (see VFIO `DEVICE_SET_IRQS`).
#[repr(C)]
struct VfioIrqSetPayload {
    irq_set: VfioIrqSet,
    eventfd: i32,
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
    ///
    /// # Errors
    ///
    /// Returns error if eventfd creation or VFIO `SET_IRQS` ioctl fails.
    pub fn configure(device_fd: &OwnedFd, vector: u32) -> Result<Self> {
        let eventfd = create_eventfd()?;

        let fd_val = eventfd.as_raw_fd();

        #[expect(
            clippy::cast_possible_truncation,
            reason = "compile-time struct sizes always fit u32"
        )]
        let argsz = std::mem::size_of::<VfioIrqSetPayload>() as u32;

        let mut payload = VfioIrqSetPayload {
            irq_set: VfioIrqSet {
                argsz,
                flags: VFIO_IRQ_SET_DATA_EVENTFD | VFIO_IRQ_SET_ACTION_TRIGGER,
                index: VFIO_PCI_MSIX_IRQ_INDEX,
                start: vector,
                count: 1,
            },
            eventfd: fd_val,
        };

        if device_fd.as_raw_fd() < 0 {
            return Err(NvPmuError::Hardware(
                "VFIO device fd is invalid (negative)".into(),
            ));
        }

        debug_assert_eq!(
            payload.irq_set.argsz as usize,
            std::mem::size_of::<VfioIrqSetPayload>(),
            "VFIO argsz must cover header + eventfd"
        );

        let ioctl = VfioPtrIoctl::<OP_DEVICE_SET_IRQS, _> {
            ptr: std::ptr::addr_of_mut!(payload),
        };

        // SAFETY: `device_fd` is an open VFIO device (`OwnedFd`). `payload` is a
        // correctly sized, aligned `repr(C)` struct matching the kernel ABI for
        // `VFIO_DEVICE_SET_IRQS` with eventfd data.
        unsafe { rustix::ioctl::ioctl(device_fd.as_fd(), ioctl) }
            .map_err(|e| NvPmuError::Hardware(format!("MSI-X configure vector {vector}: {e}")))?;

        tracing::info!(vector, "MSI-X interrupt configured via eventfd");

        Ok(Self {
            eventfd,
            irq_index: vector,
        })
    }

    /// Wait for the next interrupt (blocks until GPU signals completion).
    ///
    /// # Errors
    ///
    /// Returns error if the eventfd read fails.
    pub fn wait(&self) -> Result<u64> {
        let mut buf = [0u8; 8];
        rustix::io::read(&self.eventfd, &mut buf)
            .map_err(|e| NvPmuError::Hardware(format!("eventfd read: {e}")))?;
        Ok(u64::from_ne_bytes(buf))
    }

    /// Wait for interrupt with timeout.
    ///
    /// # Errors
    ///
    /// Returns error if timeout conversion, poll, or eventfd read fails.
    pub fn wait_timeout(&self, timeout: std::time::Duration) -> Result<Option<u64>> {
        use rustix::event::{PollFd, PollFlags, poll};
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
    #[must_use]
    pub const fn vector(&self) -> u32 {
        self.irq_index
    }
}

/// Create an eventfd for interrupt notification.
fn create_eventfd() -> Result<OwnedFd> {
    use rustix::event::{EventfdFlags, eventfd};
    eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)
        .map_err(|e| NvPmuError::Hardware(format!("eventfd create: {e}")))
}

fn find_iommu_group(bdf: &str) -> Result<u32> {
    let path = toadstool_common::sysfs_paths::sysfs_pci_device_file(bdf, "iommu_group");
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
        assert_eq!(vfio_setup::VFIO_API_VERSION, 0);
        assert_eq!(vfio_setup::VFIO_TYPE1V2_IOMMU, 3);
        assert_eq!(vfio_setup::VFIO_GROUP_FLAGS_VIABLE, 1);
        assert_eq!(BAR0_REGION_INDEX, 0);
    }

    #[test]
    fn region_info_layout_is_repr_c() {
        let info = vfio_setup::VfioRegionInfo::default();
        assert_eq!(info.size, 0);
        assert_eq!(info.offset, 0);
        assert!(std::mem::size_of::<vfio_setup::VfioRegionInfo>() >= 32);
    }
}
