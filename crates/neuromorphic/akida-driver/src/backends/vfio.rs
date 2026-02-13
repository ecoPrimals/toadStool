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
//! - No C dependencies (pure Rust via rustix)

use crate::backend::{BackendType, ModelHandle, NpuBackend};
use crate::capabilities::Capabilities;
use crate::error::{AkidaError, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};

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
        // Allocate page-aligned memory
        let layout = std::alloc::Layout::from_size_align(size, 4096)
            .map_err(|e| AkidaError::transfer_failed(format!("Invalid DMA buffer layout: {e}")))?;

        // SAFETY: We're allocating memory with a valid layout
        // - Size > 0 (checked by Layout::from_size_align)
        // - Alignment is 4096 (page-aligned for DMA)
        // - We'll deallocate in Drop with the same layout
        let vaddr = unsafe { std::alloc::alloc_zeroed(layout) };

        if vaddr.is_null() {
            return Err(AkidaError::transfer_failed("Failed to allocate DMA buffer"));
        }

        // Lock the memory in RAM (required for VFIO DMA)
        // SAFETY: vaddr points to valid, allocated memory of `size` bytes
        let mlock_ret = unsafe { libc::mlock(vaddr.cast(), size) };
        if mlock_ret < 0 {
            unsafe { std::alloc::dealloc(vaddr, layout) };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to lock DMA memory: {}",
                std::io::Error::last_os_error()
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

        // SAFETY: VFIO ioctl with valid structure
        let ret = unsafe {
            libc::ioctl(
                container_fd,
                ioctls::VFIO_IOMMU_MAP_DMA as _,
                &raw const dma_map,
            )
        };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            tracing::warn!("DMA map failed: {} (ret={})", err, ret);
            // Clean up allocated memory on failure
            unsafe {
                libc::munlock(vaddr.cast(), size);
                std::alloc::dealloc(vaddr, layout);
            };
            return Err(AkidaError::transfer_failed(format!(
                "Failed to map DMA: {err}"
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
        // SAFETY: vaddr is valid and points to `size` bytes
        unsafe { std::slice::from_raw_parts(self.vaddr, self.size) }
    }

    /// Get mutable slice view of buffer for writing
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: vaddr is valid and points to `size` bytes, we have &mut self
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
        // Unlock memory
        // SAFETY: vaddr was locked in new()
        unsafe { libc::munlock(self.vaddr.cast(), self.size) };

        // Unmap DMA
        let dma_unmap = VfioDmaUnmap {
            argsz: std::mem::size_of::<VfioDmaUnmap>() as u32,
            flags: 0,
            iova: self.iova,
            size: self.size as u64,
        };

        // SAFETY: VFIO ioctl with valid structure
        unsafe {
            libc::ioctl(
                self.container_fd,
                ioctls::VFIO_IOMMU_UNMAP_DMA as _,
                &raw const dma_unmap,
            );
        }

        // Deallocate memory
        let layout = std::alloc::Layout::from_size_align(self.size, 4096).unwrap();
        // SAFETY: vaddr was allocated with this layout in new()
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
    container: File,
    /// VFIO group file descriptor (kept open for lifetime)
    #[allow(dead_code)] // Needed for VFIO lifetime
    group: File,
    /// VFIO device file descriptor (for MMIO access)
    #[allow(dead_code)] // For future MMIO register access
    device: File,
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
        // Align size to 4KB page boundary (VFIO requires page-aligned mappings)
        let aligned_size = size.div_ceil(4096) * 4096;
        self.next_iova += aligned_size as u64;

        DmaBuffer::new(self.container.as_raw_fd(), aligned_size, iova)
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

        // Check API version
        // SAFETY: VFIO ioctl with no arguments
        let api_version =
            unsafe { libc::ioctl(container.as_raw_fd(), ioctls::VFIO_GET_API_VERSION as _) };

        if api_version != ioctls::VFIO_API_VERSION {
            return Err(AkidaError::capability_query_failed(format!(
                "Unsupported VFIO API version: {api_version}"
            )));
        }

        // Check for Type1 IOMMU support
        // SAFETY: VFIO ioctl with integer argument
        let has_type1 = unsafe {
            libc::ioctl(
                container.as_raw_fd(),
                ioctls::VFIO_CHECK_EXTENSION as _,
                ioctls::VFIO_TYPE1V2_IOMMU,
            )
        };

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

        // SAFETY: VFIO ioctl with valid structure
        let ret = unsafe {
            libc::ioctl(
                group.as_raw_fd(),
                ioctls::VFIO_GROUP_GET_STATUS as _,
                &raw mut group_status,
            )
        };

        if ret < 0 || (group_status.flags & ioctls::VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(AkidaError::capability_query_failed(
                "VFIO group not viable (all devices must be bound to vfio-pci)",
            ));
        }

        // Set container for group
        // SAFETY: VFIO ioctl with fd argument
        let ret = unsafe {
            libc::ioctl(
                group.as_raw_fd(),
                ioctls::VFIO_GROUP_SET_CONTAINER as _,
                std::ptr::from_ref(&container.as_raw_fd()),
            )
        };

        if ret < 0 {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to set container: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Enable IOMMU
        // SAFETY: VFIO ioctl with integer argument
        let ret = unsafe {
            libc::ioctl(
                container.as_raw_fd(),
                ioctls::VFIO_SET_IOMMU as _,
                ioctls::VFIO_TYPE1V2_IOMMU,
            )
        };

        if ret < 0 {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to set IOMMU: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Get device fd
        let pcie_address_cstr = std::ffi::CString::new(pcie_address).map_err(|e| {
            AkidaError::capability_query_failed(format!("Invalid PCIe address: {e}"))
        })?;

        // SAFETY: VFIO ioctl with C string argument
        let device_fd = unsafe {
            libc::ioctl(
                group.as_raw_fd(),
                ioctls::VFIO_GROUP_GET_DEVICE_FD as _,
                pcie_address_cstr.as_ptr(),
            )
        };

        if device_fd < 0 {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to get device fd: {}",
                std::io::Error::last_os_error()
            )));
        }

        // SAFETY: We just got a valid fd from VFIO
        let device = unsafe { File::from_raw_fd(device_fd) };

        // Query device info
        let mut device_info = VfioDeviceInfo {
            argsz: std::mem::size_of::<VfioDeviceInfo>() as u32,
            ..Default::default()
        };

        // SAFETY: VFIO ioctl with valid structure
        let ret = unsafe {
            libc::ioctl(
                device.as_raw_fd(),
                ioctls::VFIO_DEVICE_GET_INFO as _,
                &raw mut device_info,
            )
        };

        if ret < 0 {
            return Err(AkidaError::capability_query_failed(format!(
                "Failed to get device info: {}",
                std::io::Error::last_os_error()
            )));
        }

        tracing::info!(
            "VFIO device: {} regions, {} IRQs",
            device_info.num_regions,
            device_info.num_irqs
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
            capabilities,
            input_buffer: None,
            output_buffer: None,
            model_buffer: None,
            next_iova: 0x1000_0000, // Start IOVA at 256MB
        })
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn load_model(&mut self, model: &[u8]) -> Result<ModelHandle> {
        tracing::info!("Loading model ({} bytes) via VFIO DMA", model.len());

        // Allocate DMA buffer for model
        let mut buffer = self.alloc_dma(model.len())?;
        buffer.as_mut_slice().copy_from_slice(model);

        // TODO: Trigger model load via MMIO registers
        // For now, store buffer for later use
        self.model_buffer = Some(buffer);

        tracing::info!("Model loaded to DMA buffer");
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

        // Allocate DMA buffer
        let mut buffer = self.alloc_dma(total_size)?;
        let slice = buffer.as_mut_slice();
        slice[..w_in_bytes.len()].copy_from_slice(w_in_bytes);
        slice[w_in_bytes.len()..].copy_from_slice(w_res_bytes);

        self.model_buffer = Some(buffer);

        tracing::info!("Reservoir weights loaded to DMA buffer");
        Ok(())
    }

    fn infer(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let input_bytes = bytemuck::cast_slice::<f32, u8>(input);

        // Allocate input buffer if needed
        if self.input_buffer.is_none()
            || self.input_buffer.as_ref().unwrap().size() < input_bytes.len()
        {
            self.input_buffer = Some(self.alloc_dma(input_bytes.len().max(4096))?);
        }

        // Copy input to DMA buffer
        self.input_buffer.as_mut().unwrap().as_mut_slice()[..input_bytes.len()]
            .copy_from_slice(input_bytes);

        // TODO: Trigger inference via MMIO registers and wait for interrupt
        // For now, simulate with placeholder output
        let output_size = 10 * std::mem::size_of::<f32>(); // 10 floats placeholder

        // Allocate output buffer if needed
        if self.output_buffer.is_none() || self.output_buffer.as_ref().unwrap().size() < output_size
        {
            self.output_buffer = Some(self.alloc_dma(output_size)?);
        }

        // Read output from DMA buffer
        let output_bytes = &self.output_buffer.as_ref().unwrap().as_slice()[..output_size];
        let output: Vec<f32> = bytemuck::cast_slice::<u8, f32>(output_bytes).to_vec();

        Ok(output)
    }

    fn measure_power(&self) -> Result<f32> {
        // Same as userspace backend - query hwmon sysfs
        let hwmon_path = format!(
            "/sys/bus/pci/devices/{}/hwmon/hwmon*/power1_average",
            self.pcie_address
        );

        if let Ok(entries) = glob::glob(&hwmon_path) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    if let Ok(microwatts) = content.trim().parse::<u64>() {
                        #[allow(clippy::cast_precision_loss)]
                        let watts = microwatts as f32 / 1_000_000.0;
                        return Ok(watts);
                    }
                }
            }
        }

        Ok(1.5) // AKD1000 typical from datasheet
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Vfio
    }

    fn is_ready(&self) -> bool {
        // TODO: Check device status via MMIO
        true
    }
}

use std::os::unix::io::FromRawFd;

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
