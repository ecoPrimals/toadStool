// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use crate::nv::pmu_init::pmu_reg;
use crate::vfio::device::MappedBar;

use super::{UngatingAttempt, gpc_alive, power_reg, r, snapshot_power_state, w};

/// Run progressive ungating strategies to cross the Tier 1 → Tier 2 boundary.
pub fn run_ungating_attempts(
    bar0: &MappedBar,
    pmu_running: bool,
    notes: &mut Vec<String>,
) -> Vec<UngatingAttempt> {
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
                cg_result.changes,
                cg_result.faulted,
                pri_result.alive,
                pri_result.faulted,
                pri_result.recovered,
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
    if pmu_running && !gpc_alive(r(bar0, power_reg::GPC_ENABLES)) {
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
                detail: format!("MBOX0→{mbox0_2:#010x}, MBOX1→{mbox1_2:#010x}"),
            });

            tracing::info!(
                mbox0 = format_args!("{mbox0_2:#010x}"),
                mbox1 = format_args!("{mbox1_2:#010x}"),
                gpc_after = format_args!("{gpc_post2:#010x}"),
                "Attempt 5b: PMU mailbox reinit"
            );
        }
    } else if !pmu_running {
        notes.push("PMU not running — skipping mailbox injection attempts".into());
    }

    attempts
}
