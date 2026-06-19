// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pre-memory pipeline stages: identity probe, PMC enable, PGRAPH reset, CG/PRI, PGOB.

use std::time::Instant;

use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_stages::{
    PmcEnableResult, cg_sweep, pgob_ungating, pgraph_engine_reset, pmc_enable, pri_bus_recover,
};
use crate::vfio::sovereign_strategy::{FalconWarmState, SovereignStrategy};
use crate::vfio::sovereign_types::{
    HaltBefore, SovereignInitOptions, SovereignInitResult, StageResult, StageStatus,
};

use super::context::PipelineCtx;

/// Values needed by the memory-training path after pre-memory stages succeed.
pub(crate) struct PreMemoryContinue {
    pub pmc_result: PmcEnableResult,
}

pub(crate) enum PreMemoryOutcome {
    Done(SovereignInitResult),
    Continue(PreMemoryContinue),
}

/// Run stages 1–2c (through PGOB).
pub(crate) fn run(
    ctx: &mut PipelineCtx<'_>,
    bar0: &MappedBar,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
) -> PreMemoryOutcome {
    // ── Stage 1: Identity Probe ──────────────────────────────────────────
    let t = Instant::now();
    match strategy.probe_identity(bar0) {
        Ok(id) => {
            ctx.boot0 = id.identity_raw;
            ctx.chip_id = id.identity_chip;
            ctx.stages.push(StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Ok,
                detail: Some(format!(
                    "raw=0x{:08x} chip=0x{:03x}",
                    ctx.boot0, ctx.chip_id
                )),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        Err(e) => {
            ctx.stages.push(StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return PreMemoryOutcome::Done(ctx.finish_failed());
        }
    }

    let power = strategy.power_profile();

    tracing::info!(
        gen = strategy.family_name(),
        sm = strategy.sm_version(),
        initial_mask = format!("0x{:08x}", power.initial_pmc_mask),
        rollback = power.rollback_on_devinit_failure,
        "Generation profile resolved for sovereign pipeline"
    );

    // ── Early falcon probe (before pgraph_reset destroys state) ───────
    let early_falcon_state = strategy.detect_falcon_warm_state(bar0, true);
    let falcon_already_warm = matches!(
        early_falcon_state,
        FalconWarmState::WarmPreserved { .. } | FalconWarmState::WarmRunning { .. }
    );

    if falcon_already_warm {
        tracing::info!(
            state = ?early_falcon_state,
            "falcon already warm — will skip pgraph_reset to preserve firmware"
        );
    }

    // ── Stage 2: PMC Enable (staged) ──────────────────────────────────
    if opts.halt_before == Some(HaltBefore::PmcEnable) {
        ctx.stages.push(StageResult {
            name: "pmc_enable".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=pmc_enable".into()),
            duration_ms: 0,
        });
        return PreMemoryOutcome::Done(ctx.finish_halted("pmc_enable"));
    }

    let t = Instant::now();
    let pmc_result = match pmc_enable(bar0, power) {
        Ok(result) => {
            ctx.stages.push(StageResult {
                name: "pmc_enable".into(),
                status: StageStatus::Ok,
                detail: Some(result.detail()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            result
        }
        Err(e) => {
            ctx.stages.push(StageResult {
                name: "pmc_enable".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return PreMemoryOutcome::Done(ctx.finish_failed());
        }
    };

    // ── Stage 2a: PGRAPH Engine Reset ─────────────────────────────────
    if falcon_already_warm {
        ctx.stages.push(StageResult {
            name: "pgraph_reset".into(),
            status: StageStatus::Skipped,
            detail: Some("falcon warm — skipped to preserve FECS/GPCCS".into()),
            duration_ms: 0,
        });
    } else {
        let t = Instant::now();
        match pgraph_engine_reset(bar0) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: "pgraph_reset".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                ctx.stages.push(StageResult {
                    name: "pgraph_reset".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // ── Stage 2b: CG Sweep + PRI Recovery ──────────────────────────────
    if strategy.needs_cg_sweep() {
        if opts.halt_before == Some(HaltBefore::CgSweep) {
            ctx.stages.push(StageResult {
                name: "cg_sweep".into(),
                status: StageStatus::Skipped,
                detail: Some("halt_before=cg_sweep".into()),
                duration_ms: 0,
            });
            return PreMemoryOutcome::Done(ctx.finish_halted("cg_sweep"));
        }

        let t = Instant::now();
        let cg_result = cg_sweep(bar0);
        ctx.stages.push(StageResult {
            name: "cg_sweep".into(),
            status: StageStatus::Ok,
            detail: Some(cg_result.detail),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        let t = Instant::now();
        let pri_result = pri_bus_recover(bar0);
        ctx.stages.push(StageResult {
            name: "pri_recovery".into(),
            status: if pri_result.recovered {
                StageStatus::Ok
            } else {
                StageStatus::Failed
            },
            detail: Some(format!(
                "{} alive, {} faulted, recovered={}",
                pri_result.alive, pri_result.faulted, pri_result.recovered
            )),
            duration_ms: t.elapsed().as_millis() as u64,
        });
    }

    // ── Stage 2c: PGOB Ungating ──────────────────────────────────────
    if strategy.needs_pgob_before_memory() {
        if opts.halt_before == Some(HaltBefore::PgobUngate) {
            ctx.stages.push(StageResult {
                name: "pgob_ungating".into(),
                status: StageStatus::Skipped,
                detail: Some("halt_before=pgob_ungate".into()),
                duration_ms: 0,
            });
            return PreMemoryOutcome::Done(ctx.finish_halted("pgob_ungating"));
        }

        let t = Instant::now();
        match pgob_ungating(bar0, strategy.bridge()) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: "pgob_ungating".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                ctx.stages.push(StageResult {
                    name: "pgob_ungating".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    PreMemoryOutcome::Continue(PreMemoryContinue { pmc_result })
}
