// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::nv::registers::pmc;
use crate::vfio::sovereign_tiers::classify_tier;
use toadstool_ember::pri_ring_anchor::BootServiceEvidence;

use super::super::pipeline::PipelineContext;
use super::super::rollback::deadline_exceeded;
use super::super::types::{HandoffResult, HandoffStep};

pub(crate) fn run(ctx: &mut PipelineContext<'_>) -> Option<HandoffResult> {
        // ── Step 4: Settle — wait for hardware initialization ───────────

        let t = Instant::now();
        tracing::info!(bdf = ctx.config.bdf.as_str(), seeder = ctx.config.seeder_driver.as_str(),
                       settle_ms = ctx.config.settle.as_millis() as u64,
                       "waiting for seeder hardware initialization");
        std::thread::sleep(ctx.config.settle);
        ctx.steps.push(HandoffStep {
            name: "seeder_settle".into(), ok: true,
            detail: Some(format!("{}ms settle", ctx.config.settle.as_millis())),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        // ── Post-settle GPU health check (catalyst only) ─────────────
        //
        // After the settle period, verify the seeder driver (RM) actually
        // completed DEVINIT. If PMC_ENABLE is still cold (popcount < 10),
        // the driver failed to initialize — log a clear diagnostic but
        // continue to capture whatever state exists for forensics.
        if ctx.is_catalyst {
            let t = Instant::now();
            // Map full 16MB BAR0 — FECS is at 0x409xxx, TPC at 0x504xxx.
            match crate::vfio::device::MappedBar::from_sysfs_rw(&ctx.config.bdf, 16 * 1024 * 1024) {
                Ok(bar0) => {
                    let pmc = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
                    let popcount = pmc.count_ones();
                    if popcount < ctx.hw.pmc_warm_threshold {
                        tracing::error!(
                            bdf = ctx.config.bdf.as_str(),
                            pmc = format_args!("0x{pmc:08x}"),
                            popcount,
                            "catalyst settle: RM did NOT complete DEVINIT — GPU still cold"
                        );
                        ctx.steps.push(HandoffStep {
                            name: "settle_health".into(), ok: false,
                            detail: Some(format!(
                                "RM failed DEVINIT: PMC_ENABLE=0x{pmc:08x} (popcount={popcount})"
                            )),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                    } else {
                        tracing::info!(
                            bdf = ctx.config.bdf.as_str(),
                            pmc = format_args!("0x{pmc:08x}"),
                            popcount,
                            "catalyst settle: RM DEVINIT healthy"
                        );
                        ctx.steps.push(HandoffStep {
                            name: "settle_health".into(), ok: true,
                            detail: Some(format!(
                                "PMC_ENABLE=0x{pmc:08x} (popcount={popcount}) — RM initialized"
                            )),
                            duration_ms: t.elapsed().as_millis() as u64,
                        });
                    }
                    drop(bar0);
                }
                Err(e) => {
                    tracing::warn!(
                        bdf = ctx.config.bdf.as_str(),
                        error = %e,
                        "settle health: cannot open BAR0 — RM may be holding resource0"
                    );
                }
            }
        }

        // ── Deadline check ──────────────────────────────────────────────

        if ctx.overall.elapsed() >= ctx.deadline {

            return Some(deadline_exceeded(&ctx.config.bdf, std::mem::take(&mut ctx.steps), ctx.patch_result.take(), ctx.module_loaded,
                                     &ctx.config.module_name, &ctx.sibling_state, ctx.overall));
        }

        ctx.heartbeat();
        // ── Step 4b: Catalyst Capture (if catalyst strategy) ──────────
        //
        // While the catalyst driver owns the GPU and has fully initialized
        // the compute pipeline, capture BAR0 state for preservation.
        // This is the "golden snapshot" — the catalyst's product.
        // (ctx.is_catalyst already set at Step 0e)

        if ctx.is_catalyst {
            let t = Instant::now();
            let bar0_size = 16 * 1024 * 1024; // 16 MiB
            match crate::vfio::device::MappedBar::from_sysfs_rw(&ctx.config.bdf, bar0_size) {
                Ok(catalyst_bar0) => {
                    // Quick targeted reads: tier classification + sovereign snapshot.
                    // These read ~20 specific registers and complete in microseconds.
                    // The full 16MB capture is deferred to after warm swap (back on
                    // vfio-pci), because bulk MMIO reads while the nvidia RM is
                    // active can hit PRI fault regions and hang the thread.
                    let sovereign_snap = crate::vfio::sovereign_stages::SovereignSnapshot::capture(&catalyst_bar0);
                    let tier_ev = classify_tier(&catalyst_bar0);

                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        tier = ?tier_ev.tier,
                        pmc_enable = format_args!("{:#010x}", tier_ev.pmc_enable),
                        tpc_alive = tier_ev.tpc_alive,
                        "catalyst capture: tier evidence while catalyst owns GPU"
                    );

                    tracing::info!(
                        pmc_enable = format_args!("{:#010x}", sovereign_snap.pmc_enable),
                        fecs_cpuctl = format_args!("{:#010x}", sovereign_snap.fecs_cpuctl),
                        fecs_pc = format_args!("{:#010x}", sovereign_snap.fecs_pc),
                        gpccs_cpuctl = format_args!("{:#010x}", sovereign_snap.gpccs_cpuctl),
                        pmu_cpuctl = format_args!("{:#010x}", sovereign_snap.pmu_cpuctl),
                        pgraph_status = format_args!("{:#010x}", sovereign_snap.pgraph_status),
                        "catalyst capture: sovereign snapshot registers (pre-swap)"
                    );

                    ctx.catalyst_tier = Some(tier_ev);

                    // ── ExitBootServices: capture firmware evidence ──
                    let mut evidence = BootServiceEvidence::new(
                        "gpu-falcon",
                        "FECS/GPCCS/PMU state captured pre-swap (ExitBootServices)",
                    );
                    evidence.record("bdf", &ctx.config.bdf);
                    evidence.record("fecs_cpuctl", format!("{:#010x}", sovereign_snap.fecs_cpuctl));
                    evidence.record("fecs_pc", format!("{:#010x}", sovereign_snap.fecs_pc));
                    evidence.record("gpccs_cpuctl", format!("{:#010x}", sovereign_snap.gpccs_cpuctl));
                    evidence.record("pmu_cpuctl", format!("{:#010x}", sovereign_snap.pmu_cpuctl));
                    evidence.record("pmc_enable", format!("{:#010x}", sovereign_snap.pmc_enable));
                    evidence.record("pgraph_status", format!("{:#010x}", sovereign_snap.pgraph_status));
                    // Probe TPC status across GPCs (generation-aware topology)
                    for gpc in 0..ctx.hw.gpc_count {
                        let addr = ctx.hw.tpc_base as usize + gpc as usize * ctx.hw.tpc_gpc_stride as usize;
                        if let Ok(tpc_val) = catalyst_bar0.read_u32(addr) {
                            let is_fault = tpc_val & 0xBADF_0000 == 0xBADF_0000;
                            evidence.record(
                                format!("gpc{gpc}_tpc0"),
                                format!("{:#010x}{}", tpc_val, if is_fault { " FAULT" } else { " ALIVE" }),
                            );
                        }
                    }

                    // ── FECS IMEM capture attempt (while nvidia still loaded) ──
                    // Falcon PIO: set IMEMC once with auto-increment read
                    // (bit 25), then read IMEMD sequentially.
                    let fecs_base = ctx.hw.fecs_base as usize;
                    let imemc = fecs_base + 0x180;
                    let imemd = fecs_base + 0x184;
                    // Set IMEMC: start at address 0, auto-increment read (bit 25)
                    let _ = catalyst_bar0.write_u32(imemc, 0x0200_0000);
                    std::thread::sleep(std::time::Duration::from_micros(100));
                    let mut imem_probe = Vec::with_capacity(16);
                    for _ in 0..16 {
                        let word = catalyst_bar0.read_u32(imemd).unwrap_or(0xDEAD_DEAD);
                        imem_probe.push(word);
                    }
                    let imem_nonzero = imem_probe.iter().filter(|&&w| w != 0 && w != 0xDEAD_DEAD).count();
                    let imem_faulted = imem_probe.iter().filter(|&&w| w & 0xBADF_0000 == 0xBADF_0000).count();
                    evidence.record("fecs_imem_probe_nonzero", imem_nonzero.to_string());
                    evidence.record("fecs_imem_probe_faulted", imem_faulted.to_string());
                    evidence.record("fecs_imem_word0", format!("{:#010x}", imem_probe[0]));
                    evidence.record("fecs_imem_word1", format!("{:#010x}", imem_probe[1]));
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        nonzero = imem_nonzero,
                        faulted = imem_faulted,
                        word0 = format_args!("{:#010x}", imem_probe[0]),
                        word1 = format_args!("{:#010x}", imem_probe[1]),
                        word2 = format_args!("{:#010x}", imem_probe[2]),
                        word3 = format_args!("{:#010x}", imem_probe[3]),
                        "FECS IMEM probe (PIO read) while nvidia loaded"
                    );

                    // If IMEM is readable, do full FECS + GPCCS capture
                    if imem_nonzero > 0 && imem_faulted == 0 {
                        let fw_dir = "/var/lib/toadstool/catalysts/firmware";
                        let _ = std::fs::create_dir_all(fw_dir);
                        for (name, base) in [("fecs", 0x40_9000usize), ("gpccs", 0x41_a000usize)] {
                            let imemc_r = base + 0x180;
                            let imemd_r = base + 0x184;
                            let imem_size = 32 * 1024usize;
                            let word_count = imem_size / 4;
                            let _ = catalyst_bar0.write_u32(imemc_r, 0x0200_0000);
                            std::thread::sleep(std::time::Duration::from_micros(100));
                            let mut fw_data = Vec::with_capacity(word_count);
                            for _ in 0..word_count {
                                fw_data.push(catalyst_bar0.read_u32(imemd_r).unwrap_or(0));
                            }
                            let fw_bytes: Vec<u8> = fw_data.iter()
                                .flat_map(|w| w.to_le_bytes()).collect();
                            let nonzero_bytes = fw_bytes.iter().filter(|&&b| b != 0).count();
                            let fw_path = format!("{fw_dir}/{name}_imem_{}.bin", ctx.hw.chip_name);
                            if std::fs::write(&fw_path, &fw_bytes).is_ok() {
                                evidence.record(
                                    format!("{name}_imem_captured"),
                                    format!("{} bytes, {} nonzero", fw_bytes.len(), nonzero_bytes),
                                );
                                tracing::info!(
                                    engine = name, path = fw_path.as_str(),
                                    size = fw_bytes.len(), nonzero = nonzero_bytes,
                                    "{name} IMEM firmware captured to disk"
                                );
                            }
                        }
                    }

                    // ── PCCSR scan: verify RM channel is ACTIVE (Exp 229) ──
                    if let Some(ref ev) = ctx.rm_channel_evidence
                        && ev.all_ok
                    {
                            let pccsr_base = ctx.hw.pccsr_base as usize;
                            let mut active_channels = Vec::new();
                            let mut pending_channels = Vec::new();
                            for ch in 0..ctx.hw.pccsr_channel_count {
                                let addr = pccsr_base + ch as usize * 8;
                                if let Ok(val) = catalyst_bar0.read_u32(addr) {
                                    let enabled = val & 1;
                                    let status = (val >> 24) & 0x1F;
                                    if enabled != 0 {
                                        if status >= 5 {
                                            active_channels.push((ch, val));
                                        } else {
                                            pending_channels.push((ch, val));
                                        }
                                    }
                                }
                            }
                            for &(ch, val) in &active_channels {
                                evidence.record(
                                    format!("pccsr_ch{ch}_active"),
                                    format!("{:#010x} (status={})", val, (val >> 24) & 0x1F),
                                );
                            }
                            for &(ch, val) in &pending_channels {
                                evidence.record(
                                    format!("pccsr_ch{ch}_pending"),
                                    format!("{:#010x} (status={})", val, (val >> 24) & 0x1F),
                                );
                            }
                            tracing::info!(
                                bdf = ctx.config.bdf.as_str(),
                                active = active_channels.len(),
                                pending = pending_channels.len(),
                                rm_channel_id = ?ev.channel_id,
                                "PCCSR channel scan while catalyst loaded (Exp 229)"
                            );
                    }

                    ctx.boot_evidence = Some(evidence);
                    tracing::info!(
                        bdf = ctx.config.bdf.as_str(),
                        preserved_keys = ctx.boot_evidence.as_ref().map(|e| e.preserved_state.len()).unwrap_or(0),
                        "ExitBootServices: firmware evidence captured"
                    );

                    // Drop the BAR0 mapping before warm swap to release the fd
                    drop(catalyst_bar0);

                    ctx.steps.push(HandoffStep {
                        name: "catalyst_capture".into(), ok: true,
                        detail: Some(format!(
                            "pre-swap tier={:?} (full capture deferred to post-swap)",
                            ctx.catalyst_tier.as_ref().map(|t| &t.tier),
                        )),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
                Err(e) => {
                    tracing::warn!(
                        bdf = ctx.config.bdf.as_str(),
                        err = %e,
                        "catalyst capture: failed to open BAR0 — skipping capture"
                    );
                    ctx.steps.push(HandoffStep {
                        name: "catalyst_capture".into(), ok: false,
                        detail: Some(format!("BAR0 open failed: {e}")),
                        duration_ms: t.elapsed().as_millis() as u64,
                    });
                }
            }
        }


    None
}
