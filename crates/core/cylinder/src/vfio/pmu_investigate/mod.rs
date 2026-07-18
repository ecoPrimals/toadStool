// SPDX-License-Identifier: AGPL-3.0-or-later
//! PMU Mailbox Investigation — Exp 211 Phase A/B.
//!
//! Probes the PMU falcon state after nouveau unbind to characterize
//! what power domain commands might succeed, then attempts progressive
//! ungating strategies to cross the Tier 1 → Tier 2 boundary.
//!
//! The PMU (Power Management Unit) falcon at `0x10A000+` manages GPU
//! power domains. After nouveau unbind, the PMU may still be running
//! HS firmware that accepts mailbox commands. If we can send a power
//! gating disable command, GPCs come alive and Tier 2 is unlocked.

mod phase_a;
mod phase_c;
mod ungating;

use std::time::Instant;

use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_tiers::classify_tier;

pub use phase_c::PhaseC;

/// Additional PMU registers beyond what `PmuSnapshot` captures.
pub(super) mod pmu_ext {
    pub const IRQSTAT: usize = 0x0010_A008;
    pub const IRQMASK: usize = 0x0010_A010;
    pub const OS: usize = 0x0010_A080;
    pub const FBIF_CTL: usize = 0x0010_AE00;
    pub const FBIF_TRANSCFG: usize = 0x0010_AE14;
    pub const QUEUE_HEAD_0: usize = 0x0010_A4C0;
    pub const QUEUE_HEAD_1: usize = 0x0010_A4C4;
    pub const QUEUE_HEAD_2: usize = 0x0010_A4C8;
    pub const QUEUE_HEAD_3: usize = 0x0010_A4CC;
    pub const QUEUE_TAIL_0: usize = 0x0010_A4D0;
    pub const QUEUE_TAIL_1: usize = 0x0010_A4D4;
    pub const QUEUE_TAIL_2: usize = 0x0010_A4D8;
    pub const QUEUE_TAIL_3: usize = 0x0010_A4DC;
}

/// Power-domain related registers to read before/after ungating attempts.
pub(super) mod power_reg {
    pub const GPC_ENABLES: usize = 0x0041_A004;
    pub const CE0_BASE: usize = 0x0010_4000;
    pub const PGRAPH_STATUS: usize = 0x0040_0700;
    pub const PMC_ENABLE: usize = 0x0000_0200;
    pub const PMC_CLKGATE_DISABLE: usize = 0x0000_0260;
    pub const THERM_GATE_CTRL: usize = 0x0002_0200;
    pub const PGRAPH_GPC_BCAST: usize = 0x0041_9000;
    pub const GPC_PGOB: usize = 0x0041_A028;
}

/// Result of a PMU investigation attempt (Exp 211).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PmuInvestigationResult {
    // Phase A: PMU Liveness
    pub pmu_cpuctl: u32,
    pub pmu_bootvec: u32,
    pub pmu_hwcfg: u32,
    pub pmu_pc_0: u32,
    pub pmu_pc_1: u32,
    pub pmu_pc_advancing: bool,
    pub pmu_running: bool,
    pub pmu_halted: bool,
    pub pmu_hs_locked: bool,
    pub pmu_requires_signed: bool,
    pub pmu_mailbox0: u32,
    pub pmu_mailbox1: u32,
    pub pmu_irqstat: u32,
    pub pmu_irqmask: u32,
    pub pmu_os: u32,
    pub pmu_sctl: u32,
    pub pmu_imem_kb: u32,
    pub pmu_dmem_kb: u32,
    pub pfifo_enable: u32,
    pub pfifo_enabled: bool,

    // PMU queue state (nouveau MSG_QUEUE / CMD_QUEUE)
    pub queue_heads: Vec<u32>,
    pub queue_tails: Vec<u32>,

    // PMU FBIF (DMA window)
    pub fbif_ctl: u32,
    pub fbif_transcfg: u32,

    // Phase B: Power domain state before/after
    pub gpc_enables_before: u32,
    pub ce_status_before: u32,
    pub pgraph_status_before: u32,
    pub tier_before: u8,
    pub tier_before_name: String,

    // Phase B: Ungating attempts
    pub attempts: Vec<UngatingAttempt>,

    // Post-attempt state
    pub gpc_enables_after: u32,
    pub ce_status_after: u32,
    pub pgraph_status_after: u32,
    pub tier_after: u8,
    pub tier_after_name: String,

    // Tier changed?
    pub tier_advanced: bool,
    pub elapsed_ms: u64,
    pub notes: Vec<String>,

    // Phase C: FBIF / DMEM access probe
    pub phase_c: Option<PhaseC>,
}

/// Record of a single ungating attempt.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UngatingAttempt {
    pub name: String,
    pub description: String,
    pub gpc_before: u32,
    pub gpc_after: u32,
    pub ce_before: u32,
    pub ce_after: u32,
    pub succeeded: bool,
    pub detail: String,
}

pub(super) fn r(bar0: &MappedBar, reg: usize) -> u32 {
    bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD)
}

pub(super) fn w(bar0: &MappedBar, reg: usize, val: u32) {
    let _ = bar0.write_u32(reg, val);
}

pub(super) fn gpc_alive(val: u32) -> bool {
    !is_pri_fault(val) && val != 0
}

#[cfg(test)]
fn ce_alive(val: u32) -> bool {
    !is_pri_fault(val)
}

/// Read GPC and CE status for before/after comparison.
pub(super) fn snapshot_power_state(bar0: &MappedBar) -> (u32, u32, u32) {
    (
        r(bar0, power_reg::GPC_ENABLES),
        r(bar0, power_reg::CE0_BASE),
        r(bar0, power_reg::PGRAPH_STATUS),
    )
}

/// Run the full PMU investigation (Phase A liveness + Phase B ungating attempts).
pub fn investigate_pmu(bar0: &MappedBar) -> PmuInvestigationResult {
    let start = Instant::now();
    let mut notes: Vec<String> = Vec::new();

    // ── Phase A: PMU Liveness Probe ─────────────────────────────────

    let phase_a = phase_a::run_phase_a(bar0);
    notes.extend(phase_a.notes);

    // ── Phase B: Power Domain Probes ────────────────────────────────

    let tier_evidence_before = classify_tier(bar0);
    let (gpc_before, ce_before, pgraph_before) = snapshot_power_state(bar0);

    notes.push(format!(
        "Pre-attempt: tier={} gpc={gpc_before:#010x} ce={ce_before:#010x} pgraph={pgraph_before:#010x}",
        tier_evidence_before.tier
    ));

    tracing::info!(
        tier = %tier_evidence_before.tier,
        gpc = format_args!("{gpc_before:#010x}"),
        ce = format_args!("{ce_before:#010x}"),
        pgraph = format_args!("{pgraph_before:#010x}"),
        "Phase B: power domain state before attempts"
    );

    let attempts = ungating::run_ungating_attempts(bar0, phase_a.snapshot.is_running, &mut notes);

    // ── Phase C: FBIF / DMEM access probe ──────────────────────────

    let phase_c = if !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
        notes.push("Running Phase C: DMEM access / queue-based PG probe...".into());
        Some(phase_c::investigate_pmu_phase_c(bar0))
    } else {
        notes.push("Phase C skipped — GPCs already alive".into());
        None
    };

    if let Some(ref pc) = phase_c {
        notes.extend(pc.notes.iter().map(|n| format!("[Phase C] {n}")));
    }

    // ── Post-attempt classification ─────────────────────────────────

    let tier_evidence_after = classify_tier(bar0);
    let (gpc_after, ce_after, pgraph_after) = snapshot_power_state(bar0);
    let tier_advanced = tier_evidence_after.tier > tier_evidence_before.tier;

    if tier_advanced {
        notes.push(format!(
            "TIER ADVANCED: {} → {} — ungating succeeded!",
            tier_evidence_before.tier, tier_evidence_after.tier,
        ));
        tracing::info!(
            before = %tier_evidence_before.tier,
            after = %tier_evidence_after.tier,
            "TIER ADVANCED — sovereign compute may be unlocked"
        );
    } else {
        notes.push(format!(
            "Tier unchanged: {} — all ungating attempts failed to cross the power domain boundary",
            tier_evidence_after.tier,
        ));

        // Summarize findings for next steps
        if phase_a.hs_locked {
            notes.push(
                "NEXT: PMU is HS-locked. The mailbox command format may need \
                 to match the HS firmware's MSG queue protocol (header + payload), \
                 not simple MBOX0/MBOX1 writes. See nouveau gv100_pmu.c for the \
                 queue-based message passing protocol."
                    .into(),
            );
        }
        if !phase_a.snapshot.is_running {
            notes.push(
                "NEXT: PMU is not running. Consider the kernel patch path — \
                 modify nouveau gv100_gr_fini() to skip GPC power-down during unbind."
                    .into(),
            );
        }
        notes.push(
            "NEXT: The K80 (incoming, unsigned falcons) may provide an \
             alternate Tier 2 path via direct PIO falcon upload."
                .into(),
        );
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    tracing::info!(
        tier_before = tier_evidence_before.tier.level(),
        tier_after = tier_evidence_after.tier.level(),
        tier_advanced,
        attempts = attempts.len(),
        elapsed_ms,
        "PMU investigation complete"
    );

    let snapshot = phase_a.snapshot;

    PmuInvestigationResult {
        pmu_cpuctl: snapshot.cpuctl,
        pmu_bootvec: snapshot.bootvec,
        pmu_hwcfg: snapshot.hwcfg,
        pmu_pc_0: phase_a.pc_0,
        pmu_pc_1: phase_a.pc_1,
        pmu_pc_advancing: phase_a.pc_advancing,
        pmu_running: snapshot.is_running,
        pmu_halted: snapshot.cpuctl & (1 << 4) != 0,
        pmu_hs_locked: phase_a.hs_locked,
        pmu_requires_signed: snapshot.requires_signed(),
        pmu_mailbox0: snapshot.mailbox0,
        pmu_mailbox1: snapshot.mailbox1,
        pmu_irqstat: phase_a.irqstat,
        pmu_irqmask: phase_a.irqmask,
        pmu_os: phase_a.os_reg,
        pmu_sctl: snapshot.sctl,
        pmu_imem_kb: snapshot.imem_size_kb(),
        pmu_dmem_kb: snapshot.dmem_size_kb(),
        pfifo_enable: snapshot.pfifo_enable,
        pfifo_enabled: snapshot.pfifo_enabled,
        queue_heads: phase_a.queue_heads,
        queue_tails: phase_a.queue_tails,
        fbif_ctl: phase_a.fbif_ctl,
        fbif_transcfg: phase_a.fbif_transcfg,
        gpc_enables_before: gpc_before,
        ce_status_before: ce_before,
        pgraph_status_before: pgraph_before,
        tier_before: tier_evidence_before.tier.level(),
        tier_before_name: tier_evidence_before.tier.description().to_string(),
        attempts,
        gpc_enables_after: gpc_after,
        ce_status_after: ce_after,
        pgraph_status_after: pgraph_after,
        tier_after: tier_evidence_after.tier.level(),
        tier_after_name: tier_evidence_after.tier.description().to_string(),
        tier_advanced,
        elapsed_ms,
        notes,
        phase_c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmu_ext_registers_in_pmu_range() {
        const { assert!(pmu_ext::IRQSTAT >= 0x10_A000) };
        const { assert!(pmu_ext::QUEUE_TAIL_3 < 0x10_B000) };
    }

    #[test]
    fn power_reg_constants() {
        assert_eq!(power_reg::GPC_ENABLES, 0x0041_A004);
        assert_eq!(power_reg::CE0_BASE, 0x0010_4000);
        assert_eq!(power_reg::PMC_ENABLE, 0x0000_0200);
    }

    #[test]
    fn gpc_alive_check() {
        assert!(!gpc_alive(0xBADF_5545));
        assert!(!gpc_alive(0xBADF_1100));
        assert!(!gpc_alive(0xDEAD_DEAD));
        assert!(!gpc_alive(0));
        assert!(gpc_alive(0x0000_000F));
    }

    #[test]
    fn ce_alive_check() {
        assert!(!ce_alive(0xBADF_3000));
        assert!(!ce_alive(0xDEAD_DEAD));
        assert!(ce_alive(0x0000_0000));
        assert!(ce_alive(0x0000_0001));
    }
}
