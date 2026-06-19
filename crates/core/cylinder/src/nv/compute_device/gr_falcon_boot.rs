// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPCCS + FECS falcon boot and FECS channel setup protocol.

use crate::error::DriverResult;
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::{DmaBackend, MappedBar};

use super::super::nv_gsp_bridge::NvGspBridge;

/// Send FECS method commands to set up a channel for context switching.
///
/// Sequence (from nouveau `gf100_gr_init`):
/// 1. Set watchdog timeout
/// 2. INIT_CTXSW — initialize FECS context switching tables
/// 3. BIND_CHANNEL — register our instance block with FECS
/// 4. COMMIT — tell FECS to copy golden context into our GR buffer
pub(crate) fn fecs_setup_channel(bar0: &MappedBar, channel: &VfioChannel) -> DriverResult<()> {
    use crate::vfio::channel::fecs;

    let inst_iova = channel.instance_iova();

    match fecs::fecs_set_watchdog_timeout(bar0, 0x7FFF_FFFF) {
        Ok(r) => tracing::info!(status = r.status, "FECS watchdog set"),
        Err(e) => tracing::warn!(error = %e, "FECS watchdog timeout set failed (non-fatal)"),
    }

    match fecs::fecs_init_ctxsw(bar0) {
        Ok(r) => {
            tracing::info!(
                status = r.status,
                mailbox0 = format_args!("{:#x}", r.mailbox0),
                "FECS INIT_CTXSW completed"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "FECS INIT_CTXSW failed — context switching may not work");
        }
    }

    match fecs::fecs_bind_channel(bar0, inst_iova) {
        Ok(r) => {
            tracing::info!(
                status = r.status,
                mailbox0 = format_args!("{:#x}", r.mailbox0),
                inst_iova = format_args!("{inst_iova:#x}"),
                "FECS BIND_CHANNEL completed"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "FECS BIND_CHANNEL failed");
        }
    }

    match fecs::fecs_commit(bar0, inst_iova) {
        Ok(r) => {
            tracing::info!(
                status = r.status,
                mailbox0 = format_args!("{:#x}", r.mailbox0),
                "FECS COMMIT completed — golden context should be loaded"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "FECS COMMIT failed");
        }
    }

    // Query the GR context image size for diagnostics
    match fecs::fecs_discover_image_size(bar0) {
        Ok(size) => {
            tracing::info!(
                gr_ctx_size = size,
                gr_ctx_size_hex = format_args!("{size:#x}"),
                "FECS reports GR context image size"
            );
        }
        Err(e) => {
            tracing::debug!(error = %e, "FECS DISCOVER_IMAGE_SIZE failed (non-fatal)");
        }
    }

    Ok(())
}

/// Boot GPCCS then FECS after PFIFO channel infrastructure exists.
///
/// FECS and GPCCS are a pair — FECS self-halts if GPCCS is not running.
/// Returns `true` if FECS HS boot succeeded.
pub(crate) fn boot_gpccs_fecs_deferred(
    bar0: &MappedBar,
    bridge: &NvGspBridge,
    dma_backend: &DmaBackend,
    bdf: &str,
) -> bool {
    use crate::vfio::channel::registers::{falcon, pmc};

    // Ensure all engines are enabled in PMC_ENABLE before touching
    // GPC registers. GPCCS registers at 0x41Axxx are behind the GR
    // engine clock gate and return 0xbadf5040 (PRI fault) when
    // GR/GPC is clock-gated.
    let pmc_before = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
    let _ = bar0.write_u32(pmc::ENABLE, 0xFFFF_FFFF);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let pmc_after = bar0.read_u32(pmc::ENABLE).unwrap_or(0);
    tracing::info!(
        bdf = %bdf,
        pmc_before = format_args!("{pmc_before:#010x}"),
        pmc_after = format_args!("{pmc_after:#010x}"),
        "GR init: PMC glow-plug all engines"
    );

    // 1. Boot GPCCS first
    tracing::info!(bdf = %bdf, "GR init: booting GPCCS falcon");
    match bridge.boot_falcon_hs(
        bar0,
        "GPCCS",
        falcon::GPCCS_BASE,
        dma_backend,
        super::super::nv_gsp_bridge::GPCCS_FW_CODE_IOVA,
        super::super::nv_gsp_bridge::GPCCS_FW_DATA_IOVA,
    ) {
        Ok((ctl, mb0)) => {
            tracing::info!(
                bdf = %bdf,
                gpccs_cpuctl = format_args!("{ctl:#010x}"),
                gpccs_mb0 = format_args!("{mb0:#010x}"),
                "GPCCS HS boot complete"
            );
        }
        Err(e) => {
            tracing::warn!(bdf = %bdf, error = %e, "GPCCS HS boot failed");
        }
    }

    // 2. Boot FECS
    tracing::info!(bdf = %bdf, "GR init: booting FECS falcon");
    let mut fecs_hs_booted = false;
    match bridge.boot_falcon_hs(
        bar0,
        "FECS",
        falcon::FECS_BASE,
        dma_backend,
        super::super::nv_gsp_bridge::FECS_FW_CODE_IOVA,
        super::super::nv_gsp_bridge::FECS_FW_DATA_IOVA,
    ) {
        Ok((ctl, mb0)) => {
            fecs_hs_booted = true;
            tracing::info!(
                bdf = %bdf,
                fecs_cpuctl = format_args!("{ctl:#010x}"),
                fecs_mb0 = format_args!("{mb0:#010x}"),
                "FECS HS boot complete (post-channel-creation)"
            );
        }
        Err(e) => {
            tracing::warn!(bdf = %bdf, error = %e, "FECS HS boot failed");
        }
    }

    // 3. Check FECS state via both CPUCTL and CPUCTL_ALIAS.
    // On Volta HS falcons, CPUCTL at 0x100 may be security-locked and
    // always show HRESET, while CPUCTL_ALIAS at 0x130 shows the true state.
    if fecs_hs_booted {
        let fecs_base = falcon::FECS_BASE;

        // Immediate check via both registers
        let ctl = bar0.read_u32(fecs_base + falcon::CPUCTL).unwrap_or(0xDEAD);
        let ctl_alias = bar0
            .read_u32(fecs_base + falcon::CPUCTL_ALIAS)
            .unwrap_or(0xDEAD);
        let pc = bar0.read_u32(fecs_base + falcon::PC).unwrap_or(0xDEAD);
        let mb0 = bar0
            .read_u32(fecs_base + falcon::MAILBOX0)
            .unwrap_or(0xDEAD);
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl = format_args!("{ctl:#010x}"),
            fecs_cpuctl_alias = format_args!("{ctl_alias:#010x}"),
            fecs_pc = format_args!("{pc:#010x}"),
            fecs_mb0 = format_args!("{mb0:#010x}"),
            "FECS post-boot: CPUCTL vs CPUCTL_ALIAS (HS security check)"
        );

        // Wait 100ms and check stability
        std::thread::sleep(std::time::Duration::from_millis(100));
        let ctl2 = bar0.read_u32(fecs_base + falcon::CPUCTL).unwrap_or(0xDEAD);
        let ctl2_alias = bar0
            .read_u32(fecs_base + falcon::CPUCTL_ALIAS)
            .unwrap_or(0xDEAD);
        let pc2 = bar0.read_u32(fecs_base + falcon::PC).unwrap_or(0xDEAD);
        let mb02 = bar0
            .read_u32(fecs_base + falcon::MAILBOX0)
            .unwrap_or(0xDEAD);
        let gpccs_ctl = bar0
            .read_u32(falcon::GPCCS_BASE + falcon::CPUCTL)
            .unwrap_or(0xDEAD);
        let gpccs_alias = bar0
            .read_u32(falcon::GPCCS_BASE + falcon::CPUCTL_ALIAS)
            .unwrap_or(0xDEAD);
        let gpccs_pc = bar0
            .read_u32(falcon::GPCCS_BASE + falcon::PC)
            .unwrap_or(0xDEAD);
        let fecs_alive =
            ctl2_alias & falcon::CPUCTL_HRESET == 0 && ctl2_alias & falcon::CPUCTL_HALTED == 0;
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl = format_args!("{ctl2:#010x}"),
            fecs_cpuctl_alias = format_args!("{ctl2_alias:#010x}"),
            fecs_pc = format_args!("{pc2:#010x}"),
            fecs_mb0 = format_args!("{mb02:#010x}"),
            fecs_alive,
            gpccs_cpuctl = format_args!("{gpccs_ctl:#010x}"),
            gpccs_cpuctl_alias = format_args!("{gpccs_alias:#010x}"),
            gpccs_pc = format_args!("{gpccs_pc:#010x}"),
            "GR falcon stability check (100ms post-boot)"
        );
    }

    fecs_hs_booted
}

/// Boot GPCCS + FECS with nouveau firmware (catalyst path).
pub(crate) fn boot_gpccs_fecs_catalyst(
    bar0: &MappedBar,
    bridge: &NvGspBridge,
    dma_backend: &DmaBackend,
    bdf: &str,
    log_prefix: &str,
) {
    use crate::vfio::channel::registers::falcon;

    if !bridge.has_gr_firmware() {
        return;
    }

    tracing::info!(bdf = %bdf, "{log_prefix}: booting GPCCS + FECS with nouveau firmware");

    match bridge.boot_falcon_hs(
        bar0,
        "GPCCS",
        falcon::GPCCS_BASE,
        dma_backend,
        super::super::nv_gsp_bridge::GPCCS_FW_CODE_IOVA,
        super::super::nv_gsp_bridge::GPCCS_FW_DATA_IOVA,
    ) {
        Ok((ctl, mb0)) => {
            tracing::info!(
                bdf = %bdf,
                gpccs_cpuctl = format_args!("{ctl:#010x}"),
                gpccs_mb0 = format_args!("{mb0:#010x}"),
                "{log_prefix}: GPCCS boot complete"
            );
        }
        Err(e) => tracing::warn!(bdf = %bdf, error = %e, "{log_prefix}: GPCCS boot failed"),
    }

    match bridge.boot_falcon_hs(
        bar0,
        "FECS",
        falcon::FECS_BASE,
        dma_backend,
        super::super::nv_gsp_bridge::FECS_FW_CODE_IOVA,
        super::super::nv_gsp_bridge::FECS_FW_DATA_IOVA,
    ) {
        Ok((ctl, mb0)) => {
            tracing::info!(
                bdf = %bdf,
                fecs_cpuctl = format_args!("{ctl:#010x}"),
                fecs_mb0 = format_args!("{mb0:#010x}"),
                "{log_prefix}: FECS boot complete"
            );
        }
        Err(e) => tracing::warn!(bdf = %bdf, error = %e, "{log_prefix}: FECS boot failed"),
    }
}

/// Attempt PIO FECS re-boot after destructive GR reset.
pub(crate) fn reboot_fecs_after_reset(
    bar0: &MappedBar,
    bridge: &NvGspBridge,
    dma_backend: &DmaBackend,
) -> bool {
    use crate::vfio::channel::registers::falcon;

    if !bridge.has_gr_firmware() {
        return false;
    }

    tracing::info!("ungating: attempting PIO FECS re-boot");
    match bridge.boot_falcon_hs(
        bar0,
        "FECS",
        falcon::FECS_BASE,
        dma_backend,
        super::super::nv_gsp_bridge::FECS_FW_CODE_IOVA,
        super::super::nv_gsp_bridge::FECS_FW_DATA_IOVA,
    ) {
        Ok((ctl, mb0)) => {
            tracing::info!(
                fecs_cpuctl = format_args!("{ctl:#010x}"),
                fecs_mb0 = format_args!("{mb0:#010x}"),
                "ungating: FECS re-boot succeeded"
            );
            true
        }
        Err(e) => {
            tracing::warn!(%e, "ungating: FECS re-boot failed");
            false
        }
    }
}
