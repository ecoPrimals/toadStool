// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_tiers::classify_tier;

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
            gpc_tpc0,
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
    pub(crate) fn new(bar0: &MappedBar, offset: usize, value: u32) -> Self {
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
    let tier = classify_tier(bar0);
    (snapshot, tier)
}
