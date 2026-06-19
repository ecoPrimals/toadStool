// SPDX-License-Identifier: AGPL-3.0-or-later
//! Post-memory stages: falcon boot, GR init, and final device verification.

use std::time::Instant;

use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_stages::{falcon_boot, gr_init};
use crate::vfio::sovereign_strategy::SovereignStrategy;
use crate::vfio::sovereign_types::{
    HaltBefore, SovereignInitOptions, SovereignInitResult, StageResult, StageStatus,
};

use super::context::PipelineCtx;
use super::memory_path::MemoryPathContinue;

pub(crate) fn run(
    ctx: &mut PipelineCtx<'_>,
    bar0: &MappedBar,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
    mem: MemoryPathContinue,
) -> SovereignInitResult {
    ctx.warm_detected = mem.boot_state.is_warm();
    ctx.boot_state = Some(mem.boot_state);
    let warm_detected = ctx.warm_detected;

    let sm = strategy.sm_version();
    let bridge = strategy.bridge();

    if opts.halt_before == Some(HaltBefore::FalconBoot) {
        ctx.stages.push(StageResult {
            name: "falcon_boot".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=falcon_boot".into()),
            duration_ms: 0,
        });
        return ctx.finish_halted("falcon_boot");
    }

    if mem.early_falcon_done {
        ctx.stages.push(StageResult {
            name: "falcon_boot".into(),
            status: StageStatus::Skipped,
            detail: Some("already completed in early_falcon_boot".into()),
            duration_ms: 0,
        });
    }

    let falcon_warm_state = strategy.detect_falcon_warm_state(bar0, warm_detected);
    let t = Instant::now();
    let warm_fecs_preserved = if mem.early_falcon_done {
        true
    } else {
        match falcon_boot(
            bar0,
            sm,
            opts.dma_backend.as_ref(),
            falcon_warm_state,
            bridge,
            strategy.falcon_boot_style(),
        ) {
            Ok(detail) => {
                let preserved = detail.contains("warm-preserved")
                    || detail.contains("warm-running")
                    || detail.contains("ACR boot OK");
                ctx.stages.push(StageResult {
                    name: "falcon_boot".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                preserved
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
                    ctx.stages.push(StageResult {
                        name: "falcon_boot".into(),
                        status: StageStatus::Skipped,
                        detail: Some(format!(
                            "warm-gated: FECS/GR gated, no firmware provider ({err_str})"
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    true
                } else {
                    ctx.stages.push(StageResult {
                        name: "falcon_boot".into(),
                        status: StageStatus::Failed,
                        detail: Some(err_str),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                    return ctx.finish_failed();
                }
            }
        }
    };

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
        ctx.stages.push(StageResult {
            name: "gr_init".into(),
            status: StageStatus::Skipped,
            detail: Some(reason.into()),
            duration_ms: 0,
        });
        if opts.halt_before == Some(HaltBefore::GrInit) {
            return ctx.finish_halted("gr_init");
        }
    } else {
        let t = Instant::now();
        match gr_init(bar0, sm, bridge) {
            Ok(detail) => {
                ctx.stages.push(StageResult {
                    name: "gr_init".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                ctx.stages.push(StageResult {
                    name: "gr_init".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                return ctx.finish_failed();
            }
        }
    }

    if opts.halt_before == Some(HaltBefore::Verify) {
        ctx.stages.push(StageResult {
            name: "verify".into(),
            status: StageStatus::Skipped,
            detail: Some("halt_before=verify".into()),
            duration_ms: 0,
        });
        return ctx.finish_halted("verify");
    }

    let t = Instant::now();
    match strategy.verify_device(bar0) {
        Ok(detail) => {
            ctx.stages.push(StageResult {
                name: "verify".into(),
                status: StageStatus::Ok,
                detail: Some(detail),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
        Err(e) => {
            ctx.stages.push(StageResult {
                name: "verify".into(),
                status: StageStatus::Failed,
                detail: Some(e.to_string()),
                duration_ms: t.elapsed().as_millis() as u64,
            });
            return ctx.finish_failed();
        }
    }

    ctx.finish_success()
}
