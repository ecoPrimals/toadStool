// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO engine initialization and diagnostic readback for Volta+ GPUs.
//!
//! Implements the engine bring-up sequence from nouveau's `gk104_fifo_init()`,
//! `gk104_fifo_init_pbdmas()`, `gf100_runq_init()`, and `gk208_runq_init()`.

mod diag;
mod discover;
mod kepler;
mod volta;

pub use discover::{discover_ce_runlist, find_pbdma_for_runlist};
pub use kepler::init_pfifo_engine_kepler;
pub use volta::init_pfifo_engine_with;

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;
use crate::vfio::dma::DmaBuffer;
use crate::vfio::device::DmaBackend;

use super::mmu;
use super::page_tables;
use super::registers::{self, falcon, pbdma, pccsr, pfifo, ramfc};
use super::registers::{
    FAULT_BUF_IOVA, INSTANCE_IOVA, PBDMA_TARGET_SYS_MEM_COHERENT, PD0_IOVA, PD1_IOVA, PD2_IOVA,
    PD3_IOVA, PT0_IOVA, RUNLIST_IOVA, TARGET_SYS_MEM_COHERENT,
};
use super::VfioChannel;

use diag::log_pfifo_diagnostics;

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
