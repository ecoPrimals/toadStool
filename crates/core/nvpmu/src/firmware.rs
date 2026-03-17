// SPDX-License-Identifier: AGPL-3.0-only
//! Firmware inventory probing for NVIDIA GPU subsystems.
//!
//! Probes `/lib/firmware/nvidia/{chip}/` for each firmware component
//! required by nouveau: PMU, GSP, ACR, GR, SEC2, NVDEC.

use std::path::{Path, PathBuf};

const FIRMWARE_BASE: &str = "/lib/firmware/nvidia";

/// Presence status of a firmware component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum FwStatus {
    Present,
    Missing,
}

impl FwStatus {
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }
}

/// Firmware inventory for a specific NVIDIA chip.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FirmwareInventory {
    pub chip: String,
    pub pmu: FwStatus,
    pub gsp: FwStatus,
    pub acr: FwStatus,
    pub gr: FwStatus,
    pub sec2: FwStatus,
    pub nvdec: FwStatus,
}

impl FirmwareInventory {
    /// Probe the filesystem for firmware availability.
    #[must_use]
    pub fn probe(chip: &str) -> Self {
        let base = PathBuf::from(FIRMWARE_BASE).join(chip);
        Self {
            chip: chip.to_string(),
            pmu: probe_dir(&base.join("pmu")),
            gsp: probe_gsp(&base),
            acr: probe_dir(&base.join("acr")),
            gr: probe_dir(&base.join("gr")),
            sec2: probe_dir(&base.join("sec2")),
            nvdec: probe_dir(&base.join("nvdec")),
        }
    }

    /// Whether nouveau can create compute channels.
    ///
    /// Requires GR firmware plus either PMU (Volta/Turing) or GSP (Ampere+).
    #[must_use]
    pub const fn compute_viable(&self) -> bool {
        self.gr.is_present() && (self.pmu.is_present() || self.gsp.is_present())
    }

    /// List components that block compute.
    #[must_use]
    pub fn compute_blockers(&self) -> Vec<&'static str> {
        let mut blockers = Vec::new();
        if !self.gr.is_present() {
            blockers.push("gr");
        }
        if !self.pmu.is_present() && !self.gsp.is_present() {
            blockers.push("pmu/gsp");
        }
        blockers
    }

    /// Whether a software PMU is needed (PMU firmware missing, no GSP fallback).
    #[must_use]
    pub const fn needs_software_pmu(&self) -> bool {
        !self.pmu.is_present() && !self.gsp.is_present()
    }
}

fn probe_dir(path: &Path) -> FwStatus {
    if path.is_dir() {
        FwStatus::Present
    } else {
        FwStatus::Missing
    }
}

/// GSP firmware lives as files in the chip directory, not a subdirectory.
fn probe_gsp(chip_dir: &Path) -> FwStatus {
    if chip_dir.join("gsp").is_dir() {
        return FwStatus::Present;
    }
    if let Ok(entries) = std::fs::read_dir(chip_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("gsp-") {
                return FwStatus::Present;
            }
        }
    }
    FwStatus::Missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_nonexistent_chip() {
        let inv = FirmwareInventory::probe("nonexistent_chip_zzzz");
        assert_eq!(inv.pmu, FwStatus::Missing);
        assert_eq!(inv.gsp, FwStatus::Missing);
        assert!(!inv.compute_viable());
        assert!(inv.needs_software_pmu());
    }

    #[test]
    fn compute_blockers_without_gr() {
        let inv = FirmwareInventory {
            chip: "test".into(),
            pmu: FwStatus::Missing,
            gsp: FwStatus::Missing,
            acr: FwStatus::Present,
            gr: FwStatus::Missing,
            sec2: FwStatus::Present,
            nvdec: FwStatus::Present,
        };
        let blockers = inv.compute_blockers();
        assert!(blockers.contains(&"gr"));
        assert!(blockers.contains(&"pmu/gsp"));
    }

    #[test]
    fn compute_viable_with_gsp() {
        let inv = FirmwareInventory {
            chip: "ad104".into(),
            pmu: FwStatus::Missing,
            gsp: FwStatus::Present,
            acr: FwStatus::Present,
            gr: FwStatus::Present,
            sec2: FwStatus::Present,
            nvdec: FwStatus::Present,
        };
        assert!(inv.compute_viable());
        assert!(!inv.needs_software_pmu());
    }
}
