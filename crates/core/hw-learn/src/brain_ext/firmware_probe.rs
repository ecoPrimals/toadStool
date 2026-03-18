// SPDX-License-Identifier: AGPL-3.0-or-later
//! Firmware inventory probing — re-exports from toadstool-sysmon.
//!
//! The actual probing lives in sysmon (where filesystem access belongs).
//! This module re-exports the types and adds hw-learn-specific helpers.

pub use toadstool_sysmon::{FirmwareInventory, FwStatus};

/// Extension trait for `FirmwareInventory` with learning-specific queries.
pub trait FirmwareInventoryExt {
    /// Whether this GPU can serve as a teacher (has working compute).
    fn can_teach(&self) -> bool;
    /// Whether this GPU needs learning assistance.
    fn needs_learning(&self) -> bool;
    /// Summary string for display.
    fn status_summary(&self) -> String;
}

impl FirmwareInventoryExt for FirmwareInventory {
    fn can_teach(&self) -> bool {
        self.compute_viable
    }

    fn needs_learning(&self) -> bool {
        !self.compute_viable && self.blocking_reason.is_some()
    }

    fn status_summary(&self) -> String {
        if self.compute_viable {
            "compute viable — can serve as teacher".to_string()
        } else if let Some(reason) = &self.blocking_reason {
            format!("compute blocked: {reason}")
        } else {
            "compute status unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amd_can_teach() {
        let inv = FirmwareInventory {
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
        assert!(inv.can_teach());
        assert!(!inv.needs_learning());
    }

    #[test]
    fn blocked_gpu_needs_learning() {
        let inv = FirmwareInventory {
            compute_viable: false,
            blocking_reason: Some("missing PMU firmware".into()),
            ..Default::default()
        };
        assert!(!inv.can_teach());
        assert!(inv.needs_learning());
    }

    #[test]
    fn status_summary_format() {
        let viable = FirmwareInventory {
            compute_viable: true,
            ..Default::default()
        };
        assert!(viable.status_summary().contains("teacher"));

        let blocked = FirmwareInventory {
            compute_viable: false,
            blocking_reason: Some("missing PMU".into()),
            ..Default::default()
        };
        assert!(blocked.status_summary().contains("missing PMU"));
    }
}
