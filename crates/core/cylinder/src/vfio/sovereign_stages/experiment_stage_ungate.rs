// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::nv::gsp_bridge::GspBridge;
use crate::nv::nv_gsp_bridge::NvGspBridge;
use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;

use super::experiment_snapshot::{ExperimentResult, ExperimentWrite, SovereignSnapshot};
use super::power::{cg_sweep, pri_bus_recover};

/// Stage 4: GPC MMU init + sw_nonctx.bin replay.
///
/// Only proceeds if GPCs showed life in stage 2-3. Writes GPC MMU init
/// registers and replays sw_nonctx.bin firmware blob. Higher risk — large
/// write sequence.
pub(crate) fn experiment_stage_4_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    use crate::nv::registers::gpc;

    // Pre-check: is GPC domain alive?
    let gpc0 = bar0.read_u32(gpc::gpc_base(0) as usize).unwrap_or(0xDEAD_DEAD);
    let gpc_bcast = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0);
    let gpc_alive = !is_pri_fault(gpc0)
        || (!is_pri_fault(gpc_bcast) && gpc_bcast != 0);

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

    let bridge = NvGspBridge::new(chip);
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
    let tpc_alive = !is_pri_fault(tpc0_post);
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
    let gpc_alive = !is_pri_fault(gpc0)
        || (!is_pri_fault(gpc_bcast) && gpc_bcast != 0);

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
        if last_pc != pc_before && !is_pri_fault(last_pc) {
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

/// Stage 6: Full 5-phase ungating + PGRAPH reset (Exp 217).
///
/// Higher risk than stages 1-5 — includes PGRAPH engine reset which
/// may change FECS state. Use after stages 1-3 confirm GPC fabric is
/// alive but TPCs remain PRI-faulted.
pub(crate) fn experiment_stage_6_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    let bridge = NvGspBridge::new(chip);

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

    let tpc_alive = !is_pri_fault(tpc0);
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
        let tpc_alive_final = !is_pri_fault(tpc0_final);
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
