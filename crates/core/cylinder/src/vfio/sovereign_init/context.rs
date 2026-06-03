// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mutable state carried through the sovereign init pipeline.

use std::time::Instant;

use crate::vfio::boot_state::SovereignBootState;
use crate::vfio::channel::hbm2_training::TrainingLog;
use crate::vfio::sovereign_types::{SovereignInitResult, StageResult};

use super::result;

/// Accumulated pipeline state shared across stage modules.
pub(crate) struct PipelineCtx<'a> {
    pub bdf: &'a str,
    pub boot0: u32,
    pub chip_id: u32,
    pub stages: Vec<StageResult>,
    pub pipeline_start: Instant,
    pub training_log: Option<TrainingLog>,
    pub warm_detected: bool,
    pub boot_state: Option<SovereignBootState>,
}

impl<'a> PipelineCtx<'a> {
    pub(crate) fn new(bdf: &'a str, pipeline_start: Instant) -> Self {
        Self {
            bdf,
            boot0: 0,
            chip_id: 0,
            stages: Vec::new(),
            pipeline_start,
            training_log: None,
            warm_detected: false,
            boot_state: None,
        }
    }

    pub(crate) fn finish_failed(&mut self) -> SovereignInitResult {
        result::finish(
            self.bdf,
            self.boot0,
            self.chip_id,
            std::mem::take(&mut self.stages),
            self.training_log.take(),
            self.pipeline_start,
            self.warm_detected,
            self.boot_state.take(),
        )
    }

    pub(crate) fn finish_halted(&mut self, stage: &str) -> SovereignInitResult {
        result::finish_halted(
            self.bdf,
            self.boot0,
            self.chip_id,
            stage,
            std::mem::take(&mut self.stages),
            self.pipeline_start,
        )
    }

    pub(crate) fn finish_success(&mut self) -> SovereignInitResult {
        let training_writes = self.training_log.as_ref().map(|l| l.write_count());
        SovereignInitResult {
            bdf: self.bdf.to_string(),
            identity_chip: self.chip_id,
            identity_raw: self.boot0,
            all_ok: true,
            compute_ready: true,
            halted_at: None,
            stages: std::mem::take(&mut self.stages),
            total_ms: self.pipeline_start.elapsed().as_millis() as u64,
            training_writes,
            warm_detected: self.warm_detected,
            boot_state: self.boot_state.take(),
        }
    }
}
