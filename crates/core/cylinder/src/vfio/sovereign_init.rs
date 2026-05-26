// SPDX-License-Identifier: AGPL-3.0-or-later
//! Staged sovereign device initialization pipeline.
//!
//! Orchestrates the full path from cold/warm device to compute-ready state.
//! The pipeline is vendor/generation-agnostic — all hardware-specific
//! decisions are delegated to a [`SovereignStrategy`] implementation.
//!
//! # Stages
//!
//! ```text
//! 1.   bar0_probe          — Chip ID verification, PMC liveness check
//! 2.   pmc_enable          — Staged engine clock enable (strategy-aware mask)
//! 2b.  cg_sweep            — Clock gating disable across all domains (strategy)
//! 2a.  pgraph_reset        — PGRAPH engine reset (PMC_ENABLE bit 12 toggle)
//! 2c.  pri_recovery        — PRI bus fault acknowledge + ringmaster re-enumerate
//! 2d.  pgob_ungating       — PGRAPH GPC broadcast ungate (strategy)
//! 2e.  early_falcon_boot   — [cold ACR only] ACR DMA boot before memory training
//! 3.   memory_training     — Memory controller bring-up
//! 3b.  pmc_full            — Full engine ungating (post-devinit, strategy)
//! 3c.  engine_ungate       — Replay captured init sequences (strategy)
//! 4.   falcon_boot         — Microcontroller firmware boot (skipped if early)
//! 5.   gr_init             — GR engine register programming
//! 6.   verify              — Final memory/timer verification
//! ```
//!
//! On cold Volta+ GPUs with secure boot (HBM2 + AcrDmaHs), stage 2e runs
//! falcon_boot early so the PMU can drive HBM2 calibration in stage 3.
//!
//! # Contract
//!
//! The pipeline returns [`SovereignInitResult`] with per-stage outcomes.
//! Glowplug expects `all_ok`, `compute_ready`, and `halted_at` fields.
//!
//! **Important:** `compute_ready` means the init pipeline passed (PTIMER,
//! PRAMIN, PMC readback), NOT that shader dispatch is possible. On VFIO GPUs
//! where GPCCS is HS fuse-locked (Volta+), `compute_ready=true` coexists with
//! `classify_tier()` returning Tier 1 (WarmInfrastructure). Use
//! `sovereign.classify_tier` for dispatch readiness assessment.

use std::time::Instant;

use crate::nv::pri::is_pri_fault;
use crate::vfio::channel::hbm2_training::TrainingLog;
use crate::vfio::device::MappedBar;
use crate::vfio::boot_state::{SovereignBootState, probe_boot_state};
use crate::vfio::sovereign_stages::{
    MemoryTrainingResult, PMC_ENABLE, PmcEnableResult,
    cg_sweep, dispatch_memory_training, falcon_boot, gr_init,
    pgob_ungating, pgraph_engine_reset, pmc_enable, pmc_enable_full, pmc_enable_rollback,
    pramin_sentinel_test, pri_bus_recover,
};
use crate::vfio::sovereign_strategy::SovereignStrategy;

pub use crate::vfio::sovereign_types::{
    HaltBefore, SovereignInitOptions, SovereignInitResult, StageResult, StageStatus,
};

/// Run the full sovereign init pipeline on a device.
///
/// `bar0` must be a valid mapped BAR0 region from an active device.
/// `strategy` encodes all vendor/generation-specific decisions.
///
/// All MMIO in the probe stage uses fork isolation; subsequent stages
/// use direct BAR0 access (the controller's r/w helpers already have PRI
/// fault recovery).
pub fn sovereign_init(
    bar0: &MappedBar,
    bdf: &str,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
) -> SovereignInitResult {
    let pipeline_start = Instant::now();
    let mut stages: Vec<StageResult> = Vec::new();
    let mut chip_id = 0u32;
    let mut boot0 = 0u32;
    let mut training_log: Option<TrainingLog> = None;

    // ── Stage 1: Identity Probe ──────────────────────────────────────────
    let t = Instant::now();
    match strategy.probe_identity(bar0) {
        Ok(id) => {
            boot0 = id.identity_raw;
            chip_id = id.identity_chip;
            stages.push(StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Ok,
                detail: Some(format!("raw=0x{boot0:08x} chip=0x{chip_id:03x}")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        Err(e) => {
            stages.push(StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return finish(bdf, boot0, chip_id, stages, None, pipeline_start, false, None);
        }
    }

    let power = strategy.power_profile();
    let sm = strategy.sm_version();
    let bridge = strategy.bridge();

    tracing::info!(
        gen = strategy.family_name(),
        sm,
        initial_mask = format!("0x{:08x}", power.initial_pmc_mask),
        rollback = power.rollback_on_devinit_failure,
        "Generation profile resolved for sovereign pipeline"
    );

    // ── Early falcon probe (before pgraph_reset destroys state) ───────
    //
    // pgraph_reset toggles PMC_ENABLE bit 12 which resets the PGRAPH
    // engine, killing FECS/GPCCS firmware state. If the falcon is already
    // warm-preserved or warm-running from a previous sovereign.init, we
    // can skip both pgraph_reset and the expensive ACR falcon_boot.
    let early_falcon_state = strategy.detect_falcon_warm_state(bar0, true);
    let falcon_already_warm = matches!(
        early_falcon_state,
        crate::vfio::sovereign_strategy::FalconWarmState::WarmPreserved { .. }
            | crate::vfio::sovereign_strategy::FalconWarmState::WarmRunning { .. }
    );

    if falcon_already_warm {
        tracing::info!(
            state = ?early_falcon_state,
            "falcon already warm — will skip pgraph_reset to preserve firmware"
        );
    }

    // ── Stage 2: PMC Enable (staged) ──────────────────────────────────
    if opts.halt_before == Some(HaltBefore::PmcEnable) {
        stages.push(StageResult {
            name: "pmc_enable".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=pmc_enable".into()),
            duration_ms: 0,
        });
        return finish_halted(bdf, boot0, chip_id, "pmc_enable", stages, pipeline_start);
    }

    let t = Instant::now();
    let pmc_result: PmcEnableResult = match pmc_enable(bar0, power) {
        Ok(result) => {
            stages.push(StageResult {
                name: "pmc_enable".into(),
                status: StageStatus::Ok,
                detail: Some(result.detail()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            result
        }
        Err(e) => {
            stages.push(StageResult {
                name: "pmc_enable".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return finish(bdf, boot0, chip_id, stages, None, pipeline_start, false, None);
        }
    };

    // ── Stage 2a: PGRAPH Engine Reset ─────────────────────────────────
    // Toggle GR bit in PMC_ENABLE to reset PGRAPH's internal PRI fabric.
    // After UEFI POST the GPC ring stations retain stale fault state;
    // without this reset, GPC/FECS/GPCCS registers read 0xBADF.
    //
    // SKIP when falcon is already warm — pgraph_reset would destroy the
    // running FECS/GPCCS firmware, forcing a 700ms+ ACR re-boot.
    if falcon_already_warm {
        stages.push(StageResult {
            name: "pgraph_reset".into(),
            status: StageStatus::Skipped,
            detail: Some("falcon warm — skipped to preserve FECS/GPCCS".into()),
            duration_ms: 0,
        });
    } else {
        let t = Instant::now();
        match pgraph_engine_reset(bar0) {
            Ok(detail) => {
                stages.push(StageResult {
                    name: "pgraph_reset".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                stages.push(StageResult {
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
            stages.push(StageResult {
                name: "cg_sweep".into(),
                status: StageStatus::Skipped,
                detail: Some("halt_before=cg_sweep".into()),
                duration_ms: 0,
            });
            return finish_halted(bdf, boot0, chip_id, "cg_sweep", stages, pipeline_start);
        }

        let t = Instant::now();
        let cg_result = cg_sweep(bar0);
        stages.push(StageResult {
            name: "cg_sweep".into(),
            status: StageStatus::Ok,
            detail: Some(cg_result.detail),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        let t = Instant::now();
        let pri_result = pri_bus_recover(bar0);
        stages.push(StageResult {
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
            stages.push(StageResult {
                name: "pgob_ungating".into(),
                status: StageStatus::Skipped,
                detail: Some("halt_before=pgob_ungate".into()),
                duration_ms: 0,
            });
            return finish_halted(bdf, boot0, chip_id, "pgob_ungating", stages, pipeline_start);
        }

        let t = Instant::now();
        match pgob_ungating(bar0, bridge) {
            Ok(detail) => {
                stages.push(StageResult {
                    name: "pgob_ungating".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                stages.push(StageResult {
                    name: "pgob_ungating".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // ── Stage 3: Memory Training ──────────────────────────────────────
    if opts.halt_before == Some(HaltBefore::MemoryTraining) {
        stages.push(StageResult {
            name: "memory_training".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=memory_training".into()),
            duration_ms: 0,
        });
        return finish_halted(bdf, boot0, chip_id, "memory_training", stages, pipeline_start);
    }

    let pmc_before = bar0.read_u32(PMC_ENABLE).unwrap_or(0);

    // Unified boot state probe — single source of truth for warm/cold
    let t_probe = Instant::now();
    let boot_state = probe_boot_state(bar0, Some(&|b, w| strategy.detect_falcon_warm_state(b, w)));
    let warm_detected = boot_state.is_warm();

    tracing::info!(
        bdf,
        boot_state = %boot_state.summary(),
        "boot state probed"
    );

    stages.push(StageResult {
        name: "boot_state_probe".into(),
        status: StageStatus::Ok,
        detail: Some(boot_state.summary()),
        duration_ms: t_probe.elapsed().as_millis() as u64,
    });

    // ── Cold early-exit: skip doomed memory_training on cold GPUs ────
    //
    // HBM2 training requires the boot ROM during power-on reset — a software-only
    // pipeline cannot train cold memory. When skip_cold_memory_training is set and
    // the GPU is cold, return immediately with compute_ready=false rather than
    // wasting ~10s on a doomed dispatch_memory_training call.
    if opts.skip_cold_memory_training && !warm_detected {
        tracing::info!(
            bdf,
            "cold GPU detected with skip_cold_memory_training — skipping doomed stages"
        );
        stages.push(StageResult {
            name: "memory_training".into(),
            status: StageStatus::Skipped,
            detail: Some("cold GPU: HBM2 training requires power-on reset".into()),
            duration_ms: 0,
        });
        return SovereignInitResult {
            bdf: bdf.to_string(),
            identity_chip: chip_id,
            identity_raw: boot0,
            all_ok: true,
            compute_ready: false,
            halted_at: Some("memory_training (cold_early_exit)".into()),
            stages,
            total_ms: pipeline_start.elapsed().as_millis() as u64,
            training_writes: None,
            warm_detected: false,
            boot_state: Some(boot_state),
        };
    }

    // ── Cold ACR reorder: falcon boot before memory training ─────────
    //
    // On Volta+ with secure boot (HBM2 + AcrDmaHs), a cold GPU cannot
    // train memory without the PMU falcon running signed firmware.  The
    // PMU is loaded via ACR boot (falcon_boot stage).  If we detect a
    // cold GPU that needs falcon-before-memory, promote falcon_boot
    // ahead of memory_training so the PMU can drive HBM2 calibration.
    let mut early_falcon_done = false;
    if !warm_detected && strategy.needs_falcon_before_memory() && opts.dma_backend.is_some() {
        tracing::info!(
            "cold secure-boot GPU: running falcon_boot before memory_training"
        );

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
                stages.push(StageResult {
                    name: "early_falcon_boot".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                early_falcon_done = true;

                // Give PMU time to run devinit after ACR loads it.
                // The PMU's signed firmware auto-runs devinit on cold GPUs
                // when it detects needs_post.  Allow up to 2s for HBM2
                // training to complete.
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
                stages.push(StageResult {
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
    match dispatch_memory_training(mem_strategy, bar0, bdf, warm_detected, pmc_before, opts) {
        MemoryTrainingResult::Ok(detail) => {
            devinit_ok = true;
            stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Ok,
                detail: Some(detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::OkWithLog(log) => {
            devinit_ok = true;
            let writes = log.write_count();
            training_log = Some(log);
            stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Ok,
                detail: Some(format!("{writes} register writes")),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::Skipped(reason) => {
            devinit_ok = true;
            stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Skipped,
                detail: Some(reason),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        MemoryTrainingResult::Failed(e) => {
            stages.push(StageResult {
                name: "memory_training".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            if power.rollback_on_devinit_failure {
                let rollback_detail = match pmc_enable_rollback(bar0, pmc_result.before) {
                    Ok(()) => format!(
                        "rolled back PMC_ENABLE to 0x{:08x}",
                        pmc_result.before
                    ),
                    Err(e) => format!("rollback attempted but failed: {e}"),
                };
                stages.push(StageResult {
                    name: "pmc_rollback".into(),
                    status: StageStatus::Ok,
                    detail: Some(rollback_detail),
                    duration_ms: 0,
                });
            }

            return finish(
                bdf,
                boot0,
                chip_id,
                stages,
                training_log,
                pipeline_start,
                warm_detected,
                Some(boot_state),
            );
        }
    }

    // ── Stage 3b: Post-devinit full engine ungating ─────────────────
    if devinit_ok && power.full_enable_after_devinit && pmc_result.mask != 0xFFFF_FFFF {
        let t = Instant::now();
        match pmc_enable_full(bar0) {
            Ok(detail) => {
                stages.push(StageResult {
                    name: "pmc_full_enable".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                stages.push(StageResult {
                    name: "pmc_full_enable".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // ── Stage 3c: Engine Ungating ──────────────────────────────────────
    // Strategy-driven: replay captured init sequences before falcon boot.
    {
        let has_strategy_sequences = strategy.engine_ungate_sequences().is_some();
        let has_opts_sequences =
            !opts.engine_init_sequences.is_empty() || opts.kepler_gr_init.is_some();

        if has_strategy_sequences || has_opts_sequences {
            if opts.halt_before == Some(HaltBefore::EngineUngate) {
                stages.push(StageResult {
                    name: "engine_ungate".into(),
                    status: StageStatus::Skipped,
                    detail: Some("halt_before=engine_ungate".into()),
                    duration_ms: 0,
                });
                return finish_halted(
                    bdf,
                    boot0,
                    chip_id,
                    "engine_ungate",
                    stages,
                    pipeline_start,
                );
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
                stages.push(StageResult {
                    name: "engine_ungate".into(),
                    status: StageStatus::Skipped,
                    detail: Some("no init sequences provided".into()),
                    duration_ms: 0,
                });
            } else {
                for (engine_name, seq, status_reg) in &ungate_list {
                    let t = Instant::now();
                    let stage_name = format!("engine_ungate:{engine_name}");
                    match engine_ungate(bar0, seq, engine_name, *status_reg) {
                        Ok(detail) => {
                            stages.push(StageResult {
                                name: stage_name,
                                status: StageStatus::Ok,
                                detail: Some(detail),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });
                        }
                        Err(e) => {
                            stages.push(StageResult {
                                name: stage_name,
                                status: StageStatus::Failed,
                                detail: Some(e),
                                duration_ms: t.elapsed().as_millis() as u64,
                            });
                            return finish(
                                bdf,
                                boot0,
                                chip_id,
                                stages,
                                training_log,
                                pipeline_start,
                                warm_detected,
                                Some(boot_state),
                            );
                        }
                    }
                }
            }
        }
    }

    // ── Stage 4: Falcon Boot ────────────────────────────────────────────
    if opts.halt_before == Some(HaltBefore::FalconBoot) {
        stages.push(StageResult {
            name: "falcon_boot".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=falcon_boot".into()),
            duration_ms: 0,
        });
        return finish_halted(bdf, boot0, chip_id, "falcon_boot", stages, pipeline_start);
    }

    if early_falcon_done {
        stages.push(StageResult {
            name: "falcon_boot".into(),
            status: StageStatus::Skipped,
            detail: Some("already completed in early_falcon_boot".into()),
            duration_ms: 0,
        });
    }

    let falcon_warm_state = strategy.detect_falcon_warm_state(bar0, warm_detected);
    let t = Instant::now();
    let (_falcon_detail, warm_fecs_preserved) = if early_falcon_done {
        ("early_falcon_boot".to_string(), true)
    } else {
        match falcon_boot(bar0, sm, opts.dma_backend.as_ref(), falcon_warm_state, bridge, strategy.falcon_boot_style()) {
            Ok(detail) => {
                let preserved = detail.contains("warm-preserved")
                    || detail.contains("warm-running")
                    || detail.contains("ACR boot OK");
                stages.push(StageResult {
                    name: "falcon_boot".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail.clone()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                (detail, preserved)
            }
            Err(e) => {
                let err_str = e.to_string();
                let is_stub_unsupported =
                    warm_detected && err_str.contains("requires firmware provider");
                if is_stub_unsupported {
                    tracing::info!(
                        "falcon_boot failed on warm GPU (no firmware provider) — \
                         continuing to verify"
                    );
                    stages.push(StageResult {
                        name: "falcon_boot".into(),
                        status: StageStatus::Skipped,
                        detail: Some(format!(
                            "warm-gated: FECS/GR gated, no firmware provider ({err_str})"
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    ("warm-gated".to_string(), true)
                } else {
                    stages.push(StageResult {
                        name: "falcon_boot".into(),
                        status: StageStatus::Failed,
                        detail: Some(err_str),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return finish(
                        bdf,
                        boot0,
                        chip_id,
                        stages,
                        training_log,
                        pipeline_start,
                        warm_detected,
                        Some(boot_state),
                    );
                }
            }
        }
    };

    // ── Stage 5: GR Init ────────────────────────────────────────────────
    let skip_gr = !strategy.needs_gr_init_after_falcon()
        || warm_fecs_preserved
        || opts.skip_gr_init
        || opts.halt_before == Some(HaltBefore::GrInit);
    if skip_gr {
        let reason = if !strategy.needs_gr_init_after_falcon() {
            "boot strategy does not require post-falcon GR init"
        } else if warm_fecs_preserved {
            "FECS warm-preserved/running: skipping re-bootstrap"
        } else if opts.skip_gr_init {
            "skip_gr_init=true"
        } else {
            "halt_before=gr_init"
        };
        stages.push(StageResult {
            name: "gr_init".into(),
            status: StageStatus::Skipped,
            detail: Some(reason.into()),
            duration_ms: 0,
        });
        if opts.halt_before == Some(HaltBefore::GrInit) {
            return finish_halted(bdf, boot0, chip_id, "gr_init", stages, pipeline_start);
        }
    } else {
        let t = Instant::now();
        match gr_init(bar0, sm, bridge) {
            Ok(detail) => {
                stages.push(StageResult {
                    name: "gr_init".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                stages.push(StageResult {
                    name: "gr_init".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                return finish(
                    bdf,
                    boot0,
                    chip_id,
                    stages,
                    training_log,
                    pipeline_start,
                    warm_detected,
                    Some(boot_state),
                );
            }
        }
    }

    // ── Stage 6: Verify ─────────────────────────────────────────────────
    if opts.halt_before == Some(HaltBefore::Verify) {
        stages.push(StageResult {
            name: "verify".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=verify".into()),
            duration_ms: 0,
        });
        return finish_halted(bdf, boot0, chip_id, "verify", stages, pipeline_start);
    }

    let t = Instant::now();
    match strategy.verify_device(bar0) {
        Ok(detail) => {
            stages.push(StageResult {
                name: "verify".into(),
                status: StageStatus::Ok,
                detail: Some(detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        Err(e) => {
            stages.push(StageResult {
                name: "verify".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return finish(
                bdf,
                boot0,
                chip_id,
                stages,
                training_log,
                pipeline_start,
                warm_detected,
                Some(boot_state),
            );
        }
    }

    // All stages passed
    let training_writes = training_log.as_ref().map(|l| l.write_count());
    SovereignInitResult {
        bdf: bdf.to_string(),
        identity_chip: chip_id,
        identity_raw: boot0,
        all_ok: true,
        compute_ready: true,
        halted_at: None,
        stages,
        total_ms: pipeline_start.elapsed().as_millis() as u64,
        training_writes,
        warm_detected,
        boot_state: Some(boot_state),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "pipeline result builder aggregates all stage outputs"
)]
fn finish(
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

/// Replay a captured GrInitSequence to ungate Kepler PGRAPH.
///
/// Applies only the PGRAPH-domain writes from the sequence, then
/// verifies that PGRAPH_STATUS (0x400700) is no longer PRI-faulted.
/// Generalized engine ungating — replays a [`GrInitSequence`] for any
/// named engine and optionally validates a status register afterward.
///
/// `engine_name` is used for logging/stage naming (e.g. "PGRAPH", "CE").
/// `status_reg` is an optional BAR0 offset to read-back after applying
/// the sequence; if the read-back returns a PRI fault, the ungate failed.
fn engine_ungate(
    bar0: &MappedBar,
    seq: &crate::nv::gr_init::GrInitSequence,
    engine_name: &str,
    status_reg: Option<usize>,
) -> Result<String, String> {
    let applied = seq.apply(bar0)?;

    if let Some(reg) = status_reg {
        let status = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(status) {
            return Err(format!(
                "{engine_name} still gated after {applied} writes (status=0x{status:08x})"
            ));
        }
        Ok(format!(
            "{engine_name} ungated: {applied} writes applied, status=0x{status:08x}"
        ))
    } else {
        Ok(format!("{engine_name} ungated: {applied} writes applied"))
    }
}

/// PGRAPH status register (GK110+).
const PGRAPH_STATUS: usize = 0x0040_0700;

fn finish_halted(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_id_to_sm_covers_titan_v() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0x140), 70);
    }

    #[test]
    fn chip_id_to_sm_covers_k80() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0x0E7), 35);
    }

    #[test]
    fn chip_id_to_sm_unknown_defaults_to_70() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0xFFF), 70);
    }

    #[test]
    fn stage_status_serde_roundtrip() {
        let json = serde_json::to_string(&StageStatus::Ok).unwrap();
        assert_eq!(json, "\"ok\"");
        let back: StageStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StageStatus::Ok);
    }

    #[test]
    fn sovereign_init_result_display_halted() {
        let r = SovereignInitResult {
            bdf: "0000:03:00.0".into(),
            identity_chip: 0x140,
            identity_raw: 0x140000A1,
            all_ok: true,
            compute_ready: false,
            halted_at: Some("memory_training".into()),
            stages: vec![],
            total_ms: 42,
            training_writes: None,
            warm_detected: false,
            boot_state: None,
        };
        let s = r.to_string();
        assert!(s.contains("HALTED@memory_training"));
        assert!(s.contains("42ms"));
    }

    #[test]
    fn sovereign_init_result_display_ready() {
        let r = SovereignInitResult {
            bdf: "0000:03:00.0".into(),
            identity_chip: 0x140,
            identity_raw: 0x140000A1,
            all_ok: true,
            compute_ready: true,
            halted_at: None,
            stages: vec![StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Ok,
                detail: None,
                duration_ms: 1,
            }],
            total_ms: 100,
            training_writes: Some(42),
            warm_detected: true,
            boot_state: None,
        };
        let s = r.to_string();
        assert!(s.contains("COMPUTE_READY"));
        assert!(s.contains("0x140"));
    }

    #[test]
    fn halt_before_serde_roundtrip() {
        let json = serde_json::to_string(&HaltBefore::MemoryTraining).unwrap();
        assert_eq!(json, "\"memory_training\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::MemoryTraining);
    }

    #[test]
    fn halt_before_cg_sweep_serde() {
        let json = serde_json::to_string(&HaltBefore::CgSweep).unwrap();
        assert_eq!(json, "\"cg_sweep\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::CgSweep);
    }

    #[test]
    fn halt_before_pgob_ungate_serde() {
        let json = serde_json::to_string(&HaltBefore::PgobUngate).unwrap();
        assert_eq!(json, "\"pgob_ungate\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::PgobUngate);
    }

    #[test]
    fn result_backward_compat_aliases() {
        let json = r#"{
            "bdf": "0000:03:00.0",
            "chip_id": 320,
            "boot0": 335544481,
            "all_ok": true,
            "compute_ready": true,
            "stages": [],
            "total_ms": 100,
            "hbm2_writes": 42,
            "warm_detected": false
        }"#;
        let r: SovereignInitResult = serde_json::from_str(json).unwrap();
        assert_eq!(r.identity_chip, 320);
        assert_eq!(r.identity_raw, 335544481);
        assert_eq!(r.training_writes, Some(42));
    }

    #[test]
    fn options_default_has_no_halt() {
        let opts = SovereignInitOptions::default();
        assert!(opts.halt_before.is_none());
        assert!(opts.golden_state.is_none());
        assert!(opts.vbios_rom.is_none());
        assert!(!opts.skip_gr_init);
    }
}
