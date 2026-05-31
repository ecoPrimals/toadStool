// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-stage implementations for [`crate::vfio::sovereign_init::sovereign_init`].

use std::time::{Duration, Instant};

use crate::vfio::device::MappedBar;

mod devinit;
mod gr;
mod memory;
mod pmc;
mod power;

#[cfg(test)]
mod tests;

pub use crate::error::SovereignStagesError;

pub(crate) use devinit::verify;
pub(crate) use gr::{falcon_boot, gr_init};
pub(crate) use memory::{
    chip_id_to_sm, dispatch_memory_training, gddr5_training, is_warm_gpu, pramin_sentinel_test,
    run_hbm2_training,
};
pub use memory::{MemoryTrainingResult, MemoryTrainingStrategy};
pub(crate) use pmc::{
    PMC_ENABLE, PMC_INTR_EN_0, PmcEnableResult, bar0_probe, pgraph_engine_reset, pmc_enable,
    pmc_enable_full, pmc_enable_rollback,
};
pub(crate) use power::{cg_sweep, pgob_ungating, pri_bus_recover};

// ── Sovereign Experiment Infrastructure ──────────────────────────────

/// A complete register snapshot of all tier-relevant GPU domains.
///
/// Captured in a single pass via BAR0 MMIO reads. Supports diff display
/// to visualize the effect of experiment stages on the GPU state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SovereignSnapshot {
    pub pmc_enable: u32,
    pub pmc_intr_en: u32,
    pub pfifo_enable: u32,
    pub pgraph_status: u32,
    pub gpc_bcast: u32,
    pub fecs_cpuctl: u32,
    pub fecs_cpuctl_alias: u32,
    pub fecs_pc: u32,
    pub fecs_mailbox0: u32,
    pub gpccs_cpuctl: u32,
    pub gpccs_pc: u32,
    pub pmu_cpuctl: u32,
    pub pmu_pc: u32,
    /// Per-GPC unit registers at 0x500000 + gpc*0x8000 (up to 6 GPCs).
    pub gpc_per_unit: Vec<u32>,
    /// Per-GPC TPC0 registers at 0x504000 + gpc*0x8000 (up to 6 GPCs).
    pub gpc_tpc0: Vec<u32>,
    /// CE instance registers: CE0..CE5 at 0x104000 + i*0x1000.
    pub ce: Vec<u32>,
    pub pbdma0_intr: u32,
    pub therm_gate: u32,
    pub pri_ringmaster_intr: u32,
}

impl SovereignSnapshot {
    /// Capture a full register snapshot from BAR0 in one pass.
    pub(crate) fn capture(bar0: &MappedBar) -> Self {
        use crate::nv::registers::{ce, gpc, pgraph, pmc, pri};
        use crate::vfio::channel::registers::falcon;

        let r = |off: usize| bar0.read_u32(off).unwrap_or(0xDEAD_DEAD);

        let mut gpc_per_unit = Vec::with_capacity(6);
        let mut gpc_tpc0 = Vec::with_capacity(6);
        for g in 0..6u32 {
            gpc_per_unit.push(r(gpc::gpc_base(g) as usize));
            gpc_tpc0.push(r(gpc::gpc_tpc0(g) as usize));
        }

        let mut ce_vals = Vec::with_capacity(6);
        for i in 0..6u32 {
            ce_vals.push(r(ce::ce_base(i) as usize));
        }

        SovereignSnapshot {
            pmc_enable: r(pmc::ENABLE as usize),
            pmc_intr_en: r(pmc::INTR_EN_0 as usize),
            pfifo_enable: r(pgraph::PFIFO_ENABLE as usize),
            pgraph_status: r(pgraph::STATUS as usize),
            gpc_bcast: r(gpc::BCAST_ENABLES as usize),
            fecs_cpuctl: r(falcon::FECS_BASE + falcon::CPUCTL),
            fecs_cpuctl_alias: r(falcon::FECS_BASE + falcon::CPUCTL_ALIAS),
            fecs_pc: r(falcon::FECS_BASE + falcon::PC),
            fecs_mailbox0: r(falcon::FECS_BASE + falcon::MAILBOX0),
            gpccs_cpuctl: r(falcon::GPCCS_BASE + falcon::CPUCTL),
            gpccs_pc: r(falcon::GPCCS_BASE + falcon::PC),
            pmu_cpuctl: r(falcon::PMU_BASE + falcon::CPUCTL),
            pmu_pc: r(falcon::PMU_BASE + falcon::PC),
            gpc_per_unit,
            gpc_tpc0: gpc_tpc0,
            ce: ce_vals,
            pbdma0_intr: r(pgraph::PBDMA0_INTR as usize),
            therm_gate: r(pgraph::THERM_GATE as usize),
            pri_ringmaster_intr: r(pri::RINGMASTER_INTR_STATUS as usize),
        }
    }

    /// Produce a human-readable diff between `before` and `after` snapshots.
    pub(crate) fn diff(before: &Self, after: &Self) -> Vec<String> {
        let mut lines = Vec::new();
        macro_rules! cmp {
            ($field:ident, $name:expr) => {
                if before.$field != after.$field {
                    lines.push(format!(
                        "{}: {:#010x} -> {:#010x}",
                        $name, before.$field, after.$field
                    ));
                }
            };
        }
        cmp!(pmc_enable, "PMC_ENABLE");
        cmp!(pmc_intr_en, "PMC_INTR_EN");
        cmp!(pfifo_enable, "PFIFO_ENABLE");
        cmp!(pgraph_status, "PGRAPH_STATUS");
        cmp!(gpc_bcast, "GPC_BCAST");
        cmp!(fecs_cpuctl, "FECS_CPUCTL");
        cmp!(fecs_cpuctl_alias, "FECS_CPUCTL_ALIAS");
        cmp!(fecs_pc, "FECS_PC");
        cmp!(fecs_mailbox0, "FECS_MAILBOX0");
        cmp!(gpccs_cpuctl, "GPCCS_CPUCTL");
        cmp!(gpccs_pc, "GPCCS_PC");
        cmp!(pmu_cpuctl, "PMU_CPUCTL");
        cmp!(pmu_pc, "PMU_PC");
        cmp!(pbdma0_intr, "PBDMA0_INTR");
        cmp!(therm_gate, "THERM_GATE");
        cmp!(pri_ringmaster_intr, "PRI_RM_INTR");

        for (i, (b, a)) in before.gpc_per_unit.iter().zip(&after.gpc_per_unit).enumerate() {
            if b != a {
                lines.push(format!("GPC{i}_UNIT: {b:#010x} -> {a:#010x}"));
            }
        }
        for (i, (b, a)) in before.gpc_tpc0.iter().zip(&after.gpc_tpc0).enumerate() {
            if b != a {
                lines.push(format!("GPC{i}_TPC0: {b:#010x} -> {a:#010x}"));
            }
        }
        for (i, (b, a)) in before.ce.iter().zip(&after.ce).enumerate() {
            if b != a {
                lines.push(format!("CE{i}: {b:#010x} -> {a:#010x}"));
            }
        }

        if lines.is_empty() {
            lines.push("(no changes)".into());
        }
        lines
    }

    /// Produce a structured diff between two snapshots.
    ///
    /// Returns one [`SnapshotDelta`] per field that differs, suitable for
    /// JSON serialization and programmatic comparison.
    pub fn diff_structured(a: &Self, b: &Self) -> Vec<SnapshotDelta> {
        let mut deltas = Vec::new();
        macro_rules! cmp_field {
            ($field:ident, $name:expr) => {
                if a.$field != b.$field {
                    deltas.push(SnapshotDelta {
                        field: $name.into(),
                        before: a.$field,
                        after: b.$field,
                    });
                }
            };
        }
        cmp_field!(pmc_enable, "PMC_ENABLE");
        cmp_field!(pmc_intr_en, "PMC_INTR_EN");
        cmp_field!(pfifo_enable, "PFIFO_ENABLE");
        cmp_field!(pgraph_status, "PGRAPH_STATUS");
        cmp_field!(gpc_bcast, "GPC_BCAST");
        cmp_field!(fecs_cpuctl, "FECS_CPUCTL");
        cmp_field!(fecs_cpuctl_alias, "FECS_CPUCTL_ALIAS");
        cmp_field!(fecs_pc, "FECS_PC");
        cmp_field!(fecs_mailbox0, "FECS_MAILBOX0");
        cmp_field!(gpccs_cpuctl, "GPCCS_CPUCTL");
        cmp_field!(gpccs_pc, "GPCCS_PC");
        cmp_field!(pmu_cpuctl, "PMU_CPUCTL");
        cmp_field!(pmu_pc, "PMU_PC");
        cmp_field!(pbdma0_intr, "PBDMA0_INTR");
        cmp_field!(therm_gate, "THERM_GATE");
        cmp_field!(pri_ringmaster_intr, "PRI_RM_INTR");

        for (i, (va, vb)) in a.gpc_per_unit.iter().zip(&b.gpc_per_unit).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("GPC{i}_UNIT"),
                    before: *va,
                    after: *vb,
                });
            }
        }
        for (i, (va, vb)) in a.gpc_tpc0.iter().zip(&b.gpc_tpc0).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("GPC{i}_TPC0"),
                    before: *va,
                    after: *vb,
                });
            }
        }
        for (i, (va, vb)) in a.ce.iter().zip(&b.ce).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("CE{i}"),
                    before: *va,
                    after: *vb,
                });
            }
        }

        deltas
    }
}

/// A single field difference between two [`SovereignSnapshot`]s.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotDelta {
    /// Register/field name (e.g. "PMC_ENABLE", "GPC0_TPC0").
    pub field: String,
    /// Value in the first snapshot.
    pub before: u32,
    /// Value in the second snapshot.
    pub after: u32,
}

/// Result of a single experiment stage execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperimentResult {
    pub stage: u32,
    pub stage_name: String,
    pub before: SovereignSnapshot,
    pub after: SovereignSnapshot,
    pub diff: Vec<String>,
    pub writes: Vec<ExperimentWrite>,
    pub notes: Vec<String>,
}

/// A single BAR0 write performed during an experiment stage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperimentWrite {
    pub offset: String,
    pub value: String,
    pub readback: String,
}

impl ExperimentWrite {
    fn new(bar0: &MappedBar, offset: usize, value: u32) -> Self {
        let _ = bar0.write_u32(offset, value);
        let readback = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
        ExperimentWrite {
            offset: format!("{offset:#08x}"),
            value: format!("{value:#010x}"),
            readback: format!("{readback:#010x}"),
        }
    }
}

/// Read-only snapshot capture — no mutating writes.
///
/// Returns the current [`SovereignSnapshot`] and [`TierEvidence`] for the
/// given BAR0 mapping. Used by the `sovereign.snapshot` RPC.
pub fn sovereign_snapshot_only(
    bar0: &MappedBar,
) -> (SovereignSnapshot, crate::vfio::sovereign_tiers::TierEvidence) {
    let snapshot = SovereignSnapshot::capture(bar0);
    let tier = crate::vfio::sovereign_tiers::classify_tier(bar0);
    (snapshot, tier)
}

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
        let is_fault = crate::nv::pri::is_pri_fault(unit);
        if !is_fault {
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

/// Stage 4: GPC MMU init + sw_nonctx.bin replay.
///
/// Only proceeds if GPCs showed life in stage 2-3. Writes GPC MMU init
/// registers and replays sw_nonctx.bin firmware blob. Higher risk — large
/// write sequence.
pub(crate) fn experiment_stage_4(bar0: &MappedBar) -> ExperimentResult {
    experiment_stage_4_with_chip(bar0, "gv100", 70)
}

/// Stage 4 with explicit chip/SM version parameters.
pub(crate) fn experiment_stage_4_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    use crate::nv::registers::gpc;

    // Pre-check: is GPC domain alive?
    let gpc0 = bar0.read_u32(gpc::gpc_base(0) as usize).unwrap_or(0xDEAD_DEAD);
    let gpc_bcast = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0);
    let gpc_alive = !crate::nv::pri::is_pri_fault(gpc0)
        || (!crate::nv::pri::is_pri_fault(gpc_bcast) && gpc_bcast != 0);

    if !gpc_alive {
        notes.push(format!(
            "ABORT: GPC domain not alive (gpc0={gpc0:#010x}, bcast={gpc_bcast:#010x}). \
             Run stages 2-3 first."
        ));
        let after = SovereignSnapshot::capture(bar0);
        let diff = SovereignSnapshot::diff(&before, &after);
        return ExperimentResult {
            stage: 4,
            stage_name: "GPC MMU init (aborted — GPCs not alive)".into(),
            before,
            after,
            diff,
            writes,
            notes,
        };
    }

    // GPC MMU init sequence (from nouveau gf100_grctx)
    let mmu_writes: &[(usize, u32)] = &[
        (gpc::BCAST_MMU_CTRL as usize, 0x0000_0001),
        (gpc::BCAST_MMU_PM_UNIT_MASK as usize, 0x0000_0000),
        (gpc::BCAST_MMU_PM_REQ_MASK as usize, 0x0000_0000),
        (gpc::BCAST_MMU_DEBUG_B0 as usize, 0x0000_0000),
        (gpc::BCAST_MMU_DEBUG_WR as usize, 0xFFFF_FFFF),
        (gpc::BCAST_MMU_DEBUG_RD as usize, 0x0000_0007),
    ];

    for &(offset, value) in mmu_writes {
        writes.push(ExperimentWrite::new(bar0, offset, value));
    }
    notes.push(format!("GPC MMU init: {} writes applied", mmu_writes.len()));

    let bridge = crate::nv::nv_gsp_bridge::NvGspBridge::new(chip);
    use crate::nv::gsp_bridge::GspBridge;
    let has_fw = bridge.has_gr_firmware();
    notes.push(format!("NvGspBridge({chip}): firmware present = {has_fw}"));
    match bridge.apply_gr_bar0_init(bar0, sm) {
        Ok(()) => notes.push("sw_nonctx replay: completed with REAL firmware data".into()),
        Err(e) => notes.push(format!("sw_nonctx replay: {e}")),
    }

    // Probe TPC registers after sw_nonctx broadcast writes
    let tpc0_post = bar0.read_u32(gpc::gpc_tpc0(0) as usize).unwrap_or(0xDEAD_DEAD);
    let tpc0_sm_post = bar0.read_u32(gpc::gpc_tpc0_sm(0) as usize).unwrap_or(0xDEAD_DEAD);
    let bcast_tpc_post = bar0.read_u32(gpc::BCAST_TPC_CTRL as usize).unwrap_or(0xDEAD_DEAD);
    notes.push(format!(
        "Post-sw_nonctx TPC probe: tpc0_ctrl={tpc0_post:#010x}, \
         tpc0_sm={tpc0_sm_post:#010x}, bcast_tpc={bcast_tpc_post:#010x}"
    ));
    let tpc_alive = !crate::nv::pri::is_pri_fault(tpc0_post);
    notes.push(format!("TPC PRI station alive = {tpc_alive}"));

    // Post-init PRI recovery
    let pri = pri_bus_recover(bar0);
    notes.push(format!(
        "Post-init PRI: alive={}, faulted={}, recovered={}",
        pri.alive, pri.faulted, pri.recovered
    ));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 4,
        stage_name: "GPC MMU init + sw_nonctx replay".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 5: FECS resume via CPUCTL_ALIAS.
///
/// Attempts to resume the halted FECS falcon by writing STARTCPU to
/// CPUCTL_ALIAS (0x409130). Only proceeds if GPCs are alive. Polls PC
/// for advancement, then tries INIT_CTXSW. Medium risk — FECS may
/// trigger falcon exception if TPCs are still gated.
pub(crate) fn experiment_stage_5(bar0: &MappedBar) -> ExperimentResult {
    use crate::nv::registers::gpc;
    use crate::vfio::channel::registers::falcon;

    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Pre-check: GPCs alive?
    let gpc0 = bar0.read_u32(gpc::gpc_base(0) as usize).unwrap_or(0xDEAD_DEAD);
    let gpc_bcast = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0);
    let gpc_alive = !crate::nv::pri::is_pri_fault(gpc0)
        || (!crate::nv::pri::is_pri_fault(gpc_bcast) && gpc_bcast != 0);

    if !gpc_alive {
        notes.push(format!(
            "ABORT: GPCs not alive (gpc0={gpc0:#010x}, bcast={gpc_bcast:#010x}). \
             Run stages 2-4 first."
        ));
        let after = SovereignSnapshot::capture(bar0);
        let diff = SovereignSnapshot::diff(&before, &after);
        return ExperimentResult {
            stage: 5,
            stage_name: "FECS resume (aborted — GPCs not alive)".into(),
            before,
            after,
            diff,
            writes,
            notes,
        };
    }

    // Capture pre-resume FECS state
    let pc_before = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    let cpuctl_before = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    notes.push(format!(
        "FECS pre-resume: cpuctl_alias={cpuctl_before:#010x}, pc={pc_before:#010x}"
    ));

    // Write STARTCPU to CPUCTL_ALIAS to resume the halted falcon
    writes.push(ExperimentWrite::new(
        bar0,
        falcon::FECS_BASE + falcon::CPUCTL_ALIAS,
        falcon::CPUCTL_STARTCPU,
    ));
    notes.push("FECS CPUCTL_ALIAS <- STARTCPU (0x02)".into());

    // Poll FECS PC for advancement (up to 50ms)
    std::thread::sleep(Duration::from_millis(5));
    let mut pc_advanced = false;
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut last_pc = pc_before;
    while Instant::now() < deadline {
        last_pc = bar0
            .read_u32(falcon::FECS_BASE + falcon::PC)
            .unwrap_or(0xDEAD);
        if last_pc != pc_before && !crate::nv::pri::is_pri_fault(last_pc) {
            pc_advanced = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    let cpuctl_after = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    let halted = cpuctl_after & falcon::CPUCTL_HALTED != 0;
    let in_hreset = cpuctl_after & falcon::CPUCTL_HRESET != 0;
    notes.push(format!(
        "FECS post-resume: cpuctl_alias={cpuctl_after:#010x} (halted={halted}, hreset={in_hreset}), \
         pc={last_pc:#010x} (advanced={pc_advanced})"
    ));

    // If PC advanced, try INIT_CTXSW via FECS method mailbox
    if pc_advanced && !halted {
        notes.push("FECS running — attempting INIT_CTXSW".into());
        let mb0_before = bar0
            .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
            .unwrap_or(0);

        // Write method: INIT_CTXSW = 0x10
        let _ = bar0.write_u32(falcon::FECS_BASE + falcon::MTHD_DATA, 0);
        writes.push(ExperimentWrite::new(
            bar0,
            falcon::FECS_BASE + falcon::MTHD_CMD,
            0x8000_0010, // INIT_CTXSW with trigger bit
        ));

        // Poll for completion (bit 0 of MTHD_CMD clears)
        let mthd_deadline = Instant::now() + Duration::from_millis(100);
        let mut mthd_done = false;
        while Instant::now() < mthd_deadline {
            let cmd = bar0
                .read_u32(falcon::FECS_BASE + falcon::MTHD_CMD)
                .unwrap_or(0xDEAD);
            if cmd & 0x8000_0000 == 0 {
                mthd_done = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }

        let mb0_after = bar0
            .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
            .unwrap_or(0);
        notes.push(format!(
            "INIT_CTXSW: done={mthd_done}, mb0: {mb0_before:#010x} -> {mb0_after:#010x}"
        ));
    } else if !pc_advanced {
        notes.push("FECS PC did not advance — falcon may need full re-bootstrap".into());
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 5,
        stage_name: "FECS resume via CPUCTL_ALIAS".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 6: Full 5-phase ungating sequence (Exp 217).
///
/// Combines CG sweep + PRI recovery + PGOB + real `sw_nonctx.bin` +
/// destructive PGRAPH reset + PRI re-enumerate + second `sw_nonctx.bin`
/// replay. This is the "throw everything at it" sequence from
/// `compute_device.rs`, extracted here for controlled experiment use.
///
/// Higher risk than stages 1-5 — includes PGRAPH engine reset which
/// may change FECS state. Use after stages 1-3 confirm GPC fabric is
/// alive but TPCs remain PRI-faulted.
pub(crate) fn experiment_stage_6(bar0: &MappedBar) -> ExperimentResult {
    experiment_stage_6_with_chip(bar0, "gv100", 70)
}

/// Stage 6 with explicit chip/SM version parameters.
pub(crate) fn experiment_stage_6_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    use std::time::Duration;

    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    let bridge = crate::nv::nv_gsp_bridge::NvGspBridge::new(chip);
    use crate::nv::gsp_bridge::GspBridge;

    // Phase 1: CG sweep + PRI recovery + PGOB
    let cg = cg_sweep(bar0);
    notes.push(format!(
        "Phase 1a: CG sweep — {} changes, {} faulted",
        cg.changes, cg.faulted
    ));

    let pri1 = pri_bus_recover(bar0);
    notes.push(format!(
        "Phase 1b: PRI recovery — alive={}, faulted={}",
        pri1.alive, pri1.faulted
    ));

    use crate::nv::registers::{gpc, pmc as pmc_reg, pri};

    // PGOB ungate via GPC broadcast
    let pgob_regs: &[(usize, u32)] = &[
        (pmc_reg::CLKGATE_DISABLE as usize, 0x0000_0000),
        (gpc::BCAST_PGOB as usize, 0x0000_0000),
        (gpc::BCAST_CONTROL as usize, 0x0000_0110),
    ];
    for &(off, val) in pgob_regs {
        writes.push(ExperimentWrite::new(bar0, off, val));
    }
    notes.push("Phase 1c: PGOB ungate broadcast writes applied".into());

    // Phase 2: Force PRI enumerate
    writes.push(ExperimentWrite::new(bar0, pri::COMMAND as usize, 2));
    std::thread::sleep(Duration::from_millis(10));
    notes.push("Phase 2: Forced PRI ringmaster enumerate".into());

    // Phase 3: GPC MMU init
    let mmu_writes: &[(usize, u32)] = &[
        (gpc::BCAST_MMU_CTRL as usize, 0x0000_0001),
        (gpc::BCAST_MMU_PM_UNIT_MASK as usize, 0x0000_0000),
        (gpc::BCAST_MMU_PM_REQ_MASK as usize, 0x0000_0000),
        (gpc::BCAST_MMU_DEBUG_B0 as usize, 0x0000_0000),
        (gpc::BCAST_MMU_DEBUG_WR as usize, 0xFFFF_FFFF),
        (gpc::BCAST_MMU_DEBUG_RD as usize, 0x0000_0007),
    ];
    for &(off, val) in mmu_writes {
        writes.push(ExperimentWrite::new(bar0, off, val));
    }

    // Extra GPC MMU writes from nouveau gm200_gr_init_gpc_mmu
    let a4 = bar0.read_u32(gpc::BCAST_MMU_DEBUG_CTRL as usize).unwrap_or(0);
    writes.push(ExperimentWrite::new(bar0, gpc::BCAST_MMU_DEBUG_CTRL as usize, a4 | 0x0300_0000));
    notes.push("Phase 3: GPC MMU init + extended MMU writes".into());

    // Phase 4: sw_nonctx.bin replay
    match bridge.apply_gr_bar0_init(bar0, sm) {
        Ok(()) => notes.push("Phase 4: sw_nonctx.bin replay completed".into()),
        Err(e) => notes.push(format!("Phase 4: sw_nonctx.bin replay failed: {e}")),
    }

    // Phase 5: Second PRI recovery after sw_nonctx writes
    let pri2 = pri_bus_recover(bar0);
    notes.push(format!(
        "Phase 5: Post-init PRI recovery — alive={}, faulted={}",
        pri2.alive, pri2.faulted
    ));

    use crate::nv::registers::ce;
    use crate::vfio::channel::registers::falcon;

    // Probe TPC + CE state post-ungating
    let tpc0 = bar0.read_u32(gpc::gpc_tpc0(0) as usize).unwrap_or(0xDEAD_DEAD);
    let tpc0_sm = bar0.read_u32(gpc::gpc_tpc0_sm(0) as usize).unwrap_or(0xDEAD_DEAD);
    let ce0 = bar0.read_u32(ce::ce_base(0) as usize).unwrap_or(0xDEAD_DEAD);
    let ce4 = bar0.read_u32(ce::ce_base(4) as usize).unwrap_or(0xDEAD_DEAD);
    let fecs_pc = bar0.read_u32(falcon::FECS_BASE + falcon::PC).unwrap_or(0);
    let gpc0 = bar0.read_u32(gpc::gpc_base(0) as usize).unwrap_or(0xDEAD_DEAD);

    notes.push(format!("Final TPC probe: tpc0_ctrl={tpc0:#010x}, tpc0_sm={tpc0_sm:#010x}"));
    notes.push(format!("Final CE probe: ce0={ce0:#010x}, ce4={ce4:#010x}"));
    notes.push(format!("Final state: gpc0={gpc0:#010x}, fecs_pc={fecs_pc:#010x}"));

    let tpc_alive = !crate::nv::pri::is_pri_fault(tpc0);
    notes.push(format!("TPC PRI station alive = {tpc_alive}"));

    // If TPC still faulted, try destructive PGRAPH reset as last resort
    if !tpc_alive {
        notes.push("TPC still faulted — attempting destructive PGRAPH reset".into());

        // PMC GR engine reset: clear bit 12, wait, set bit 12
        let pmc_val = bar0.read_u32(pmc_reg::ENABLE as usize).unwrap_or(0);
        let _ = bar0.write_u32(pmc_reg::ENABLE as usize, pmc_val & !(1 << 12));
        std::thread::sleep(Duration::from_millis(10));
        let _ = bar0.write_u32(pmc_reg::ENABLE as usize, pmc_val | (1 << 12));
        std::thread::sleep(Duration::from_millis(50));
        notes.push("PGRAPH reset: PMC bit 12 toggled".into());

        // PRI re-enumerate after reset
        let _ = bar0.write_u32(pri::COMMAND as usize, 2);
        std::thread::sleep(Duration::from_millis(10));

        // Re-apply sw_nonctx.bin after reset
        match bridge.apply_gr_bar0_init(bar0, 70) {
            Ok(()) => notes.push("Post-reset sw_nonctx.bin replay completed".into()),
            Err(e) => notes.push(format!("Post-reset sw_nonctx.bin replay failed: {e}")),
        }

        // Final PRI recovery
        let pri3 = pri_bus_recover(bar0);
        notes.push(format!(
            "Post-reset PRI recovery — alive={}, faulted={}",
            pri3.alive, pri3.faulted
        ));

        let tpc0_final = bar0.read_u32(gpc::gpc_tpc0(0) as usize).unwrap_or(0xDEAD_DEAD);
        let tpc_alive_final = !crate::nv::pri::is_pri_fault(tpc0_final);
        notes.push(format!(
            "Post-reset TPC: tpc0={tpc0_final:#010x}, alive={tpc_alive_final}"
        ));
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 6,
        stage_name: "Full 5-phase ungating + PGRAPH reset (Exp 217)".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Auto-detect chip identity from BAR0 MMIO reads.
///
/// Probes NVIDIA BOOT0 at offset 0 first; if unrecognized, probes AMD GRBM_STATUS
/// at offset 0x8010. Distinguishes:
/// - NVIDIA GPU (chip name + SM version)
/// - AMD GPU present (cold boot not implemented — probe-only via [`VegaInit`])
/// - No responsive GPU (unmapped BAR0 or all-ones reads)
///
/// [`VegaInit`]: crate::vfio::amd_metal::VegaInit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChipDetection {
    /// NVIDIA GPU identified from BOOT0.
    Nvidia {
        /// Chip codename (e.g. `"gv100"`).
        chip: &'static str,
        /// SM architecture version (e.g. 70).
        sm: u32,
    },
    /// AMD GPU identified from GRBM register map.
    ///
    /// Warm detection works via [`VegaInit::probe`]; cold boot (`devinit`,
    /// `engine_init`) is not implemented.
    AmdPresent {
        /// GPU family label (e.g. `"Vega 20"`).
        family: &'static str,
        /// GRBM_STATUS register value at probe time.
        grbm_status: u32,
    },
    /// BAR0 reads indicate no responsive GPU.
    NotFound {
        /// BOOT0 (offset 0) read value.
        boot0: u32,
        /// GRBM_STATUS (offset 0x8010) read value.
        grbm_status: u32,
    },
}

impl ChipDetection {
    /// Human-readable diagnostic for operators and experiment logs.
    #[must_use]
    pub fn diagnostic(&self) -> String {
        match self {
            Self::Nvidia { chip, sm } => format!("NVIDIA {chip} (SM {sm})"),
            Self::AmdPresent { family, grbm_status } => format!(
                "AMD {family} present (GRBM_STATUS=0x{grbm_status:08x}) — \
                 cold boot not implemented; warm probe via VegaInit only"
            ),
            Self::NotFound { boot0, grbm_status } => format!(
                "no GPU found (BOOT0=0x{boot0:08x}, GRBM_STATUS=0x{grbm_status:08x})"
            ),
        }
    }
}

/// AMD Vega GRBM_STATUS offset within BAR0 (GFX906 / MI50 register map).
pub(crate) const AMD_GRBM_STATUS: u32 = 0x8010;

/// Auto-detect chip from BAR0 MMIO.
#[must_use]
pub fn detect_chip(bar0: &MappedBar) -> ChipDetection {
    let boot0 = bar0.read_u32(0x0000_0000).unwrap_or(0xFFFF_FFFF);

    if boot0 != 0 && boot0 != 0xFFFF_FFFF {
        if let Some(sm) = crate::nv::identity::boot0_to_sm(boot0) {
            let chip = crate::nv::identity::chip_name(sm);
            return ChipDetection::Nvidia { chip, sm };
        }
    }

    let grbm_status = bar0
        .read_u32(AMD_GRBM_STATUS as usize)
        .unwrap_or(0xFFFF_FFFF);
    if grbm_status != 0 && grbm_status != 0xFFFF_FFFF {
        tracing::info!(
            grbm_status = format!("0x{grbm_status:08x}"),
            "detect_chip: AMD GPU present — cold boot not implemented"
        );
        return ChipDetection::AmdPresent {
            family: "Vega 20",
            grbm_status,
        };
    }

    ChipDetection::NotFound {
        boot0,
        grbm_status,
    }
}

/// Legacy `(chip_name, sm_version)` tuple for experiment stages.
///
/// Returns `("unknown", 0)` for AMD or unrecognized hardware.
pub(crate) fn detect_chip_legacy(bar0: &MappedBar) -> (&'static str, u32) {
    match detect_chip(bar0) {
        ChipDetection::Nvidia { chip, sm } => (chip, sm),
        ChipDetection::AmdPresent { .. } => ("amd-vega20", 0),
        ChipDetection::NotFound { .. } => ("unknown", 0),
    }
}

/// Execute an experiment stage by number (1-6).
///
/// Accepts an optional `chip` override (e.g. `"gv100"`, `"gk210"`).
/// When `None`, auto-detects from BOOT0.
pub fn run_experiment_stage(
    bar0: &MappedBar,
    stage: u32,
    chip_override: Option<&str>,
) -> Result<ExperimentResult, String> {
    let (auto_chip, auto_sm) = detect_chip_legacy(bar0);
    let chip = chip_override.unwrap_or(auto_chip);
    let sm = auto_sm;

    match stage {
        1 => Ok(experiment_stage_1(bar0)),
        2 => Ok(experiment_stage_2(bar0)),
        3 => Ok(experiment_stage_3(bar0)),
        4 => Ok(experiment_stage_4_with_chip(bar0, chip, sm)),
        5 => Ok(experiment_stage_5(bar0)),
        6 => Ok(experiment_stage_6_with_chip(bar0, chip, sm)),
        _ => Err(format!("invalid stage {stage}: must be 1-6")),
    }
}
