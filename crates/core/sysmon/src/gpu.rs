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

use toadstool_core::silicon::{RtCoreGen, SiliconCapabilities, SiliconUnit, TensorCoreGen};

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

impl GpuDevice {
    /// Derive silicon capabilities from PCI device ID and sysfs vendor info.
    ///
    /// More precise than the wgpu name-based heuristic: uses the actual
    /// PCI device ID to look up known SM/CU counts and GPU architecture.
    #[must_use]
    pub fn silicon_capabilities(&self) -> SiliconCapabilities {
        match self.vendor {
            GpuVendor::Nvidia => nvidia_silicon(self.device_id),
            GpuVendor::Amd => amd_silicon(self.device_id),
            GpuVendor::Intel => intel_silicon(),
            GpuVendor::Unknown => SiliconCapabilities::default(),
        }
    }
}

/// NVIDIA silicon capabilities by PCI device ID.
///
/// Covers Volta, Turing, Ampere, Ada families. TMU and ROP counts
/// come from published GPU specs (not estimated from name strings).
fn nvidia_silicon(device_id: u32) -> SiliconCapabilities {
    let (tensor_gen, rt_gen, tmu, rop) = match device_id {
        // Volta (GV100): Titan V, Tesla V100
        0x1D81 | 0x1DB1 | 0x1DB4..=0x1DBA => (Some(TensorCoreGen::Volta), None, 320_u32, 96_u32),
        // Turing (TU102): RTX 2080 Ti, Titan RTX, Quadro RTX 8000/6000
        0x1E02..=0x1E3F | 0x1E82..=0x1EBF => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            288,
            96,
        ),
        // Turing (TU104): RTX 2080/2080 Super, Quadro RTX 5000
        0x1E04..=0x1E7F | 0x1F02..=0x1F3F => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            192,
            64,
        ),
        // Turing (TU106): RTX 2070/2060
        0x1E84..=0x1EFF | 0x1F82..=0x1FBF => (
            Some(TensorCoreGen::Turing),
            Some(RtCoreGen::Turing),
            120,
            64,
        ),
        // Turing (TU116/TU117): GTX 1660/1650 — no tensor/RT cores
        0x2182..=0x21FF => (None, None, 96, 48),
        // Ampere (GA102): RTX 3090/3080 Ti/3080
        0x2204..=0x223F => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            328,
            112,
        ),
        // Ampere (GA104): RTX 3070/3060 Ti
        0x2484..=0x24BF => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            192,
            96,
        ),
        // Ampere (GA106): RTX 3060
        0x2504..=0x253F => (
            Some(TensorCoreGen::Ampere),
            Some(RtCoreGen::Ampere),
            112,
            48,
        ),
        // A100 (GA100)
        0x20B0..=0x20BF | 0x20F1..=0x20FF => (Some(TensorCoreGen::Ampere), None, 432, 160),
        // Ada Lovelace (AD102): RTX 4090/4080
        0x2684..=0x26BF => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 512, 176),
        // Ada Lovelace (AD104): RTX 4070 Ti/4070
        0x2704..=0x273F => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 240, 80),
        // Ada Lovelace (AD106): RTX 4060 Ti/4060
        0x2784..=0x27BF => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 136, 48),
        // Ada Lovelace (AD107): RTX 4060 mobile / 4050
        0x2804..=0x283F => (Some(TensorCoreGen::Ada), Some(RtCoreGen::Ada), 96, 32),
        // Unknown NVIDIA — conservative baseline
        _ => (None, None, 128, 64),
    };

    let has_tensor = tensor_gen.is_some();
    let has_rt = rt_gen.is_some();

    let mut units = vec![
        SiliconUnit::ShaderCore,
        SiliconUnit::TextureUnit,
        SiliconUnit::Rop,
        SiliconUnit::Rasterizer,
        SiliconUnit::DepthBuffer,
        SiliconUnit::Tessellator,
    ];
    if has_tensor {
        units.push(SiliconUnit::TensorCore);
    }
    if has_rt {
        units.push(SiliconUnit::RtCore);
    }
    units.push(SiliconUnit::VideoEncoder);

    SiliconCapabilities {
        tensor_cores: tensor_gen,
        rt_cores: rt_gen,
        has_video_encoder: true,
        estimated_tmu_count: tmu,
        estimated_rop_count: rop,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: units,
    }
}

/// AMD silicon capabilities by device ID.
fn amd_silicon(device_id: u32) -> SiliconCapabilities {
    let (rt_gen, tmu, rop) = match device_id {
        // RDNA 3: Navi 31 (RX 7900 XTX/XT)
        0x744C | 0x7448 => (Some(RtCoreGen::Ampere), 384, 192),
        // RDNA 3: Navi 32 (RX 7800 XT/7700 XT)
        0x7480..=0x749F => (Some(RtCoreGen::Ampere), 240, 96),
        // RDNA 3: Navi 33 (RX 7600)
        0x7400..=0x743F => (Some(RtCoreGen::Ampere), 128, 64),
        // RDNA 2: Navi 21 (RX 6950 XT/6900 XT/6800 XT)
        0x73BF | 0x73A5 | 0x73AF => (Some(RtCoreGen::Turing), 320, 128),
        // RDNA 2: Navi 22 (RX 6700 XT)
        0x73DF | 0x73FF => (Some(RtCoreGen::Turing), 160, 64),
        // CDNA: MI50/MI60 (no RT, no rasterizer in compute mode)
        0x66A0..=0x66AF => (None, 256, 64),
        // Unknown AMD
        _ => (None, 128, 64),
    };

    let mut units = vec![
        SiliconUnit::ShaderCore,
        SiliconUnit::TextureUnit,
        SiliconUnit::Rop,
        SiliconUnit::Rasterizer,
        SiliconUnit::DepthBuffer,
        SiliconUnit::Tessellator,
    ];
    if rt_gen.is_some() {
        units.push(SiliconUnit::RtCore);
    }
    units.push(SiliconUnit::VideoEncoder);

    SiliconCapabilities {
        tensor_cores: None,
        rt_cores: rt_gen,
        has_video_encoder: true,
        estimated_tmu_count: tmu,
        estimated_rop_count: rop,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: units,
    }
}

/// Intel GPU silicon — conservative baseline (no tensor/RT).
fn intel_silicon() -> SiliconCapabilities {
    SiliconCapabilities {
        tensor_cores: None,
        rt_cores: None,
        has_video_encoder: true,
        estimated_tmu_count: 64,
        estimated_rop_count: 32,
        rasterizer_available: true,
        tessellator_available: true,
        available_units: vec![
            SiliconUnit::ShaderCore,
            SiliconUnit::TextureUnit,
            SiliconUnit::Rop,
            SiliconUnit::Rasterizer,
            SiliconUnit::DepthBuffer,
            SiliconUnit::Tessellator,
            SiliconUnit::VideoEncoder,
        ],
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
    fn test_nvidia_silicon_volta() {
        let caps = nvidia_silicon(0x1D81); // Titan V
        assert_eq!(caps.tensor_cores, Some(TensorCoreGen::Volta));
        assert!(caps.rt_cores.is_none());
        assert_eq!(caps.estimated_tmu_count, 320);
        assert_eq!(caps.estimated_rop_count, 96);
        assert!(caps.has_unit(SiliconUnit::TensorCore));
        assert!(!caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_nvidia_silicon_ada() {
        let caps = nvidia_silicon(0x2684); // RTX 4090
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
        let caps = nvidia_silicon(0x2182); // GTX 1660 (TU116)
        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_none());
        assert!(!caps.has_unit(SiliconUnit::TensorCore));
        assert!(!caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_amd_silicon_rdna3() {
        let caps = amd_silicon(0x744C); // RX 7900 XTX
        assert!(caps.tensor_cores.is_none());
        assert!(caps.rt_cores.is_some());
        assert_eq!(caps.estimated_tmu_count, 384);
        assert!(caps.has_unit(SiliconUnit::RtCore));
    }

    #[test]
    fn test_intel_silicon_baseline() {
        let caps = intel_silicon();
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
