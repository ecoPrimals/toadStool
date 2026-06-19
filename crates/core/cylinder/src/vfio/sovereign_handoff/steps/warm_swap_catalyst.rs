// SPDX-License-Identifier: AGPL-3.0-or-later
//! Catalyst-specific warm swap phases: post-unbind BAR0 diagnostics,
//! FECS INIT_CTXSW, and snapshot persistence.
//!
//! Extracted from `warm_swap::run()` to keep the main pipeline orchestration
//! readable while preserving the full register-level hardware flow.

use std::time::{Duration, Instant};

use crate::nv::registers::{falcon, gpc, pmc, pri};
use crate::vfio::device::MappedBar;
use crate::vfio::warm_capture::Bar0Snapshot;

use super::super::pipeline::PipelineContext;
use super::super::types::HandoffStep;

fn breadcrumb(msg: &str) {
    crate::vfio::sovereign_handoff::forensics::breadcrumb(msg);
}

/// Post-unbind PCI config + BAR0 diagnostic probe.
///
/// Reads PCI command register and power state, then opens BAR0 to sample
/// key FECS/GPC/PRI registers. Pure read-only — no writes, no side effects.
pub(crate) fn post_unbind_diagnostic(ctx: &PipelineContext<'_>) {
    breadcrumb("step6: post-unbind PCI diag start");
    let pci_config_path = crate::linux_paths::sysfs_pci_device_file(&ctx.config.bdf, "config");
    let pci_cmd = std::fs::read(&pci_config_path).ok().and_then(|data| {
        if data.len() >= 6 {
            Some(u16::from_le_bytes([data[4], data[5]]))
        } else {
            None
        }
    });
    let pci_pm_ctrl = std::fs::read(&pci_config_path).ok().and_then(|data| {
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

    if let Ok(diag_bar0) = MappedBar::from_sysfs_rw(&ctx.config.bdf, 16 * 1024 * 1024) {
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
        let pri_intr = diag_bar0
            .read_u32(pri::INTR_STATUS as usize)
            .unwrap_or(0xDEAD);
        let pri_status = diag_bar0
            .read_u32(pri::STATUS_ENUM as usize)
            .unwrap_or(0xDEAD);
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
}

/// Attempt FECS INIT_CTXSW on the warm post-swap BAR0.
///
/// Probes FECS state, optionally unhalts the falcon, sends INIT_CTXSW,
/// and records the result as a pipeline step. Gated on PRI fault state
/// to avoid wedging the bus with writes to a faulted PRI ring.
pub(crate) fn attempt_fecs_init_ctxsw(
    bar0: &MappedBar,
    ctx: &mut PipelineContext<'_>,
    pri_faults_persistent: bool,
    pri_intr: u32,
) {
    let fecs_t = Instant::now();
    let fecs_pc = bar0.read_u32(falcon::FECS_CTXSW_PC as usize).unwrap_or(0);
    let fecs_cpuctl = bar0
        .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
        .unwrap_or(0);
    let fecs_os = bar0
        .read_u32((falcon::FECS_BASE + falcon::PC) as usize)
        .unwrap_or(0);
    let gpccs_cpuctl = bar0
        .read_u32((falcon::GPCCS_BASE + falcon::CPUCTL) as usize)
        .unwrap_or(0);
    let gpc_en = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0);
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

    if pri_faults_persistent {
        breadcrumb("FECS INIT_CTXSW SKIPPED — PRI faults persistent, writes unsafe");
        tracing::warn!(
            bdf = ctx.config.bdf.as_str(),
            pri_intr = format_args!("0x{pri_intr:08x}"),
            fecs_alive,
            "FECS INIT_CTXSW SKIPPED — PRI ring faults not clearable, GPU writes could wedge bus"
        );
        ctx.steps.push(HandoffStep {
            name: "fecs_init_ctxsw".into(),
            ok: false,
            detail: Some(format!(
                "SKIPPED: PRI faults persistent (0x{pri_intr:08x}), writes unsafe"
            )),
            duration_ms: fecs_t.elapsed().as_millis() as u64,
        });
    } else if fecs_alive {
        if fecs_halted {
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                "FECS halted — attempting unhalt (CPUCTL START_CPU)"
            );
            let _ = bar0.write_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize, 0x2);
            std::thread::sleep(Duration::from_millis(200));
            let pc_after = bar0.read_u32(falcon::FECS_CTXSW_PC as usize).unwrap_or(0);
            let cpuctl_after = bar0
                .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                .unwrap_or(0);
            tracing::info!(
                bdf = ctx.config.bdf.as_str(),
                fecs_pc_after = format_args!("0x{pc_after:08x}"),
                fecs_cpuctl_after = format_args!("0x{cpuctl_after:08x}"),
                "FECS unhalt result"
            );
        }

        tracing::info!(
            bdf = ctx.config.bdf.as_str(),
            "FECS accessible — sending INIT_CTXSW"
        );
        match crate::vfio::channel::fecs::fecs_init_ctxsw(bar0) {
            Ok(r) => {
                std::thread::sleep(Duration::from_secs(1));
                let tpc0 = bar0
                    .read_u32((gpc::gpc_tpc0(0) + 0x100) as usize)
                    .unwrap_or(0xdead);
                let gpc_en_post = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0);
                let gpccs_post = bar0
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
                    name: "fecs_init_ctxsw".into(),
                    ok: r.status == 0,
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
                    name: "fecs_init_ctxsw".into(),
                    ok: false,
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
            name: "fecs_init_ctxsw".into(),
            ok: false,
            detail: Some(format!("FECS PRI fault: cpuctl=0x{fecs_cpuctl:08x}")),
            duration_ms: fecs_t.elapsed().as_millis() as u64,
        });
    }
}

/// Capture and persist BAR0 domain snapshot + catalyst replay sequence.
///
/// Returns `(alive_count, snapshot_path)` on success.
pub(crate) fn capture_and_persist_snapshot(
    bar0: &MappedBar,
    ctx: &mut PipelineContext<'_>,
    cap_start: Instant,
    open_start: Instant,
) -> (usize, Option<String>) {
    let domains = ctx.hw.bar0_domains;
    let full_snapshot =
        Bar0Snapshot::capture_domains(bar0, &ctx.config.bdf, "catalyst-post-swap", domains);
    let alive = full_snapshot.alive_count();

    tracing::info!(
        bdf = ctx.config.bdf.as_str(),
        total_regs = full_snapshot.len(),
        alive_regs = alive,
        capture_ms = cap_start.elapsed().as_millis() as u64,
        open_ms = open_start.elapsed().as_millis() as u64,
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
            tracing::info!(
                path = snapshot_path.as_str(),
                bytes = json.len(),
                "catalyst capture: snapshot persisted"
            );
        }
    }

    let chip_family = crate::nv::gr_init::ChipFamily::from_sm(ctx.hw.sm);
    let replay = full_snapshot.to_catalyst_replay(chip_family, "470.256.02", ctx.hw.bar0_domains);
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

    (alive, Some(snapshot_path))
}
