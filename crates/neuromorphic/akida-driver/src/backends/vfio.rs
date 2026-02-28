//! VFIO NPU backend — Pure Rust with DMA support
//!
//! This backend uses Linux VFIO (Virtual Function I/O) to provide:
//!
// FFI/ioctl casts are intentional - VFIO API requires specific types
#![allow(clippy::cast_possible_truncation)]
//! - DMA transfers (fast bulk data movement)
//! - Interrupt support (no polling)
//! - IOMMU isolation (security)
//! - Pure Rust implementation (no C kernel module)
//!
//! # Requirements
//!
//! 1. IOMMU enabled in BIOS and kernel (`intel_iommu=on` or `amd_iommu=on`)
//! 2. Device unbound from native driver and bound to `vfio-pci`
//! 3. User in `vfio` group or root permissions
//!
//! # Setup Commands
//!
//! ```bash
//! # Unbind from native driver
//! echo "0000:a1:00.0" > /sys/bus/pci/drivers/akida/unbind
//!
//! # Bind to vfio-pci
//! echo "1e7c bca1" > /sys/bus/pci/drivers/vfio-pci/new_id
//!
//! # Grant user access
//! sudo chown $USER /dev/vfio/$IOMMU_GROUP
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  User App   │────▶│  VFIO API   │────▶│   IOMMU     │
//! │  (Rust)     │     │  (Rust)     │     │  (Hardware) │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!                            │                   │
//!                            ▼                   ▼
//!                     ┌─────────────┐     ┌─────────────┐
//!                     │  DMA Buffer │────▶│   Akida     │
//!                     │  (Pinned)   │     │   NPU       │
//!                     └─────────────┘     └─────────────┘
//! ```
//!
//! # Deep Debt Compliance
//!
//! - Runtime discovery (IOMMU groups, device capabilities)
//! - Minimal unsafe (well-encapsulated VFIO ioctls)
//! - Safe public API
//! - No C dependencies for mmap/mlock (pure Rust via rustix)
//! - VFIO ioctls use libc: rustix::ioctl requires Ioctl trait impl per variant;
//!   VFIO has 9+ ioctls with varied semantics (int, struct, fd ptr, C string).

use super::read_hwmon_power;
use crate::backend::{BackendType, ModelHandle, NpuBackend};
use crate::capabilities::Capabilities;
use crate::error::{AkidaError, Result};
use crate::mmio::{regs, Bar, MappedRegion};
use rustix::mm::{mlock, munlock};
use std::fs::OpenOptions;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::io::{AsRawFd, RawFd};

/// Parameters for polling a status register.
#[derive(Clone, Copy)]
struct PollConfig<'a> {
    reg: usize,
    done_mask: u32,
    error_mask: u32,
    max_polls: u32,
    yield_interval: u32,
    timeout_msg: &'a str,
    error_msg: &'a str,
}

/// VFIO ioctl numbers (from Linux kernel headers)
///
/// These are calculated as: _IO(';', base + offset)
/// where _IO is: ((type as u64) << 8) | nr
mod ioctls {
    use std::os::raw::c_ulong;

    /// Helper to create ioctl number: _IO(type, nr) = (type << 8) | nr
    const fn io(ty: u8, nr: u8) -> c_ulong {
        ((ty as c_ulong) << 8) | (nr as c_ulong)
    }

    pub const VFIO_TYPE: u8 = b';';
    pub const VFIO_BASE: u8 = 100;

    // VFIO container ioctls
    pub const VFIO_GET_API_VERSION: c_ulong = io(VFIO_TYPE, VFIO_BASE);
    pub const VFIO_CHECK_EXTENSION: c_ulong = io(VFIO_TYPE, VFIO_BASE + 1);
    pub const VFIO_SET_IOMMU: c_ulong = io(VFIO_TYPE, VFIO_BASE + 2);

    // VFIO group ioctls
    pub const VFIO_GROUP_GET_STATUS: c_ulong = io(VFIO_TYPE, VFIO_BASE + 3);
    pub const VFIO_GROUP_SET_CONTAINER: c_ulong = io(VFIO_TYPE, VFIO_BASE + 4);
    pub const VFIO_GROUP_GET_DEVICE_FD: c_ulong = io(VFIO_TYPE, VFIO_BASE + 6);

    // VFIO device ioctls
    pub const VFIO_DEVICE_GET_INFO: c_ulong = io(VFIO_TYPE, VFIO_BASE + 7);
    #[allow(dead_code)] // For future region queries
    pub const VFIO_DEVICE_GET_REGION_INFO: c_ulong = io(VFIO_TYPE, VFIO_BASE + 8);
    #[allow(dead_code)] // For future IRQ support
    pub const VFIO_DEVICE_GET_IRQ_INFO: c_ulong = io(VFIO_TYPE, VFIO_BASE + 9);
    #[allow(dead_code)] // For future IRQ support
    pub const VFIO_DEVICE_SET_IRQS: c_ulong = io(VFIO_TYPE, VFIO_BASE + 10);
    #[allow(dead_code)] // For future device reset
    pub const VFIO_DEVICE_RESET: c_ulong = io(VFIO_TYPE, VFIO_BASE + 11);

    // IOMMU DMA mapping
    pub const VFIO_IOMMU_MAP_DMA: c_ulong = io(VFIO_TYPE, VFIO_BASE + 13);
    pub const VFIO_IOMMU_UNMAP_DMA: c_ulong = io(VFIO_TYPE, VFIO_BASE + 14);

    // API version
    pub const VFIO_API_VERSION: i32 = 0;

    // IOMMU types
    #[allow(dead_code)] // Type1 v1
    pub const VFIO_TYPE1_IOMMU: u32 = 1;
    pub const VFIO_TYPE1V2_IOMMU: u32 = 3;

    // Group status flags
    pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1 << 0;
    #[allow(dead_code)] // For status checking
    pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 1 << 1;

    // DMA map flags
    pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1 << 0;
    pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 1 << 1;
}

/// Safe wrapper for VFIO ioctls that return an int (no arg).
///
/// # Errors
///
/// Returns `Err` if the kernel returns a negative value (errno).
#[inline]
fn vfio_ioctl_int(fd: BorrowedFd<'_>, op: std::os::raw::c_ulong) -> Result<i32> {
    let ret = unsafe {
        // SAFETY: VFIO ioctl with no arg. fd valid from caller; op is VFIO constant.
        libc::ioctl(fd.as_raw_fd(), op as _, 0)
    };
    if ret < 0 {
        Err(AkidaError::capability_query_failed(format!(
            "ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(ret)
    }
}

/// Safe wrapper for VFIO ioctls that take a u32 arg and return int.
#[inline]
fn vfio_ioctl_int_arg(fd: BorrowedFd<'_>, op: std::os::raw::c_ulong, arg: u32) -> Result<i32> {
    let ret = unsafe {
        // SAFETY: VFIO ioctl with u32 arg (e.g. CHECK_EXTENSION, SET_IOMMU). fd valid; arg is value.
        libc::ioctl(fd.as_raw_fd(), op as _, arg)
    };
    if ret < 0 {
        Err(AkidaError::capability_query_failed(format!(
            "ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(ret)
    }
}

/// Safe wrapper for VFIO _IOWR ioctls (read/write struct).
#[inline]
fn vfio_ioctl_iowr<T>(fd: BorrowedFd<'_>, op: std::os::raw::c_ulong, arg: &mut T) -> Result<()> {
    let ret = unsafe {
        // SAFETY: _IOWR ioctl reads/writes arg. fd valid; arg points to valid struct; layout matches kernel.
        libc::ioctl(fd.as_raw_fd(), op as _, arg)
    };
    if ret < 0 {
        Err(AkidaError::capability_query_failed(format!(
            "ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}

/// Safe wrapper for VFIO _IOW ioctls (write-only struct).
#[inline]
fn vfio_ioctl_iow<T>(fd: BorrowedFd<'_>, op: std::os::raw::c_ulong, arg: &T) -> Result<()> {
    let ret = unsafe {
        // SAFETY: _IOW ioctl reads arg. fd valid; arg points to valid struct; layout matches kernel.
        libc::ioctl(fd.as_raw_fd(), op as _, arg)
    };
    if ret < 0 {
        Err(AkidaError::capability_query_failed(format!(
            "ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}

/// Safe wrapper for VFIO ioctls that take a pointer arg and return int (e.g. fd).
#[inline]
fn vfio_ioctl_ptr_arg(
    fd: BorrowedFd<'_>,
    op: std::os::raw::c_ulong,
    arg: *const std::ffi::c_void,
) -> Result<i32> {
    let ret = unsafe {
        // SAFETY: ioctl with pointer arg. fd valid; arg valid for kernel to read (e.g. C string, fd ptr).
        libc::ioctl(fd.as_raw_fd(), op as _, arg)
    };
    if ret < 0 {
        Err(AkidaError::capability_query_failed(format!(
            "ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(ret)
    }
}

/// VFIO device info structure
#[repr(C)]
#[derive(Debug, Default)]
struct VfioDeviceInfo {
    argsz: u32,
    flags: u32,
    num_regions: u32,
    num_irqs: u32,
}

/// VFIO region info structure
#[repr(C)]
#[derive(Debug, Default)]
#[allow(dead_code)] // For future region queries
struct VfioRegionInfo {
    argsz: u32,
    flags: u32,
    index: u32,
    cap_offset: u32,
    size: u64,
    offset: u64,
}

/// VFIO group status structure
#[repr(C)]
#[derive(Debug, Default)]
struct VfioGroupStatus {
    argsz: u32,
    flags: u32,
}

/// VFIO DMA map structure
#[repr(C)]
#[derive(Debug, Default)]
struct VfioDmaMap {
    argsz: u32,
    flags: u32,
    vaddr: u64,
    iova: u64,
    size: u64,
}

/// VFIO DMA unmap structure
#[repr(C)]
#[derive(Debug, Default)]
struct VfioDmaUnmap {
    argsz: u32,
    flags: u32,
    iova: u64,
    size: u64,
}

/// DMA buffer for fast data transfer
#[derive(Debug)]
pub struct DmaBuffer {
    /// Virtual address (user-space)
    vaddr: *mut u8,
    /// IOVA (device-visible address)
    iova: u64,
    /// Size in bytes
    size: usize,
    /// Container fd for cleanup
    container_fd: RawFd,
}

impl DmaBuffer {
    /// Create a new DMA buffer
    fn new(container_fd: RawFd, size: usize, iova: u64) -> Result<Self> {
        // Bounds check: size must be positive and page-aligned for DMA
        if size == 0 {
            return Err(AkidaError::transfer_failed("DMA buffer size must be > 0"));
        }
        // Allocate page-aligned memory
        let layout = std::alloc::Layout::from_size_align(size, 4096)
            .map_err(|e| AkidaError::transfer_failed(format!("Invalid DMA buffer layout: {e}")))?;

        // SAFETY: Raw alloc_zeroed necessary for page-aligned DMA buffer (4096). Invariants:
        // (1) Layout from from_size_align, size>0, align 4096 power-of-two; (2) returns valid
        // ptr for layout.size() bytes or null on OOM; (3) dealloc in Drop with same layout.
        // Caller guarantees: size from aligned_size, dealloc on Drop.
        let vaddr = unsafe { std::alloc::alloc_zeroed(layout) };

        if vaddr.is_null() {
            return Err(AkidaError::transfer_failed("Failed to allocate DMA buffer"));
        }

        // SAFETY: mlock necessary for VFIO DMA (prevents swap, ensures physical pages).
        // Invariants: (1) vaddr from alloc_zeroed, valid for size bytes; (2) size matches
        // layout.size(); (3) region [vaddr, vaddr+size) entirely within allocation.
        if let Err(e) = unsafe { mlock(vaddr.cast(), size) } {
            // SAFETY: vaddr allocated above with layout; cleanup on error path before return.
            unsafe { std::alloc::dealloc(vaddr, layout) };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to lock DMA memory: {e}"
            )));
        }

        // Map the buffer for DMA
        // Truncation safe: struct sizes fit in u32
        #[allow(clippy::cast_possible_truncation)]
        let dma_map = VfioDmaMap {
            argsz: std::mem::size_of::<VfioDmaMap>() as u32,
            flags: ioctls::VFIO_DMA_MAP_FLAG_READ | ioctls::VFIO_DMA_MAP_FLAG_WRITE,
            vaddr: vaddr as u64,
            iova,
            size: size as u64,
        };

        tracing::debug!(
            "DMA map attempt: vaddr={:#x}, iova={:#x}, size={:#x}, flags={:#x}",
            dma_map.vaddr,
            dma_map.iova,
            dma_map.size,
            dma_map.flags
        );

        // Use safe ioctl wrapper; validates return code
        let container_borrowed = unsafe {
            // SAFETY: container_fd valid from VFIO container open; caller guarantees it outlives this call.
            BorrowedFd::borrow_raw(container_fd)
        };
        if let Err(e) = vfio_ioctl_iow(container_borrowed, ioctls::VFIO_IOMMU_MAP_DMA, &dma_map) {
            tracing::warn!("DMA map failed: {e}");
            // Clean up allocated memory on failure
            // SAFETY: vaddr was allocated above with this exact layout and mlock'd
            // successfully, so munlock and dealloc are valid cleanup operations.
            unsafe {
                let _ = munlock(vaddr.cast(), size);
                std::alloc::dealloc(vaddr, layout);
            };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to map DMA: {e}"
            )));
        }

        tracing::debug!("Created DMA buffer: vaddr={vaddr:p}, iova={iova:#x}, size={size:#x}");

        Ok(Self {
            vaddr,
            iova,
            size,
            container_fd,
        })
    }

    /// Get slice view of buffer for reading
    pub fn as_slice(&self) -> &[u8] {
        debug_assert!(
            !self.vaddr.is_null(),
            "DmaBuffer vaddr is null (invalid state)"
        );
        debug_assert!(self.size > 0, "DmaBuffer size is 0 (invalid state)");
        // SAFETY: (1) vaddr from alloc in new(), valid for size; (2) we own the allocation;
        // (3) &self ensures no concurrent mutation; (4) size unchanged since allocation.
        unsafe { std::slice::from_raw_parts(self.vaddr, self.size) }
    }

    /// Get mutable slice view of buffer for writing
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        debug_assert!(
            !self.vaddr.is_null(),
            "DmaBuffer vaddr is null (invalid state)"
        );
        debug_assert!(self.size > 0, "DmaBuffer size is 0 (invalid state)");
        // SAFETY: (1) vaddr valid for size; (2) &mut self gives exclusive access;
        // (3) no aliasing; (4) size and alignment correct for [u8].
        unsafe { std::slice::from_raw_parts_mut(self.vaddr, self.size) }
    }

    /// Get IOVA (device address)
    pub const fn iova(&self) -> u64 {
        self.iova
    }

    /// Get size
    pub const fn size(&self) -> usize {
        self.size
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        // SAFETY: munlock necessary - vaddr was mlock'd in new(); must unlock before dealloc.
        unsafe {
            let _ = munlock(self.vaddr.cast(), self.size);
        };

        // Unmap DMA
        let dma_unmap = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: self.iova,
            size: self.size as u64,
        };

        // Use safe ioctl wrapper; ignore error in Drop (can't propagate)
        let container_borrowed = unsafe {
            // SAFETY: container_fd valid from VFIO container; DmaBuffer dropped before container.
            BorrowedFd::borrow_raw(self.container_fd)
        };
        let _ = vfio_ioctl_iow(container_borrowed, ioctls::VFIO_IOMMU_UNMAP_DMA, &dma_unmap);

        // Deallocate memory (use safe Layout::from_size_align - eliminates one unsafe block)
        let layout = std::alloc::Layout::from_size_align(self.size, 4096)
            .expect("Layout valid: size from alloc in new(), 4096 is power-of-two");
        // SAFETY: dealloc necessary; must match alloc_zeroed in new(). Invariants: (1) vaddr
        // from alloc in new(); (2) layout matches; (3) munlock already called; (4) no refs.
        unsafe { std::alloc::dealloc(self.vaddr, layout) };

        tracing::debug!("Freed DMA buffer at iova={:#x}", self.iova);
    }
}

// SAFETY: DmaBuffer owns its memory exclusively
unsafe impl Send for DmaBuffer {}

// SAFETY: DmaBuffer provides exclusive access via &mut self for writes
// Reads are safe from multiple threads (memory is owned)
unsafe impl Sync for DmaBuffer {}

/// VFIO NPU backend with DMA support
#[derive(Debug)]
pub struct VfioBackend {
    /// PCIe address
    pcie_address: String,
    /// VFIO container file descriptor
    container: std::fs::File,
    /// VFIO group file descriptor (kept open for lifetime)
    #[allow(dead_code)] // Needed for VFIO lifetime
    group: std::fs::File,
    /// VFIO device file descriptor (for MMIO access)
    #[allow(dead_code)] // Needed for VFIO device lifetime management
    device: OwnedFd,
    /// BAR0 control registers (MMIO mapped)
    control_regs: MappedRegion,
    /// Device capabilities
    capabilities: Capabilities,
    /// Input DMA buffer
    input_buffer: Option<DmaBuffer>,
    /// Output DMA buffer
    output_buffer: Option<DmaBuffer>,
    /// Model DMA buffer
    model_buffer: Option<DmaBuffer>,
    /// Next available IOVA
    next_iova: u64,
    /// Whether a model has been loaded
    model_loaded: bool,
}

impl VfioBackend {
    /// Find IOMMU group for a PCIe device
    fn find_iommu_group(pcie_address: &str) -> Result<u32> {
        let iommu_group_path = format!("/sys/bus/pci/devices/{pcie_address}/iommu_group");

        let link = std::fs::read_link(&iommu_group_path).map_err(|e| {
            AkidaError::capability_query_failed(format!(
                "Cannot read IOMMU group for {pcie_address}: {e}. Is IOMMU enabled?"
            ))
        })?;

        let group_name = link
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AkidaError::capability_query_failed("Invalid IOMMU group path"))?;

        group_name.parse::<u32>().map_err(|e| {
            AkidaError::capability_query_failed(format!("Invalid IOMMU group number: {e}"))
        })
    }

    /// Allocate a DMA buffer
    ///
    /// # Errors
    ///
    /// Returns an error if DMA buffer allocation or IOMMU mapping fails.
    pub fn alloc_dma(&mut self, size: usize) -> Result<DmaBuffer> {
        let iova = self.next_iova;
        let aligned_size = size.div_ceil(4096) * 4096;
        self.next_iova += aligned_size as u64;
        DmaBuffer::new(self.container.as_raw_fd(), aligned_size, iova)
    }

    /// Write a 64-bit IOVA address and size to MMIO registers (addr_lo, addr_hi, size_reg).
    #[allow(clippy::cast_possible_truncation)]
    fn write_iova_regs(
        &self,
        addr_lo: usize,
        addr_hi: usize,
        size_reg: usize,
        iova: u64,
        size: usize,
    ) {
        self.control_regs.write32(addr_lo, iova as u32);
        self.control_regs.write32(addr_hi, (iova >> 32) as u32);
        self.control_regs.write32(size_reg, size as u32);
    }

    /// Check the device is not busy. Returns `Err` if BUSY bit is set.
    fn check_not_busy(&self, op: &str) -> Result<()> {
        let status = self.control_regs.read32(regs::STATUS);
        if status & regs::status::BUSY != 0 {
            return Err(AkidaError::hardware_error(format!(
                "Device busy, cannot {op}"
            )));
        }
        Ok(())
    }

    /// Poll a status register until `done_mask` bit is set, returning the poll count.
    /// Returns `Err` if `error_mask` bit is set or `max_polls` is exceeded.
    fn poll_register(&self, cfg: PollConfig<'_>) -> Result<u32> {
        let PollConfig {
            reg,
            done_mask,
            error_mask,
            max_polls,
            yield_interval,
            timeout_msg,
            error_msg,
        } = cfg;
        for i in 0..max_polls {
            let val = self.control_regs.read32(reg);
            if val & done_mask != 0 {
                return Ok(i + 1);
            }
            if val & error_mask != 0 {
                return Err(AkidaError::hardware_error(error_msg));
            }
            if i % yield_interval == 0 {
                std::thread::yield_now();
            }
        }
        Err(AkidaError::hardware_error(timeout_msg))
    }
}

impl NpuBackend for VfioBackend {
    fn init(pcie_address: &str) -> Result<Self> {
        tracing::info!("Initializing VFIO backend for {pcie_address}");

        // Find IOMMU group
        let iommu_group = Self::find_iommu_group(pcie_address)?;
        tracing::debug!("IOMMU group: {iommu_group}");

        // Open VFIO container
        let container = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/vfio/vfio")
            .map_err(|e| {
                AkidaError::capability_query_failed(format!("Cannot open /dev/vfio/vfio: {e}"))
            })?;

        let api_version = vfio_ioctl_int(container.as_fd(), ioctls::VFIO_GET_API_VERSION)?;

        if api_version != ioctls::VFIO_API_VERSION {
            return Err(AkidaError::capability_query_failed(format!(
                "Unsupported VFIO API version: {api_version}"
            )));
        }

        let has_type1 = vfio_ioctl_int_arg(
            container.as_fd(),
            ioctls::VFIO_CHECK_EXTENSION,
            ioctls::VFIO_TYPE1V2_IOMMU,
        )?;

        if has_type1 != 1 {
            return Err(AkidaError::capability_query_failed(
                "VFIO Type1v2 IOMMU not supported",
            ));
        }

        // Open IOMMU group
        let group_path = format!("/dev/vfio/{iommu_group}");
        let group = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&group_path)
            .map_err(|e| {
                AkidaError::capability_query_failed(format!("Cannot open {group_path}: {e}"))
            })?;

        // Check group is viable
        let mut group_status = VfioGroupStatus {
            argsz: std::mem::size_of::<VfioGroupStatus>() as u32,
            flags: 0,
        };

        vfio_ioctl_iowr(
            group.as_fd(),
            ioctls::VFIO_GROUP_GET_STATUS,
            &mut group_status,
        )?;

        if (group_status.flags & ioctls::VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(AkidaError::capability_query_failed(
                "VFIO group not viable (all devices must be bound to vfio-pci)",
            ));
        }

        let container_fd = container.as_raw_fd();
        vfio_ioctl_ptr_arg(
            group.as_fd(),
            ioctls::VFIO_GROUP_SET_CONTAINER,
            std::ptr::from_ref(&container_fd).cast(),
        )?;

        vfio_ioctl_int_arg(
            container.as_fd(),
            ioctls::VFIO_SET_IOMMU,
            ioctls::VFIO_TYPE1V2_IOMMU,
        )?;

        // Get device fd
        let pcie_address_cstr = std::ffi::CString::new(pcie_address).map_err(|e| {
            AkidaError::capability_query_failed(format!("Invalid PCIe address: {e}"))
        })?;

        let device_fd = vfio_ioctl_ptr_arg(
            group.as_fd(),
            ioctls::VFIO_GROUP_GET_DEVICE_FD,
            pcie_address_cstr.as_ptr().cast(),
        )?;

        // Take ownership of the fd returned by the kernel
        let device = unsafe {
            // SAFETY: device_fd from successful VFIO_GROUP_GET_DEVICE_FD; kernel returns valid fd.
            // We take ownership; OwnedFd will close it on drop.
            OwnedFd::from_raw_fd(device_fd)
        };

        // Query device info
        let mut device_info = VfioDeviceInfo {
            argsz: std::mem::size_of::<VfioDeviceInfo>() as u32,
            ..Default::default()
        };

        vfio_ioctl_iowr(
            device.as_fd(),
            ioctls::VFIO_DEVICE_GET_INFO,
            &mut device_info,
        )?;

        tracing::info!(
            "VFIO device: {} regions, {} IRQs",
            device_info.num_regions,
            device_info.num_irqs
        );

        // Map BAR0 for control registers
        let control_regs = MappedRegion::map(&device, Bar::Control)?;
        tracing::info!(
            "Mapped BAR0 control registers ({} bytes)",
            control_regs.size()
        );

        // Query capabilities from sysfs (same as userspace backend)
        let capabilities = Capabilities::from_sysfs(pcie_address)?;

        tracing::info!(
            "Initialized VFIO backend for {pcie_address}: {} NPUs, {} MB SRAM",
            capabilities.npu_count,
            capabilities.memory_mb
        );

        Ok(Self {
            pcie_address: pcie_address.to_string(),
            container,
            group,
            device,
            control_regs,
            capabilities,
            input_buffer: None,
            output_buffer: None,
            model_buffer: None,
            next_iova: 0x1000_0000, // Start IOVA at 256MB
            model_loaded: false,
        })
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn load_model(&mut self, model: &[u8]) -> Result<ModelHandle> {
        tracing::info!("Loading model ({} bytes) via VFIO DMA", model.len());
        self.check_not_busy("load model")?;

        let mut buffer = self.alloc_dma(model.len())?;
        buffer.as_mut_slice().copy_from_slice(model);

        self.write_iova_regs(
            regs::MODEL_ADDR_LO,
            regs::MODEL_ADDR_HI,
            regs::MODEL_SIZE,
            buffer.iova(),
            model.len(),
        );
        self.control_regs.write32(regs::MODEL_LOAD, 1);
        tracing::debug!(
            "Triggered model load: IOVA={:#x}, size={}",
            buffer.iova(),
            model.len()
        );

        let polls = self.poll_register(PollConfig {
            reg: regs::STATUS,
            done_mask: regs::status::MODEL_LOADED,
            error_mask: regs::status::ERROR,
            max_polls: 1_000_000,
            yield_interval: 1_000,
            timeout_msg: "Model load timed out",
            error_msg: "Model load failed with device error",
        })?;
        tracing::info!("Model loaded successfully after {polls} polls");

        self.model_buffer = Some(buffer);
        self.model_loaded = true;
        Ok(ModelHandle::new(0))
    }

    fn load_reservoir(&mut self, w_in: &[f32], w_res: &[f32]) -> Result<()> {
        let w_in_bytes = bytemuck::cast_slice::<f32, u8>(w_in);
        let w_res_bytes = bytemuck::cast_slice::<f32, u8>(w_res);
        let total_size = w_in_bytes.len() + w_res_bytes.len();

        tracing::info!(
            "Loading reservoir via VFIO DMA: w_in={} floats, w_res={} floats",
            w_in.len(),
            w_res.len()
        );

        self.check_not_busy("load reservoir")?;

        // Allocate DMA buffer
        let mut buffer = self.alloc_dma(total_size)?;
        let slice = buffer.as_mut_slice();
        slice[..w_in_bytes.len()].copy_from_slice(w_in_bytes);
        slice[w_in_bytes.len()..].copy_from_slice(w_res_bytes);

        self.write_iova_regs(
            regs::MODEL_ADDR_LO,
            regs::MODEL_ADDR_HI,
            regs::MODEL_SIZE,
            buffer.iova(),
            total_size,
        );
        self.control_regs.write32(regs::MODEL_LOAD, 1);

        let polls = self.poll_register(PollConfig {
            reg: regs::STATUS,
            done_mask: regs::status::MODEL_LOADED,
            error_mask: regs::status::ERROR,
            max_polls: 1_000_000,
            yield_interval: 1_000,
            timeout_msg: "Reservoir load timed out",
            error_msg: "Reservoir load failed with device error",
        })?;
        tracing::info!("Reservoir loaded successfully after {polls} polls");

        self.model_buffer = Some(buffer);
        self.model_loaded = true;
        Ok(())
    }

    fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        if !self.model_loaded {
            return Err(AkidaError::hardware_error("No model loaded"));
        }

        self.check_not_busy("run inference")?;
        let status = self.control_regs.read32(regs::STATUS);
        if status & regs::status::READY == 0 {
            return Err(AkidaError::hardware_error("Device not ready"));
        }

        let input_bytes = bytemuck::cast_slice::<f32, u8>(input);

        // Ensure input DMA buffer is large enough
        if self
            .input_buffer
            .as_ref()
            .is_none_or(|b| b.size() < input_bytes.len())
        {
            self.input_buffer = Some(self.alloc_dma(input_bytes.len().max(4096))?);
        }
        let input_buf = self.input_buffer.as_mut().ok_or_else(|| {
            AkidaError::hardware_error("Input DMA buffer missing after allocation")
        })?;
        input_buf.as_mut_slice()[..input_bytes.len()].copy_from_slice(input_bytes);

        let output_size: usize = 4096; // 1024 floats max
        if self
            .output_buffer
            .as_ref()
            .is_none_or(|b| b.size() < output_size)
        {
            self.output_buffer = Some(self.alloc_dma(output_size)?);
        }

        let input_iova = self
            .input_buffer
            .as_ref()
            .ok_or_else(|| AkidaError::hardware_error("Input DMA buffer missing after allocation"))?
            .iova();
        let output_iova = self
            .output_buffer
            .as_ref()
            .ok_or_else(|| {
                AkidaError::hardware_error("Output DMA buffer missing after allocation")
            })?
            .iova();

        self.write_iova_regs(
            regs::INPUT_ADDR_LO,
            regs::INPUT_ADDR_HI,
            regs::INPUT_SIZE,
            input_iova,
            input_bytes.len(),
        );
        self.write_iova_regs(
            regs::OUTPUT_ADDR_LO,
            regs::OUTPUT_ADDR_HI,
            regs::OUTPUT_SIZE,
            output_iova,
            output_size,
        );

        self.control_regs.write32(regs::INFER_START, 1);
        tracing::debug!(
            "Triggered inference: input_iova={input_iova:#x}, output_iova={output_iova:#x}"
        );

        let polls = self.poll_register(PollConfig {
            reg: regs::INFER_STATUS,
            done_mask: 0x1,
            error_mask: 0x2,
            max_polls: 10_000_000,
            yield_interval: 10_000,
            timeout_msg: "Inference timed out",
            error_msg: "Inference failed with device error",
        })?;

        let actual_output_size = self.control_regs.read32(regs::OUTPUT_SIZE) as usize;
        let output_floats = actual_output_size.min(output_size) / std::mem::size_of::<f32>();
        tracing::debug!("Inference completed after {polls} polls, output: {output_floats} floats");

        let output_bytes = &self
            .output_buffer
            .as_ref()
            .ok_or_else(|| {
                AkidaError::hardware_error("Output DMA buffer missing after allocation")
            })?
            .as_slice()[..output_floats * std::mem::size_of::<f32>()];
        Ok(bytemuck::cast_slice::<u8, f32>(output_bytes).to_vec())
    }

    fn measure_power(&self) -> Result<f32> {
        if let Some(watts) = read_hwmon_power(&self.pcie_address) {
            return Ok(watts);
        }

        Ok(1.5) // AKD1000 typical from datasheet
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Vfio
    }

    fn is_ready(&self) -> bool {
        // Check device status via MMIO
        let status = self.control_regs.read32(regs::STATUS);
        let ready = status & regs::status::READY != 0;
        let not_busy = status & regs::status::BUSY == 0;
        let no_error = status & regs::status::ERROR == 0;
        ready && not_busy && no_error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_iommu_group() {
        // This test requires actual hardware with IOMMU
        let pcie_address = "0000:a1:00.0";

        match VfioBackend::find_iommu_group(pcie_address) {
            Ok(group) => {
                println!("IOMMU group for {pcie_address}: {group}");
            }
            Err(e) => {
                println!("IOMMU group lookup failed (expected if no hardware): {e}");
            }
        }
    }

    #[test]
    fn test_vfio_backend_init() {
        // This test requires actual hardware bound to vfio-pci
        let pcie_address = "0000:a1:00.0";

        match VfioBackend::init(pcie_address) {
            Ok(backend) => {
                println!("VFIO backend initialized");
                println!("   NPUs: {}", backend.capabilities().npu_count);
                println!("   SRAM: {} MB", backend.capabilities().memory_mb);
            }
            Err(e) => {
                println!("VFIO backend unavailable (expected if no hardware): {e}");
            }
        }
    }
}
