// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability gap analysis — what's preventing compute on a GPU.

use crate::distiller::{GpuArch, Vendor};
use serde::{Deserialize, Serialize};
use toadstool_sysmon::{FirmwareInventory, FwStatus};

/// What's preventing compute on a GPU.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    /// Missing firmware components.
    pub missing_firmware: Vec<String>,
    /// Whether the driver supports compute at all.
    pub driver_supports_compute: bool,
    /// Whether kernel version is sufficient.
    pub kernel_sufficient: bool,
    /// Severity: how hard is this to fix?
    pub severity: GapSeverity,
    /// Human-readable summary.
    pub summary: String,
}

/// How hard the gap is to bridge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GapSeverity {
    /// Trivially fixable (install firmware, update kernel).
    Easy,
    /// Requires specific firmware or driver version.
    Medium,
    /// Requires reverse engineering or hardware learning.
    Hard,
    /// Unknown — needs investigation.
    Unknown,
}

impl CapabilityGap {
    /// Diagnose the capability gap for a GPU.
    #[must_use]
    pub fn diagnose(firmware: &FirmwareInventory, arch: &GpuArch) -> Self {
        let mut missing_firmware = Vec::new();

        match arch.vendor {
            Vendor::Nvidia => {
                if firmware.pmu == FwStatus::Missing {
                    missing_firmware.push("PMU".into());
                }
                if firmware.gsp == FwStatus::Missing {
                    missing_firmware.push("GSP".into());
                }
                if firmware.acr == FwStatus::Missing {
                    missing_firmware.push("ACR".into());
                }
                if firmware.gr == FwStatus::Missing {
                    missing_firmware.push("GR".into());
                }
            }
            Vendor::Intel => {
                if firmware.guc == FwStatus::Missing {
                    missing_firmware.push("GuC".into());
                }
                if firmware.huc == FwStatus::Missing {
                    missing_firmware.push("HuC".into());
                }
            }
            Vendor::Amd => {
                // AMD typically doesn't need firmware for compute
            }
        }

        let severity = if missing_firmware.is_empty() {
            GapSeverity::Unknown
        } else if arch.vendor == Vendor::Nvidia
            && missing_firmware.contains(&"PMU".to_string())
            && firmware.gsp == FwStatus::Missing
        {
            // Both PMU and GSP missing — hard problem (desktop Volta)
            GapSeverity::Hard
        } else if missing_firmware.len() == 1 {
            GapSeverity::Medium
        } else {
            GapSeverity::Hard
        };

        let summary = if missing_firmware.is_empty() {
            if firmware.compute_viable {
                "no gap — compute should work".to_string()
            } else {
                firmware
                    .blocking_reason
                    .clone()
                    .unwrap_or_else(|| "unknown blocker".to_string())
            }
        } else {
            format!("missing firmware: {}", missing_firmware.join(", "))
        };

        Self {
            missing_firmware,
            driver_supports_compute: true,
            kernel_sufficient: true,
            severity,
            summary,
        }
    }
}

impl std::fmt::Display for CapabilityGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nvidia_arch(generation: &str, cc: &str) -> GpuArch {
        GpuArch {
            vendor: Vendor::Nvidia,
            generation: generation.into(),
            chip: "test".into(),
            compute_class: cc.into(),
        }
    }

    #[test]
    fn missing_pmu_and_gsp_is_hard() {
        let fw = FirmwareInventory {
            pmu: FwStatus::Missing,
            gsp: FwStatus::Missing,
            ..Default::default()
        };
        let gap = CapabilityGap::diagnose(&fw, &nvidia_arch("Volta", "sm70"));
        assert_eq!(gap.severity, GapSeverity::Hard);
        assert!(gap.missing_firmware.contains(&"PMU".to_string()));
        assert!(gap.missing_firmware.contains(&"GSP".to_string()));
    }

    #[test]
    fn amd_no_gap() {
        let fw = FirmwareInventory {
            pmu: FwStatus::NotRequired,
            gsp: FwStatus::NotRequired,
            acr: FwStatus::NotRequired,
            gr: FwStatus::NotRequired,
            sec2: FwStatus::NotRequired,
            guc: FwStatus::NotRequired,
            huc: FwStatus::NotRequired,
            compute_viable: true,
            blocking_reason: None,
        };
        let arch = GpuArch {
            vendor: Vendor::Amd,
            generation: "RDNA2".into(),
            chip: "Navi21".into(),
            compute_class: "gfx1030".into(),
        };
        let gap = CapabilityGap::diagnose(&fw, &arch);
        assert!(gap.missing_firmware.is_empty());
    }

    #[test]
    fn gap_display() {
        let fw = FirmwareInventory {
            pmu: FwStatus::Missing,
            ..Default::default()
        };
        let gap = CapabilityGap::diagnose(&fw, &nvidia_arch("Volta", "sm70"));
        let display = format!("{gap}");
        assert!(display.contains("PMU"));
    }
}
