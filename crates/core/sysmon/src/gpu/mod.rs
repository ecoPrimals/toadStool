// SPDX-License-Identifier: AGPL-3.0-only
//! GPU monitoring via `/sys/class/drm/` and vendor-specific sysfs.
//!
//! Discovers GPU devices from DRM subsystem and reads hardware telemetry:
//! temperature, power, clock frequency, VRAM usage, fan speed.
//!
//! - **AMD** (`amdgpu`): hwmon sysfs + `mem_info_vram_*`
//! - **NVIDIA** (`nouveau`/`nvidia`): hwmon sysfs where available
//! - **Intel** (`i915`): hwmon sysfs where available
//!
//! All reads are best-effort — missing or busy sensors return `None` instead
//! of errors. GPU runtime power management can make sensors temporarily
//! unavailable ("device or resource busy").

mod firmware;
mod silicon_tables;
mod sysfs;

pub use firmware::{FirmwareInventory, FwStatus};

use std::path::{Path, PathBuf};

use toadstool_core::silicon::SiliconCapabilities;

use sysfs::{
    find_hwmon_dir, find_iommu_group, read_sysfs_hex, read_sysfs_string, read_sysfs_u64,
    read_sysfs_uevent_field,
};

/// Vendor identification for GPU-specific sysfs paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    /// AMD GPU (PCI vendor 0x1002).
    Amd,
    /// NVIDIA GPU (PCI vendor 0x10de).
    Nvidia,
    /// Intel integrated or discrete GPU (PCI vendor 0x8086).
    Intel,
    /// Unknown or unsupported vendor.
    Unknown,
}

impl GpuVendor {
    pub(crate) const fn from_pci_vendor(vendor_id: u32) -> Self {
        match vendor_id {
            0x1002 => Self::Amd,
            0x10de => Self::Nvidia,
            0x8086 => Self::Intel,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amd => f.write_str("AMD"),
            Self::Nvidia => f.write_str("NVIDIA"),
            Self::Intel => f.write_str("Intel"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// A discovered GPU device with its sysfs paths.
#[derive(Debug, Clone)]
pub struct GpuDevice {
    /// DRM card index (e.g. 0 for card0)
    pub card_index: u32,
    /// PCI slot name (e.g. "0000:25:00.0")
    pub pci_slot: String,
    /// Vendor
    pub vendor: GpuVendor,
    /// PCI device ID
    pub device_id: u32,
    /// DRM driver name (e.g. "amdgpu", "nvidia", "nouveau", "i915")
    pub driver: String,
    /// Path to device sysfs directory
    pub(crate) sysfs_device: PathBuf,
    /// Path to hwmon directory (if found)
    hwmon_path: Option<PathBuf>,
}

/// Hardware telemetry snapshot for a single GPU.
#[derive(Debug, Clone, Default)]
pub struct GpuTelemetry {
    /// GPU temperature in degrees Celsius (from hwmon `temp1_input` millidegrees).
    pub temperature_celsius: Option<f64>,
    /// Average power draw in watts (from hwmon `power1_average` microwatts).
    pub power_watts: Option<f64>,
    /// Power cap in watts (from hwmon `power1_cap` microwatts).
    pub power_cap_watts: Option<f64>,
    /// GPU core clock in `MHz` (from hwmon `freq1_input`).
    pub core_clock_mhz: Option<f64>,
    /// Memory clock in `MHz` (from hwmon `freq2_input`).
    pub memory_clock_mhz: Option<f64>,
    /// Fan speed in RPM (from hwmon `fan1_input`).
    pub fan_rpm: Option<u64>,
    /// GPU utilization percentage (AMD `gpu_busy_percent`).
    pub utilization_percent: Option<f64>,
    /// Total VRAM in bytes (AMD: `mem_info_vram_total`).
    pub vram_total_bytes: Option<u64>,
    /// Used VRAM in bytes (AMD: `mem_info_vram_used`).
    pub vram_used_bytes: Option<u64>,
}

impl GpuTelemetry {
    /// VRAM utilization as a percentage (0.0 - 100.0).
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "VRAM bytes are well within f64 mantissa range for practical GPU sizes"
    )]
    pub fn vram_utilization_percent(&self) -> Option<f64> {
        match (self.vram_used_bytes, self.vram_total_bytes) {
            (Some(used), Some(total)) if total > 0 => Some(used as f64 / total as f64 * 100.0),
            _ => None,
        }
    }
}

/// `PCIe` topology information for a GPU device.
#[derive(Debug, Clone)]
pub struct PcieTopology {
    /// `PCIe` generation (3, 4, 5)
    pub generation: Option<u32>,
    /// Link width (x1, x4, x8, x16)
    pub width: Option<u32>,
    /// NUMA node (-1 if not applicable)
    pub numa_node: Option<i32>,
    /// IOMMU group number
    pub iommu_group: Option<u32>,
}

/// Discover all GPU devices from `/sys/class/drm/card*`.
///
/// Returns an empty vec on non-Linux or if `/sys/class/drm/` doesn't exist.
#[must_use]
pub fn discover_gpus() -> Vec<GpuDevice> {
    let drm_dir = Path::new("/sys/class/drm");
    if !drm_dir.exists() {
        return Vec::new();
    }

    let mut gpus = Vec::new();
    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return gpus;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        let Some(index_str) = name_str.strip_prefix("card") else {
            continue;
        };
        let Ok(card_index) = index_str.parse::<u32>() else {
            continue;
        };

        let device_path = entry.path().join("device");
        if !device_path.exists() {
            continue;
        }

        let vendor_id = read_sysfs_hex(&device_path.join("vendor")).unwrap_or(0);
        let device_id = read_sysfs_hex(&device_path.join("device")).unwrap_or(0);
        let vendor = GpuVendor::from_pci_vendor(vendor_id);

        let pci_slot = read_sysfs_uevent_field(&device_path.join("uevent"), "PCI_SLOT_NAME")
            .unwrap_or_default();
        let driver =
            read_sysfs_uevent_field(&device_path.join("uevent"), "DRIVER").unwrap_or_default();

        let hwmon_path = find_hwmon_dir(&device_path);

        gpus.push(GpuDevice {
            card_index,
            pci_slot,
            vendor,
            device_id,
            driver,
            sysfs_device: device_path,
            hwmon_path,
        });
    }

    gpus.sort_by_key(|g| g.card_index);
    gpus
}

impl GpuDevice {
    /// Read current hardware telemetry. All fields are best-effort.
    ///
    /// Sensors that are unavailable (runtime PM, permission, driver) return `None`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "hwmon values (millidegrees, microwatts, Hz) are well within f64 range"
    )]
    pub fn telemetry(&self) -> GpuTelemetry {
        let mut t = GpuTelemetry::default();

        if let Some(hwmon) = &self.hwmon_path {
            t.temperature_celsius =
                read_sysfs_u64(&hwmon.join("temp1_input")).map(|v| v as f64 / 1000.0);
            t.power_watts =
                read_sysfs_u64(&hwmon.join("power1_average")).map(|v| v as f64 / 1_000_000.0);
            t.power_cap_watts =
                read_sysfs_u64(&hwmon.join("power1_cap")).map(|v| v as f64 / 1_000_000.0);
            t.core_clock_mhz =
                read_sysfs_u64(&hwmon.join("freq1_input")).map(|v| v as f64 / 1_000_000.0);
            t.memory_clock_mhz =
                read_sysfs_u64(&hwmon.join("freq2_input")).map(|v| v as f64 / 1_000_000.0);
            t.fan_rpm = read_sysfs_u64(&hwmon.join("fan1_input"));
        }

        if self.vendor == GpuVendor::Amd {
            t.utilization_percent =
                read_sysfs_u64(&self.sysfs_device.join("gpu_busy_percent")).map(|v| v as f64);
            t.vram_total_bytes = read_sysfs_u64(&self.sysfs_device.join("mem_info_vram_total"));
            t.vram_used_bytes = read_sysfs_u64(&self.sysfs_device.join("mem_info_vram_used"));
        }

        t
    }

    /// Render node path (e.g. `/dev/dri/renderD128`).
    #[must_use]
    pub fn render_node(&self) -> PathBuf {
        PathBuf::from(format!("/dev/dri/renderD{}", 128 + self.card_index))
    }

    /// Card device path (e.g. `/dev/dri/card0`).
    #[must_use]
    pub fn card_path(&self) -> PathBuf {
        PathBuf::from(format!("/dev/dri/card{}", self.card_index))
    }

    /// Read `PCIe` topology information for this GPU.
    #[must_use]
    pub fn pcie_topology(&self) -> PcieTopology {
        let generation = read_sysfs_string(&self.sysfs_device.join("current_link_speed"))
            .and_then(|s| sysfs::parse_pcie_gen(&s));
        let width = read_sysfs_string(&self.sysfs_device.join("current_link_width"))
            .and_then(|s| s.trim().parse().ok());
        let numa_node = read_sysfs_string(&self.sysfs_device.join("numa_node"))
            .and_then(|s| s.trim().parse().ok());
        let iommu_group = find_iommu_group(&self.sysfs_device);

        PcieTopology {
            generation,
            width,
            numa_node,
            iommu_group,
        }
    }

    /// Derive silicon capabilities from PCI device ID and sysfs vendor info.
    ///
    /// More precise than the wgpu name-based heuristic: uses the actual
    /// PCI device ID to look up known SM/CU counts and GPU architecture.
    #[must_use]
    pub fn silicon_capabilities(&self) -> SiliconCapabilities {
        match self.vendor {
            GpuVendor::Nvidia => silicon_tables::nvidia_silicon(self.device_id),
            GpuVendor::Amd => silicon_tables::amd_silicon(self.device_id),
            GpuVendor::Intel => silicon_tables::intel_silicon(),
            GpuVendor::Unknown => SiliconCapabilities::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use toadstool_core::silicon::{RtCoreGen, SiliconUnit, TensorCoreGen};

    use super::*;

    #[test]
    fn test_gpu_vendor_from_pci() {
        assert_eq!(GpuVendor::from_pci_vendor(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_vendor(0x10de), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_vendor(0x8086), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_pci_vendor(0xFFFF), GpuVendor::Unknown);
    }

    #[test]
    fn test_gpu_vendor_display() {
        assert_eq!(GpuVendor::Amd.to_string(), "AMD");
        assert_eq!(GpuVendor::Nvidia.to_string(), "NVIDIA");
        assert_eq!(GpuVendor::Intel.to_string(), "Intel");
    }

    #[test]
    fn test_discover_gpus_returns_vec() {
        let gpus = discover_gpus();
        for gpu in &gpus {
            assert!(!gpu.driver.is_empty() || gpu.vendor == GpuVendor::Unknown);
            assert!(gpu.card_path().to_string_lossy().contains("card"));
            assert!(gpu.render_node().to_string_lossy().contains("renderD"));
        }
    }

    #[test]
    fn test_telemetry_vram_utilization() {
        let mut t = GpuTelemetry::default();
        assert!(t.vram_utilization_percent().is_none());

        t.vram_total_bytes = Some(16_000_000_000);
        t.vram_used_bytes = Some(4_000_000_000);
        let pct = t.vram_utilization_percent().unwrap();
        assert!((pct - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_telemetry_vram_zero_total() {
        let t = GpuTelemetry {
            vram_total_bytes: Some(0),
            vram_used_bytes: Some(0),
            ..Default::default()
        };
        assert!(t.vram_utilization_percent().is_none());
    }

    #[test]
    fn test_parse_pcie_gen() {
        assert_eq!(sysfs::parse_pcie_gen("8.0 GT/s PCIe"), Some(3));
        assert_eq!(sysfs::parse_pcie_gen("16.0 GT/s PCIe"), Some(4));
        assert_eq!(sysfs::parse_pcie_gen("32.0 GT/s PCIe"), Some(5));
        assert_eq!(sysfs::parse_pcie_gen("unknown"), None);
    }

    #[test]
    fn test_nvidia_silicon_volta() {
        let caps = silicon_tables::nvidia_silicon(0x1D81);
        assert_eq!(caps.tensor_cores, Some(TensorCoreGen::Volta));
        assert!(caps.rt_cores.is_none());
        assert_eq!(caps.estimated_tmu_count, 320);
        assert_eq!(caps.estimated_rop_count, 96);
        assert!(caps.has_unit(SiliconUnit::TensorCore));
        assert!(!caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_nvidia_silicon_ada() {
        let caps = silicon_tables::nvidia_silicon(0x2684);
        assert_eq!(caps.tensor_cores, Some(TensorCoreGen::Ada));
        assert_eq!(caps.rt_cores, Some(RtCoreGen::Ada));
        assert_eq!(caps.estimated_tmu_count, 512);
        assert_eq!(caps.estimated_rop_count, 176);
        assert!(caps.has_unit(SiliconUnit::TensorCore));
        assert!(caps.has_unit(SiliconUnit::RtCore));
        assert!(caps.has_unit(SiliconUnit::VideoEncoder));
    }

    #[test]
    fn test_nvidia_silicon_gtx_1660_no_tensor() {
        let caps = silicon_tables::nvidia_silicon(0x2182);
        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_none());
        assert!(!caps.has_unit(SiliconUnit::TensorCore));
        assert!(!caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_amd_silicon_rdna3() {
        let caps = silicon_tables::amd_silicon(0x744C);
        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_some());
        assert_eq!(caps.estimated_tmu_count, 384);
        assert!(caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_intel_silicon_baseline() {
        let caps = silicon_tables::intel_silicon();
        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_none());
        assert!(caps.has_video_encoder);
        assert_eq!(caps.available_units.len(), 7);
    }

    #[test]
    #[ignore = "requires GPU hardware"]
    fn test_discover_gpus_on_hardware() {
        let gpus = discover_gpus();
        assert!(!gpus.is_empty(), "Expected at least one GPU on hardware");

        for gpu in &gpus {
            println!(
                "card{}: {} {:04x} driver={} pci={}",
                gpu.card_index, gpu.vendor, gpu.device_id, gpu.driver, gpu.pci_slot
            );

            let telem = gpu.telemetry();
            println!(
                "  temp: {:?}°C, power: {:?}W, vram: {:?}/{:?} bytes",
                telem.temperature_celsius,
                telem.power_watts,
                telem.vram_used_bytes,
                telem.vram_total_bytes
            );

            let topo = gpu.pcie_topology();
            println!(
                "  PCIe gen{:?} x{:?}, NUMA {:?}, IOMMU group {:?}",
                topo.generation, topo.width, topo.numa_node, topo.iommu_group
            );
        }
    }
}
