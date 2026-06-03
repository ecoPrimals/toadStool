// SPDX-License-Identifier: AGPL-3.0-or-later
//! Post-init channel readiness diagnostics for VFIO open.

use crate::vfio::device::MappedBar;

use super::channel_init::ChannelInitResult;

/// Log whether the channel is ready for PBDMA dispatch after `open_vfio` setup.
pub(super) fn log_channel_readiness(
    bar0: &MappedBar,
    bdf: &str,
    init: &ChannelInitResult,
    fecs_ready: bool,
) {
    use crate::vfio::channel::registers::{falcon, pccsr, pfifo};

    let rl_base = bar0
        .read_u32(pfifo::runlist_base(init.channel.runlist_id_hint()))
        .unwrap_or(0xDEAD_DEAD);
    let pccsr_val = bar0
        .read_u32(pccsr::channel(init.channel.id()))
        .unwrap_or(0);
    let pccsr_status = pccsr::status(pccsr_val);
    let fecs_alias = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    let fecs_pc = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    let fecs_alive = fecs_alias & falcon::CPUCTL_HRESET == 0
        && fecs_alias & falcon::CPUCTL_HALTED == 0
        && fecs_alias & 0xBADF_0000 != 0xBADF_0000;

    let dispatch_ready = rl_base != 0 && pccsr_status >= 5 && fecs_alive;

    tracing::info!(
        bdf = %bdf,
        runlist_base = format_args!("{rl_base:#010x}"),
        pccsr_status,
        pccsr_status_name = pccsr::status_name(pccsr_val),
        fecs_alive,
        fecs_pc = format_args!("{fecs_pc:#010x}"),
        has_gr_ctx = init.gr_ctx.is_some(),
        dispatch_ready,
        "open_vfio: post-init channel readiness diagnostic"
    );

    if !dispatch_ready && fecs_ready {
        tracing::warn!(
            bdf = %bdf,
            "fecs_ready=true but channel NOT dispatch-ready — \
             dispatch will proceed but may return zeros"
        );
    }
}
