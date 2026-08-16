// SPDX-License-Identifier: AGPL-3.0-or-later
//! Memory training path: boot-state probe, cold early exit, early falcon, devinit, engine ungate.

use std::time::Instant;

use crate::vfio::boot_state::{SovereignBootState, probe_boot_state};
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_stages::{
    MemoryTrainingResult, PMC_ENABLE, dispatch_memory_training, falcon_boot, pmc_enable_full,
    pmc_enable_rollback, pramin_sentinel_test,
};
use crate::vfio::sovereign_strategy::SovereignStrategy;
use crate::vfio::sovereign_types::{
    HaltBefore, SovereignInitOptions, SovereignInitResult, StageResult, StageStatus,
};

use super::context::PipelineCtx;
use super::engine_ungate::{self, PGRAPH_STATUS};
use super::pre_memory::PreMemoryContinue;

/// State carried into falcon boot / GR init after memory path completes.
pub(crate) struct MemoryPathContinue {
    pub early_falcon_done: bool,
    pub boot_state: SovereignBootState,
}

pub(crate) enum MemoryPathOutcome {
    Done(SovereignInitResult),
    Continue(MemoryPathContinue),
}

pub(crate) fn run(
    ctx: &mut PipelineCtx<'_>,
    bar0: &MappedBar,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
    pre: &PreMemoryContinue,
) -> MemoryPathOutcome {
    // ── Stage 3: Memory Training ──────────────────────────────────────
    if opts.halt_before == Some(HaltBefore::MemoryTraining) {
        ctx.stages.push(StageResult {
            name: "memory_training".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=memory_training".into()),
            duration_ms: 0,
        });
        return MemoryPathOutcome::Done(ctx.finish_halted("memory_training"));
    }

    let pmc_before = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
    let power = strategy.power_profile();
    let sm = strategy.sm_version();
    let bridge = strategy.bridge();

    let t_probe = Instant::now();
    let boot_state = probe_boot_state(bar0, Some(&|b, w| strategy.detect_falcon_warm_state(b, w)));
    let warm_detected = boot_state.is_warm();
    ctx.warm_detected = warm_detected;

    tracing::info!(
        bdf = ctx.bdf,
        boot_state = %boot_state.summary(),
        "boot state probed"
    );

    ctx.stages.push(StageResult {
        name: "boot_state_probe".into(),
        status: StageStatus::Ok,
        detail: Some(boot_state.summary()),
        duration_ms: t_probe.elapsed().as_millis() as u64,
    });

    if opts.skip_cold_memory_training && !warm_detected {
        tracing::info!(
            bdf = ctx.bdf,
            "cold GPU detected with skip_cold_memory_training — skipping doomed stages"
        );
        ctx.stages.push(StageResult {
            name: "memory_training".into(),
            status: StageStatus::Skipped,
            detail: Some(format!(
                "cold GPU: {} training requires power-on reset",
                crate::nv::generation::profile_for_sm(opts.sm_version.unwrap_or(70)).memory_type
            )),
            duration_ms: 0,
        });
        return MemoryPathOutcome::Done(cold_early_exit(ctx, boot_state));
    }

    let mut early_falcon_done = false;
    if !warm_detected && strategy.needs_falcon_before_memory() && opts.dma_backend.is_some() {
        tracing::info!("cold secure-boot GPU: running falcon_boot before memory_training");

        let falcon_warm_state = strategy.detect_falcon_warm_state(bar0, false);
        let t = Instant::now();
        match falcon_boot(
            bar0,
            sm,
            opts.dma_backend.as_ref(),
            falcon_warm_state,
            bridge,
            strategy.falcon_boot_style(),
        ) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: "early_falcon_boot".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                early_falcon_done = true;

                tracing::info!("waiting for PMU devinit after ACR boot...");
                let poll_start = Instant::now();
                let poll_timeout = std::time::Duration::from_secs(2);
                while poll_start.elapsed() < poll_timeout {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if pramin_sentinel_test(bar0) {
                        tracing::info!(
                            elapsed_ms = poll_start.elapsed().as_millis(),
                            "VRAM alive after early falcon boot — PMU devinit succeeded"
                        );
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "early falcon_boot failed — falling through to memory_training"
                );
                ctx.stages.push(StageResult {
                    name: "early_falcon_boot".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    let mem_strategy = strategy.memory_strategy();
    let t = Instant::now();
    let devinit_ok;
    match dispatch_memory_training(mem_strategy, bar0, ctx.bdf, warm_detected, pmc_before, opts) {
        MemoryTrainingResult::Ok(detail) => {
            devinit_ok = true;
            ctx.stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Ok,
                detail: Some(detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::OkWithLog(log) => {
            devinit_ok = true;
            let writes = log.write_count();
            ctx.training_log = Some(log);
            ctx.stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Ok,
                detail: Some(format!("{writes} register writes")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::Skipped(reason) => {
            devinit_ok = true;
            ctx.stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Skipped,
                detail: Some(reason),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::Failed(e) => {
            ctx.stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            if power.rollback_on_devinit_failure {
                let rollback_detail = match pmc_enable_rollback(bar0, pre.pmc_result.before) {
                    Ok(()) => format!("rolled back PMC_ENABLE to 0x{:08x}", pre.pmc_result.before),
                    Err(e) => format!("rollback attempted but failed: {e}"),
                };
                ctx.stages.push(StageResult {
                    name: "pmc_rollback".into(),
                    status: StageStatus::Ok,
                    detail: Some(rollback_detail),
                    duration_ms: 0,
                });
            }

            ctx.boot_state = Some(boot_state);
            return MemoryPathOutcome::Done(ctx.finish_failed());
        }
    }

    if devinit_ok && power.full_enable_after_devinit && pre.pmc_result.mask != 0xFFFF_FFFF {
        let t = Instant::now();
        match pmc_enable_full(bar0) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: "pmc_full_enable".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                ctx.stages.push(StageResult {
                    name: "pmc_full_enable".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    if let Some(result) = run_engine_ungate(ctx, bar0, opts, strategy, &boot_state) {
        return result;
    }

    MemoryPathOutcome::Continue(MemoryPathContinue {
        early_falcon_done,
        boot_state,
    })
}

fn cold_early_exit(
    ctx: &mut PipelineCtx<'_>,
    boot_state: SovereignBootState,
) -> SovereignInitResult {
    SovereignInitResult {
        bdf: ctx.bdf.to_string(),
        identity_chip: ctx.chip_id,
        identity_raw: ctx.boot0,
        all_ok: true,
        compute_ready: false,
        halted_at: Some("memory_training (cold_early_exit)".into()),
        stages: std::mem::take(&mut ctx.stages),
        total_ms: ctx.pipeline_start.elapsed().as_millis() as u64,
        training_writes: None,
        warm_detected: false,
        boot_state: Some(boot_state),
    }
}

fn run_engine_ungate(
    ctx: &mut PipelineCtx<'_>,
    bar0: &MappedBar,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
    boot_state: &SovereignBootState,
) -> Option<MemoryPathOutcome> {
    let has_strategy_sequences = strategy.engine_ungate_sequences().is_some();
    let has_opts_sequences =
        !opts.engine_init_sequences.is_empty() || opts.kepler_gr_init.is_some();

    if !has_strategy_sequences && !has_opts_sequences {
        return None;
    }

    if opts.halt_before == Some(HaltBefore::EngineUngate) {
        ctx.stages.push(StageResult {
            name: "engine_ungate".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=engine_ungate".into()),
            duration_ms: 0,
        });
        return Some(MemoryPathOutcome::Done(ctx.finish_halted("engine_ungate")));
    }

    let mut ungate_list: Vec<(&str, &crate::nv::gr_init::GrInitSequence, Option<usize>)> =
        Vec::new();

    if let Some(seqs) = strategy.engine_ungate_sequences() {
        for (name, seq, reg) in seqs {
            ungate_list.push((name.as_str(), seq, *reg));
        }
    }

    if ungate_list.is_empty() {
        for (name, seq, reg) in &opts.engine_init_sequences {
            ungate_list.push((name.as_str(), seq, *reg));
        }
    }

    if ungate_list.is_empty()
        && let Some(ref gr_init_seq) = opts.kepler_gr_init
    {
        ungate_list.push(("PGRAPH", gr_init_seq, Some(PGRAPH_STATUS)));
    }

    if ungate_list.is_empty() {
        ctx.stages.push(StageResult {
            name: "engine_ungate".into(),
            status: StageStatus::Skipped,
            detail: Some("no init sequences provided".into()),
            duration_ms: 0,
        });
        return None;
    }

    for (engine_name, seq, status_reg) in &ungate_list {
        let t = Instant::now();
        let stage_name = format!("engine_ungate:{engine_name}");
        match engine_ungate::engine_ungate(bar0, seq, engine_name, *status_reg) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: stage_name,
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                ctx.stages.push(StageResult {
                    name: stage_name,
                    status: StageStatus::Failed,
                    detail: Some(e),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                ctx.boot_state = Some(boot_state.clone());
                return Some(MemoryPathOutcome::Done(ctx.finish_failed()));
            }
        }
    }

    None
}
