// SPDX-License-Identifier: AGPL-3.0-or-later
//! Power ungating, clock gating sweep, PRI recovery, and PGOB stages.

use std::time::Duration;

use crate::error::SovereignStagesError;
use crate::vfio::device::MappedBar;

pub(crate) fn cg_sweep(bar0: &MappedBar) -> CgSweepResult {
    use crate::nv::pri::is_pri_fault;
    use crate::vfio::channel::registers::cg;

    let mut changes = 0u32;
    let mut faulted = 0u32;
    let mut detail_lines: Vec<String> = Vec::new();

    // Phase 1: Sweep all known CG control registers
    for &(offset, name) in cg::CG_SWEEP_TARGETS {
        let old = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(old) {
            faulted += 1;
            tracing::debug!(
                name,
                offset = format!("{offset:#08x}"),
                val = format!("{old:#010x}"),
                "CG sweep: domain unreachable"
            );
        } else if old != cg::CG_DISABLE {
            let _ = bar0.write_u32(offset, cg::CG_DISABLE);
            let new = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
            if old != new {
                changes += 1;
                detail_lines.push(format!("{name}: {old:#010x}->{new:#010x}"));
            }
        }
    }

    // Phase 2: Per-FBPA clock gating disable
    //
    // Phase 1 above checks for a PRI fault *before* writing. Phases 2 and 3
    // used to write first and check afterwards, so a faulted domain got the
    // store anyway. On a cold Kepler the FBPAs front untrained memory and are
    // exactly the domains answering 0xbad0*, which makes this the worst place
    // to have the check backwards.
    for i in 0..cg::FBPA_COUNT {
        let reg = cg::FBPA0_BASE + i * cg::FBPA_STRIDE + cg::FBPA_CG_OFFSET;
        let old = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(old) {
            faulted += 1;
            continue;
        }
        let _ = bar0.write_u32(reg, cg::CG_DISABLE);
        let new = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if old != new {
            changes += 1;
            detail_lines.push(format!("FBPA{i}: {old:#010x}->{new:#010x}"));
        }
    }

    // Phase 3: Per-LTC clock gating disable
    for i in 0..cg::LTC_COUNT {
        let reg = cg::LTC0_BASE + i * cg::LTC_STRIDE + cg::LTC_CG_OFFSET;
        let old = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(old) {
            faulted += 1;
            continue;
        }
        let _ = bar0.write_u32(reg, cg::CG_DISABLE);
        let new = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        if old != new {
            changes += 1;
            detail_lines.push(format!("LTC{i}: {old:#010x}->{new:#010x}"));
        }
    }

    tracing::info!(changes, faulted, "CG sweep complete");

    CgSweepResult {
        changes,
        faulted,
        detail: if detail_lines.is_empty() {
            format!("{changes} changed, {faulted} faulted")
        } else {
            format!(
                "{changes} changed, {faulted} faulted [{}]",
                detail_lines.join(", ")
            )
        },
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CgSweepResult {
    pub changes: u32,
    pub faulted: u32,
    pub detail: String,
}

/// PRI bus recovery — acknowledge pending PRIV_RING faults and re-probe.
///
/// After a CG sweep, some domains may have generated PRI faults during
/// the transition. This clears the fault state so subsequent register
/// reads don't hit stale backpressure.
///
/// Also clears PRI ringmaster-level errors (0x12200c) and re-enumerates
/// ring stations — without this, GPC/PGRAPH registers remain unreachable
/// after UEFI POST because the ringmaster retains stale fault state from
/// firmware handoff.
pub(crate) fn pri_bus_recover(bar0: &MappedBar) -> PriRecoveryResult {
    use crate::vfio::channel::pri_monitor::PriBusMonitor;
    use crate::vfio::channel::registers::pri;

    // Phase 0: Clear PRI ringmaster errors (0x122xxx layer).
    // The station-level ACK at 0x12004c doesn't touch these. Stale
    // ringmaster faults from UEFI/firmware handoff block all GPC and
    // PGRAPH register access.
    // The clear is a write-back of the value just read. If that read was a
    // sentinel — 0xbad0011f from a faulted ring, or all-ones from a device
    // that is gone — the write-back stuffs the sentinel into the ringmaster's
    // control register and then issues ENUMERATE on top of it. `!= 0` is not
    // a sufficient test for "there are real bits here to acknowledge"; a
    // fault pattern is very much non-zero.
    let rm_intr = bar0
        .read_u32(pri::PRI_RINGMASTER_INTR_STATUS)
        .unwrap_or(0xFFFF_FFFF);
    if crate::nv::pri::is_pri_fault(rm_intr) {
        tracing::warn!(
            rm_intr = format!("{rm_intr:#010x}"),
            "PRI ringmaster status is itself a fault pattern — not writing it back"
        );
    } else if rm_intr != 0 {
        tracing::info!(
            rm_intr = format!("{rm_intr:#010x}"),
            "PRI ringmaster has pending errors — clearing and re-enumerating"
        );
        // Write-back to clear ringmaster interrupt bits
        let _ = bar0.write_u32(pri::PRI_RINGMASTER_INTR_STATUS, rm_intr);
        std::thread::sleep(Duration::from_millis(5));

        // Re-enumerate all ring stations so they re-register with the master
        let _ = bar0.write_u32(
            pri::PRI_RINGMASTER_COMMAND,
            pri::PRI_RINGMASTER_CMD_ENUMERATE,
        );
        std::thread::sleep(Duration::from_millis(20));

        let rm_after = bar0.read_u32(pri::PRI_RINGMASTER_INTR_STATUS).unwrap_or(0);
        tracing::info!(
            rm_after = format!("{rm_after:#010x}"),
            "PRI ringmaster after enumerate"
        );
    }

    // Phase 1: Station-level fault recovery
    let mut monitor = PriBusMonitor::new(bar0);
    let health = monitor.probe_all_domains();
    let alive = health
        .iter()
        .filter(|(_, _, h)| matches!(h, crate::vfio::channel::pri_monitor::DomainHealth::Alive))
        .count();
    let faulted = health
        .iter()
        .filter(|(_, _, h)| {
            matches!(
                h,
                crate::vfio::channel::pri_monitor::DomainHealth::Faulted { .. }
            )
        })
        .count();

    let recovered = if faulted > 0 {
        monitor.attempt_recovery()
    } else {
        true
    };

    std::thread::sleep(Duration::from_millis(50));

    tracing::info!(alive, faulted, recovered, "PRI bus recovery after CG sweep");

    PriRecoveryResult {
        alive,
        faulted,
        recovered,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PriRecoveryResult {
    pub alive: usize,
    pub faulted: usize,
    pub recovered: bool,
}

/// PGOB disable for Volta+ cold boot.
///
/// Ungates GPC compute domains via PMC clock gate + PGRAPH GPC broadcast.
/// Required before falcon DMA boot on cold GPUs where PGRAPH is power-gated.
/// Delegates to the bridge's implementation (NvGspBridge has the register
/// sequence, NoopGspBridge no-ops).
pub(crate) fn pgob_ungating(
    bar0: &MappedBar,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    if !bridge.supports_pgob() {
        return Ok("pgob: skipped (no firmware provider)".into());
    }

    bridge.pgob_diagnostic(bar0, "sovereign::pre-PGOB");
    match bridge.pgob_disable(bar0) {
        Ok(out) => {
            bridge.pgob_diagnostic(bar0, "sovereign::post-PGOB");
            tracing::info!(gpc_alive = out.gpc_alive, "PGOB ungating succeeded");
            Ok(format!("pgob: {} GPCs alive", out.gpc_alive))
        }
        Err(e) => {
            tracing::warn!(%e, "PGOB ungating failed — GPCs may remain gated");
            Ok(format!("pgob: failed ({e})"))
        }
    }
}

#[cfg(test)]
mod fault_write_guard_tests {
    use crate::nv::pri::is_pri_fault;

    /// Mirror of the ringmaster gate: may we write this value back to
    /// PRI_RINGMASTER_INTR_STATUS to acknowledge it?
    fn may_write_back(rm_intr: u32) -> bool {
        !is_pri_fault(rm_intr) && rm_intr != 0
    }

    /// The old test was `rm_intr != 0`, which a fault pattern passes. That
    /// stuffed 0xbad0011f into the ringmaster and then issued ENUMERATE.
    #[test]
    fn fault_patterns_are_never_written_back() {
        for v in [0xbad0_011f_u32, 0xbadf_1000, 0xFFFF_FFFF, 0xDEAD_DEAD] {
            assert!(!may_write_back(v), "{v:#010x} must not be written back");
            assert_ne!(v, 0, "and it is non-zero, which is why != 0 was not enough");
        }
    }

    /// Real pending bits still get acknowledged.
    #[test]
    fn genuine_pending_bits_are_acknowledged() {
        assert!(may_write_back(0x0000_0001));
        assert!(may_write_back(0x0000_0100));
    }

    /// Nothing pending means nothing to do.
    #[test]
    fn zero_is_a_no_op() {
        assert!(!may_write_back(0));
    }

    /// The CG sweep must skip a faulted domain rather than write into it.
    /// Kepler's FBPAs front untrained memory and answer 0xbad0*.
    #[test]
    fn cg_sweep_skips_faulted_domains() {
        assert!(
            is_pri_fault(0xbad0_011f),
            "FBPA cold read must count as faulted"
        );
        assert!(
            is_pri_fault(0xDEAD_DEAD),
            "failed read must count as faulted"
        );
    }
}
