// SPDX-License-Identifier: AGPL-3.0-or-later
//! Catalyst warm-handoff path: PRI ungating and nouveau falcon boot.

use crate::nv::gsp_bridge::GspBridge;
use crate::nv::registers::gpc;
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::{DmaBackend, MappedBar};

use super::super::generation::GenerationProfile;
use super::super::nv_gsp_bridge::NvGspBridge;
use super::gr_falcon_boot::{boot_gpccs_fecs_catalyst, fecs_setup_channel};
use super::gr_ungating::{UngatingLog, force_pri_enumerate, ungate_gr_engine};

/// Catalyst path: targeted PRI recovery + GPC ungating + nouveau falcon boot.
pub(super) fn handle_catalyst_path(
    bar0: &MappedBar,
    dma_backend: &DmaBackend,
    channel: &VfioChannel,
    profile: &GenerationProfile,
    bdf: &str,
) {
    use crate::vfio::channel::registers::falcon;

    let bridge = NvGspBridge::new(profile.firmware_chip);

    let tpc_before = (0..6u32)
        .map(|gpc_id| {
            let addr = (gpc::gpc_tpc0(gpc_id) + 0x0c) as usize;
            bar0.read_u32(addr).unwrap_or(0xDEAD_DEAD)
        })
        .collect::<Vec<_>>();
    tracing::info!(
        bdf = %bdf,
        tpc_before = ?tpc_before,
        "catalyst: TPC state before PRI ungating"
    );

    let _ = ungate_gr_engine(
        bar0,
        &bridge,
        profile,
        UngatingLog {
            prefix: "catalyst",
            log_phases: true,
            ack_pmc_intr: true,
            verbose_mmu: false,
        },
    );

    let tpc_after = (0..6u32)
        .map(|gpc_id| {
            let addr = (gpc::gpc_tpc0(gpc_id) + 0x0c) as usize;
            bar0.read_u32(addr).unwrap_or(0xDEAD_DEAD)
        })
        .collect::<Vec<_>>();
    let tpc_survived = tpc_after
        .iter()
        .any(|&v| v != 0 && v != 0xDEAD_DEAD && v & 0xBADF_0000 != 0xBADF_0000);
    tracing::info!(
        bdf = %bdf,
        tpc_after = ?tpc_after,
        tpc_survived,
        "catalyst: TPC state after PRI ungating (no PGRAPH reset)"
    );

    boot_gpccs_fecs_catalyst(bar0, &bridge, dma_backend, bdf, "catalyst");

    std::thread::sleep(std::time::Duration::from_millis(100));

    let fecs_alive = crate::vfio::channel::fecs::fecs_is_alive(bar0);
    let fecs_pc = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    let fecs_alias = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    tracing::info!(
        bdf = %bdf,
        fecs_alive,
        fecs_pc = format_args!("{fecs_pc:#010x}"),
        fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
        "catalyst: FECS state after nouveau boot"
    );

    if fecs_alive {
        match fecs_setup_channel(bar0, channel) {
            Ok(()) => {
                tracing::info!(bdf = %bdf, "catalyst: FECS channel setup succeeded");
            }
            Err(e) => {
                tracing::warn!(bdf = %bdf, error = %e, "catalyst: FECS channel setup failed");
            }
        }
    } else if bridge.has_gr_firmware() {
        tracing::info!(
            bdf = %bdf,
            "catalyst: FECS not alive — attempting PGRAPH engine reset"
        );
        match crate::vfio::sovereign_stages::pgraph_engine_reset(bar0) {
            Ok(detail) => tracing::info!(%detail, "catalyst: PGRAPH reset"),
            Err(e) => tracing::warn!(%e, "catalyst: PGRAPH reset failed"),
        }

        force_pri_enumerate(bar0, false, "catalyst", false);

        let _ = bridge.apply_gr_bar0_init(bar0, *profile.sm_range.start());
        let _ = crate::vfio::sovereign_stages::pri_bus_recover(bar0);

        let _ = bridge.boot_falcon_hs(
            bar0,
            "GPCCS",
            falcon::GPCCS_BASE,
            dma_backend,
            super::super::nv_gsp_bridge::GPCCS_FW_CODE_IOVA,
            super::super::nv_gsp_bridge::GPCCS_FW_DATA_IOVA,
        );
        if let Ok((ctl, _)) = bridge.boot_falcon_hs(
            bar0,
            "FECS",
            falcon::FECS_BASE,
            dma_backend,
            super::super::nv_gsp_bridge::FECS_FW_CODE_IOVA,
            super::super::nv_gsp_bridge::FECS_FW_DATA_IOVA,
        ) {
            tracing::info!(
                bdf = %bdf,
                fecs_cpuctl = format_args!("{ctl:#010x}"),
                "catalyst: FECS boot after PGRAPH reset"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
            if crate::vfio::channel::fecs::fecs_is_alive(bar0) {
                let _ = fecs_setup_channel(bar0, channel);
            }
        }
    }
}
