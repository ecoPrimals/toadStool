// SPDX-License-Identifier: AGPL-3.0-or-later
//! GR engine ungating — CG sweep, PRI recovery, PGOB, GPC MMU, sw_nonctx.bin replay.

use crate::nv::gsp_bridge::GspBridge;
use crate::nv::registers::{gpc, pfb, pmc, pri as nv_pri};
use crate::vfio::channel::registers::pri;
use crate::vfio::device::MappedBar;

use super::super::generation::GenerationProfile;
use super::super::nv_gsp_bridge::NvGspBridge;

/// Result of a GR ungating sequence.
pub(crate) struct UngatingResult {
    pub cg_changes: u32,
    pub pri_alive: usize,
    #[expect(
        dead_code,
        reason = "diagnostic field — populated for tracing, not consumed yet"
    )]
    pub pri_faulted: usize,
}

/// Logging context for GR ungating operations.
pub(crate) struct UngatingLog<'a> {
    pub prefix: &'a str,
    /// Log CG/PRI/PGOB phases with tracing::info.
    pub log_phases: bool,
    /// Ack PMC PRI interrupt after force enumerate.
    pub ack_pmc_intr: bool,
    /// Log GPC MMU init with register values.
    pub verbose_mmu: bool,
}

/// Run the standard GR ungating sequence: CG sweep → PRI → PGOB → force
/// enumerate → GPC MMU init → sw_nonctx.bin → post-init PRI recovery.
pub(crate) fn ungate_gr_engine(
    bar0: &MappedBar,
    bridge: &NvGspBridge,
    profile: &GenerationProfile,
    log: UngatingLog<'_>,
) -> UngatingResult {
    let cg = crate::vfio::sovereign_stages::cg_sweep(bar0);
    if log.log_phases {
        tracing::info!(
            changes = cg.changes,
            faulted = cg.faulted,
            "{}: CG sweep",
            log.prefix
        );
    }

    let pri = crate::vfio::sovereign_stages::pri_bus_recover(bar0);
    if log.log_phases {
        tracing::info!(
            alive = pri.alive,
            faulted = pri.faulted,
            "{}: PRI recovery",
            log.prefix
        );
    }

    if log.log_phases {
        match crate::vfio::sovereign_stages::pgob_ungating(bar0, bridge) {
            Ok(detail) => tracing::info!(%detail, "{}: PGOB", log.prefix),
            Err(e) => tracing::warn!(%e, "{}: PGOB failed", log.prefix),
        }
    } else {
        let _ = crate::vfio::sovereign_stages::pgob_ungating(bar0, bridge);
    }

    force_pri_enumerate(bar0, log.ack_pmc_intr, log.prefix, log.log_phases);
    init_gpc_mmu(bar0, log.verbose_mmu, log.prefix);

    let _ = bridge.apply_gr_bar0_init(bar0, *profile.sm_range.start());

    let pri2 = crate::vfio::sovereign_stages::pri_bus_recover(bar0);
    if log.log_phases {
        tracing::info!(alive = pri2.alive, "{}: post-init PRI recovery", log.prefix);
    }

    UngatingResult {
        cg_changes: cg.changes,
        pri_alive: pri2.alive,
        pri_faulted: pri2.faulted,
    }
}

/// Force PRI ring enumerate unconditionally.
pub(crate) fn force_pri_enumerate(bar0: &MappedBar, ack_pmc_intr: bool, prefix: &str, log: bool) {
    let _ = bar0.write_u32(pri::PRI_RINGMASTER_INTR_STATUS, 0xFFFF_FFFF);
    let _ = bar0.write_u32(
        pri::PRI_RINGMASTER_COMMAND,
        pri::PRI_RINGMASTER_CMD_ENUMERATE,
    );
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = bar0.write_u32(nv_pri::STATION_ACK as usize, 2);
    if ack_pmc_intr {
        let pmc_intr = bar0.read_u32(pmc::INTR as usize).unwrap_or(0);
        if pmc_intr & (1 << 26) != 0 {
            let _ = bar0.write_u32(pmc::INTR as usize, 1 << 26);
        }
    }
    if log {
        tracing::info!("{prefix}: forced PRI ring enumerate + ACK");
    }
}

/// GPC MMU init (nouveau gm200_gr_init_gpc_mmu).
pub(crate) fn init_gpc_mmu(bar0: &MappedBar, verbose: bool, prefix: &str) {
    let shadow_base = gpc::BCAST_MMU_DEBUG_CTRL - 0x24;
    let fb_mmu = bar0.read_u32(pfb::MMU_CTRL as usize).unwrap_or(0);
    let _ = bar0.write_u32(shadow_base as usize, fb_mmu & 0x0001_FFFF);
    let _ = bar0.write_u32((shadow_base + 0x10) as usize, 0);
    let _ = bar0.write_u32((shadow_base + 0x14) as usize, 0);
    let cc4 = bar0.read_u32(0x100cc4).unwrap_or(0);
    let cc8 = bar0.read_u32(0x100cc8).unwrap_or(0);
    let ccc = bar0.read_u32(0x100ccc).unwrap_or(0);
    let _ = bar0.write_u32((shadow_base + 0x30) as usize, cc4);
    let _ = bar0.write_u32((shadow_base + 0x34) as usize, cc8);
    let _ = bar0.write_u32((shadow_base + 0x38) as usize, ccc);
    // GV100 specific: enable additional MMU modes
    let a4 = bar0
        .read_u32(gpc::BCAST_MMU_DEBUG_CTRL as usize)
        .unwrap_or(0);
    let _ = bar0.write_u32(gpc::BCAST_MMU_DEBUG_CTRL as usize, a4 | 0x0300_0000);
    if verbose {
        tracing::info!(
            fb_mmu = format_args!("{fb_mmu:#010x}"),
            a4_after = format_args!("{:#010x}", a4 | 0x0300_0000),
            "{prefix}: GPC MMU init"
        );
    }
}
