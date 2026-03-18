// SPDX-License-Identifier: AGPL-3.0-or-later
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

use std::path::{Path, PathBuf};

/// Vendor identification for GPU-specific sysfs paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Amd,
    Nvidia,
    Intel,
    Unknown,
}

impl GpuVendor {
    const fn from_pci_vendor(vendor_id: u32) -> Self {
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
}

/// Firmware status for a single firmware component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FwStatus {
    /// Firmware present (file exists and directory non-empty).
    Present,
    /// Firmware file not found in `/lib/firmware/`.
    Missing,
    /// Firmware not required for this GPU/driver combination.
    NotRequired,
    /// Status unknown (couldn't probe).
    Unknown,
}

impl std::fmt::Display for FwStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present => f.write_str("present"),
            Self::Missing => f.write_str("missing"),
            Self::NotRequired => f.write_str("not_required"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

/// Firmware inventory for a GPU — which firmware blobs are available.
///
/// Different vendors need different firmware:
/// - **NVIDIA**: PMU (Volta-), GSP (Turing+), ACR, GR, SEC2
/// - **Intel**: `GuC` (Graphics microController), `HuC` (HEVC codec)
/// - **AMD**: Usually none required (fully open driver)
#[derive(Debug, Clone)]
pub struct FirmwareInventory {
    /// NVIDIA Power Management Unit firmware.
    pub pmu: FwStatus,
    /// NVIDIA GPU System Processor firmware (Turing+).
    pub gsp: FwStatus,
    /// NVIDIA Application Context Rewriting firmware.
    pub acr: FwStatus,
    /// NVIDIA Graphics engine firmware.
    pub gr: FwStatus,
    /// NVIDIA SEC2 (secure engine) firmware.
    pub sec2: FwStatus,
    /// Intel Graphics microController firmware.
    pub guc: FwStatus,
    /// Intel HEVC/codec firmware.
    pub huc: FwStatus,
    /// Whether compute is expected to work given the firmware state.
    pub compute_viable: bool,
    /// What's blocking compute, if anything.
    pub blocking_reason: Option<String>,
}

impl Default for FirmwareInventory {
    fn default() -> Self {
        Self {
            pmu: FwStatus::Unknown,
            gsp: FwStatus::Unknown,
            acr: FwStatus::Unknown,
            gr: FwStatus::Unknown,
            sec2: FwStatus::Unknown,
            guc: FwStatus::Unknown,
            huc: FwStatus::Unknown,
            compute_viable: false,
            blocking_reason: None,
        }
    }
}

impl GpuDevice {
    /// Probe firmware availability for this GPU.
    ///
    /// Checks `/lib/firmware/nvidia/` (NVIDIA), `/lib/firmware/i915/` (Intel),
    /// or returns all-not-required (AMD).
    #[must_use]
    pub fn firmware_inventory(&self) -> FirmwareInventory {
        match self.vendor {
            GpuVendor::Nvidia => probe_nvidia_firmware(self.device_id, &self.driver),
            GpuVendor::Intel => probe_intel_firmware(self.device_id),
            GpuVendor::Amd => FirmwareInventory {
                pmu: FwStatus::NotRequired,
                gsp: FwStatus::NotRequired,
                acr: FwStatus::NotRequired,
                gr: FwStatus::NotRequired,
                sec2: FwStatus::NotRequired,
                guc: FwStatus::NotRequired,
                huc: FwStatus::NotRequired,
                compute_viable: true,
                blocking_reason: None,
            },
            GpuVendor::Unknown => FirmwareInventory::default(),
        }
    }
}

fn probe_nvidia_firmware(device_id: u32, driver: &str) -> FirmwareInventory {
    let chip_name = nvidia_chip_name(device_id);
    let base = Path::new("/lib/firmware/nvidia");

    let pmu = check_firmware_component(base, &chip_name, "pmu");
    let gsp = check_firmware_component(base, &chip_name, "gsp");
    let acr = check_firmware_component(base, &chip_name, "acr");
    let gr = check_firmware_component(base, &chip_name, "gr");
    let sec2 = check_firmware_component(base, &chip_name, "sec2");

    let is_nouveau = driver.contains("nouveau");
    let (compute_viable, blocking_reason) = if is_nouveau {
        if gsp == FwStatus::Present || pmu == FwStatus::Present {
            (true, None)
        } else {
            (
                false,
                Some("missing PMU and GSP firmware — compute channels unavailable".into()),
            )
        }
    } else {
        (true, None)
    };

    FirmwareInventory {
        pmu,
        gsp,
        acr,
        gr,
        sec2,
        guc: FwStatus::NotRequired,
        huc: FwStatus::NotRequired,
        compute_viable,
        blocking_reason,
    }
}

fn probe_intel_firmware(device_id: u32) -> FirmwareInventory {
    let base = Path::new("/lib/firmware/i915");
    let dev_hex = format!("{device_id:04x}");

    let guc = check_firmware_glob(base, &dev_hex, "guc");
    let huc = check_firmware_glob(base, &dev_hex, "huc");

    let compute_viable = guc == FwStatus::Present;
    let blocking_reason = if compute_viable {
        None
    } else {
        Some("GuC firmware missing — compute engine disabled".into())
    };

    FirmwareInventory {
        pmu: FwStatus::NotRequired,
        gsp: FwStatus::NotRequired,
        acr: FwStatus::NotRequired,
        gr: FwStatus::NotRequired,
        sec2: FwStatus::NotRequired,
        guc,
        huc,
        compute_viable,
        blocking_reason,
    }
}

/// Map NVIDIA PCI device ID to chip codename for firmware lookup.
fn nvidia_chip_name(device_id: u32) -> String {
    match device_id {
        0x1D81 | 0x1DB1 | 0x1DB4 | 0x1DB5 | 0x1DB6 | 0x1DB7 | 0x1DBA => "gv100".into(),
        0x1E02..=0x1E3F | 0x1E82..=0x1EBF | 0x2182..=0x21BF => "tu102".into(),
        0x1E04..=0x1E7F | 0x1F02..=0x1F3F | 0x2184..=0x21FF => "tu104".into(),
        0x1E84..=0x1EFF | 0x1F82..=0x1FBF => "tu106".into(),
        0x2204..=0x223F => "ga102".into(),
        0x2484..=0x24BF | 0x2504..=0x253F => "ga104".into(),
        0x2684..=0x26BF | 0x2704..=0x273F => "ad102".into(),
        0x2784..=0x27BF | 0x2804..=0x283F => "ad104".into(),
        _ => format!("dev{device_id:04x}"),
    }
}

fn check_firmware_component(base: &Path, chip: &str, component: &str) -> FwStatus {
    let dir = base.join(chip).join(component);
    if dir.exists()
        && std::fs::read_dir(&dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return FwStatus::Present;
    }
    let flat = base.join(format!("{chip}_{component}.bin"));
    if flat.exists() {
        FwStatus::Present
    } else {
        FwStatus::Missing
    }
}

fn check_firmware_glob(base: &Path, device_hint: &str, component: &str) -> FwStatus {
    let Ok(entries) = std::fs::read_dir(base) else {
        return FwStatus::Missing;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_lowercase();
        if name_str.contains(device_hint) && name_str.contains(component) {
            return FwStatus::Present;
        }
    }
    FwStatus::Missing
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

impl GpuDevice {
    /// Read `PCIe` topology information for this GPU.
    #[must_use]
    pub fn pcie_topology(&self) -> PcieTopology {
        let generation = read_sysfs_string(&self.sysfs_device.join("current_link_speed"))
            .and_then(|s| parse_pcie_gen(&s));
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
}

// --- sysfs helpers ---

fn read_sysfs_u64(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_sysfs_hex(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let trimmed = s.trim().trim_start_matches("0x");
    u32::from_str_radix(trimmed, 16).ok()
}

fn read_sysfs_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn read_sysfs_uevent_field(uevent_path: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(uevent_path).ok()?;
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(val) = line.strip_prefix(&prefix) {
            return Some(val.to_string());
        }
    }
    None
}

fn find_hwmon_dir(device_path: &Path) -> Option<PathBuf> {
    let hwmon_parent = device_path.join("hwmon");
    let entries = std::fs::read_dir(&hwmon_parent).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("hwmon") {
            return Some(entry.path());
        }
    }
    None
}

fn find_iommu_group(device_path: &Path) -> Option<u32> {
    let link = std::fs::read_link(device_path.join("iommu_group")).ok()?;
    let name = link.file_name()?.to_string_lossy();
    name.parse().ok()
}

fn parse_pcie_gen(speed_str: &str) -> Option<u32> {
    let s = speed_str.trim();
    if s.contains("32.0") || s.contains("32 GT") {
        Some(5)
    } else if s.contains("16.0") || s.contains("16 GT") {
        Some(4)
    } else if s.contains("8.0") || s.contains("8 GT") {
        Some(3)
    } else if s.contains("5.0") || s.contains("5 GT") {
        Some(2)
    } else if s.contains("2.5") {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
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
        // On any Linux system with DRM, this should return something
        // On CI without GPU, it returns an empty vec — both are valid
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
        assert_eq!(parse_pcie_gen("8.0 GT/s PCIe"), Some(3));
        assert_eq!(parse_pcie_gen("16.0 GT/s PCIe"), Some(4));
        assert_eq!(parse_pcie_gen("32.0 GT/s PCIe"), Some(5));
        assert_eq!(parse_pcie_gen("unknown"), None);
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
