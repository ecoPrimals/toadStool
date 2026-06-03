// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

use super::experiment_snapshot::{ExperimentResult, ExperimentWrite, SovereignSnapshot};
use super::pmc::PMC_ENABLE;
use super::power::{cg_sweep, pri_bus_recover};

/// Stage 1: PFIFO enable + CG sweep.
///
/// Enables the PFIFO engine and disables clock gating across all accessible
/// domains. This is the safest stage — standard init operations, fully reversible.
pub(crate) fn experiment_stage_1(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Write PFIFO_ENABLE = 1
    writes.push(ExperimentWrite::new(bar0, crate::nv::registers::pgraph::PFIFO_ENABLE as usize, 0x1));
    notes.push(format!("PFIFO_ENABLE: was {:#010x}", before.pfifo_enable));

    // CG sweep to disable clock gating
    let cg = cg_sweep(bar0);
    notes.push(format!("CG sweep: {}", cg.detail));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 1,
        stage_name: "PFIFO enable + CG sweep".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 2: PGOB ungate — GPC power domain ungating.
///
/// Disables PMC clock gating, ensures PGRAPH is enabled in PMC, writes GPC
/// broadcast PGOB control registers, and polls PGRAPH_STATUS.
/// Medium risk — writes to power gating control.
pub(crate) fn experiment_stage_2(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    use crate::nv::registers::{gpc, pgraph, pmc as pmc_reg};

    // Step 1: PMC clock gate disable
    writes.push(ExperimentWrite::new(bar0, pmc_reg::CLKGATE_DISABLE as usize, 0x1));
    notes.push("PMC_CLKGATE_DISABLE = 1".into());

    // Step 2: Ensure GR engine enabled in PMC_ENABLE (bit 12)
    let pmc = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
    if pmc & (1 << 12) == 0 {
        writes.push(ExperimentWrite::new(bar0, PMC_ENABLE, pmc | (1 << 12)));
        notes.push(format!("PMC_ENABLE: set GR bit (was {pmc:#010x})"));
    } else {
        notes.push(format!("PMC_ENABLE: GR bit already set ({pmc:#010x})"));
    }

    // Step 3: GPC broadcast PGOB control = 0x110
    writes.push(ExperimentWrite::new(bar0, gpc::BCAST_CONTROL as usize, 0x0000_0110));
    notes.push("GPC_BCAST_PGOB_CONTROL = 0x110".into());

    // Step 4: Per-GPC PGOB disable (broadcast offset + 0x1028)
    writes.push(ExperimentWrite::new(bar0, gpc::BCAST_PGOB as usize, 0x0));
    notes.push("GPC_PGOB_PER_GPC = 0x0 (disable power gating)".into());

    // Step 5: Poll PGRAPH_STATUS for up to 100ms
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut last_status = 0xDEAD_DEAD_u32;
    while Instant::now() < deadline {
        last_status = bar0.read_u32(pgraph::STATUS as usize).unwrap_or(0xDEAD_DEAD);
        if last_status >> 16 != 0xBADF {
            break;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
    notes.push(format!("PGRAPH_STATUS poll result: {last_status:#010x}"));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 2,
        stage_name: "PGOB ungate".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 3: PRI ring recovery + enumerate.
///
/// Clears PRI faults, re-enumerates ring stations, and probes GPC per-unit
/// and TPC registers. Low risk — standard PRI recovery.
pub(crate) fn experiment_stage_3(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    use crate::nv::registers::{gpc, pri};

    // Clear PRI ringmaster interrupt status
    let rm_intr = bar0.read_u32(pri::RINGMASTER_INTR_STATUS as usize).unwrap_or(0);
    if rm_intr != 0 {
        writes.push(ExperimentWrite::new(bar0, pri::RINGMASTER_INTR_STATUS as usize, rm_intr));
        notes.push(format!("PRI_RM_INTR: cleared {rm_intr:#010x}"));
    } else {
        notes.push("PRI_RM_INTR: already clear".into());
    }

    // Re-enumerate ring stations
    writes.push(ExperimentWrite::new(bar0, pri::RINGMASTER_COMMAND as usize, 0x4));
    notes.push("PRI_RINGMASTER_CMD: ENUMERATE".into());
    std::thread::sleep(Duration::from_millis(20));

    // Run full PRI bus recovery
    let pri = pri_bus_recover(bar0);
    notes.push(format!(
        "PRI recovery: alive={}, faulted={}, recovered={}",
        pri.alive, pri.faulted, pri.recovered
    ));

    // Wait for ring to settle
    std::thread::sleep(Duration::from_millis(50));

    // Probe individual GPC registers for liveness
    for g in 0..6u32 {
        let unit = bar0.read_u32(gpc::gpc_base(g) as usize).unwrap_or(0xDEAD_DEAD);
        let tpc0 = bar0.read_u32(gpc::gpc_tpc0(g) as usize).unwrap_or(0xDEAD_DEAD);
        if !is_pri_fault(unit) {
            notes.push(format!(
                "GPC{g}: unit={unit:#010x} tpc0={tpc0:#010x} (alive)"
            ));
        }
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 3,
        stage_name: "PRI ring recovery + enumerate".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}
