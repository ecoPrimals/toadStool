// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pipeline result builders for normal completion, failure, and debug halts.

use std::time::Instant;

use crate::vfio::boot_state::SovereignBootState;
use crate::vfio::channel::hbm2_training::TrainingLog;
use crate::vfio::sovereign_types::{SovereignInitResult, StageResult, StageStatus};

#[expect(
    clippy::too_many_arguments,
    reason = "pipeline result builder aggregates all stage outputs"
)]
pub(crate) fn finish(
    bdf: &str,
    boot0: u32,
    chip_id: u32,
    stages: Vec<StageResult>,
    training_log: Option<TrainingLog>,
    start: Instant,
    warm: bool,
    boot_state: Option<SovereignBootState>,
) -> SovereignInitResult {
    SovereignInitResult {
        bdf: bdf.to_string(),
        identity_chip: chip_id,
        identity_raw: boot0,
        all_ok: false,
        compute_ready: false,
        halted_at: None,
        stages,
        total_ms: start.elapsed().as_millis() as u64,
        training_writes: training_log.as_ref().map(|l| l.write_count()),
        warm_detected: warm,
        boot_state,
    }
}

pub(crate) fn finish_halted(
    bdf: &str,
    boot0: u32,
    chip_id: u32,
    stage: &str,
    stages: Vec<StageResult>,
    start: Instant,
) -> SovereignInitResult {
    SovereignInitResult {
        bdf: bdf.to_string(),
        identity_chip: chip_id,
        identity_raw: boot0,
        all_ok: stages.iter().all(|s| s.status != StageStatus::Failed),
        compute_ready: false,
        halted_at: Some(stage.to_string()),
        stages,
        total_ms: start.elapsed().as_millis() as u64,
        training_writes: None,
        warm_detected: false,
        boot_state: None,
    }
}
