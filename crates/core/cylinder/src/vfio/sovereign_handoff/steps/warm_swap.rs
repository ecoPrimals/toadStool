// SPDX-License-Identifier: AGPL-3.0-or-later
//! Step 5–7a of the sovereign warm-handoff pipeline: bridge pin, driver swap,
//! catalyst BAR0 capture, FECS INIT_CTXSW, and early tier classification.
//!
//! Catalyst-specific sub-operations (post-unbind diagnostic, FECS init, snapshot
//! persistence) live in `warm_swap_catalyst` for readability.

use std::time::{Duration, Instant};

use crate::nv::registers::{pmc, pri};
use crate::vfio::guarded_sysfs;
use crate::vfio::sovereign_tiers::classify_tier;

use super::super::pipeline::PipelineContext;
use super::super::rollback::halt_result;
use super::super::types::{HandoffResult, HandoffStep};
use super::warm_swap_catalyst;

const DRIVER_SWAP_TIMEOUT: Duration = Duration::from_secs(5);
const PRI_RING_SETTLE: Duration = Duration::from_millis(10);

fn breadcrumb(msg: &str) {
    crate::vfio::sovereign_handoff::forensics::breadcrumb(msg);
}

/// Shorthand for the common halt-and-return pattern in warm_swap.
fn halt_swap(
    ctx: &mut PipelineContext<'_>,
    detail: String,
    duration_ms: u64,
) -> Option<HandoffResult> {
    ctx.steps.push(HandoffStep {
        name: "warm_swap".into(),
        ok: false,
        detail: Some(detail),
        duration_ms,
    });
    Some(halt_result(
        &ctx.config.bdf,
        "warm_swap",
        std::mem::take(&mut ctx.steps),
        ctx.patch_result.take(),
        ctx.module_loaded,
        false,
        ctx.overall,
        &ctx.sibling_state,
        &ctx.config.module_name,
        ctx.needs_device_rollback,
    ))
}

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
    breadcrumb("warm_swap::run ENTERED");

    // ── Step 5: Pin bridges + disable FLR + suppress SBR ───────────
    let t = Instant::now();
    breadcrumb("step5: pin_bridge_hierarchy");
    guarded_sysfs::pin_bridge_hierarchy(&ctx.config.bdf);
    breadcrumb("step5: disable_flr");
    guarded_sysfs::disable_flr(&ctx.config.bdf);
    breadcrumb("step5: suppress_bus_reset");
    if let Err(e) = guarded_sysfs::suppress_bus_reset(&ctx.config.bdf) {
        tracing::warn!(
            bdf = ctx.config.bdf.as_str(),
            error = %e,
            "failed to suppress SBR before warm swap — state may be lost"
        );
    }
    ctx.steps.push(HandoffStep {
        name: "prepare_warm_swap".into(),
        ok: true,
        detail: Some("bridge pinned, FLR disabled, SBR suppressed".into()),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    ctx.heartbeat();

    // ── Pre-unbind interrupt kill (catalyst only) ──────────────────
    if ctx.is_catalyst {
        pre_unbind_interrupt_defenses(ctx);
    }

    // ── Step 6: Warm swap — seeder → final driver (GUARDED) ───────
    let t = Instant::now();
    breadcrumb("step6: reading current driver");
    if let Some(ref current) = guarded_sysfs::read_current_driver(&ctx.config.bdf) {
        let remaining = ctx.deadline.saturating_sub(ctx.overall.elapsed());
        let unbind_result = if ctx.is_catalyst {
            breadcrumb("step6: UNBIND FIRE — writing to sysfs unbind NOW");
            guarded_sysfs::sysfs_unbind_fire_and_poll(&ctx.config.bdf, current, remaining).map(
                |elapsed| {
                    breadcrumb(&format!(
                        "step6: UNBIND COMPLETE — took {}s",
                        elapsed.as_secs()
                    ));
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        elapsed_s = elapsed.as_secs(),
                        "catalyst teardown completed via fire-and-poll"
                    );
                },
            )
        } else {
            let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
            guarded_sysfs::sysfs_write_guarded(
                &unbind_path,
                &ctx.config.bdf,
                guarded_sysfs::UNBIND_TIMEOUT,
            )
        };
        if let Err(e) = unbind_result {
            return halt_swap(
                ctx,
                format!("unbind {current} failed: {e}"),
                t.elapsed().as_millis() as u64,
            );
        }
    }

    breadcrumb("step6: post-unbind — driver detached");

    if ctx.is_catalyst {
        warm_swap_catalyst::post_unbind_diagnostic(ctx);
        if let Some(result) = catalyst_rebind(ctx, t) {
            return Some(result);
        }
    } else if let Some(result) = standard_rebind(ctx, t) {
        return Some(result);
    }

    ctx.heartbeat();

    // ── Step 7a: Deferred catalyst full capture (BEFORE sibling rebind) ──
    if ctx.is_catalyst {
        catalyst_bar0_capture(ctx);
    }

    None
}

/// Pre-unbind interrupt defenses for catalyst GPUs.
///
/// Quenches all interrupt paths (BAR0 INTR_EN, PCI MSI/MSI-X, INTx, Bus Master)
/// then engages the IRQ clutch to clean MSI/IRQ state before unbind.
fn pre_unbind_interrupt_defenses(ctx: &mut PipelineContext<'_>) {
    breadcrumb("pre-unbind: quench_interrupts");
    crate::nv::registers::pmc::quench_interrupts(
        &ctx.config.bdf,
        &ctx.hw.interrupt_profile,
        "pre-unbind",
    );
    breadcrumb("pre-unbind: disable_pci_msi");
    crate::nv::registers::pmc::disable_pci_msi(&ctx.config.bdf, "pre-unbind");
    breadcrumb("pre-unbind: intx_disable");
    crate::nv::registers::pmc::intx_disable(&ctx.config.bdf, "pre-unbind");
    breadcrumb("pre-unbind: disable_bus_master");
    crate::nv::registers::pmc::disable_bus_master(&ctx.config.bdf, "pre-unbind");
    breadcrumb("pre-unbind: ALL interrupt defenses complete");

    breadcrumb("pre-unbind: engaging IRQ clutch");
    match guarded_sysfs::engage_irq_clutch(&ctx.config.bdf) {
        Ok(()) => {
            breadcrumb("pre-unbind: IRQ clutch engaged — disengaging");
            if let Err(e) = guarded_sysfs::disengage_irq_clutch() {
                tracing::warn!(error = %e, "IRQ clutch disengage failed (non-fatal)");
            }
            breadcrumb("pre-unbind: IRQ clutch complete — MSI/IRQ cleaned");
            ctx.irq_clutch_engaged = true;
        }
        Err(e) => {
            tracing::error!(
                bdf = ctx.config.bdf.as_str(),
                error = %e,
                "pre-unbind IRQ clutch FAILED — unbind will hit stale IRQ state"
            );
            breadcrumb("pre-unbind: IRQ clutch failed");
        }
    }
}

/// Catalyst rebind: misfire mode (clutch failed) or full rebind (clutch succeeded).
fn catalyst_rebind(ctx: &mut PipelineContext<'_>, t: Instant) -> Option<HandoffResult> {
    if !ctx.irq_clutch_engaged {
        // ── MISFIRE MODE (clutch failed) ─────────────────────────
        breadcrumb("step6: MISFIRE MODE — clutch failed, skipping drivers_probe");
        let override_set = matches!(
            guarded_sysfs::sysfs_write_guarded(
                &ctx.override_path,
                &ctx.config.final_driver,
                DRIVER_SWAP_TIMEOUT,
            ),
            Ok(()),
        );
        let current_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);
        let swap_elapsed = t.elapsed();
        tracing::warn!(
            bdf = ctx.config.bdf.as_str(),
            current_driver = current_driver.as_deref().unwrap_or("none"),
            override_set,
            "catalyst MISFIRE: IRQ clutch failed pre-unbind, drivers_probe skipped. \
                 GPU left unbound to prevent lockup."
        );
        breadcrumb(&format!(
            "step6: MISFIRE — driver={}, override_set={}, {}s",
            current_driver.as_deref().unwrap_or("none"),
            override_set,
            swap_elapsed.as_secs(),
        ));
        ctx.steps.push(HandoffStep {
            name: "warm_swap".into(),
            ok: true,
            detail: Some(format!(
                "MISFIRE: {} unbind OK, clutch FAILED, drivers_probe SKIPPED \
                     (override={}, driver={}, {}s)",
                ctx.config.seeder_driver,
                override_set,
                current_driver.as_deref().unwrap_or("none"),
                swap_elapsed.as_secs(),
            )),
            duration_ms: swap_elapsed.as_millis() as u64,
        });
    } else {
        // ── FULL REBIND (clutch succeeded) ───────────────────────
        let poll_deadline = ctx.deadline.saturating_sub(ctx.overall.elapsed());
        let poll_start = Instant::now();
        let poll_interval = DRIVER_SWAP_TIMEOUT;
        let mut override_set = false;
        let mut probe_sent = false;
        let mut final_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);

        breadcrumb("step6: clutch OK — attempting full rebind via drivers_probe");

        while final_driver.as_deref() != Some(ctx.config.final_driver.as_str()) {
            if poll_start.elapsed() >= poll_deadline {
                return halt_swap(
                    ctx,
                    format!(
                        "poll for {} timed out (driver={:?}, override_set={}, probe_sent={})",
                        ctx.config.final_driver, final_driver, override_set, probe_sent,
                    ),
                    t.elapsed().as_millis() as u64,
                );
            }

            if !override_set
                && matches!(
                    guarded_sysfs::sysfs_write_guarded(
                        &ctx.override_path,
                        &ctx.config.final_driver,
                        DRIVER_SWAP_TIMEOUT,
                    ),
                    Ok(()),
                )
            {
                override_set = true;
                breadcrumb("step6: driver_override set");
                tracing::info!(
                    bdf = ctx.config.bdf.as_str(),
                    "catalyst: driver_override set to {}",
                    ctx.config.final_driver
                );
            }

            if override_set && !probe_sent {
                breadcrumb("step6: sending drivers_probe");
                if matches!(
                    guarded_sysfs::sysfs_write_guarded(
                        &ctx.probe_path,
                        &ctx.config.bdf,
                        DRIVER_SWAP_TIMEOUT,
                    ),
                    Ok(()),
                ) {
                    probe_sent = true;
                    breadcrumb("step6: drivers_probe sent");
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        "catalyst: drivers_probe sent (IRQ clutch cleaned pre-unbind)"
                    );
                }
            }

            std::thread::sleep(poll_interval);
            final_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);
        }

        let swap_elapsed = t.elapsed();
        breadcrumb(&format!(
            "step6: full rebind complete — driver={}, {}s",
            ctx.config.final_driver,
            swap_elapsed.as_secs(),
        ));
        tracing::info!(
            bdf = ctx.config.bdf.as_str(),
            final_driver = ctx.config.final_driver.as_str(),
            elapsed_s = swap_elapsed.as_secs(),
            "catalyst warm_swap: final driver bound (clutch-cleaned rebind)"
        );
        ctx.steps.push(HandoffStep {
            name: "warm_swap".into(),
            ok: true,
            detail: Some(format!(
                "{} → {} (clutch-cleaned, {}s)",
                ctx.config.seeder_driver,
                ctx.config.final_driver,
                swap_elapsed.as_secs()
            )),
            duration_ms: swap_elapsed.as_millis() as u64,
        });
    }
    None
}

/// Standard (non-catalyst) rebind: override + guarded drivers_probe.
fn standard_rebind(ctx: &mut PipelineContext<'_>, t: Instant) -> Option<HandoffResult> {
    if let Err(e) = guarded_sysfs::sysfs_write(&ctx.override_path, &ctx.config.final_driver) {
        return halt_swap(
            ctx,
            format!("override to {} failed: {e}", ctx.config.final_driver),
            t.elapsed().as_millis() as u64,
        );
    }

    if let Err(e) = guarded_sysfs::sysfs_write_guarded(
        &ctx.probe_path,
        &ctx.config.bdf,
        guarded_sysfs::PROBE_TIMEOUT,
    ) {
        return halt_swap(
            ctx,
            format!(
                "guarded drivers_probe for {} failed: {e}",
                ctx.config.final_driver
            ),
            t.elapsed().as_millis() as u64,
        );
    }

    let final_bound = guarded_sysfs::read_current_driver(&ctx.config.bdf);
    let swap_ok = final_bound.as_deref() == Some(ctx.config.final_driver.as_str());
    ctx.steps.push(HandoffStep {
        name: "warm_swap".into(),
        ok: swap_ok,
        detail: Some(format!(
            "{} → {} (warm_preserved={})",
            ctx.config.seeder_driver,
            final_bound.as_deref().unwrap_or("none"),
            swap_ok
        )),
        duration_ms: t.elapsed().as_millis() as u64,
    });

    if !swap_ok {
        return halt_swap(
            ctx,
            format!(
                "final driver mismatch: expected {}, got {:?}",
                ctx.config.final_driver, final_bound
            ),
            t.elapsed().as_millis() as u64,
        );
    }
    None
}

/// Catalyst full BAR0 capture, FECS INIT_CTXSW, and early tier classification.
fn catalyst_bar0_capture(ctx: &mut PipelineContext<'_>) {
    breadcrumb("catalyst_full_capture: starting BAR0 open");
    let t = Instant::now();
    let bar0_size = 16 * 1024 * 1024;
    tracing::info!(
        bdf = ctx.config.bdf.as_str(),
        pipeline_elapsed_s = ctx.overall.elapsed().as_secs(),
        "catalyst profile: starting BAR0 open (from_sysfs_rw)"
    );
    match crate::vfio::device::MappedBar::from_sysfs_rw(&ctx.config.bdf, bar0_size) {
        Ok(post_swap_bar0) => {
            breadcrumb("catalyst_full_capture: BAR0 open OK, pre-flight check");
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                open_ms = t.elapsed().as_millis() as u64,
                "catalyst profile: BAR0 mmap open succeeded"
            );

            let boot0 = post_swap_bar0
                .read_u32(pmc::BOOT0 as usize)
                .unwrap_or(0xFFFF_FFFF);
            let pmc_en = post_swap_bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
            let pri_intr = post_swap_bar0
                .read_u32(pri::INTR_STATUS as usize)
                .unwrap_or(0);

            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                boot0 = format_args!("0x{boot0:08x}"),
                pmc_enable = format_args!("0x{pmc_en:08x}"),
                pri_ring_intr = format_args!("0x{pri_intr:08x}"),
                "catalyst pre-flight: device alive check"
            );

            if boot0 == 0xFFFF_FFFF {
                breadcrumb("catalyst_full_capture: ABORT — BOOT0=0xFFFFFFFF, device gone");
                tracing::error!(
                    bdf = ctx.config.bdf.as_str(),
                    "catalyst capture ABORTED: BOOT0=0xFFFFFFFF — GPU has fallen off the bus"
                );
                ctx.steps.push(HandoffStep {
                    name: "catalyst_full_capture".into(),
                    ok: false,
                    detail: Some("BOOT0=0xFFFFFFFF — device not responding, scan skipped".into()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
                drop(post_swap_bar0);
            } else {
                let pri_faults_persistent = if pri_intr != 0 {
                    let _ = post_swap_bar0.write_u32(pri::COMMAND as usize, 0x2);
                    std::thread::sleep(PRI_RING_SETTLE);
                    let pri_after = post_swap_bar0
                        .read_u32(pri::INTR_STATUS as usize)
                        .unwrap_or(0);
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        pri_intr_before = format_args!("0x{pri_intr:08x}"),
                        pri_intr_after = format_args!("0x{pri_after:08x}"),
                        "catalyst pre-flight: PRI ring fault ack"
                    );
                    pri_after != 0
                } else {
                    false
                };

                breadcrumb("catalyst_full_capture: pre-flight OK, starting domain scan");

                let cap_start = Instant::now();
                let (alive, snapshot_path) = warm_swap_catalyst::capture_and_persist_snapshot(
                    &post_swap_bar0,
                    ctx,
                    cap_start,
                    t,
                );

                ctx.catalyst_alive_count = Some(alive);
                ctx.catalyst_snapshot_path = snapshot_path.clone();

                ctx.steps.push(HandoffStep {
                    name: "catalyst_full_capture".into(),
                    ok: true,
                    detail: Some(format!(
                        "BAR0 post-swap: {} alive regs, snapshot={}, open_ms={}, capture_ms={}",
                        alive,
                        snapshot_path.as_deref().unwrap_or("none"),
                        t.elapsed().as_millis(),
                        cap_start.elapsed().as_millis(),
                    )),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                warm_swap_catalyst::attempt_fecs_init_ctxsw(
                    &post_swap_bar0,
                    ctx,
                    pri_faults_persistent,
                    pri_intr,
                );

                let _tier_t = Instant::now();
                let warm_tier = classify_tier(&post_swap_bar0);
                tracing::info!(
                    bdf = ctx.config.bdf.as_str(),
                    tier = ?warm_tier.tier,
                    tpc_alive = warm_tier.tpc_alive,
                    gpc_enables = warm_tier.gpc_enables,
                    tpc_status = warm_tier.tpc_status.map(|v| format!("0x{v:08x}")),
                    "early tier classification (warm BAR0, pre-PRI-recovery)"
                );
                ctx.catalyst_tier = Some(warm_tier);
            }
        }
        Err(e) => {
            tracing::warn!(
                bdf = ctx.config.bdf.as_str(),
                err = %e,
                open_ms = t.elapsed().as_millis() as u64,
                "catalyst capture: post-swap BAR0 open failed"
            );
            ctx.steps.push(HandoffStep {
                name: "catalyst_full_capture".into(),
                ok: false,
                detail: Some(format!(
                    "post-swap BAR0 open failed ({}ms): {e}",
                    t.elapsed().as_millis()
                )),
                duration_ms: t.elapsed().as_millis() as u64,
            });
        }
    }
}
