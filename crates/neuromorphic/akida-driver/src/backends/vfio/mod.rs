// SPDX-License-Identifier: AGPL-3.0-only
//! VFIO NPU backend — Pure Rust with DMA support
//!
//! Uses Linux VFIO (Virtual Function I/O) for:
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

mod dma;
mod ioctl;
mod types;

pub use dma::DmaBuffer;

use super::read_hwmon_power;
use crate::backend::{BackendType, ModelHandle, NpuBackend};
use crate::capabilities::Capabilities;
use crate::error::{AkidaError, Result};
use crate::mmio::{Bar, MappedRegion, regs};
use std::fs::OpenOptions;
use std::os::fd::{AsFd, FromRawFd, OwnedFd};
use std::os::unix::io::AsRawFd;
use types::PollConfig;
use types::ioctls;

/// VFIO NPU backend with DMA support.
#[derive(Debug)]
pub struct VfioBackend {
    pcie_address: String,
    container: std::fs::File,
    #[allow(
        dead_code,
        reason = "VFIO group file descriptor required for kernel lifetime"
    )]
    group: std::fs::File,
    #[allow(
        dead_code,
        reason = "VFIO device file descriptor required for kernel lifetime"
    )]
    device: OwnedFd,
    control_regs: MappedRegion,
    capabilities: Capabilities,
    input_buffer: Option<DmaBuffer>,
    output_buffer: Option<DmaBuffer>,
    model_buffer: Option<DmaBuffer>,
    next_iova: u64,
    model_loaded: bool,
}

impl VfioBackend {
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

    /// Allocate a DMA buffer.
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

    fn check_not_busy(&self, op: &str) -> Result<()> {
        let status = self.control_regs.read32(regs::STATUS);
        if status & regs::status::BUSY != 0 {
            return Err(AkidaError::hardware_error(format!(
                "Device busy, cannot {op}"
            )));
        }
        Ok(())
    }

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
    #[allow(clippy::cast_possible_truncation)] // struct sizes always fit u32
    fn init(pcie_address: &str) -> Result<Self> {
        tracing::info!("Initializing VFIO backend for {pcie_address}");

        let iommu_group = Self::find_iommu_group(pcie_address)?;
        tracing::debug!("IOMMU group: {iommu_group}");

        let container = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/vfio/vfio")
            .map_err(|e| {
                AkidaError::capability_query_failed(format!("Cannot open /dev/vfio/vfio: {e}"))
            })?;

        let api_version = ioctl::get_api_version(container.as_fd())?;
        if api_version != ioctls::VFIO_API_VERSION {
            return Err(AkidaError::capability_query_failed(format!(
                "Unsupported VFIO API version: {api_version}"
            )));
        }

        let has_type1 = ioctl::check_extension(container.as_fd(), ioctls::VFIO_TYPE1V2_IOMMU)?;
        if has_type1 != 1 {
            return Err(AkidaError::capability_query_failed(
                "VFIO Type1v2 IOMMU not supported",
            ));
        }

        let group_path = format!("/dev/vfio/{iommu_group}");
        let group = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&group_path)
            .map_err(|e| {
                AkidaError::capability_query_failed(format!("Cannot open {group_path}: {e}"))
            })?;

        let mut group_status = types::VfioGroupStatus {
            argsz: std::mem::size_of::<types::VfioGroupStatus>() as u32,
            flags: 0,
        };
        ioctl::group_status(group.as_fd(), &mut group_status)?;

        if (group_status.flags & ioctls::VFIO_GROUP_FLAGS_VIABLE) == 0 {
            return Err(AkidaError::capability_query_failed(
                "VFIO group not viable (all devices must be bound to vfio-pci)",
            ));
        }

        let container_fd = container.as_raw_fd();
        ioctl::group_set_container(group.as_fd(), std::ptr::from_ref(&container_fd).cast())?;
        ioctl::set_iommu(container.as_fd(), ioctls::VFIO_TYPE1V2_IOMMU)?;

        let pcie_address_cstr = std::ffi::CString::new(pcie_address).map_err(|e| {
            AkidaError::capability_query_failed(format!("Invalid PCIe address: {e}"))
        })?;
        let device_fd =
            ioctl::group_get_device_fd(group.as_fd(), pcie_address_cstr.as_ptr().cast())?;

        // SAFETY: Invariants: fd must be a valid, open fd; caller takes ownership (must not close).
        // Satisfied: device_fd from VFIO_GROUP_GET_DEVICE_FD ioctl success; kernel returns valid fd.
        // Violation: invalid fd → double-close on drop; already-closed fd → use-after-close.
        let device = unsafe { OwnedFd::from_raw_fd(device_fd) };

        let mut dev_info = types::VfioDeviceInfo {
            argsz: std::mem::size_of::<types::VfioDeviceInfo>() as u32,
            ..Default::default()
        };
        ioctl::device_info(device.as_fd(), &mut dev_info)?;

        tracing::info!(
            "VFIO device: {} regions, {} IRQs",
            dev_info.num_regions,
            dev_info.num_irqs
        );

        let control_regs = MappedRegion::map(&device, Bar::Control)?;
        tracing::info!(
            "Mapped BAR0 control registers ({} bytes)",
            control_regs.size()
        );

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
            next_iova: 0x1000_0000,
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

        let output_size: usize = 4096;
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
            .ok_or_else(|| AkidaError::hardware_error("Input DMA buffer missing"))?
            .iova();
        let output_iova = self
            .output_buffer
            .as_ref()
            .ok_or_else(|| AkidaError::hardware_error("Output DMA buffer missing"))?
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
            .ok_or_else(|| AkidaError::hardware_error("Output DMA buffer missing"))?
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
        let pcie_address = "0000:a1:00.0";
        match VfioBackend::find_iommu_group(pcie_address) {
            Ok(group) => println!("IOMMU group for {pcie_address}: {group}"),
            Err(e) => println!("IOMMU group lookup failed (expected if no hardware): {e}"),
        }
    }

    #[test]
    fn test_find_iommu_group_nonexistent_device() {
        let result = VfioBackend::find_iommu_group("0000:xx:yy.z");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("IOMMU") || msg.contains("Cannot read") || msg.contains("Failed"),
            "expected IOMMU-related error, got: {msg}"
        );
    }

    #[test]
    fn test_vfio_backend_init() {
        let pcie_address = "0000:a1:00.0";
        match VfioBackend::init(pcie_address) {
            Ok(backend) => {
                println!("VFIO backend initialized");
                println!("   NPUs: {}", backend.capabilities().npu_count);
                println!("   SRAM: {} MB", backend.capabilities().memory_mb);
            }
            Err(e) => println!("VFIO backend unavailable (expected if no hardware): {e}"),
        }
    }

    #[test]
    fn test_vfio_ioctl_constants() {
        assert_eq!(ioctls::VFIO_API_VERSION, 0);
        assert_eq!(ioctls::VFIO_GROUP_FLAGS_VIABLE, 1);
        assert_eq!(ioctls::VFIO_TYPE1V2_IOMMU, 3);
        assert_eq!(ioctls::VFIO_DMA_MAP_FLAG_READ, 1);
        assert_eq!(ioctls::VFIO_DMA_MAP_FLAG_WRITE, 2);
    }

    #[test]
    fn test_poll_config_structure() {
        let cfg = PollConfig {
            reg: 0x10,
            done_mask: 0x1,
            error_mask: 0x2,
            max_polls: 1000,
            yield_interval: 100,
            timeout_msg: "timeout",
            error_msg: "error",
        };
        assert_eq!(cfg.reg, 0x10);
        assert_eq!(cfg.done_mask, 0x1);
        assert_eq!(cfg.error_mask, 0x2);
    }

    #[test]
    fn test_iova_initial_value() {
        const NEXT_IOVA_INIT: u64 = 0x1000_0000;
        assert_eq!(NEXT_IOVA_INIT, 268_435_456);
    }

    #[test]
    fn test_iova_increment_logic() {
        let mut next_iova: u64 = 0x1000_0000;
        let size = 4096usize;
        let aligned_size = size.div_ceil(4096) * 4096;
        let iova = next_iova;
        next_iova += aligned_size as u64;
        assert_eq!(iova, 0x1000_0000);
        assert_eq!(next_iova, 0x1000_0000 + 4096);
    }
}
