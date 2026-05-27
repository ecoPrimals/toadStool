// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO engine initialization and diagnostic readback for Volta+ GPUs.
//!
//! Implements the engine bring-up sequence from nouveau's `gk104_fifo_init()`,
//! `gk104_fifo_init_pbdmas()`, `gf100_runq_init()`, and `gk208_runq_init()`.

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;

use super::registers::*;

/// Behavioral knobs for the PFIFO bring-up sequence.
///
/// Differences between the `VfioChannel::create` path and the diagnostic
/// runner are expressed here rather than as code forks.
#[derive(Debug, Clone)]
pub struct PfifoInitConfig {
    /// Clear PRIV_RING faults (5× ACK retry) before touching engine regs.
    /// Needed after driver swap (nouveau → vfio); skippable on warm GPU.
    pub clear_priv_ring: bool,
    /// Write `0xFFFF_FFFF` to `PMC_ENABLE` to un-gate all engines.
    /// Diagnostic runner may prefer to preserve nouveau's PMC state.
    pub pmc_glow_plug: bool,
    /// Milliseconds to wait after `PFIFO_ENABLE = 1`.
    pub pfifo_settle_ms: u64,
    /// Re-clear PRIV_RING and retry if `PFIFO_ENABLE` reads back 0.
    pub retry_on_priv_fault: bool,
    /// Reset PFIFO via PMC_ENABLE bit 8 toggle. Set false for warm
    /// handoff to preserve the PFIFO scheduler state from nouveau.
    pub pmc_pfifo_reset: bool,
    /// Force-clear all PBDMA registers to remove stale channel state.
    /// Set false for warm handoff to preserve PBDMA configuration.
    pub pbdma_force_clear: bool,
    /// Flush all runlists with count=0 during init. On warm handoff,
    /// skip this: the empty flush tells FECS "no channels" which causes
    /// it to disable GR and halt. Our channel submit replaces the
    /// runlist immediately after, but FECS won't wake to process it.
    pub flush_empty_runlists: bool,
    /// Preempt all active runlists during init to clear stale channel
    /// state. On warm handoff, skip this: the preempt forces FECS to
    /// unload all channels; with none remaining FECS disables GR.
    pub preempt_runlists: bool,
    /// `true` → write `SCHED_EN (0x2504) = 1`; `false` → write `SCHED_DISABLE (0x2630) = 0`.
    pub use_sched_en: bool,
    /// Milliseconds to wait after empty-runlist flush.
    pub post_flush_settle_ms: u64,
    /// Skip the PFIFO_ENABLE 0→1 toggle. On GV100, writing PFIFO_ENABLE=0
    /// during warm handoff disrupts the running PFIFO scheduler state even
    /// though the register reads 0 on this generation. Preserving the
    /// existing state allows PBDMA to continue servicing channels.
    pub skip_pfifo_toggle: bool,
}

impl Default for PfifoInitConfig {
    /// Standard init for `VfioChannel::create` — aggressive fault clearing,
    /// full glow plug, long settle, retry.
    fn default() -> Self {
        Self {
            clear_priv_ring: true,
            pmc_glow_plug: true,
            pfifo_settle_ms: 50,
            retry_on_priv_fault: true,
            pmc_pfifo_reset: true,
            pbdma_force_clear: true,
            flush_empty_runlists: true,
            preempt_runlists: true,
            use_sched_en: true,
            post_flush_settle_ms: 20,
            skip_pfifo_toggle: false,
        }
    }
}

impl PfifoInitConfig {
    /// Config for the diagnostic runner — lighter touch, preserves
    /// nouveau's warm state, shorter settle.
    #[must_use]
    pub fn diagnostic() -> Self {
        Self {
            clear_priv_ring: false,
            pmc_glow_plug: false,
            pfifo_settle_ms: 10,
            retry_on_priv_fault: false,
            pmc_pfifo_reset: false,
            pbdma_force_clear: false,
            flush_empty_runlists: false,
            preempt_runlists: false,
            use_sched_en: false,
            post_flush_settle_ms: 0,
            skip_pfifo_toggle: true,
        }
    }

    /// Config for warm handoff from nouveau — preserves FECS/GPCCS state
    /// but resets PFIFO scheduler to clear stale channel mappings.
    ///
    /// PMC_ENABLE bit 8 (HOST/PFIFO) is toggled to reset the scheduler
    /// and PBDMAs. This does NOT affect FECS/GPCCS which are in the GR
    /// engine (separate PMC bit). Without this reset, the scheduler's
    /// internal channel-to-PBDMA mappings remain from nouveau's session,
    /// and our runlist submission + PBDMA programming fails to activate
    /// the channel (ch_state stays 0, GP_GET never advances).
    ///
    /// `pbdma_force_clear` is FALSE: the PMC PFIFO reset already clears
    /// PBDMA hardware state. The destructive force-clear (writing 0 to
    /// all PBDMA registers 0x000..0x1FC) creates invalid GPFIFO pointers
    /// that latch persistent INTR_0 errors (GPPTR_INVALID, GPENTRY_INVALID,
    /// DEVICE). These latched errors cause the scheduler to refuse loading
    /// channels on the errored PBDMA, leaving channels stuck in PENDING.
    /// Stale interrupt flags are cleared separately via the non-force path.
    #[must_use]
    pub fn warm_handoff() -> Self {
        Self {
            clear_priv_ring: true,
            pmc_glow_plug: false,
            pfifo_settle_ms: 10,
            retry_on_priv_fault: true,
            pmc_pfifo_reset: true,
            pbdma_force_clear: false,
            flush_empty_runlists: false,
            preempt_runlists: false,
            use_sched_en: true,
            post_flush_settle_ms: 10,
            skip_pfifo_toggle: false,
        }
    }

    /// Warm handoff with FECS preservation — skips PMC PFIFO reset and PFIFO
    /// toggle because both cascade into the GR engine and force FECS back to
    /// HRESET on Volta. Use after a successful FECS HS boot.
    #[must_use]
    pub fn warm_fecs_alive() -> Self {
        Self {
            clear_priv_ring: true,
            pmc_glow_plug: false,
            pfifo_settle_ms: 10,
            retry_on_priv_fault: true,
            pmc_pfifo_reset: false,
            pbdma_force_clear: false,
            flush_empty_runlists: false,
            preempt_runlists: false,
            use_sched_en: true,
            post_flush_settle_ms: 10,
            skip_pfifo_toggle: true,
        }
    }

    /// Unified config selection based on GPU thermal state.
    ///
    /// Cold GPUs get full aggressive init. Warm GPUs with confirmed FECS
    /// preservation get the gentlest path; other warm states use standard
    /// warm handoff.
    #[must_use]
    pub fn for_thermal_state(warm: bool, fecs_preserved: bool) -> Self {
        if !warm {
            Self::default()
        } else if fecs_preserved {
            Self::warm_fecs_alive()
        } else {
            Self::warm_handoff()
        }
    }
}

/// Discover the CE (Copy Engine) runlist ID from the engine topology table.
///
/// Returns `Some(runlist_id)` if a CE engine is found, `None` otherwise.
/// This is independent of the GR runlist used for compute dispatch.
///
/// Uses the GV100 PTOP_DEVICE_INFO_V2 format:
/// - kind=1 (DATA): engine type at bits [7:2]
/// - kind=2 (ENUM): runlist at bits [17:14]
/// - bit 31: CHAIN (end of this engine's record)
pub fn discover_ce_runlist(bar0: &MappedBar) -> Option<u32> {
    let mut cur_type: u32 = 0xFFFF;
    let mut cur_runlist: u32 = 0xFFFF;
    for i in 0..64_u32 {
        let data = bar0.read_u32(0x0002_2700 + (i as usize) * 4).unwrap_or(0);
        if data == 0 {
            break;
        }
        let kind = data & 3;
        match kind {
            1 => cur_type = (data >> 2) & 0x3F,
            2 => cur_runlist = (data >> 14) & 0xF,
            _ => {}
        }
        if data & (1 << 31) != 0 {
            if cur_type == 1 && cur_runlist != 0xFFFF {
                return Some(cur_runlist);
            }
            cur_type = 0xFFFF;
            cur_runlist = 0xFFFF;
        }
    }
    None
}

/// Find the PBDMA that serves a given runlist ID.
///
/// On GV100, `RUNLIST_PBDMA_MAP(i)` at `0x2390 + i*4` (indexed by runlist ID)
/// contains a bitmask of PBDMAs that can service that runlist. Returns the
/// lowest-numbered PBDMA from the mask.
pub fn find_pbdma_for_runlist(bar0: &MappedBar, target_runlist: u32) -> Option<usize> {
    if target_runlist > 31 {
        return None;
    }
    let pbdma_mask = bar0.read_u32(0x0000_2390 + (target_runlist as usize) * 4).unwrap_or(0);
    if pbdma_mask == 0 || pbdma_mask > 0x00FF_FFFF {
        return None;
    }
    Some(pbdma_mask.trailing_zeros() as usize)
}

/// Enable the PFIFO engine in PMC, discover PBDMAs, and initialize.
///
/// Returns the RUNQ selector (0-based index into the PBDMAs serving
/// runlist 0) and the target runlist ID.
///
/// After VFIO FLR the GPU's engine clock domains are gated — PFIFO
/// registers read `0xBAD0_DA00`. We must enable the engine in
/// `NV_PMC_ENABLE` first, matching nouveau's `gp100_mc_init()`.
///
/// # Errors
///
/// Returns error if BAR0 reads indicate D3hot or no PBDMAs are found.
#[expect(
    dead_code,
    reason = "default PFIFO init — used when channel creation uses default config"
)]
pub(super) fn init_pfifo_engine(bar0: &MappedBar) -> DriverResult<(u32, u32)> {
    init_pfifo_engine_with(bar0, &PfifoInitConfig::default())
}

/// Configurable PFIFO engine initialization.
///
/// Same as [`init_pfifo_engine`] but takes a [`PfifoInitConfig`] to
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

/// Kepler (GK104/GK110) PFIFO engine initialization.
///
/// GK104+ PFIFO init following nouveau's `gk104_fifo_init()`.
///
/// On GK104+, PBDMA count comes from `PMC_SUBDEV_ENABLE` (0x204), not
/// the `PFIFO_PBDMA_MAP` register (which is unreliable on warm handoff).
/// Uses GK104 global runlist base/submit. Returns `(runq, runlist_id)`.
pub fn init_pfifo_engine_kepler(
    guard: &crate::nv::hardware_guard::GuardedBar<'_>,
) -> DriverResult<(u32, u32)> {
    let gw = |reg: u32, val: u32| {
        guard.write_u32(reg, val).map_err(|refusal| {
            DriverError::SubmitFailed(Cow::Owned(format!("PFIFO init {reg:#x}: {refusal}")))
        })
    };

    let boot0 = guard.read_u32(0).unwrap_or(0);
    if boot0 == 0xFFFF_FFFF {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "BAR0 returns 0xFFFFFFFF — GPU in D3hot",
        )));
    }
    tracing::info!(
        boot0 = format_args!("{boot0:#010x}"),
        "Kepler PFIFO init start"
    );

    // Clear PRIV_RING faults
    let priv_intr = guard
        .read_u32(pri::PRIV_RING_INTR_STATUS as u32)
        .unwrap_or(0);
    if priv_intr != 0 {
        for _ in 0..5 {
            gw(pri::PRIV_RING_COMMAND as u32, pri::PRIV_RING_CMD_ACK)?;
            std::thread::sleep(std::time::Duration::from_millis(20));
            if guard
                .read_u32(pri::PRIV_RING_INTR_STATUS as u32)
                .unwrap_or(0)
                == 0
            {
                break;
            }
        }
    }

    // Check PFIFO domain using INTR register (0x2100) — a register that
    // exists on all Kepler+ GPUs. Do NOT use PBDMA_MAP (0x2004) which is
    // GV100+ only and always PRI-faults on Kepler.
    {
        let pfifo_intr_pre = guard.read_u32(pfifo::INTR as u32).unwrap_or(0xDEAD);
        let pbdma0_intr = guard.read_u32(pbdma::intr(0) as u32).unwrap_or(0xDEAD);
        let pfifo_faulted = pri::is_pri_error(pfifo_intr_pre) || pfifo_intr_pre == 0xDEAD_DEAD;
        tracing::info!(
            pfifo_intr = format_args!("{pfifo_intr_pre:#010x}"),
            pbdma0_intr = format_args!("{pbdma0_intr:#010x}"),
            faulted = pfifo_faulted,
            "Kepler PFIFO domain check (via 0x2100, NOT 0x2004)"
        );
    }

    // On GK210B, the PFIFO scheduler sub-block registers (0x2004, 0x2204-0x2253,
    // 0x22C0, 0x2300, 0x2504, 0x2600) are permanently PRI-faulted after VFIO
    // legacy bind. No combination of PMC resets, PRI ring re-init, or PBUS
    // resets brings them online.
    //
    // However, key registers ARE accessible:
    //   - 0x2270/0x2274: GK104 runlist base/submit (WORKS!)
    //   - 0x2390+seq*4:  PBDMA→runlist assignment table (read-only, WORKS!)
    //   - 0x252C/0x254C: BIND_ERROR/SCHED_ERROR (WORKS!)
    //   - 0x040000+:     PBDMA registers (WORKS!)
    //
    // Strategy: read the hardware's existing PBDMA→runlist assignment from
    // 0x2390 (left by Nouveau) and use that runlist ID for our submission.
    // Skip writing to the PRI-faulted registers entirely.

    // Discover PBDMA count from PMC subdevice enable (0x204).
    let pbdma_en = guard.read_u32(pmc::PBDMA_ENABLE as u32).unwrap_or(0);
    let pbdma_nr = pbdma_en.count_ones();
    if pbdma_nr == 0 {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "no PBDMAs enabled in PMC_PBDMA_ENABLE (0x204)",
        )));
    }

    // Re-enable PBDMAs (idempotent if already set by nouveau).
    gw(pmc::PBDMA_ENABLE as u32, (1u32 << pbdma_nr) - 1)?;

    // Read the hardware's PBDMA→runlist assignment table at 0x2390+seq*4.
    // This table IS accessible on GK210B (unlike 0x2600 which PRI-faults).
    // Nouveau populated it during init; we reuse whatever mapping exists.
    let mut gr_runlist_id: Option<u32> = None;
    for seq in 0..pbdma_nr {
        let rl = guard.read_u32(0x2390 + seq * 4).unwrap_or(0xFFFF);
        tracing::info!(seq, runlist = rl, "PBDMA→runlist assignment (0x2390)");
        // Runlist IDs > 31 are garbage from stale/uninitialized state.
        // Take the first valid runlist ID as our GR runlist.
        if gr_runlist_id.is_none() && rl < 32 {
            gr_runlist_id = Some(rl);
        }
    }

    let target_runlist = gr_runlist_id.unwrap_or(0);
    tracing::info!(
        pbdma_en = format_args!("{pbdma_en:#010x}"),
        pbdma_nr,
        target_runlist,
        "Kepler PBDMA discovery: using runlist from hw assignment table"
    );

    // Configure PBDMAs (nouveau gk104_fifo_init pattern).
    // Clear stale channel context from Nouveau — scheduler fails (code 32)
    // if PBDMAs still have an old channel loaded.
    for id in 0..pbdma_nr as usize {
        let b = (0x040000 + id * 0x2000) as u32;
        let pbdma_ctrl = b + 0x13C;
        let ctrl_val = guard.read_u32(pbdma_ctrl).unwrap_or(0);
        gw(pbdma_ctrl, ctrl_val & !0x1000_0100)?;

        // Clear stale GP_BASE/PUT/GET, USERD, STATE, SIGNATURE
        gw(b + 0x040, 0)?; // GP_BASE_LO
        gw(b + 0x044, 0)?; // GP_BASE_HI
        gw(b + 0x054, 0)?; // GP_PUT
        gw(b + 0x058, 0)?; // GP_GET
        gw(b + 0x0B0, 0)?; // STATE
        gw(b + 0x0D0, 0)?; // USERD_LO
        gw(b + 0x0D4, 0)?; // USERD_HI
        gw(b + 0x0C0, 0)?; // SIGNATURE

        gw(pbdma::intr(id) as u32, 0xFFFF_FFFF)?;
        gw(pbdma::intr_en(id) as u32, 0xFFFF_FEFF)?;
    }

    // Skip 0x2600 writes — PRI-faulted on GK210B. The hardware's existing
    // PBDMA→runlist assignment (from 0x2390) is used instead.

    // Clear PFIFO interrupts. Skip 0x2200 write (doesn't stick on GK210B).
    gw(pfifo::INTR as u32, 0xFFFF_FFFF)?;

    // Try to enable PFIFO caches (Nouveau: nvkm_mask 0x2200, 1, 1).
    // On GK210B this may not stick, but write it anyway for correctness.
    let _ = gw(pfifo::ENABLE as u32, 1);

    let pfifo_en = guard.read_u32(pfifo::ENABLE as u32).unwrap_or(0xDEAD);
    let rl_base = guard.read_u32(0x2270).unwrap_or(0xDEAD);
    tracing::info!(
        pfifo_en = format_args!("{pfifo_en:#010x}"),
        rl_base = format_args!("{rl_base:#010x}"),
        pbdma_nr,
        target_runlist,
        "Kepler PFIFO engine initialized (using hw runlist assignment)"
    );
    Ok((0, target_runlist))
}

/// Read back PFIFO/PBDMA/PCCSR state for diagnostics.
pub(super) fn log_pfifo_diagnostics(bar0: &MappedBar) {
    let r = |reg: usize| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

    let pfifo_intr = r(pfifo::INTR);
    let pfifo_en = r(pfifo::INTR_EN);
    let sched = r(pfifo::SCHED_EN);
    let pccsr_inst = r(pccsr::inst(0));
    let pccsr_chan = r(pccsr::channel(0));
    let pbdma0_intr = r(pbdma::intr(0));
    let pbdma0_hce = r(pbdma::hce_intr(0));
    let pbdma1_intr = r(pbdma::intr(1));
    let engn0_status = r(0x0000_2640);
    let pbdma0_idle = r(0x0000_3080);
    let pbdma1_idle = r(0x0000_3084);
    let rl0_info = r(0x0000_2284);
    let pmc_enable = r(0x0000_0200);
    let bind_err = r(0x0000_252C);
    let sched_dis = r(0x0000_2630);
    let preempt = r(0x0000_2634);
    let runl_submit_info = r(0x0000_2270);
    let doorbell_test = r(0x0081_0090);
    let pbdma_map = r(0x0000_2004);

    tracing::debug!(
        pmc_enable = format_args!("{pmc_enable:#010x}"),
        sched = format_args!("{sched:#010x}"),
        sched_dis = format_args!("{sched_dis:#010x}"),
        preempt = format_args!("{preempt:#010x}"),
        pfifo_intr = format_args!("{pfifo_intr:#010x}"),
        pfifo_en = format_args!("{pfifo_en:#010x}"),
        pccsr_inst = format_args!("{pccsr_inst:#010x}"),
        pccsr_chan = format_args!("{pccsr_chan:#010x}"),
        pbdma0_intr = format_args!("{pbdma0_intr:#010x}"),
        pbdma0_hce = format_args!("{pbdma0_hce:#010x}"),
        pbdma1_intr = format_args!("{pbdma1_intr:#010x}"),
        pbdma0_idle = format_args!("{pbdma0_idle:#010x}"),
        pbdma1_idle = format_args!("{pbdma1_idle:#010x}"),
        engn0_status = format_args!("{engn0_status:#010x}"),
        rl0_info = format_args!("{rl0_info:#010x}"),
        bind_err = format_args!("{bind_err:#010x}"),
        runl_submit_info = format_args!("{runl_submit_info:#010x}"),
        doorbell_test = format_args!("{doorbell_test:#010x}"),
        pbdma_map = format_args!("{pbdma_map:#010x}"),
        "PFIFO diagnostics"
    );

    let mut seq = 0_usize;
    for pid in 0..32_usize {
        if pbdma_map & (1 << pid) == 0 {
            continue;
        }
        let b = 0x040000 + pid * 0x2000;
        let rl_assign = r(0x2390 + seq * 4);
        tracing::debug!(
            pbdma = pid,
            seq,
            runlist = rl_assign,
            gp_base_hi = format_args!("{:#010x}", r(b + 0x44)),
            gp_base_lo = format_args!("{:#010x}", r(b + 0x40)),
            gp_put = format_args!("{:#010x}", r(b + 0x54)),
            gp_fetch = format_args!("{:#010x}", r(b + 0x48)),
            userd_hi = format_args!("{:#010x}", r(b + 0xD4)),
            userd_lo = format_args!("{:#010x}", r(b + 0xD0)),
            "PBDMA state"
        );
        seq += 1;
    }
}

use crate::vfio::dma::DmaBuffer;
use crate::vfio::device::DmaBackend;

use super::mmu;
use super::page_tables;
use super::registers::{self, falcon, pbdma, pccsr, ramfc};
use super::VfioChannel;

impl VfioChannel {
    pub(super) fn create_with_config(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
        pfifo_cfg: &PfifoInitConfig,
    ) -> DriverResult<Self> {
        // Guard pages: IOVA 0x0000–0x2FFF. Stale PBDMAs (from nouveau's old
        // channels) may issue DMA reads to IOVA 0x0 when their instance pointer
        // is zero. The RAMIN PDB at offset 0x200 triggers an IO_PAGE_FAULT at
        // exactly IOVA 0x200. Mapping zeroed guard pages lets these reads
        // succeed harmlessly instead of wedging the PBDMA via IOMMU faults.
        let guard_0 = DmaBuffer::new(container.clone(), 4096, 0x0000)?;
        let guard_1 = DmaBuffer::new(container.clone(), 4096, 0x1000)?;
        let guard_2 = DmaBuffer::new(container.clone(), 4096, 0x2000)?;
        let guard_pages = vec![guard_0, guard_1, guard_2];
        tracing::info!("IOMMU guard pages mapped at IOVA 0x0000–0x2FFF");

        let instance = DmaBuffer::new(container.clone(), 4096, INSTANCE_IOVA)?;
        let runlist = DmaBuffer::new(container.clone(), 4096, RUNLIST_IOVA)?;
        let pd3 = DmaBuffer::new(container.clone(), 4096, PD3_IOVA)?;
        let pd2 = DmaBuffer::new(container.clone(), 4096, PD2_IOVA)?;
        let pd1 = DmaBuffer::new(container.clone(), 4096, PD1_IOVA)?;
        let pd0 = DmaBuffer::new(container.clone(), 4096, PD0_IOVA)?;
        let pt0 = DmaBuffer::new(container.clone(), 4096, PT0_IOVA)?;
        let fault_buf = DmaBuffer::new(container.clone(), 4096, FAULT_BUF_IOVA)?;

        let mut chan = Self {
            instance,
            runlist,
            pd3,
            pd2,
            pd1,
            pd0,
            pt0,
            fault_buf,
            guard_pages,
            channel_id,
            runlist_id: 0,
        };

        let pfifo_trace = |bar0: &MappedBar, label: &str| {
            let en = bar0.read_u32(registers::pfifo::ENABLE).unwrap_or(0xDEAD);
            let intr = bar0.read_u32(registers::pfifo::INTR).unwrap_or(0xDEAD);
            tracing::debug!(
                en = format_args!("{en:#010x}"),
                intr = format_args!("{intr:#010x}"),
                "{label}"
            );
        };

        let fecs_probe = |bar0: &MappedBar, label: &str| {
            let ctl = bar0.read_u32(registers::falcon::FECS_BASE + registers::falcon::CPUCTL).unwrap_or(0xDEAD);
            let ctl_alias = bar0.read_u32(registers::falcon::FECS_BASE + registers::falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let pc = bar0.read_u32(registers::falcon::FECS_BASE + registers::falcon::PC).unwrap_or(0xDEAD);
            tracing::info!(
                cpuctl = format_args!("{ctl:#010x}"),
                cpuctl_alias = format_args!("{ctl_alias:#010x}"),
                pc = format_args!("{pc:#010x}"),
                "FECS probe: {label}"
            );
        };

        fecs_probe(bar0, "before-pfifo-init");

        let (runq, discovered_runlist_id) = init_pfifo_engine_with(bar0, pfifo_cfg)?;
        chan.runlist_id = discovered_runlist_id;
        pfifo_trace(bar0, "after-pfifo-init");
        fecs_probe(bar0, "after-pfifo-init");

        // Configure BAR2 in PHYSICAL mode targeting system memory.
        // The VRAM-based BAR2 setup (VIRTUAL mode) fails on cold VFIO cards
        // because VRAM is not initialized. PHYSICAL mode bypasses page tables
        // and gives PFIFO a direct path to system memory via PCIe+IOMMU.
        {
            let bar2_val: u32 = 2 << 28; // target=COH, mode=PHYSICAL, ptr=0
            bar0.write_u32(registers::misc::PBUS_BAR2_BLOCK, bar2_val)
                .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("BAR2_BLOCK: {e}"))))?;
            std::thread::sleep(std::time::Duration::from_millis(5));
            tracing::info!(
                bar2_block = format_args!("{bar2_val:#010x}"),
                "BAR2 set to PHYSICAL mode (SYS_MEM_COH)"
            );
        }
        pfifo_trace(bar0, "after-bar2-setup");
        fecs_probe(bar0, "after-bar2-setup");

        // Volta requires non-replayable fault buffers configured before any
        // MMU translation can succeed. Without them, FBHUB stalls on the
        // first fault entry (nowhere to write it) and subsequent PBUS reads
        // return 0xbad00200. This was the Layer 6 MMU blocker.
        mmu::configure_fault_buffers(bar0)?;
        pfifo_trace(bar0, "after-fault-buf-setup");
        fecs_probe(bar0, "after-fault-buf-setup");

        page_tables::populate_page_tables(
            chan.pd3.as_mut_slice(),
            chan.pd2.as_mut_slice(),
            chan.pd1.as_mut_slice(),
            chan.pd0.as_mut_slice(),
            chan.pt0.as_mut_slice(),
        );
        page_tables::populate_instance_block(
            chan.instance.as_mut_slice(),
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
        );
        page_tables::populate_runlist(
            chan.runlist.as_mut_slice(),
            userd_iova,
            channel_id,
            INSTANCE_IOVA,
            runq,
        );

        fecs_probe(bar0, "after-page-table-populate");

        Self::invalidate_tlb(bar0, PD3_IOVA)?;
        pfifo_trace(bar0, "after-tlb-invalidate");
        fecs_probe(bar0, "after-tlb-invalidate");

        // Clear stale PCCSR state from prior driver (nouveau residue).
        let stale = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
        if stale != 0 {
            Self::clear_stale_pccsr(bar0, channel_id, stale)?;
        }
        pfifo_trace(bar0, "after-clear-pccsr");

        // Clear stale PBDMA interrupts from prior driver without wiping
        // PBDMA config registers. nouveau leaves interrupt state that
        // blocks new GPFIFO processing after warm handoff.
        {
            let pbdma_map = bar0.read_u32(registers::pfifo::PBDMA_MAP).unwrap_or(0);
            for pid in 0..32u32 {
                if pbdma_map & (1 << pid) == 0 {
                    continue;
                }
                let intr_reg = 0x0004_0000 + (pid as usize) * 0x2000 + 0x108;
                let intr_val = bar0.read_u32(intr_reg).unwrap_or(0);
                if registers::pri::is_pri_error(intr_val) {
                    continue;
                }
                if intr_val != 0 {
                    let _ = bar0.write_u32(intr_reg, 0xFFFF_FFFF);
                    tracing::debug!(
                        pbdma = pid,
                        intr = format_args!("{intr_val:#010x}"),
                        "cleared stale PBDMA interrupt"
                    );
                }
            }
        }
        pfifo_trace(bar0, "after-clear-pbdma-intr");

        // Clear stale PFIFO-level interrupts as well.
        {
            let pfifo_intr = bar0.read_u32(registers::pfifo::INTR).unwrap_or(0);
            if pfifo_intr != 0 {
                let _ = bar0.write_u32(registers::pfifo::INTR, pfifo_intr);
                tracing::debug!(
                    intr = format_args!("{pfifo_intr:#010x}"),
                    "cleared stale PFIFO interrupt"
                );
            }
        }
        pfifo_trace(bar0, "after-clear-pfifo-intr");

        // On warm-caught GV100, FECS is typically in HARD RESET (CPUCTL bit 4)
        // after the PCI FLR during vfio-pci rebind. The GR runlist scheduler
        // requires FECS to complete context loads; without a running FECS,
        // channels get stuck in PENDING_CTX_RELOAD.
        //
        // Attempt to restart FECS: firmware should still be in IMEM from
        // nouveau's ACR load (FLR resets control registers but preserves SRAM).
        // Use CPUCTL_ALIAS (0x130) for HS-mode falcons (Volta+ v5).
        // Check FECS state via CPUCTL_ALIAS (0x130) — the host-accessible
        // register on Volta HS falcons. CPUCTL at 0x100 is security-locked
        // and always reads HRESET=1 regardless of actual falcon state.
        {
            let fecs_base = falcon::FECS_BASE;
            let cpuctl_alias = bar0.read_u32(fecs_base + falcon::CPUCTL_ALIAS).unwrap_or(0xDEAD);
            let pc = bar0.read_u32(fecs_base + falcon::PC).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(fecs_base + falcon::MAILBOX0).unwrap_or(0);
            let in_hreset = cpuctl_alias & falcon::CPUCTL_HRESET != 0;
            let halted = cpuctl_alias & falcon::CPUCTL_HALTED != 0;
            let alive = !in_hreset && !halted;

            tracing::info!(
                cpuctl_alias = format_args!("{cpuctl_alias:#010x}"),
                pc = format_args!("{pc:#010x}"),
                mb0 = format_args!("{mb0:#010x}"),
                alive,
                "FECS state before channel bind (via CPUCTL_ALIAS)"
            );

            if !alive {
                tracing::warn!(
                    cpuctl_alias = format_args!("{cpuctl_alias:#010x}"),
                    "FECS not running — GR scheduling may fail"
                );
            }
        }

        chan.bind_channel(bar0)?;
        pfifo_trace(bar0, "after-bind-channel");

        std::thread::sleep(std::time::Duration::from_millis(5));
        chan.clear_channel_faults(bar0)?;
        pfifo_trace(bar0, "after-clear-faults");

        chan.enable_channel(bar0)?;
        pfifo_trace(bar0, "after-enable-channel");

        // On warm-caught GV100, the scheduler's internal state still maps
        // PBDMAs to nouveau's old channels. A preempt forces the scheduler
        // to unload those stale mappings, then our runlist submission loads
        // our channel fresh. This preempt→submit cycle is safe because
        // it only affects PFIFO scheduling, not FECS/GPCCS falcons.
        {
            let w = |reg, val| bar0.write_u32(reg, val).ok();
            w(registers::pfifo::INTR, 0xFFFF_FFFF);
            w(registers::pfifo::GV100_PREEMPT, 1u32 << chan.runlist_id);
            for _ in 0..25 {
                std::thread::sleep(std::time::Duration::from_millis(2));
                let intr = bar0.read_u32(registers::pfifo::INTR).unwrap_or(0);
                if intr & registers::pfifo::INTR_RL_COMPLETE != 0 {
                    w(registers::pfifo::INTR, registers::pfifo::INTR_RL_COMPLETE);
                    tracing::info!("pre-submit preempt ACK received — old channels unloaded");
                    break;
                }
            }
        }
        pfifo_trace(bar0, "after-preempt-old-channels");

        chan.submit_runlist(bar0)?;
        pfifo_trace(bar0, "after-submit-runlist");

        // Wait for runlist completion and poll PCCSR to see if the scheduler
        // loaded our channel onto the PBDMA. On GV100, GR runlist scheduling
        // involves FECS; if FECS is halted, the channel gets stuck in
        // PENDING_CTX_RELOAD (STATUS=1).
        let mut scheduler_loaded = false;
        {
            let w = |reg, val| bar0.write_u32(reg, val).ok();
            w(registers::pfifo::INTR, registers::pfifo::INTR_RL_COMPLETE);
            for tick in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(5));
                let pccsr_val = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
                let status = pccsr::status(pccsr_val);
                if status >= 5 {
                    tracing::info!(
                        tick,
                        status,
                        pccsr = format_args!("{pccsr_val:#010x}"),
                        "scheduler loaded channel onto PBDMA (STATUS >= ON_PBDMA)"
                    );
                    scheduler_loaded = true;
                    break;
                }
                if tick % 10 == 9 {
                    tracing::debug!(
                        tick,
                        status,
                        pccsr = format_args!("{pccsr_val:#010x}"),
                        "waiting for scheduler context load"
                    );
                }
            }
            if !scheduler_loaded {
                let pccsr_val = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
                let status = pccsr::status(pccsr_val);
                tracing::info!(
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "scheduler did not load channel — will force-program PBDMA"
                );
            }
        }
        pfifo_trace(bar0, "after-scheduler-poll");

        // Discover the target PBDMA for this runlist.
        let target_pbdma = {
            let pbdma_map = bar0.read_u32(registers::pfifo::PBDMA_MAP).unwrap_or(0);
            let mut found: Option<usize> = None;
            let mut seq = 0_usize;
            for pid in 0..32_usize {
                if pbdma_map & (1 << pid) == 0 {
                    continue;
                }
                let rl = bar0.read_u32(0x2390 + seq * 4).unwrap_or(0xFFFF);
                if rl == chan.runlist_id {
                    found = Some(pid);
                    break;
                }
                seq += 1;
            }
            found
        };

        if !scheduler_loaded {
            if let Some(pid) = target_pbdma {
                // If the channel is stuck in PENDING (status 1) or
                // PEND_CTX_RELOAD (status 2), the scheduler is mid-load.
                // Preempt to cancel it before force-programming the PBDMA.
                let pccsr_val = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
                let status = pccsr::status(pccsr_val);
                if status == 1 || status == 2 {
                    tracing::info!(
                        status,
                        status_name = pccsr::status_name(pccsr_val),
                        "channel stuck in pending state — preempting to cancel"
                    );
                    let w = |reg, val| bar0.write_u32(reg, val).ok();
                    w(registers::pfifo::INTR, 0xFFFF_FFFF);
                    w(registers::pfifo::GV100_PREEMPT, 1u32 << chan.runlist_id);
                    for _ in 0..25 {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        let intr = bar0.read_u32(registers::pfifo::INTR).unwrap_or(0);
                        if intr & registers::pfifo::INTR_RL_COMPLETE != 0 {
                            w(registers::pfifo::INTR, registers::pfifo::INTR_RL_COMPLETE);
                            tracing::info!("preempt ACK — pending ctx reload cancelled");
                            break;
                        }
                    }
                    // Clear any resulting faults
                    let _ = bar0.write_u32(
                        pccsr::channel(channel_id),
                        pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET,
                    );
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }

                let pb = 0x0004_0000 + pid * 0x2000;
                let w = |off: usize, val: u32| bar0.write_u32(pb + off, val).ok();

                let limit2 = gpfifo_entries.ilog2();
                let userd_val = (userd_iova as u32 & 0xFFFF_FE00) | PBDMA_TARGET_SYS_MEM_COHERENT;
                let gpbase_hi = (gpfifo_iova >> 32) as u32 | (limit2 << 16);

                // DIRECT PBDMA registers (hardware-read for processing)
                w(pbdma::GP_BASE_LO, gpfifo_iova as u32);
                w(pbdma::GP_BASE_HI, gpbase_hi);
                w(pbdma::USERD_LO, userd_val);
                w(pbdma::USERD_HI, (userd_iova >> 32) as u32);
                w(pbdma::SIGNATURE, 0x0000_FACE);
                w(pbdma::CHANNEL_INFO, 0x0300_0000 | channel_id);
                w(pbdma::GP_FETCH, 0);
                w(pbdma::GP_STATE, 0);
                w(pbdma::GP_PUT, 0);

                // CTX registers (RAMFC mirror — scheduler save/restore)
                w(pbdma::CTX_USERD_LO, userd_val);
                w(pbdma::CTX_USERD_HI, (userd_iova >> 32) as u32);
                w(pbdma::CTX_SIGNATURE, 0x0000_FACE);
                w(pbdma::CTX_ACQUIRE, 0x7FFF_F902);
                w(pbdma::CTX_GP_BASE_LO, gpfifo_iova as u32);
                w(pbdma::CTX_GP_BASE_HI, gpbase_hi);
                w(pbdma::CTX_GP_PUT, 0);
                w(pbdma::CTX_GP_FETCH, 0);

                // RAMFC-specific fields
                w(ramfc::PB_HEADER, 0x2040_0000);
                w(ramfc::SUBDEVICE, 0x3000_0000 | 0xFFF);
                w(ramfc::ACQUIRE, 0x7FFF_F902);
                w(ramfc::DMA_LIMIT_REF, 0x003F_6078);

                // Clear latched PBDMA errors NOW — valid state is
                // programmed above so errors won't re-latch.
                w(0x100, 0xFFFF_FFFF); // INTR_0 W1C
                w(0x108, 0xFFFF_FFFF); // INTR_STALL W1C
                w(0x148, 0xFFFF_FFFF); // HCE_INTR W1C
                std::thread::sleep(std::time::Duration::from_millis(2));

                // Also clear PCCSR faults for this channel
                let _ = bar0.write_u32(
                    pccsr::channel(channel_id),
                    pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET,
                );
                std::thread::sleep(std::time::Duration::from_millis(2));

                // Re-enable the channel after fault clear
                let _ = bar0.write_u32(
                    pccsr::channel(channel_id),
                    pccsr::CHANNEL_ENABLE_SET,
                );

                chan.submit_runlist(bar0)?;
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Ring doorbell to wake the PBDMA.
                let _ = bar0.write_u32(
                    registers::usermode::NOTIFY_CHANNEL_PENDING,
                    channel_id,
                );
                std::thread::sleep(std::time::Duration::from_millis(50));

                let intr_0 = bar0.read_u32(pb + 0x100).unwrap_or(0);
                let intr_stall = bar0.read_u32(pb + 0x108).unwrap_or(0);
                let userd_direct = bar0.read_u32(pb + pbdma::USERD_LO).unwrap_or(0);
                let sig_direct = bar0.read_u32(pb + pbdma::SIGNATURE).unwrap_or(0);
                let ch_state = bar0.read_u32(pb + pbdma::CHANNEL_STATE).unwrap_or(0);
                let pccsr_post = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
                tracing::info!(
                    pbdma = pid,
                    gpfifo = format_args!("{gpfifo_iova:#x}"),
                    userd_direct = format_args!("{userd_direct:#010x}"),
                    sig_direct = format_args!("{sig_direct:#010x}"),
                    ch_state = format_args!("{ch_state:#010x}"),
                    intr_0 = format_args!("{intr_0:#010x}"),
                    intr_stall = format_args!("{intr_stall:#010x}"),
                    pccsr = format_args!("{pccsr_post:#010x}"),
                    pccsr_status = pccsr::status(pccsr_post),
                    "PBDMA force-programmed (program→clear_intr→enable→resubmit→doorbell)"
                );

                // If PBDMA still has errors, retry: clear again + re-doorbell
                if intr_0 != 0 {
                    w(0x100, 0xFFFF_FFFF);
                    w(0x108, 0xFFFF_FFFF);
                    w(0x148, 0xFFFF_FFFF);
                    std::thread::sleep(std::time::Duration::from_millis(5));

                    let _ = bar0.write_u32(
                        pccsr::channel(channel_id),
                        pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET,
                    );
                    let _ = bar0.write_u32(
                        pccsr::channel(channel_id),
                        pccsr::CHANNEL_ENABLE_SET,
                    );
                    chan.submit_runlist(bar0)?;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    let _ = bar0.write_u32(
                        registers::usermode::NOTIFY_CHANNEL_PENDING,
                        channel_id,
                    );
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    let intr_retry = bar0.read_u32(pb + 0x100).unwrap_or(0);
                    let pccsr_retry = bar0.read_u32(pccsr::channel(channel_id)).unwrap_or(0);
                    tracing::info!(
                        intr_0 = format_args!("{intr_retry:#010x}"),
                        pccsr = format_args!("{pccsr_retry:#010x}"),
                        pccsr_status = pccsr::status(pccsr_retry),
                        "PBDMA retry after second interrupt clear"
                    );
                }
            } else {
                tracing::warn!(
                    runlist = chan.runlist_id,
                    "no PBDMA found for target runlist — scheduler must load channel"
                );
            }
        } else if let Some(pid) = target_pbdma {
            let pb = 0x0004_0000 + pid * 0x2000;
            let ch_state = bar0.read_u32(pb + pbdma::CHANNEL_STATE).unwrap_or(0);
            let intr_0 = bar0.read_u32(pb + 0x100).unwrap_or(0);
            tracing::info!(
                pbdma = pid,
                ch_state = format_args!("{ch_state:#010x}"),
                intr_0 = format_args!("{intr_0:#010x}"),
                "scheduler loaded channel — PBDMA ready"
            );
        }

        // Post-init liveness probe: issue a runlist preempt and check for ACK.
        // On GV100, PFIFO_ENABLE (0x2200) reads 0 even when the engine is
        // functional. The preempt ACK is the authoritative liveness signal.
        let pfifo_live = {
            let w = |reg, val| bar0.write_u32(reg, val).ok();
            w(registers::pfifo::INTR, 0xFFFF_FFFF);
            w(registers::pfifo::GV100_PREEMPT, 1u32 << chan.runlist_id);
            let mut ack = false;
            for _ in 0..25 {
                std::thread::sleep(std::time::Duration::from_millis(2));
                let intr = bar0.read_u32(registers::pfifo::INTR).unwrap_or(0);
                if intr & registers::pfifo::INTR_RL_COMPLETE != 0 {
                    w(registers::pfifo::INTR, registers::pfifo::INTR_RL_COMPLETE);
                    ack = true;
                    break;
                }
            }
            ack
        };
        if pfifo_live {
            tracing::info!("PFIFO liveness probe: preempt ACK received — engine functional");
        } else {
            tracing::warn!("PFIFO liveness probe: NO preempt ACK — engine may be non-responsive");
        }

        log_pfifo_diagnostics(bar0);

        let faults = super::mmu_fault::read_mmu_faults(bar0);
        super::mmu_fault::log_mmu_faults(&faults);

        tracing::info!(
            channel_id,
            gpfifo_iova = format_args!("{gpfifo_iova:#x}"),
            userd_iova = format_args!("{userd_iova:#x}"),
            instance_iova = format_args!("{INSTANCE_IOVA:#x}"),
            pfifo_live,
            "VFIO PFIFO channel created"
        );

        Ok(chan)
    }

    /// Create a PFIFO channel on the specified runlist.
    pub fn create_on_runlist(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
        target_runlist: u32,
    ) -> DriverResult<Self> {
        let mut chan = Self::create(
            container,
            bar0,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
        )?;
        if chan.runlist_id != target_runlist {
            tracing::info!(
                from = chan.runlist_id,
                to = target_runlist,
                "overriding runlist for PBDMA isolation"
            );
            chan.runlist_id = target_runlist;
            chan.submit_runlist(bar0)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(chan)
    }

    pub(super) fn clear_stale_pccsr(bar0: &MappedBar, channel_id: u32, stale: u32) -> DriverResult<()> {
        if stale & 1 != 0 {
            bar0.write_u32(pccsr::channel(channel_id), pccsr::CHANNEL_ENABLE_CLR)
                .map_err(|e| {
                    DriverError::SubmitFailed(Cow::Owned(format!("PCCSR disable: {e}")))
                })?;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        bar0.write_u32(
            pccsr::channel(channel_id),
            pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET,
        )
        .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("PCCSR fault clear: {e}"))))?;
        std::thread::sleep(std::time::Duration::from_millis(10));

        bar0.write_u32(pccsr::inst(channel_id), 0)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("PCCSR clear inst: {e}"))))?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok(())
    }

    pub(super) fn bind_channel(&self, bar0: &MappedBar) -> DriverResult<()> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "INSTANCE_IOVA >> 12 fits u32 for our allocation range"
        )]
        let value =
            (INSTANCE_IOVA >> 12) as u32 | (TARGET_SYS_MEM_COHERENT << 28) | pccsr::INST_BIND_TRUE;
        tracing::debug!(
            value = format_args!("{value:#010x}"),
            "PCCSR inst (BIND | SYS_MEM_COH)"
        );
        bar0.write_u32(pccsr::inst(self.channel_id), value)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("PCCSR bind: {e}"))))
    }

    /// Clear stale `PBDMA_FAULTED` / `ENG_FAULTED` flags.
    pub(super) fn clear_channel_faults(&self, bar0: &MappedBar) -> DriverResult<()> {
        let ch = pccsr::channel(self.channel_id);
        let pre = bar0.read_u32(ch).unwrap_or(0);
        if pre & (pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET) != 0 {
            bar0.write_u32(ch, pccsr::CHANNEL_ENABLE_CLR)
                .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("chan disable: {e}"))))?;
            std::thread::sleep(std::time::Duration::from_millis(2));

            bar0.write_u32(ch, pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET)
                .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("fault clear: {e}"))))?;
            std::thread::sleep(std::time::Duration::from_millis(2));

            tracing::debug!(
                pre = format_args!("{pre:#010x}"),
                post = format_args!("{:#010x}", bar0.read_u32(ch).unwrap_or(0xDEAD)),
                "cleared channel faults"
            );
        }
        Ok(())
    }

    /// Enable the channel via PCCSR `ENABLE_SET` trigger.
    pub(super) fn enable_channel(&self, bar0: &MappedBar) -> DriverResult<()> {
        bar0.write_u32(pccsr::channel(self.channel_id), pccsr::CHANNEL_ENABLE_SET)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("channel enable: {e}"))))
    }

    /// Re-submit the runlist after modifying the instance block (e.g., adding
    /// a GR context pointer). Cycles the scheduler to force FECS to re-read
    /// the updated channel state.
    pub fn resubmit_runlist(&self, bar0: &MappedBar) -> DriverResult<()> {
        use registers::pfifo;

        tracing::info!(
            channel_id = self.channel_id,
            runlist_id = self.runlist_id,
            "re-submitting runlist with scheduler cycle"
        );

        // 1. Disable scheduler
        let _ = bar0.write_u32(pfifo::SCHED_DISABLE, 0xFFFF_FFFF);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 2. Clear pending interrupts
        let _ = bar0.write_u32(pfifo::INTR, 0xFFFF_FFFF);

        // 3. Preempt old runlist state
        let _ = bar0.write_u32(pfifo::GV100_PREEMPT, 1u32 << self.runlist_id);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let _ = bar0.write_u32(pfifo::INTR, pfifo::INTR_RL_COMPLETE);

        // 4. Re-enable channel
        let _ = bar0.write_u32(
            pccsr::channel(self.channel_id),
            pccsr::CHANNEL_ENABLE_SET,
        );

        // 5. Submit runlist
        self.submit_runlist(bar0)?;

        // 6. Re-enable scheduler
        let _ = bar0.write_u32(pfifo::SCHED_DISABLE, 0);
        let _ = bar0.write_u32(pfifo::SCHED_EN, 1);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 7. Wait for scheduler to process
        let mut loaded = false;
        for tick in 0..100 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let pccsr_val = bar0.read_u32(pccsr::channel(self.channel_id)).unwrap_or(0);
            let status = pccsr::status(pccsr_val);
            if status >= 5 {
                tracing::info!(
                    tick,
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "scheduler loaded channel after cycle (STATUS >= ON_PBDMA)"
                );
                loaded = true;
                break;
            }
            if tick % 20 == 19 {
                tracing::debug!(
                    tick,
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "waiting for scheduler after cycle"
                );
            }
        }
        if !loaded {
            let pccsr_val = bar0.read_u32(pccsr::channel(self.channel_id)).unwrap_or(0);
            let status = pccsr::status(pccsr_val);
            tracing::info!(
                status,
                pccsr = format_args!("{pccsr_val:#010x}"),
                "scheduler still pending after cycle"
            );
        }
        Ok(())
    }

    /// Submit runlist to PFIFO using GV100 per-runlist registers.
    ///
    /// GV100 uses per-runlist registers at stride 0x10:
    ///   BASE(rl) = 0x2270 + rl*0x10   → lower_32(iova >> 12)
    ///   SUBMIT(rl) = 0x2274 + rl*0x10 → upper_32(iova >> 12) | (count << 16)
    /// Writing SUBMIT triggers the scheduler.
    /// Source: nouveau `gv100_runl_commit()`.
    fn submit_runlist(&self, bar0: &MappedBar) -> DriverResult<()> {
        // GV100 runlist BASE register: plain (addr >> 12), NO target bits.
        // nouveau's gv100_runl_commit writes lower_32_bits(addr >> 12) directly.
        // Previously we OR'd in (TARGET_SYS_MEM_COHERENT << 28) which corrupted
        // the address, making the scheduler read runlist from 0x200000000000.
        let rl_base = registers::pfifo::gv100_runlist_base_value(RUNLIST_IOVA);
        let rl_submit = registers::pfifo::gv100_runlist_submit_value(RUNLIST_IOVA, 2);

        tracing::debug!(
            runlist_id = self.runlist_id,
            rl_base = format_args!("{rl_base:#010x}"),
            rl_submit = format_args!("{rl_submit:#010x}"),
            "submitting runlist (gv100 per-RL)"
        );

        bar0.write_u32(registers::pfifo::runlist_base(self.runlist_id), rl_base)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("runlist base: {e}"))))?;
        bar0.write_u32(registers::pfifo::runlist_submit(self.runlist_id), rl_submit)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("runlist submit: {e}"))))
    }
}
