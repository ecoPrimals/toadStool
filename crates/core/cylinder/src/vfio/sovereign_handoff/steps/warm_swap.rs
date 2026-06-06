// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::nv::registers::{falcon, gpc, pmc, pri};
use crate::vfio::guarded_sysfs;
use crate::vfio::sovereign_tiers::classify_tier;
use crate::vfio::warm_capture::Bar0Snapshot;

use super::super::pipeline::PipelineContext;
use super::super::rollback::halt_result;
use super::super::types::{HandoffResult, HandoffStep};

fn breadcrumb(msg: &str) {
    crate::vfio::sovereign_handoff::forensics::breadcrumb(msg);
}

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        breadcrumb("warm_swap::run ENTERED");
        // ── Step 5: Pin bridges + disable FLR + suppress SBR ───────────
        //
        // The RPC handler calls prepare_anchor_release() before dropping the
        // VfioAnchor. For cold GPUs (Exp 229), SBR was intentionally allowed
        // during anchor release so RM could cold-boot with a clean PCIe reset.
        // Now that RM init is complete (seeder_settle finished), we MUST
        // suppress SBR before the warm swap to preserve the RM-initialized
        // state. For warm GPUs, SBR was already suppressed — this is idempotent.

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
            name: "prepare_warm_swap".into(), ok: true,
            detail: Some("bridge pinned, FLR disabled, SBR suppressed".into()),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        ctx.heartbeat();

        // ── Pre-unbind interrupt kill (Exp 233 Run #3/#4 IRQ storm fix) ──
        // Defense-in-depth: disable ALL interrupt paths BEFORE unbind.
        // 1. GPU-side: quench INTR_EN via BAR0 (stops GPU from asserting IRQs)
        // 2. PCI config: disable MSI/MSI-X (stops PCI from routing MSI)
        // 3. PCI config: disable INTx (stops legacy pin interrupts)
        // 4. PCI config: disable Bus Master (physically prevents MSI memory
        //    writes — the nuclear option; MSI cannot fire without DMA)
        //
        // Run #4 still locked up despite MSI disable alone. The GPU firmware
        // or driver internals may re-enable MSI. Bus Master disable is the
        // hardware-level guarantee: no memory writes → no MSI delivery.
        if ctx.is_catalyst {
            breadcrumb("pre-unbind: quench_interrupts");
            crate::nv::registers::pmc::quench_interrupts(
                &ctx.config.bdf, &ctx.hw.interrupt_profile, "pre-unbind",
            );
            breadcrumb("pre-unbind: disable_pci_msi");
            crate::nv::registers::pmc::disable_pci_msi(
                &ctx.config.bdf, "pre-unbind",
            );
            breadcrumb("pre-unbind: intx_disable");
            crate::nv::registers::pmc::intx_disable(
                &ctx.config.bdf, "pre-unbind",
            );
            breadcrumb("pre-unbind: disable_bus_master");
            crate::nv::registers::pmc::disable_bus_master(
                &ctx.config.bdf, "pre-unbind",
            );
            breadcrumb("pre-unbind: ALL interrupt defenses complete");

            // IRQ Clutch (PRE-UNBIND): nv_close_device is NOP'd so NVIDIA's
            // free_irq + pci_disable_msi never ran. The kernel's unbind path
            // will try to tear down stale irq_domain / msi_device_data →
            // irq_domain_remove WARNING → corrupted state → vfio-pci probe
            // hits freed memory → hard lockup.
            //
            // The clutch calls pci_free_irq_vectors() BEFORE unbind, while
            // the MSI data structures are still valid. This properly tears
            // down the IRQ domain so unbind finds clean state.
            //
            // MUST run after interrupt quench (GPU silenced) but before the
            // sysfs unbind write.
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

        // ── Step 6: Warm swap — seeder → final driver (GUARDED) ─────────

        let t = Instant::now();
        breadcrumb("step6: reading current driver");
        if let Some(ref current) = guarded_sysfs::read_current_driver(&ctx.config.bdf) {
            let remaining = ctx.deadline.saturating_sub(ctx.overall.elapsed());
            let unbind_result = if ctx.is_catalyst {
                breadcrumb("step6: UNBIND FIRE — writing to sysfs unbind NOW");
                guarded_sysfs::sysfs_unbind_fire_and_poll(
                    &ctx.config.bdf, current, remaining,
                )
                .map(|elapsed| {
                    breadcrumb(&format!("step6: UNBIND COMPLETE — took {}s", elapsed.as_secs()));
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        elapsed_s = elapsed.as_secs(),
                        "catalyst teardown completed via fire-and-poll"
                    );
                })
            } else {
                let unbind_path = crate::linux_paths::sysfs_pci_driver_unbind(current);
                guarded_sysfs::sysfs_write_guarded(
                    &unbind_path, &ctx.config.bdf, guarded_sysfs::UNBIND_TIMEOUT,
                )
            };
            if let Err(e) = unbind_result {
                ctx.steps.push(HandoffStep {
                    name: "warm_swap".into(), ok: false,
                    detail: Some(format!("unbind {current} failed: {e}")),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(&ctx.config.bdf, "warm_swap", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                   ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                                   &ctx.config.module_name, ctx.needs_device_rollback));
            }
        }

        breadcrumb("step6: post-unbind — driver detached");

        if ctx.is_catalyst {
            breadcrumb("step6: post-unbind PCI diag start");
            // Diagnostic: probe PCI config space and BAR0 between unbind and rebind.
            // Read PCI command register to check if bus mastering was disabled.
            let pci_config_path = crate::linux_paths::sysfs_pci_device_file(
                &ctx.config.bdf, "config",
            );
            let pci_cmd = std::fs::read(&pci_config_path)
                .ok()
                .and_then(|data| {
                    if data.len() >= 6 {
                        Some(u16::from_le_bytes([data[4], data[5]]))
                    } else {
                        None
                    }
                });
            let pci_pm_ctrl = std::fs::read(&pci_config_path)
                .ok()
                .and_then(|data| {
                    // PM cap at offset 0x60, PMCSR at +0x04 = 0x64
                    if data.len() >= 0x66 {
                        Some(u16::from_le_bytes([data[0x64], data[0x65]]))
                    } else {
                        None
                    }
                });
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                pci_cmd = pci_cmd.map(|v| format!("{:#06x}", v)),
                bus_master = pci_cmd.map(|v| v & 0x4 != 0),
                mem_space = pci_cmd.map(|v| v & 0x2 != 0),
                pm_state = pci_pm_ctrl.map(|v| format!("D{}", v & 0x3)),
                "post-unbind PCI config: command register and power state"
            );

            if let Ok(diag_bar0) = crate::vfio::device::MappedBar::from_sysfs_rw(
                &ctx.config.bdf, 16 * 1024 * 1024,
            ) {
                let fecs_cpuctl = diag_bar0
                    .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                    .unwrap_or(0xDEAD);
                let fecs_hw_pc = diag_bar0
                    .read_u32((falcon::FECS_BASE + 0x11C) as usize)
                    .unwrap_or(0xDEAD);
                let fecs_ctxsw = diag_bar0
                    .read_u32(falcon::FECS_CTXSW_PC as usize)
                    .unwrap_or(0xDEAD);
                let gpccs_cpuctl = diag_bar0
                    .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                    .unwrap_or(0xDEAD);
                let pri_intr = diag_bar0.read_u32(pri::INTR_STATUS as usize).unwrap_or(0xDEAD);
                let pri_status = diag_bar0.read_u32(pri::STATUS_ENUM as usize).unwrap_or(0xDEAD);
                let pmc_enable = diag_bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0xDEAD);
                let tpc0 = diag_bar0
                    .read_u32(gpc::gpc_tpc0(0) as usize)
                    .unwrap_or(0xDEAD);
                tracing::info!(
                    bdf = ctx.config.bdf.as_str(),
                    fecs_cpuctl = format_args!("{:#010x}", fecs_cpuctl),
                    fecs_hw_pc = format_args!("{:#010x}", fecs_hw_pc),
                    fecs_ctxsw = format_args!("{:#010x}", fecs_ctxsw),
                    gpccs_cpuctl = format_args!("{:#010x}", gpccs_cpuctl),
                    pmc_enable = format_args!("{:#010x}", pmc_enable),
                    pri_ring_intr = format_args!("{:#010x}", pri_intr),
                    pri_ring_status = format_args!("{:#010x}", pri_status),
                    tpc0_ctrl = format_args!("{:#010x}", tpc0),
                    "catalyst diagnostic: BAR0 state AFTER unbind, BEFORE rebind"
                );
                drop(diag_bar0);
            }

            // Rebind strategy depends on whether the pre-unbind IRQ clutch
            // succeeded. If the clutch cleaned the MSI/IRQ state, drivers_probe
            // should be safe. If it failed, fall back to misfire mode (leave
            // GPU unbound) to prevent hard lockups.
            if !ctx.irq_clutch_engaged {
                // ── MISFIRE MODE (clutch failed) ─────────────────────────
                breadcrumb("step6: MISFIRE MODE — clutch failed, skipping drivers_probe");

                let override_set = matches!(
                    guarded_sysfs::sysfs_write_guarded(
                        &ctx.override_path, &ctx.config.final_driver,
                        Duration::from_secs(5),
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
                    name: "warm_swap".into(), ok: true,
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
                // Pre-unbind clutch cleaned MSI/IRQ state → unbind should
                // have completed without irq_domain_remove corruption →
                // drivers_probe should bind vfio-pci cleanly.
                let poll_deadline = ctx.deadline.saturating_sub(ctx.overall.elapsed());
                let poll_start = Instant::now();
                let poll_interval = Duration::from_secs(5);
                let mut override_set = false;
                let mut probe_sent = false;
                let mut final_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);

                breadcrumb("step6: clutch OK — attempting full rebind via drivers_probe");

                while final_driver.as_deref() != Some(ctx.config.final_driver.as_str()) {
                    if poll_start.elapsed() >= poll_deadline {
                        ctx.steps.push(HandoffStep {
                            name: "warm_swap".into(), ok: false,
                            detail: Some(format!(
                                "poll for {} timed out (driver={:?}, override_set={}, probe_sent={})",
                                ctx.config.final_driver, final_driver, override_set, probe_sent,
                            )),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        return Some(halt_result(&ctx.config.bdf, "warm_swap", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                           ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                                           &ctx.config.module_name, ctx.needs_device_rollback));
                    }

                    if !override_set
                        && matches!(
                            guarded_sysfs::sysfs_write_guarded(
                                &ctx.override_path, &ctx.config.final_driver,
                                Duration::from_secs(5),
                            ),
                            Ok(()),
                        )
                    {
                        override_set = true;
                        breadcrumb("step6: driver_override set");
                        tracing::info!(bdf = ctx.config.bdf.as_str(),
                            "catalyst: driver_override set to {}", ctx.config.final_driver);
                    }

                    if override_set && !probe_sent {
                        breadcrumb("step6: sending drivers_probe");
                        if matches!(
                            guarded_sysfs::sysfs_write_guarded(
                                &ctx.probe_path, &ctx.config.bdf,
                                Duration::from_secs(5),
                            ),
                            Ok(()),
                        ) {
                            probe_sent = true;
                            breadcrumb("step6: drivers_probe sent");
                            tracing::info!(bdf = ctx.config.bdf.as_str(),
                                "catalyst: drivers_probe sent (IRQ clutch cleaned pre-unbind)");
                        }
                    }

                    std::thread::sleep(poll_interval);
                    final_driver = guarded_sysfs::read_current_driver(&ctx.config.bdf);
                }

                let swap_elapsed = t.elapsed();
                breadcrumb(&format!(
                    "step6: full rebind complete — driver={}, {}s",
                    ctx.config.final_driver, swap_elapsed.as_secs(),
                ));
                tracing::info!(
                    bdf = ctx.config.bdf.as_str(),
                    final_driver = ctx.config.final_driver.as_str(),
                    elapsed_s = swap_elapsed.as_secs(),
                    "catalyst warm_swap: final driver bound (clutch-cleaned rebind)"
                );
                ctx.steps.push(HandoffStep {
                    name: "warm_swap".into(), ok: true,
                    detail: Some(format!("{} → {} (clutch-cleaned, {}s)",
                        ctx.config.seeder_driver, ctx.config.final_driver, swap_elapsed.as_secs())),
                    duration_ms: swap_elapsed.as_millis() as u64,
                });
            }
        } else {
            if let Err(e) = guarded_sysfs::sysfs_write(&ctx.override_path, &ctx.config.final_driver) {
                ctx.steps.push(HandoffStep {
                    name: "warm_swap".into(), ok: false,
                    detail: Some(format!("override to {} failed: {e}", ctx.config.final_driver)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(&ctx.config.bdf, "warm_swap", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                   ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                                   &ctx.config.module_name, ctx.needs_device_rollback));
            }

            if let Err(e) = guarded_sysfs::sysfs_write_guarded(
                &ctx.probe_path, &ctx.config.bdf, guarded_sysfs::PROBE_TIMEOUT,
            ) {
                ctx.steps.push(HandoffStep {
                    name: "warm_swap".into(), ok: false,
                    detail: Some(format!("guarded drivers_probe for {} failed: {e}", ctx.config.final_driver)),
                    duration_ms: t.elapsed().as_millis() as u64,
                });

                return Some(halt_result(&ctx.config.bdf, "warm_swap", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                   ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                                   &ctx.config.module_name, ctx.needs_device_rollback));
            }

            let final_bound = guarded_sysfs::read_current_driver(&ctx.config.bdf);
            let swap_ok = final_bound.as_deref() == Some(ctx.config.final_driver.as_str());
            ctx.steps.push(HandoffStep {
                name: "warm_swap".into(), ok: swap_ok,
                detail: Some(format!("{} → {} (warm_preserved={})",
                    ctx.config.seeder_driver, final_bound.as_deref().unwrap_or("none"), swap_ok)),
                duration_ms: t.elapsed().as_millis() as u64,
            });

            if !swap_ok {
                return Some(halt_result(&ctx.config.bdf, "warm_swap", std::mem::take(&mut ctx.steps), ctx.patch_result.take(),
                                   ctx.module_loaded, false, ctx.overall, &ctx.sibling_state,
                                   &ctx.config.module_name, ctx.needs_device_rollback));
            }
        }

        ctx.heartbeat();
        // ── Step 7a: Deferred catalyst full capture (BEFORE sibling rebind) ──
        //
        // For catalyst: capture BAR0 immediately after warm_swap while the
        // register state is warm-preserved. This MUST happen before sibling
        // rebind (step 6b) because rebind_siblings_to_vfio does sysfs
        // writes that contend with the nvidia RM teardown's PCI device lock,
        // blocking for 7+ minutes. The BAR0 mmap via resource0 is safe once
        // vfio-pci owns the device — it bypasses the PCI config lock path.

        if ctx.is_catalyst {
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

                    // ── Device-alive pre-flight before 641K-register scan ──
                    //
                    // Reading a hung GPU register causes a PCIe completion
                    // timeout that freezes the CPU — no kernel error, no
                    // recovery. Probe two lightweight registers first:
                    //   BOOT0 (0x0): chip identity, always readable if alive
                    //   PMC_ENABLE (0x200): engine clock gates, 0 = all off
                    // Also ack any pending PRI ring faults so the scan doesn't
                    // hit a stalled PRI client.
                    let boot0 = post_swap_bar0.read_u32(pmc::BOOT0 as usize).unwrap_or(0xFFFF_FFFF);
                    let pmc_en = post_swap_bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
                    let pri_intr = post_swap_bar0.read_u32(pri::INTR_STATUS as usize).unwrap_or(0);

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
                            name: "catalyst_full_capture".into(), ok: false,
                            detail: Some("BOOT0=0xFFFFFFFF — device not responding, scan skipped".into()),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                        drop(post_swap_bar0);
                        // fall through — skip scan, continue pipeline
                    } else {
                    // Ack PRI ring faults before scanning PRI-dependent domains.
                    // Track whether faults cleared — persistent faults mean
                    // write operations (FECS INIT_CTXSW) can wedge the PRI ring.
                    let pri_faults_persistent = if pri_intr != 0 {
                        let _ = post_swap_bar0.write_u32(pri::COMMAND as usize, 0x2);
                        std::thread::sleep(Duration::from_millis(10));
                        let pri_after = post_swap_bar0.read_u32(pri::INTR_STATUS as usize).unwrap_or(0);
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
                    let domains = ctx.hw.bar0_domains;
                    let full_snapshot = Bar0Snapshot::capture_domains(
                        &post_swap_bar0, &ctx.config.bdf, "catalyst-post-swap", domains,
                    );
                    let alive = full_snapshot.alive_count();

                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        total_regs = full_snapshot.len(),
                        alive_regs = alive,
                        capture_ms = cap_start.elapsed().as_millis() as u64,
                        open_ms = t.elapsed().as_millis() as u64,
                        num_domains = domains.len(),
                        "catalyst capture: domain-scoped BAR0 snapshot (post-swap, vfio-pci safe)"
                    );

                    let snapshot_path = std::env::temp_dir()
                        .join(format!(
                            "toadstool-catalyst-{}.json",
                            ctx.config.bdf.replace([':', '.'], "-")
                        ))
                        .display()
                        .to_string();
                    if let Ok(json) = full_snapshot.to_json() {
                        if let Err(e) = std::fs::write(&snapshot_path, &json) {
                            tracing::warn!(err = %e, path = snapshot_path.as_str(),
                                           "catalyst capture: failed to persist snapshot");
                        } else {
                            tracing::info!(path = snapshot_path.as_str(),
                                           bytes = json.len(),
                                           "catalyst capture: snapshot persisted");
                            ctx.catalyst_snapshot_path = Some(snapshot_path.clone());
                        }
                    }

                    let chip_family = crate::nv::gr_init::ChipFamily::from_sm(ctx.hw.sm);
                    let replay = full_snapshot.to_catalyst_replay(
                        chip_family,
                        "470.256.02",
                        ctx.hw.bar0_domains,
                    );
                    let replay_path = std::env::temp_dir()
                        .join(format!(
                            "toadstool-catalyst-replay-{}.json",
                            ctx.config.bdf.replace([':', '.'], "-")
                        ))
                        .display()
                        .to_string();
                    if let Ok(json) = replay.to_json() {
                        if let Err(e) = std::fs::write(&replay_path, &json) {
                            tracing::warn!(err = %e, "catalyst capture: failed to persist replay");
                        } else {
                            tracing::info!(
                                path = replay_path.as_str(),
                                writes = replay.len(),
                                domains = replay.domains().len(),
                                "catalyst capture: replay sequence persisted"
                            );
                        }
                    }

                    ctx.catalyst_alive_count = Some(alive);

                    ctx.steps.push(HandoffStep {
                        name: "catalyst_full_capture".into(), ok: true,
                        detail: Some(format!(
                            "BAR0 post-swap: {} alive regs, snapshot={}, open_ms={}, capture_ms={}",
                            alive,
                            ctx.catalyst_snapshot_path.as_deref().unwrap_or("none"),
                            t.elapsed().as_millis(),
                            cap_start.elapsed().as_millis(),
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });

                    // ── FECS INIT_CTXSW: trigger GR init while BAR0 is warm ──
                    //
                    // CRITICAL: this must happen NOW, using the same BAR0 mapping
                    // that captured alive registers. The PRI ring recovery step
                    // (below) issues enumerate/start commands that destroy the
                    // PRI routing RM set up, causing FECS/TPC reads to PRI-fault.
                    // By sending INIT_CTXSW here, FECS is still accessible.
                    let fecs_t = Instant::now();
                    let fecs_pc = post_swap_bar0
                        .read_u32(falcon::FECS_CTXSW_PC as usize)
                        .unwrap_or(0);
                    let fecs_cpuctl = post_swap_bar0
                        .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                        .unwrap_or(0);
                    let fecs_os = post_swap_bar0
                        .read_u32((falcon::FECS_BASE + falcon::PC) as usize)
                        .unwrap_or(0);
                    let gpccs_cpuctl = post_swap_bar0
                        .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                        .unwrap_or(0);
                    let gpc_en = post_swap_bar0
                        .read_u32(gpc::BCAST_ENABLES as usize)
                        .unwrap_or(0);
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        fecs_pc = format_args!("0x{fecs_pc:08x}"),
                        fecs_os = format_args!("0x{fecs_os:08x}"),
                        fecs_cpuctl = format_args!("0x{fecs_cpuctl:08x}"),
                        gpccs_cpuctl = format_args!("0x{gpccs_cpuctl:08x}"),
                        gpc_enables = format_args!("0x{gpc_en:08x}"),
                        "pre-INIT_CTXSW: FECS state (using warm post-swap BAR0)"
                    );

                    let fecs_halted = fecs_cpuctl & 0x10 != 0;
                    let fecs_alive = fecs_cpuctl & 0xBADF_0000 != 0xBADF_0000;

                    // Gate: skip ALL GPU register WRITES when PRI faults
                    // are persistent. Reads are safe (return 0xBADF5040),
                    // but writes can wedge the PRI ring and cause a PCIe
                    // completion timeout that freezes the CPU.
                    if pri_faults_persistent {
                        breadcrumb("FECS INIT_CTXSW SKIPPED — PRI faults persistent, writes unsafe");
                        tracing::warn!(
                            bdf = ctx.config.bdf.as_str(),
                            pri_intr = format_args!("0x{pri_intr:08x}"),
                            fecs_alive,
                            "FECS INIT_CTXSW SKIPPED — PRI ring faults not clearable, GPU writes could wedge bus"
                        );
                        ctx.steps.push(HandoffStep {
                            name: "fecs_init_ctxsw".into(), ok: false,
                            detail: Some(format!(
                                "SKIPPED: PRI faults persistent (0x{pri_intr:08x}), writes unsafe"
                            )),
                            duration_ms: fecs_t.elapsed().as_millis() as u64,
                        });
                    } else if fecs_alive {
                        if fecs_halted {
                            tracing::info!(bdf = ctx.config.bdf.as_str(),
                                "FECS halted — attempting unhalt (CPUCTL START_CPU)");
                            let _ = post_swap_bar0.write_u32(
                                (falcon::FECS_BASE + falcon::CPUCTL) as usize,
                                0x2,
                            );
                            std::thread::sleep(Duration::from_millis(200));
                            let pc_after = post_swap_bar0
                                .read_u32(falcon::FECS_CTXSW_PC as usize)
                                .unwrap_or(0);
                            let cpuctl_after = post_swap_bar0
                                .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                                .unwrap_or(0);
                            tracing::info!(
                                bdf = ctx.config.bdf.as_str(),
                                fecs_pc_after = format_args!("0x{pc_after:08x}"),
                                fecs_cpuctl_after = format_args!("0x{cpuctl_after:08x}"),
                                "FECS unhalt result"
                            );
                        }

                        tracing::info!(bdf = ctx.config.bdf.as_str(),
                            "FECS accessible — sending INIT_CTXSW");
                        match crate::vfio::channel::fecs::fecs_init_ctxsw(&post_swap_bar0) {
                            Ok(r) => {
                                std::thread::sleep(Duration::from_secs(1));
                                let tpc0 = post_swap_bar0
                                    .read_u32((gpc::gpc_tpc0(0) + 0x100) as usize)
                                    .unwrap_or(0xdead);
                                let gpc_en_post = post_swap_bar0
                                    .read_u32(gpc::BCAST_ENABLES as usize)
                                    .unwrap_or(0);
                                let gpccs_post = post_swap_bar0
                                    .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
                                    .unwrap_or(0);
                                tracing::info!(
                                    bdf = ctx.config.bdf.as_str(),
                                    status = r.status,
                                    mailbox0 = format_args!("0x{:08x}", r.mailbox0),
                                    tpc0_ctrl = format_args!("0x{tpc0:08x}"),
                                    gpc_enables = format_args!("0x{gpc_en_post:08x}"),
                                    gpccs_cpuctl = format_args!("0x{gpccs_post:08x}"),
                                    "FECS INIT_CTXSW result (pre-PRI-recovery)"
                                );
                                ctx.steps.push(HandoffStep {
                                    name: "fecs_init_ctxsw".into(), ok: r.status == 0,
                                    detail: Some(format!(
                                        "status={}, mb0=0x{:08x}, tpc0=0x{tpc0:08x}, gpc_en=0x{gpc_en_post:08x}",
                                        r.status, r.mailbox0
                                    )),
                                    duration_ms: fecs_t.elapsed().as_millis() as u64,
                                });
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "FECS INIT_CTXSW failed");
                                ctx.steps.push(HandoffStep {
                                    name: "fecs_init_ctxsw".into(), ok: false,
                                    detail: Some(format!("failed: {e}")),
                                    duration_ms: fecs_t.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    } else {
                        tracing::warn!(
                            bdf = ctx.config.bdf.as_str(),
                            fecs_cpuctl = format_args!("0x{fecs_cpuctl:08x}"),
                            "FECS not accessible post-swap — skipping INIT_CTXSW"
                        );
                        ctx.steps.push(HandoffStep {
                            name: "fecs_init_ctxsw".into(), ok: false,
                            detail: Some(format!("FECS PRI fault: cpuctl=0x{fecs_cpuctl:08x}")),
                            duration_ms: fecs_t.elapsed().as_millis() as u64,
                        });
                    }

                    // ── Early tier classification using warm BAR0 ──
                    // The PRI ring recovery step below issues enumerate/start
                    // commands that destroy RM's PRI routing. Classify tier NOW
                    // while the BAR0 still reflects RM's warm state.
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
                    } // end else (device-alive pre-flight passed)
                }
                Err(e) => {
                    tracing::warn!(
                        bdf = ctx.config.bdf.as_str(),
                        err = %e,
                        open_ms = t.elapsed().as_millis() as u64,
                        "catalyst capture: post-swap BAR0 open failed"
                    );
                    ctx.steps.push(HandoffStep {
                        name: "catalyst_full_capture".into(), ok: false,
                        detail: Some(format!("post-swap BAR0 open failed ({}ms): {e}",
                            t.elapsed().as_millis())),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }
        }

    None
}
