// SPDX-License-Identifier: AGPL-3.0-or-later
//! Volta/Turing/Ampere PFIFO engine initialization.

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;

use super::super::registers::{pbdma, pmc, pfifo, pri, RUNLIST_IOVA};
use super::PfifoInitConfig;

/// Configurable PFIFO engine initialization.
///
/// Same as [`super::init::init_pfifo_engine`] but takes a [`PfifoInitConfig`] to
/// control the bring-up sequence. Use this from the diagnostic runner.
pub fn init_pfifo_engine_with(bar0: &MappedBar, cfg: &PfifoInitConfig) -> DriverResult<(u32, u32)> {
    let w = |reg: usize, val: u32| {
        bar0.write_u32(reg, val)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("PFIFO init {reg:#x}: {e}"))))
    };

    let boot0 = bar0.read_u32(0).unwrap_or(0);
    if boot0 == 0xFFFF_FFFF {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "BAR0 returns 0xFFFFFFFF — GPU in D3hot (PCIe sleep). \
             Fix: echo on > /sys/bus/pci/devices/<BDF>/power/control",
        )));
    }

    // Clear stale PRIV_RING faults before touching engine registers.
    if cfg.clear_priv_ring {
        let priv_intr = bar0.read_u32(pri::PRIV_RING_INTR_STATUS).unwrap_or(0);
        if priv_intr != 0 {
            for attempt in 0..5 {
                w(pri::PRIV_RING_COMMAND, pri::PRIV_RING_CMD_ACK)?;
                std::thread::sleep(std::time::Duration::from_millis(20));
                let status = bar0.read_u32(pri::PRIV_RING_INTR_STATUS).unwrap_or(0);
                if status == 0 {
                    tracing::info!(attempt, "PRIV_RING fault cleared");
                    break;
                }
                if attempt == 4 {
                    tracing::warn!(
                        status = format_args!("{status:#010x}"),
                        "PRIV_RING fault persists after 5 ACK attempts"
                    );
                }
            }
        }
        let pmc_intr = bar0.read_u32(pri::PMC_INTR).unwrap_or(0);
        let priv_after = bar0.read_u32(pri::PRIV_RING_INTR_STATUS).unwrap_or(0);
        tracing::info!(
            priv_before = format_args!("{priv_intr:#010x}"),
            priv_after = format_args!("{priv_after:#010x}"),
            pmc_intr = format_args!("{pmc_intr:#010x}"),
            "PRIV_RING fault clear"
        );
    }

    // Glow plug — enable all engines in PMC_ENABLE (0x200).
    // NB: DEVICE_ENABLE (0x600) is NOT present on GV100 (returns 0xBAD00200
    // PBUS timeout). Do not write it.
    let pmc_before = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
    if cfg.pmc_glow_plug {
        w(pmc::ENABLE, 0xFFFF_FFFF)?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let pmc_after = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
    tracing::info!(
        pmc_before = format_args!("{pmc_before:#010x}"),
        pmc_after = format_args!("{pmc_after:#010x}"),
        "PMC glow plug"
    );

    // PMC-level PFIFO reset: bit 8 per gk104_mc_reset (NOT bit 1).
    // On GV100, bit 1 of PMC_ENABLE is not the PFIFO engine control.
    // nouveau's gk104_mc_reset uses device-specific engine→bit mappings;
    // for PFIFO (NVKM_ENGINE_FIFO) the bit is 8.
    if cfg.pmc_pfifo_reset {
        let pmc_cur = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
        const PFIFO_BIT: u32 = 1 << 8;
        w(pmc::ENABLE, pmc_cur & !PFIFO_BIT)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
        w(pmc::ENABLE, pmc_cur | PFIFO_BIT)?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let rb = bar0.read_u32(pmc::ENABLE).unwrap_or(0xDEAD);
        tracing::info!(
            pmc_cur = format_args!("{pmc_cur:#010x}"),
            pmc_after = format_args!("{rb:#010x}"),
            "PMC PFIFO reset (bit 8)"
        );
    } else {
        tracing::info!("PMC PFIFO reset skipped (warm handoff)");
    }

    // Initialize PFIFO — verify the enable write takes effect.
    // On warm handoff, skip the 0→1 toggle: writing PFIFO_ENABLE=0
    // disrupts the running scheduler on GV100 (where the register
    // reads 0 even when functional). The preempt ACK liveness probe
    // is the authoritative check for PFIFO state.
    let pfifo_en = bar0.read_u32(pfifo::ENABLE).unwrap_or(0);
    if cfg.skip_pfifo_toggle {
        tracing::info!(
            pfifo_en = format_args!("{pfifo_en:#010x}"),
            "PFIFO toggle skipped (warm handoff — preserving scheduler state)"
        );
    } else {
        w(pfifo::ENABLE, 0)?;
        std::thread::sleep(std::time::Duration::from_millis(1));
        w(pfifo::ENABLE, 1)?;
        std::thread::sleep(std::time::Duration::from_millis(cfg.pfifo_settle_ms));
        let readback = bar0.read_u32(pfifo::ENABLE).unwrap_or(0xDEAD);

        if readback == 0 && cfg.retry_on_priv_fault {
            tracing::warn!("PFIFO_ENABLE=0 after first write — retrying with PRI fault re-clear");
            let priv_st = bar0.read_u32(pri::PRIV_RING_INTR_STATUS).unwrap_or(0);
            if priv_st != 0 {
                for _ in 0..5 {
                    w(pri::PRIV_RING_COMMAND, pri::PRIV_RING_CMD_ACK)?;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    if bar0.read_u32(pri::PRIV_RING_INTR_STATUS).unwrap_or(0) == 0 {
                        break;
                    }
                }
            }
            w(pfifo::ENABLE, 1)?;
            std::thread::sleep(std::time::Duration::from_millis(cfg.pfifo_settle_ms));
        }
        tracing::info!(
            pfifo_before = format_args!("{pfifo_en:#010x}"),
            pfifo_after = format_args!("{readback:#010x}"),
            "PFIFO enable"
        );
    }

    let r = |reg: usize| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

    // Preempt ALL active runlists to clear the scheduler's stale channel
    // table from nouveau's previous session. On warm handoff this is
    // SKIPPED: the preempt forces FECS to unload all channels; with none
    // remaining FECS disables the GR engine and halts permanently.
    if cfg.preempt_runlists {
        let cur_map = r(pfifo::PBDMA_MAP);
        let mut rl_mask: u32 = 0;
        let mut seq = 0_usize;
        for pid in 0..32_usize {
            if cur_map & (1 << pid) == 0 {
                continue;
            }
            let rl = r(0x2390 + seq * 4);
            if rl < 32 {
                rl_mask |= 1 << rl;
            }
            seq += 1;
        }
        if rl_mask != 0 {
            w(pfifo::INTR, 0xFFFF_FFFF)?;
            w(pfifo::GV100_PREEMPT, rl_mask)?;
            let mut got_ack = false;
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(2));
                let intr = r(pfifo::INTR);
                if intr & pfifo::INTR_RL_COMPLETE != 0 {
                    w(pfifo::INTR, pfifo::INTR_RL_COMPLETE)?;
                    got_ack = true;
                    break;
                }
            }
            tracing::info!(
                rl_mask = format_args!("{rl_mask:#010x}"),
                got_ack,
                "runlist preempt"
            );
        }
    } else {
        tracing::info!(
            "runlist preempt skipped (warm handoff — preserving FECS GR scheduling state)"
        );
    }

    // Force-clear PBDMA registers to remove nouveau's stale channel context.
    // This mirrors the diagnostic runner's Phase 4 — without it, PBDMAs may
    // attempt DMA fetches from nouveau's now-unmapped GPFIFO addresses.
    if cfg.pbdma_force_clear {
        let cur_map = r(pfifo::PBDMA_MAP);
        for pid in 0..32_usize {
            if cur_map & (1 << pid) == 0 {
                continue;
            }
            let b = 0x0004_0000 + pid * 0x2000;
            for off in (0x000..=0x1FC).step_by(4) {
                let _ = w(b + off, 0);
            }
            for off in [
                0x040, 0x044, 0x050, 0x054, 0x058, 0x0B0, 0x0D0, 0x0D4, 0x0C0, 0x13C,
            ] {
                let _ = w(b + off, 0);
            }
            let _ = w(b + 0x100, 0xFFFF_FFFF); // clear INTR_0
            let _ = w(b + 0x108, 0xFFFF_FFFF); // clear INTR_STALL
            let _ = w(b + 0x110, 0);
            let _ = w(b + 0x148, 0xFFFF_FFFF); // clear HCE_INTR
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        tracing::info!("PBDMA registers force-cleared");
    } else {
        // Warm handoff: don't nuke PBDMA register state, but DO clear
        // stale interrupt flags left by nouveau's teardown. Without this,
        // latched errors (GPPTR_INVALID, DEVICE, HCE) prevent the PBDMA
        // from scheduling our new channel after the runlist update.
        //
        // GV100 PBDMA interrupt registers (per-PBDMA, stride 0x2000):
        //   0x100: INTR_0     — primary interrupt status (W1C)
        //   0x108: INTR_STALL — stall interrupt status (W1C)
        //   0x148: HCE_INTR   — HCE interrupt status (W1C)
        // All three must be cleared; nouveau teardown can latch errors
        // in any of them.
        let cur_map = r(pfifo::PBDMA_MAP);
        for pid in 0..32_usize {
            if cur_map & (1 << pid) == 0 {
                continue;
            }
            let b = 0x0004_0000 + pid * 0x2000;
            let intr0 = bar0.read_u32(b + 0x100).unwrap_or(0);
            let intr_stall = bar0.read_u32(b + 0x108).unwrap_or(0);
            let hce_intr = bar0.read_u32(b + 0x148).unwrap_or(0);
            if pri::is_pri_error(intr0) {
                tracing::debug!(
                    pbdma = pid,
                    intr0 = format_args!("{intr0:#010x}"),
                    "warm handoff: PBDMA returns PRI error — skipping"
                );
                continue;
            }
            let any_set = intr0 != 0 || intr_stall != 0 || hce_intr != 0;
            if any_set {
                tracing::info!(
                    pbdma = pid,
                    intr0 = format_args!("{intr0:#010x}"),
                    intr_stall = format_args!("{intr_stall:#010x}"),
                    hce_intr = format_args!("{hce_intr:#010x}"),
                    "warm handoff: clearing stale PBDMA interrupts"
                );
                let _ = w(b + 0x100, 0xFFFF_FFFF);
                let _ = w(b + 0x108, 0xFFFF_FFFF);
                let _ = w(b + 0x148, 0xFFFF_FFFF);
            }
        }
        tracing::info!("PBDMA force-clear skipped (warm handoff), interrupts cleared");
    }

    // Discover PBDMAs and their runlist assignments.
    let pbdma_map = bar0.read_u32(pfifo::PBDMA_MAP).unwrap_or(0);
    if pbdma_map == 0 {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "no PBDMAs found in PBDMA_MAP (0x2004)",
        )));
    }

    let mut gr_runlist: Option<u32> = None;
    let mut ce_runlist: Option<u32> = None;
    let mut cur_type: u32 = 0xFFFF;
    let mut cur_runlist: u32 = 0xFFFF;
    for i in 0..64_u32 {
        let data = bar0.read_u32(0x0002_2700 + (i as usize) * 4).unwrap_or(0);
        if data == 0 {
            break;
        }
        let kind = data & 3;
        match kind {
            // GV100 PTOP_DEVICE_INFO_V2 format (nouveau gv100_top.c):
            // kind=1 (DATA): engine type at bits [7:2]
            // kind=2 (ENUM): runlist at bits [17:14], engine enum at bits [13:2]
            // kind=3 (ENGINE_DATA): reset2/fault2 (not runlist)
            1 => cur_type = (data >> 2) & 0x3F,
            2 => cur_runlist = (data >> 14) & 0xF,
            _ => {}
        }
        if data & (1 << 31) != 0 {
            let engine_name = match cur_type {
                0 => "GR",
                1 => "CE",
                2 => "NVDEC",
                3 => "SEC2",
                8 => "MSENC",
                _ => "unknown",
            };
            tracing::info!(
                engine_type = cur_type,
                engine_name,
                runlist = cur_runlist,
                entry = i,
                data = format_args!("{data:#010x}"),
                "engine topology entry"
            );
            if cur_type == 0 && gr_runlist.is_none() && cur_runlist != 0xFFFF {
                gr_runlist = Some(cur_runlist);
            }
            if cur_type == 1 && ce_runlist.is_none() && cur_runlist != 0xFFFF {
                ce_runlist = Some(cur_runlist);
            }
            cur_type = 0xFFFF;
            cur_runlist = 0xFFFF;
        }
    }

    if let Some(ce_rl) = ce_runlist {
        tracing::info!(ce_runlist = ce_rl, "CE (Copy Engine) runlist discovered");
    }

    let mut pbdma_ids: Vec<u32> = Vec::new();
    for pid in 0..32_u32 {
        if pbdma_map & (1 << pid) != 0 {
            pbdma_ids.push(pid);
        }
    }
    let mut pbdma_runlists: Vec<(u32, u32)> = Vec::new();
    for (seq, &pid) in pbdma_ids.iter().enumerate() {
        let rl = bar0.read_u32(0x0000_2390 + seq * 4).unwrap_or(0xFFFF);
        pbdma_runlists.push((pid, rl));
    }

    let target_runlist = gr_runlist.unwrap_or_else(|| pbdma_runlists.first().map_or(0, |e| e.1));

    tracing::info!(
        pbdma_map = format_args!("{pbdma_map:#010x}"),
        target_runlist,
        "PBDMA/runlist discovery"
    );

    // Per-PBDMA init (gk104_fifo_init_pbdmas + gk208_runq_init).
    for id in 0..32_usize {
        if pbdma_map & (1 << id) == 0 {
            continue;
        }
        let b = 0x0004_0000 + id * 0x2000;
        w(pbdma::intr(id), 0xFFFF_FFFF)?;
        w(pbdma::intr_en(id), 0xFFFF_FEFF)?;
        w(b + 0x13C, 0)?;
        w(pbdma::hce_intr(id), 0)?;
        w(pbdma::hce_intr_en(id), 0)?;
        w(b + 0x164, 0xFFFF_FFFF)?;
    }

    {
        let ck = bar0.read_u32(pfifo::ENABLE).unwrap_or(0xDEAD);
        tracing::debug!(pfifo_en = format_args!("{ck:#010x}"), "after PBDMA init");
    }

    // Clear + enable PFIFO interrupts and scheduler.
    w(pfifo::INTR, 0xFFFF_FFFF)?;
    w(pfifo::INTR_EN, 0x7FFF_FFFF)?;
    if cfg.use_sched_en {
        w(pfifo::SCHED_EN, 1)?;
    } else {
        w(pfifo::SCHED_DISABLE, 0)?;
    }

    {
        let ck = bar0.read_u32(pfifo::ENABLE).unwrap_or(0xDEAD);
        let intr = bar0.read_u32(pfifo::INTR).unwrap_or(0xDEAD);
        tracing::debug!(
            pfifo_en = format_args!("{ck:#010x}"),
            intr = format_args!("{intr:#010x}"),
            "after scheduler enable"
        );
    }

    // GV100 per-runlist registers at stride 0x10 — flush with count=0.
    // On warm handoff this is SKIPPED: the empty flush tells FECS "no
    // channels on GR runlist" which causes FECS to disable the GR engine
    // and halt. Our channel submit replaces the runlist immediately after,
    // but FECS won't wake to process it. The preempt above already cleared
    // stale channels; our submit_runlist() will overwrite the runlist.
    if cfg.flush_empty_runlists {
        let mut flushed_runlists = std::collections::HashSet::new();
        let rl_base_val = pfifo::gv100_runlist_base_value(RUNLIST_IOVA);
        let rl_submit_val = pfifo::gv100_runlist_submit_value(RUNLIST_IOVA, 0);
        for &(_, rl) in &pbdma_runlists {
            if rl > 31 || !flushed_runlists.insert(rl) {
                continue;
            }
            w(pfifo::runlist_base(rl), rl_base_val)?;
            w(pfifo::runlist_submit(rl), rl_submit_val)?;
            std::thread::sleep(std::time::Duration::from_millis(10));
            let intr = bar0.read_u32(pfifo::INTR).unwrap_or(0);
            if intr & 0x4000_0000 != 0 {
                let _ = bar0.read_u32(pfifo::RUNLIST_ACK);
                w(pfifo::RUNLIST_ACK, 1u32 << rl)?;
                w(pfifo::INTR, 0x4000_0000)?;
                tracing::debug!(runlist = rl, "ACK'd empty runlist completion");
            }
            tracing::debug!(runlist = rl, "flushed runlist (empty, GV100 per-RL)");
        }
    } else {
        tracing::info!(
            "empty runlist flush skipped (warm handoff — preserving FECS GR scheduling state)"
        );
    }
    if cfg.post_flush_settle_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(cfg.post_flush_settle_ms));
    }

    // Confirm GR runlist via ENGN0_STATUS register.
    let engn0 = bar0.read_u32(0x0000_2640).unwrap_or(0);
    let engn0_runlist = (engn0 >> 12) & 0xF;
    if gr_runlist.is_none() && engn0_runlist <= 31 {
        gr_runlist = Some(engn0_runlist);
    }
    let target_runlist = gr_runlist.unwrap_or_else(|| pbdma_runlists.first().map_or(0, |e| e.1));

    let runq: u32 = 0;
    tracing::info!(target_runlist, runq, "PFIFO engine initialized");
    Ok((runq, target_runlist))
}
