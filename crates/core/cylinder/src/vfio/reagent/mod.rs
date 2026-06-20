// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reagent capture pipeline — systematic extraction and cataloging of
//! firmware "chemical agents" that enable GPU compute.
//!
//! While nvidia is loaded as a runtime service, the diesel engine captures
//! every component that enables Tier 2 compute: BAR0 register state,
//! falcon firmware blobs, ACR boot sequences, and linux-firmware binaries.
//! These are stored as versioned "reagents" for late-stage sovereign
//! Tier 2 replay without the vendor driver.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Track A: Runtime Services (nvidia loaded, Tier 2 now)      │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Track B: Reagent Capture (extract while alive)             │
//! │   BAR0 snapshot → falcon IMEM/DMEM → ACR sequence → blobs │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Track C: Late-stage replay (sovereign Tier 2 from reagents)│
//! └─────────────────────────────────────────────────────────────┘
//! ```

mod capture;
mod catalog;
mod mmiotrace;
mod vram_capture;

pub use capture::{
    ReagentCaptureResult, RuntimeServicesConfig, StrategyResult, execute_reagent_capture,
};
pub use catalog::{FirmwareBlob, catalog_linux_firmware};
pub use mmiotrace::{MmiotraceReagentSummary, distill_mmiotrace_to_reagent};
pub use vram_capture::{capture_vram_firmware, read_vram_via_pramin, vram_firmware_addrs};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Errors from reagent capture and persistence operations.
#[derive(Debug, thiserror::Error)]
pub enum ReagentError {
    #[error("Failed to create reagent dir {path}: {source}")]
    CreateDirFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to serialize manifest: {0}")]
    Serialize(serde_json::Error),

    #[error("Failed to write manifest: {0}")]
    WriteManifest(std::io::Error),

    #[error("Failed to read manifest {path}: {source}")]
    ReadManifest {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse manifest: {0}")]
    ParseManifest(serde_json::Error),

    #[error("mkdir {path}: {source}")]
    MirrorMkdirFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse mmiotrace: {0}")]
    MmiotraceParse(#[from] crate::error::ChannelError),

    #[error("Failed to write recipe: {0}")]
    WriteRecipe(std::io::Error),

    #[error("Requested {len} bytes exceeds PRAMIN window size {max}")]
    PraminSizeExceeded { len: usize, max: usize },

    #[error("PRAMIN register access failed: {0}")]
    PraminAccess(#[from] crate::error::DriverError),

    #[error(
        "{name} VRAM capture mostly zeros ({nonzero}/{total} nonzero) — firmware may not be staged at 0x{addr:x}"
    )]
    VramCaptureEmpty {
        name: &'static str,
        nonzero: usize,
        total: usize,
        addr: u64,
    },

    #[error("No BAR0 resource at {path}")]
    NoBar0Resource { path: String },

    #[error("mkdir firmware: {0}")]
    CatalystMkdirFailed(std::io::Error),
}

/// Default runtime reagent storage directory.
pub fn reagent_store_dir() -> String {
    crate::linux_paths::data_subdir("reagents")
}

/// Default chip identifier when BOOT0 discovery is unavailable (Volta GV100).
pub const DEFAULT_REAGENT_CHIP: &str = "gv100";

/// Default nvidia driver version when `/proc/driver/nvidia/version` is unavailable.
pub const DEFAULT_REAGENT_DRIVER_VERSION: &str = "470.256.02";

/// Discover GPU chip name from BAR0 BOOT0 via [`GenerationProfile`].
///
/// Reads PMC BOOT0 at offset 0, decodes SM version, and returns
/// `profile.firmware_chip`. Returns `None` when BAR0 is unreadable or BOOT0
/// is zero/`0xFFFF_FFFF`.
#[must_use]
pub fn discover_chip_from_bar0(bar0: &crate::vfio::device::MappedBar) -> Option<&'static str> {
    let boot0 = bar0.read_u32(0).unwrap_or(0);
    if boot0 == 0 || boot0 == 0xFFFF_FFFF {
        return None;
    }
    let sm = crate::nv::identity::boot0_to_sm(boot0)?;
    Some(crate::nv::generation::profile_for_sm(sm).firmware_chip)
}

/// Discover GPU chip name from sysfs BAR0 for a PCI BDF.
///
/// Opens `resource0` read/write and delegates to [`discover_chip_from_bar0`].
/// Used by reagent capture when the RPC caller omits an explicit `chip`.
#[must_use]
pub fn discover_chip_from_bdf(bdf: &str) -> Option<&'static str> {
    const BAR0_SIZE: usize = 16 * 1024 * 1024;
    let bar0 = crate::vfio::device::MappedBar::from_sysfs_rw(bdf, BAR0_SIZE).ok()?;
    discover_chip_from_bar0(&bar0)
}

/// Discover the loaded nvidia kernel module version from procfs.
///
/// Parses `/proc/driver/nvidia/version` for the `major.minor.patch` token
/// in the NVRM version line (e.g. `470.256.02`).
#[must_use]
pub fn discover_nvidia_driver_version() -> Option<String> {
    let content = std::fs::read_to_string("/proc/driver/nvidia/version").ok()?;
    parse_nvidia_proc_version(&content)
}

fn parse_nvidia_proc_version(content: &str) -> Option<String> {
    for token in content.split_whitespace() {
        if token.chars().next()?.is_ascii_digit()
            && token.matches('.').count() >= 2
            && token.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return Some(token.to_owned());
        }
    }
    None
}

/// A unified artifact combining all captured chemical agents for a
/// specific GPU + driver + kernel combination.
///
/// The manifest is the receipt proving what was captured. Each field
/// points to an artifact file within the reagent directory. Optional
/// fields reflect capture strategies that may fail (e.g., IMEM capture
/// blocked by HS fuses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReagentManifest {
    /// GPU chip identifier (e.g. "GV100", "GA102").
    pub chip: String,
    /// Driver version that served as the chemical catalyst (e.g. "470.256.02").
    pub driver_version: String,
    /// Kernel version the catalyst ran on (e.g. "6.17.9-76061709-generic").
    pub kernel_version: String,
    /// PCI BDF of the device this was captured from.
    pub bdf: String,
    /// ISO 8601 timestamp of capture.
    pub captured_at: String,
    /// Path to BAR0 domain snapshot JSON (alive registers only).
    pub bar0_snapshot: Option<PathBuf>,
    /// Path to BAR0 replay sequence JSON (`GrInitSequence`).
    pub bar0_replay: Option<PathBuf>,
    /// Path to the PatchSet JSON used to prepare the catalyst driver.
    pub patch_set: Option<PathBuf>,
    /// Path to frozen patched .ko binary (41+ MB, gitignored).
    pub frozen_module: Option<PathBuf>,
    /// Firmware artifacts captured or cataloged.
    pub firmware: ReagentFirmware,
    /// Path to mmiotrace-distilled recipe JSON.
    pub mmiotrace_recipe: Option<PathBuf>,
    /// Per-artifact metadata (sizes, provenance notes).
    pub metadata: HashMap<String, String>,
    /// Capture completeness — what fraction of reagents were successfully captured.
    pub completeness: ReagentCompleteness,
}

/// Firmware reagent artifacts — the chemical agents that enable compute.
///
/// Each `Option<PathBuf>` points to a captured binary blob. Capture may
/// fail due to HS fuse enforcement, RM gating, or IMEM wipe during unbind.
/// The `source` field documents provenance for each blob.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReagentFirmware {
    /// FECS instruction memory (if captured or extracted).
    pub fecs_imem: Option<PathBuf>,
    /// FECS data memory.
    pub fecs_dmem: Option<PathBuf>,
    /// GPCCS instruction memory.
    pub gpccs_imem: Option<PathBuf>,
    /// GPCCS data memory.
    pub gpccs_dmem: Option<PathBuf>,
    /// PMU instruction memory.
    pub pmu_imem: Option<PathBuf>,
    /// PMU data memory.
    pub pmu_dmem: Option<PathBuf>,
    /// SEC2 instruction memory.
    pub sec2_imem: Option<PathBuf>,
    /// ACR boot sequence distilled from mmiotrace or BAR0 capture.
    pub acr_sequence: Option<PathBuf>,
    /// Signed firmware blobs from `/lib/firmware/nvidia/{chip}/`.
    pub linux_firmware_blobs: Vec<FirmwareBlob>,
    /// Per-blob provenance notes.
    pub provenance: HashMap<String, String>,
}

/// How complete the reagent capture was.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ReagentCompleteness {
    /// BAR0 register snapshot captured.
    pub bar0_snapshot: bool,
    /// BAR0 replay sequence generated.
    pub bar0_replay: bool,
    /// Patch set recipe captured.
    pub patch_set: bool,
    /// Any falcon IMEM/DMEM captured (pre-swap or post-recovery).
    pub falcon_firmware: bool,
    /// linux-firmware blobs cataloged.
    pub linux_firmware: bool,
    /// mmiotrace recipe distilled.
    pub mmiotrace_recipe: bool,
    /// VRAM firmware captured via PRAMIN.
    pub vram_firmware: bool,
}

impl ReagentCompleteness {
    /// Fraction of capture strategies that succeeded (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f64 {
        let total = 7.0;
        let done = [
            self.bar0_snapshot,
            self.bar0_replay,
            self.patch_set,
            self.falcon_firmware,
            self.linux_firmware,
            self.mmiotrace_recipe,
            self.vram_firmware,
        ]
        .iter()
        .filter(|&&v| v)
        .count() as f64;
        done / total
    }
}

impl ReagentManifest {
    /// Create a new empty manifest for a capture session.
    pub fn new(chip: &str, driver_version: &str, kernel_version: &str, bdf: &str) -> Self {
        let ts = chrono_iso8601_now();
        Self {
            chip: chip.to_owned(),
            driver_version: driver_version.to_owned(),
            kernel_version: kernel_version.to_owned(),
            bdf: bdf.to_owned(),
            captured_at: ts,
            bar0_snapshot: None,
            bar0_replay: None,
            patch_set: None,
            frozen_module: None,
            firmware: ReagentFirmware::default(),
            mmiotrace_recipe: None,
            metadata: HashMap::new(),
            completeness: ReagentCompleteness::default(),
        }
    }

    /// Directory name for this reagent set (chip + driver + kernel).
    #[must_use]
    pub fn dir_name(&self) -> String {
        let kernel_short = self
            .kernel_version
            .split('-')
            .next()
            .unwrap_or(&self.kernel_version);
        format!(
            "{}_nvidia{}_k{}",
            self.chip.to_lowercase(),
            self.driver_version.replace('.', ""),
            kernel_short,
        )
    }

    /// Full path to the reagent directory.
    #[must_use]
    pub fn store_path(&self) -> PathBuf {
        PathBuf::from(reagent_store_dir()).join(self.dir_name())
    }

    /// Write the manifest to the reagent store directory.
    ///
    /// Creates the directory structure if it doesn't exist.
    pub fn persist(&self) -> Result<PathBuf, ReagentError> {
        let dir = self.store_path();
        std::fs::create_dir_all(&dir).map_err(|source| ReagentError::CreateDirFailed {
            path: dir.display().to_string(),
            source,
        })?;

        let manifest_path = dir.join("manifest.json");
        let json = serde_json::to_string_pretty(self).map_err(ReagentError::Serialize)?;
        std::fs::write(&manifest_path, json).map_err(ReagentError::WriteManifest)?;

        tracing::info!(
            path = %manifest_path.display(),
            chip = %self.chip,
            completeness = format!("{:.0}%", self.completeness.fraction() * 100.0),
            "Reagent manifest persisted"
        );
        Ok(manifest_path)
    }

    /// Load a manifest from disk.
    pub fn load(path: &Path) -> Result<Self, ReagentError> {
        let data = std::fs::read_to_string(path).map_err(|source| ReagentError::ReadManifest {
            path: path.display().to_string(),
            source,
        })?;
        serde_json::from_str(&data).map_err(ReagentError::ParseManifest)
    }

    /// Mirror small JSON artifacts to a repo-side reagent directory.
    ///
    /// Large binaries (frozen .ko, full mmiotrace) are NOT copied — only
    /// manifest.json and JSON recipe files under a size threshold.
    pub fn mirror_to_repo(&self, repo_reagents_dir: &Path) -> Result<PathBuf, ReagentError> {
        const MAX_MIRROR_SIZE: u64 = 5 * 1024 * 1024; // 5 MiB threshold

        let dest_dir = repo_reagents_dir.join(self.dir_name());
        std::fs::create_dir_all(&dest_dir).map_err(|source| ReagentError::MirrorMkdirFailed {
            path: dest_dir.display().to_string(),
            source,
        })?;
        std::fs::create_dir_all(dest_dir.join("firmware")).ok();
        std::fs::create_dir_all(dest_dir.join("mmiotrace")).ok();

        let manifest_path = dest_dir.join("manifest.json");
        let json = serde_json::to_string_pretty(self).map_err(ReagentError::Serialize)?;
        std::fs::write(&manifest_path, json).map_err(ReagentError::WriteManifest)?;

        let json_sources = [
            &self.bar0_snapshot,
            &self.bar0_replay,
            &self.patch_set,
            &self.mmiotrace_recipe,
        ];
        for src in json_sources.into_iter().flatten() {
            if src.exists() {
                let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(u64::MAX);
                if size < MAX_MIRROR_SIZE
                    && let Some(name) = src.file_name()
                {
                    std::fs::copy(src, dest_dir.join(name)).ok();
                }
            }
        }

        tracing::info!(
            dest = %dest_dir.display(),
            "Reagent artifacts mirrored to repo"
        );
        Ok(manifest_path)
    }
}

fn chrono_iso8601_now() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    let days = secs / 86400;
    let year = 1970 + days / 365;
    format!("{year}-XX-XXT{hours:02}:{mins:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let manifest = ReagentManifest::new("GV100", "470.256.02", "6.17.9", "0000:02:00.0");
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let loaded: ReagentManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.chip, "GV100");
        assert_eq!(loaded.driver_version, "470.256.02");
        assert_eq!(loaded.bdf, "0000:02:00.0");
    }

    #[test]
    fn completeness_fraction() {
        let mut c = ReagentCompleteness::default();
        assert!((c.fraction() - 0.0).abs() < f64::EPSILON);

        c.bar0_snapshot = true;
        c.linux_firmware = true;
        c.patch_set = true;
        assert!((c.fraction() - 3.0 / 7.0).abs() < 0.01);
    }

    #[test]
    fn dir_name_format() {
        let m = ReagentManifest::new(
            "GV100",
            "470.256.02",
            "6.17.9-76061709-generic",
            "0000:02:00.0",
        );
        assert_eq!(m.dir_name(), "gv100_nvidia47025602_k6.17.9");
    }

    #[test]
    fn parse_nvidia_proc_version_extracts_driver_number() {
        let sample = "NVRM version: NVIDIA UNIX x86_64 Kernel Module  580.119.02  Thu Aug  7 20:09:00 UTC 2025";
        assert_eq!(
            super::parse_nvidia_proc_version(sample).as_deref(),
            Some("580.119.02")
        );
    }
}
