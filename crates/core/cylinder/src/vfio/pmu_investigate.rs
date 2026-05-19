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

use std::time::{Duration, Instant};

use crate::nv::pmu_init::{PmuSnapshot, pmu_reg};
use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_tiers::{SovereignTier, classify_tier};

/// Additional PMU registers beyond what `PmuSnapshot` captures.
mod pmu_ext {
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
mod power_reg {
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

fn r(bar0: &MappedBar, reg: usize) -> u32 {
    bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD)
}

fn w(bar0: &MappedBar, reg: usize, val: u32) {
    let _ = bar0.write_u32(reg, val);
}

fn gpc_alive(val: u32) -> bool {
    !is_pri_fault(val) && val != 0
}

fn ce_alive(val: u32) -> bool {
    !is_pri_fault(val)
}

/// Read GPC and CE status for before/after comparison.
fn snapshot_power_state(bar0: &MappedBar) -> (u32, u32, u32) {
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

    let snapshot = PmuSnapshot::capture(bar0);
    let falcon_state = snapshot.falcon_state();
    notes.push(format!("PMU falcon state: {falcon_state:?}"));
    notes.push(snapshot.summary());

    // Read PC twice to check if it's advancing
    let pc_0 = r(bar0, pmu_reg::PC);
    std::thread::sleep(Duration::from_millis(5));
    let pc_1 = r(bar0, pmu_reg::PC);
    let pc_advancing = pc_0 != pc_1 && !is_pri_fault(pc_0) && !is_pri_fault(pc_1);

    if pc_advancing {
        notes.push(format!("PMU PC advancing: {pc_0:#x} → {pc_1:#x} (delta {})", pc_1.wrapping_sub(pc_0)));
    } else if pc_0 == pc_1 {
        notes.push(format!("PMU PC static: {pc_0:#x} (may be in tight loop or halted)"));
    }

    let hs_locked = snapshot.sctl & 0x02 != 0;
    if hs_locked {
        notes.push("PMU HS-locked (SCTL bit 1 set) — secure firmware running".into());
    }

    // Extended PMU registers
    let irqstat = r(bar0, pmu_ext::IRQSTAT);
    let irqmask = r(bar0, pmu_ext::IRQMASK);
    let os_reg = r(bar0, pmu_ext::OS);
    let fbif_ctl = r(bar0, pmu_ext::FBIF_CTL);
    let fbif_transcfg = r(bar0, pmu_ext::FBIF_TRANSCFG);

    let queue_heads = vec![
        r(bar0, pmu_ext::QUEUE_HEAD_0),
        r(bar0, pmu_ext::QUEUE_HEAD_1),
        r(bar0, pmu_ext::QUEUE_HEAD_2),
        r(bar0, pmu_ext::QUEUE_HEAD_3),
    ];
    let queue_tails = vec![
        r(bar0, pmu_ext::QUEUE_TAIL_0),
        r(bar0, pmu_ext::QUEUE_TAIL_1),
        r(bar0, pmu_ext::QUEUE_TAIL_2),
        r(bar0, pmu_ext::QUEUE_TAIL_3),
    ];

    // Check if queues have pending messages
    for (i, (h, t)) in queue_heads.iter().zip(queue_tails.iter()).enumerate() {
        if h != t && !is_pri_fault(*h) && !is_pri_fault(*t) {
            notes.push(format!("PMU queue {i} has pending data: head={h:#x} tail={t:#x}"));
        }
    }

    notes.push(format!("PMU IRQSTAT={irqstat:#010x} IRQMASK={irqmask:#010x} OS={os_reg:#010x}"));
    notes.push(format!("PMU FBIF_CTL={fbif_ctl:#010x} FBIF_TRANSCFG={fbif_transcfg:#010x}"));

    tracing::info!(
        cpuctl = format_args!("{:#010x}", snapshot.cpuctl),
        pc_0 = format_args!("{:#010x}", pc_0),
        pc_1 = format_args!("{:#010x}", pc_1),
        pc_advancing,
        hs_locked,
        running = snapshot.is_running,
        halted = snapshot.cpuctl & (1 << 4) != 0,
        sctl = format_args!("{:#010x}", snapshot.sctl),
        mbox0 = format_args!("{:#010x}", snapshot.mailbox0),
        mbox1 = format_args!("{:#010x}", snapshot.mailbox1),
        pfifo = snapshot.pfifo_enabled,
        "Phase A: PMU liveness probe"
    );

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

    let mut attempts = Vec::new();

    // ── Attempt 1: CG sweep + PRI recovery ──────────────────────────
    //
    // Disable clock gating across all domains and recover the PRI bus.
    // This is the mildest intervention — purely clearing CG state.
    {
        let (gpc_pre, ce_pre, _) = snapshot_power_state(bar0);

        let cg_result = crate::vfio::sovereign_stages::cg_sweep(bar0);
        let pri_result = crate::vfio::sovereign_stages::pri_bus_recover(bar0);

        let (gpc_post, ce_post, _) = snapshot_power_state(bar0);
        let ok = gpc_alive(gpc_post) || (gpc_post != gpc_pre);

        attempts.push(UngatingAttempt {
            name: "cg_sweep_pri_recover".into(),
            description: "Clock gating disable + PRI bus recovery".into(),
            gpc_before: gpc_pre,
            gpc_after: gpc_post,
            ce_before: ce_pre,
            ce_after: ce_post,
            succeeded: gpc_alive(gpc_post),
            detail: format!(
                "CG: {} changes, {} faulted. PRI: {} alive, {} faulted, recovered={}",
                cg_result.changes, cg_result.faulted,
                pri_result.alive, pri_result.faulted, pri_result.recovered,
            ),
        });

        tracing::info!(
            gpc_before = format_args!("{gpc_pre:#010x}"),
            gpc_after = format_args!("{gpc_post:#010x}"),
            changed = ok,
            "Attempt 1: CG sweep + PRI recovery"
        );
    }

    // ── Attempt 2: PMC clock gate disable + PGOB broadcast ──────────
    //
    // Direct register writes to disable power gating via PMC and PGRAPH
    // GPC broadcast. This is what pgob_disable() does internally.
    if !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
        let (gpc_pre, ce_pre, _) = snapshot_power_state(bar0);

        // Disable PMC clock gating
        w(bar0, power_reg::PMC_CLKGATE_DISABLE, 1);

        // Ensure GR engine enabled in PMC_ENABLE (bit 12)
        let pmc = r(bar0, power_reg::PMC_ENABLE);
        if pmc & (1 << 12) == 0 {
            w(bar0, power_reg::PMC_ENABLE, pmc | (1 << 12));
        }

        // GPC broadcast control — ungate
        w(bar0, power_reg::PGRAPH_GPC_BCAST, 0x0000_0110);

        // Per-GPC power gate disable
        w(bar0, power_reg::GPC_PGOB, 0x0000_0000);

        std::thread::sleep(Duration::from_millis(10));

        let (gpc_post, ce_post, _) = snapshot_power_state(bar0);

        attempts.push(UngatingAttempt {
            name: "pmc_pgob_direct".into(),
            description: "PMC clock gate disable + PGRAPH GPC broadcast ungate".into(),
            gpc_before: gpc_pre,
            gpc_after: gpc_post,
            ce_before: ce_pre,
            ce_after: ce_post,
            succeeded: gpc_alive(gpc_post),
            detail: format!("PMC_ENABLE={pmc:#010x}, wrote BCAST=0x110, PGOB=0"),
        });

        tracing::info!(
            gpc_before = format_args!("{gpc_pre:#010x}"),
            gpc_after = format_args!("{gpc_post:#010x}"),
            "Attempt 2: PMC + PGOB direct"
        );
    }

    // ── Attempt 3: THERM power gate override ────────────────────────
    //
    // The thermal subsystem has power gating controls that may override
    // the per-engine gating. Write 0 to THERM_GATE_CTRL to attempt
    // global power gating disable.
    if !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
        let (gpc_pre, ce_pre, _) = snapshot_power_state(bar0);

        let therm_before = r(bar0, power_reg::THERM_GATE_CTRL);
        w(bar0, power_reg::THERM_GATE_CTRL, 0);
        std::thread::sleep(Duration::from_millis(5));
        let therm_after = r(bar0, power_reg::THERM_GATE_CTRL);

        let (gpc_post, ce_post, _) = snapshot_power_state(bar0);

        attempts.push(UngatingAttempt {
            name: "therm_gate_override".into(),
            description: "THERM gate control write 0 (global PG disable)".into(),
            gpc_before: gpc_pre,
            gpc_after: gpc_post,
            ce_before: ce_pre,
            ce_after: ce_post,
            succeeded: gpc_alive(gpc_post),
            detail: format!("THERM_GATE_CTRL: {therm_before:#010x} → {therm_after:#010x}"),
        });

        tracing::info!(
            therm_before = format_args!("{therm_before:#010x}"),
            therm_after = format_args!("{therm_after:#010x}"),
            gpc_after = format_args!("{gpc_post:#010x}"),
            "Attempt 3: THERM gate override"
        );
    }

    // ── Attempt 4: PMC engine enable toggle ─────────────────────────
    //
    // Toggle engine enable bits in PMC_ENABLE. On some GPUs, toggling
    // GR off and on forces a re-initialization of the power domain.
    if !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
        let (gpc_pre, ce_pre, _) = snapshot_power_state(bar0);

        let pmc = r(bar0, power_reg::PMC_ENABLE);
        // Toggle GR (bit 12) off then on
        w(bar0, power_reg::PMC_ENABLE, pmc & !(1 << 12));
        std::thread::sleep(Duration::from_millis(5));
        w(bar0, power_reg::PMC_ENABLE, pmc | (1 << 12));
        std::thread::sleep(Duration::from_millis(10));

        let (gpc_post, ce_post, _) = snapshot_power_state(bar0);

        attempts.push(UngatingAttempt {
            name: "pmc_gr_toggle".into(),
            description: "PMC_ENABLE GR bit 12 toggle (off→on reset)".into(),
            gpc_before: gpc_pre,
            gpc_after: gpc_post,
            ce_before: ce_pre,
            ce_after: ce_post,
            succeeded: gpc_alive(gpc_post),
            detail: format!("PMC_ENABLE={pmc:#010x}, toggled bit 12"),
        });

        tracing::info!(
            pmc = format_args!("{pmc:#010x}"),
            gpc_after = format_args!("{gpc_post:#010x}"),
            "Attempt 4: PMC GR toggle"
        );
    }

    // ── Attempt 5: PMU mailbox command injection ────────────────────
    //
    // If PMU is running (not halted, not PRI-faulted), try writing a
    // power gating disable command to MAILBOX0 and triggering via MAILBOX1.
    //
    // nouveau PMU message format (from nvkm/subdev/pmu/gv100.c):
    //   MBOX0: command data (unit ID + command)
    //   MBOX1: trigger (write non-zero to signal PMU)
    //
    // Known unit IDs from nouveau:
    //   0x00: PMU_UNIT_INIT
    //   0x03: PMU_UNIT_PG (power gating)
    //   0x04: PMU_UNIT_THERM
    //   0x07: PMU_UNIT_VOLT
    //
    // PG commands (from nouveau gk20a_pmu_pg.c):
    //   0x00: PG_CMD_ENG_BUF_LOAD
    //   0x01: PG_CMD_ENG_BUF_UNLOAD
    //   0x03: PG_CMD_STAT
    //   0x08: PG_CMD_ALLOW  (disables power gating)
    //   0x09: PG_CMD_DISALLOW  (enables power gating)
    if snapshot.is_running && !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
        let (gpc_pre, ce_pre, _) = snapshot_power_state(bar0);

        let mbox0_before = r(bar0, pmu_reg::MAILBOX0);
        let mbox1_before = r(bar0, pmu_reg::MAILBOX1);

        // PG_CMD_ALLOW (0x08) to unit PG (0x03):
        // Command word format: (unit << 8) | cmd
        // This is a best-effort reconstruction from nouveau source.
        let pg_allow_cmd: u32 = (0x03 << 8) | 0x08;

        w(bar0, pmu_reg::MAILBOX0, pg_allow_cmd);
        w(bar0, pmu_reg::MAILBOX1, 1); // trigger

        // Wait for PMU to process
        std::thread::sleep(Duration::from_millis(50));

        let mbox0_after = r(bar0, pmu_reg::MAILBOX0);
        let mbox1_after = r(bar0, pmu_reg::MAILBOX1);

        // Check if PMU consumed the command (MBOX0/1 changed)
        let pmu_responded = mbox0_after != pg_allow_cmd || mbox1_after != 1;

        std::thread::sleep(Duration::from_millis(50));

        let (gpc_post, ce_post, _) = snapshot_power_state(bar0);

        attempts.push(UngatingAttempt {
            name: "pmu_mailbox_pg_allow".into(),
            description: "PMU MBOX0 PG_CMD_ALLOW (unit=PG, cmd=0x08) + MBOX1 trigger".into(),
            gpc_before: gpc_pre,
            gpc_after: gpc_post,
            ce_before: ce_pre,
            ce_after: ce_post,
            succeeded: gpc_alive(gpc_post),
            detail: format!(
                "MBOX0: {mbox0_before:#010x}→{pg_allow_cmd:#010x}→{mbox0_after:#010x}, \
                 MBOX1: {mbox1_before:#010x}→1→{mbox1_after:#010x}, \
                 PMU responded: {pmu_responded}"
            ),
        });

        tracing::info!(
            mbox0_before = format_args!("{mbox0_before:#010x}"),
            mbox0_after = format_args!("{mbox0_after:#010x}"),
            mbox1_after = format_args!("{mbox1_after:#010x}"),
            pmu_responded,
            gpc_after = format_args!("{gpc_post:#010x}"),
            "Attempt 5: PMU mailbox PG_ALLOW"
        );

        // If PMU responded but GPCs still gated, try alternate command format.
        // nouveau gv100 uses a different MSG queue protocol with headers.
        if pmu_responded && !gpc_alive(gpc_post) {
            let (gpc_pre2, ce_pre2, _) = snapshot_power_state(bar0);

            // Try setting MBOX0 to known "init complete" pattern and trigger
            // power management re-evaluation
            w(bar0, pmu_reg::MAILBOX0, 0x0000_0000);
            w(bar0, pmu_reg::MAILBOX1, 0x0000_0001);
            std::thread::sleep(Duration::from_millis(50));

            let mbox0_2 = r(bar0, pmu_reg::MAILBOX0);
            let mbox1_2 = r(bar0, pmu_reg::MAILBOX1);

            let (gpc_post2, ce_post2, _) = snapshot_power_state(bar0);

            attempts.push(UngatingAttempt {
                name: "pmu_mailbox_reinit".into(),
                description: "PMU MBOX0=0 + MBOX1=1 trigger (re-init signal)".into(),
                gpc_before: gpc_pre2,
                gpc_after: gpc_post2,
                ce_before: ce_pre2,
                ce_after: ce_post2,
                succeeded: gpc_alive(gpc_post2),
                detail: format!(
                    "MBOX0→{mbox0_2:#010x}, MBOX1→{mbox1_2:#010x}"
                ),
            });

            tracing::info!(
                mbox0 = format_args!("{mbox0_2:#010x}"),
                mbox1 = format_args!("{mbox1_2:#010x}"),
                gpc_after = format_args!("{gpc_post2:#010x}"),
                "Attempt 5b: PMU mailbox reinit"
            );
        }
    } else if !snapshot.is_running {
        notes.push("PMU not running — skipping mailbox injection attempts".into());
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
        if hs_locked {
            notes.push(
                "NEXT: PMU is HS-locked. The mailbox command format may need \
                 to match the HS firmware's MSG queue protocol (header + payload), \
                 not simple MBOX0/MBOX1 writes. See nouveau gv100_pmu.c for the \
                 queue-based message passing protocol."
                    .into(),
            );
        }
        if !snapshot.is_running {
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

    PmuInvestigationResult {
        pmu_cpuctl: snapshot.cpuctl,
        pmu_bootvec: snapshot.bootvec,
        pmu_hwcfg: snapshot.hwcfg,
        pmu_pc_0: pc_0,
        pmu_pc_1: pc_1,
        pmu_pc_advancing: pc_advancing,
        pmu_running: snapshot.is_running,
        pmu_halted: snapshot.cpuctl & (1 << 4) != 0,
        pmu_hs_locked: hs_locked,
        pmu_requires_signed: snapshot.requires_signed(),
        pmu_mailbox0: snapshot.mailbox0,
        pmu_mailbox1: snapshot.mailbox1,
        pmu_irqstat: irqstat,
        pmu_irqmask: irqmask,
        pmu_os: os_reg,
        pmu_sctl: snapshot.sctl,
        pmu_imem_kb: snapshot.imem_size_kb(),
        pmu_dmem_kb: snapshot.dmem_size_kb(),
        pfifo_enable: snapshot.pfifo_enable,
        pfifo_enabled: snapshot.pfifo_enabled,
        queue_heads,
        queue_tails,
        fbif_ctl,
        fbif_transcfg,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pmu_ext_registers_in_pmu_range() {
        assert!(pmu_ext::IRQSTAT >= 0x10_A000);
        assert!(pmu_ext::QUEUE_TAIL_3 < 0x10_B000);
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
