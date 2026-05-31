// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO engine init configuration and default bring-up entry point.

use crate::error::DriverResult;
use crate::vfio::device::MappedBar;

use super::volta::init_pfifo_engine_with;

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
pub(in crate::vfio::channel) fn init_pfifo_engine(bar0: &MappedBar) -> DriverResult<(u32, u32)> {
    init_pfifo_engine_with(bar0, &PfifoInitConfig::default())
}
