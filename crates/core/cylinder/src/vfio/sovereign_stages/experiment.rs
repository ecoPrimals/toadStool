// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::vfio::device::MappedBar;

pub use super::experiment_chip::{ChipDetection, detect_chip};
pub(crate) use super::experiment_chip::{AMD_GRBM_STATUS, detect_chip_legacy};
pub use super::experiment_snapshot::{
    ExperimentResult, ExperimentWrite, SnapshotDelta, SovereignSnapshot, sovereign_snapshot_only,
};
pub(crate) use super::experiment_stage_init::{
    experiment_stage_1, experiment_stage_2, experiment_stage_3,
};
pub(crate) use super::experiment_stage_ungate::{
    experiment_stage_4_with_chip, experiment_stage_5, experiment_stage_6_with_chip,
};

/// Execute an experiment stage by number (1-6).
///
/// Accepts an optional `chip` override (e.g. `"gv100"`, `"gk210"`).
/// When `None`, auto-detects from BOOT0.
pub fn run_experiment_stage(
    bar0: &MappedBar,
    stage: u32,
    chip_override: Option<&str>,
) -> Result<ExperimentResult, String> {
    let (auto_chip, auto_sm) = detect_chip_legacy(bar0);
    let chip = chip_override.unwrap_or(auto_chip);
    let sm = auto_sm;

    match stage {
        1 => Ok(experiment_stage_1(bar0)),
        2 => Ok(experiment_stage_2(bar0)),
        3 => Ok(experiment_stage_3(bar0)),
        4 => Ok(experiment_stage_4_with_chip(bar0, chip, sm)),
        5 => Ok(experiment_stage_5(bar0)),
        6 => Ok(experiment_stage_6_with_chip(bar0, chip, sm)),
        _ => Err(format!("invalid stage {stage}: must be 1-6")),
    }
}
