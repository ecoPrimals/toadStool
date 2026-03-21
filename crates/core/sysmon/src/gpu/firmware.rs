// SPDX-License-Identifier: AGPL-3.0-only
//! GPU firmware inventory probing.
//!
//! Checks `/lib/firmware/` for vendor-specific firmware blobs required
//! by open-source GPU drivers (nouveau, i915).

use std::path::Path;

use super::GpuDevice;
use super::GpuVendor;

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
