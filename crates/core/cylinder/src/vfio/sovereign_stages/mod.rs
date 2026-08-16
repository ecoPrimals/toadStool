// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-stage implementations for [`crate::vfio::sovereign_init::sovereign_init`].

mod devinit;
mod experiment;
mod experiment_chip;
mod experiment_snapshot;
mod experiment_stage_init;
mod experiment_stage_ungate;
mod gr;
mod memory;
mod pmc;
mod power;

#[cfg(test)]
mod tests;

pub use crate::error::SovereignStagesError;

pub(crate) use devinit::verify;
pub(crate) use gr::{falcon_boot, gr_init};
pub use memory::{MemoryTrainingResult, MemoryTrainingStrategy};
pub(crate) use memory::{
    chip_id_to_sm, dispatch_memory_training, gddr5_training, is_warm_gpu, pramin_sentinel_test,
    run_hbm2_training,
};
pub(crate) use pmc::{
    DevinitState,
    PMC_ENABLE, PmcEnableResult, bar0_probe, pgraph_engine_reset, pmc_enable, pmc_enable_full,
    pmc_enable_rollback,
};
pub(crate) use power::{cg_sweep, pgob_ungating, pri_bus_recover};

#[expect(unused_imports, reason = "re-exports for sovereign stage consumers")]
pub(crate) use experiment::{
    AMD_GRBM_STATUS, detect_chip_legacy, experiment_stage_1, experiment_stage_2,
    experiment_stage_3, experiment_stage_4_with_chip, experiment_stage_5,
    experiment_stage_6_with_chip,
};
pub use experiment::{
    ChipDetection, ExperimentResult, ExperimentWrite, SnapshotDelta, SovereignSnapshot,
    detect_chip, run_experiment_stage, sovereign_snapshot_only,
};
