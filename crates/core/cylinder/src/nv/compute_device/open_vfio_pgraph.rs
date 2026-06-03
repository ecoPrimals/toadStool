// SPDX-License-Identifier: AGPL-3.0-or-later
//! PGRAPH ungating and FECS channel setup after deferred GR falcon boot.

use crate::error::DriverResult;
use crate::nv::gsp_bridge::GspBridge;
use crate::nv::registers::pgraph;
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::{DmaBackend, MappedBar};

use super::super::generation::GenerationProfile;
use super::super::nv_gsp_bridge::NvGspBridge;
use super::gr_falcon_boot::{fecs_setup_channel, reboot_fecs_after_reset};
use super::gr_ungating::{force_pri_enumerate, ungate_gr_engine, UngatingLog};

/// Handle PGRAPH ungating and FECS channel setup after deferred boot.
pub(super) fn handle_pgraph_ungating_and_fecs_setup(
    bar0: &MappedBar,
    dma_backend: &DmaBackend,
    channel: &VfioChannel,
    profile: &GenerationProfile,
    _bdf: &str,
    mut fecs_hs_booted: bool,
) -> DriverResult<bool> {
    use crate::vfio::channel::registers::falcon;

    let mthd_cmd_probe = bar0
        .read_u32(falcon::FECS_BASE + falcon::MTHD_CMD)
        .unwrap_or(0xDEAD);
    let pgraph_gated = mthd_cmd_probe & 0xBAD0_0000 == 0xBAD0_0000;

    if pgraph_gated {
        tracing::info!(
            mthd_cmd_probe = format_args!("{mthd_cmd_probe:#010x}"),
            "PGRAPH method registers gated — running full GPC ungating"
        );

        let bridge = NvGspBridge::new(profile.firmware_chip);

        let _ = ungate_gr_engine(
            bar0,
            &bridge,
            profile,
            UngatingLog {
                prefix: "ungating",
                log_phases: true,
                ack_pmc_intr: true,
                verbose_mmu: true,
            },
        );
        tracing::info!("ungating: sw_nonctx.bin applied");

        let pri2 = crate::vfio::sovereign_stages::pri_bus_recover(bar0);
        tracing::info!(
            alive = pri2.alive,
            faulted = pri2.faulted,
            "ungating: post-init PRI recovery"
        );

        let mthd_cmd_after = bar0
            .read_u32(falcon::FECS_BASE + falcon::MTHD_CMD)
            .unwrap_or(0xDEAD);
        let gpc_enables = bar0.read_u32(0x22004).unwrap_or(0xDEAD);
        let pgraph_status = bar0.read_u32(pgraph::STATUS as usize).unwrap_or(0xDEAD);
        let still_gated = mthd_cmd_after & 0xBAD0_0000 == 0xBAD0_0000;
        tracing::info!(
            mthd_cmd_after = format_args!("{mthd_cmd_after:#010x}"),
            gpc_enables = format_args!("{gpc_enables:#010x}"),
            pgraph_status = format_args!("{pgraph_status:#010x}"),
            still_gated,
            "ungating: probe after full GPC init"
        );

        if still_gated {
            let sm_ver = *profile.sm_range.start();
            if sm_ver >= 70 {
                // GV100+ (Volta/Turing/Ampere): destructive PGRAPH engine
                // reset kills the PRI ring irreversibly under VFIO.
                // The PRI ring master becomes permanently unresponsive
                // (0xbad00100) and no software recovery is possible
                // without an FLR. Skip the destructive path and rely on
                // the existing warm FECS instead.
                tracing::warn!(
                    sm = sm_ver,
                    "GPC PRI still gated but PGRAPH reset SKIPPED — \
                     destructive reset kills PRI ring on GV100+ under VFIO"
                );
            } else {
                tracing::warn!(
                    "GPC PRI still gated — full destructive GR reset + PIO FECS boot"
                );

                match crate::vfio::sovereign_stages::pgraph_engine_reset(bar0) {
                    Ok(detail) => tracing::info!(%detail, "ungating: PGRAPH engine reset"),
                    Err(e) => tracing::warn!(%e, "ungating: PGRAPH engine reset failed"),
                }

                let cg2 = crate::vfio::sovereign_stages::cg_sweep(bar0);
                let _pri2 = crate::vfio::sovereign_stages::pri_bus_recover(bar0);
                let _ = crate::vfio::sovereign_stages::pgob_ungating(bar0, &bridge);

                force_pri_enumerate(bar0, false, "ungating", false);

                let _ = bridge.apply_gr_bar0_init(bar0, *profile.sm_range.start());

                let pri3 = crate::vfio::sovereign_stages::pri_bus_recover(bar0);

                let mthd_post = bar0
                    .read_u32(falcon::FECS_BASE + falcon::MTHD_CMD)
                    .unwrap_or(0xDEAD);
                let gpc_post = bar0.read_u32(0x22004).unwrap_or(0xDEAD);
                tracing::info!(
                    cg_changes = cg2.changes,
                    pri_alive = pri3.alive,
                    mthd_cmd = format_args!("{mthd_post:#010x}"),
                    gpc_enables = format_args!("{gpc_post:#010x}"),
                    "ungating: probe after destructive GR reset"
                );

                if reboot_fecs_after_reset(bar0, &bridge, dma_backend) {
                    fecs_hs_booted = true;
                }
            }
        }
    }

    let fecs_alive = crate::vfio::channel::fecs::fecs_is_alive(bar0);
    let pc = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    tracing::info!(
        fecs_alive,
        fecs_hs_booted,
        pc = format_args!("{pc:#010x}"),
        pgraph_was_gated = pgraph_gated,
        "FECS liveness check before method protocol"
    );

    if fecs_alive {
        fecs_setup_channel(bar0, channel)?;
    } else {
        tracing::warn!(
            fecs_hs_booted,
            pgraph_was_gated = pgraph_gated,
            "FECS not alive after boot — skipping method protocol"
        );
    }

    Ok(fecs_hs_booted)
}
