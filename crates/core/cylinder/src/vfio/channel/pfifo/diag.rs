// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO/PBDMA/PCCSR diagnostic readback.

use crate::vfio::device::MappedBar;

use super::super::registers::{pbdma, pccsr, pfifo};

pub(super) fn log_pfifo_diagnostics(bar0: &MappedBar) {
    let r = |reg: usize| bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);

    let pfifo_intr = r(pfifo::INTR);
    let pfifo_en = r(pfifo::INTR_EN);
    let sched = r(pfifo::SCHED_EN);
    let pccsr_inst = r(pccsr::inst(0));
    let pccsr_chan = r(pccsr::channel(0));
    let pbdma0_intr = r(pbdma::intr(0));
    let pbdma0_hce = r(pbdma::hce_intr(0));
    let pbdma1_intr = r(pbdma::intr(1));
    let engn0_status = r(0x0000_2640);
    let pbdma0_idle = r(0x0000_3080);
    let pbdma1_idle = r(0x0000_3084);
    let rl0_info = r(0x0000_2284);
    let pmc_enable = r(0x0000_0200);
    let bind_err = r(0x0000_252C);
    let sched_dis = r(0x0000_2630);
    let preempt = r(0x0000_2634);
    let runl_submit_info = r(0x0000_2270);
    let doorbell_test = r(0x0081_0090);
    let pbdma_map = r(0x0000_2004);

    tracing::debug!(
        pmc_enable = format_args!("{pmc_enable:#010x}"),
        sched = format_args!("{sched:#010x}"),
        sched_dis = format_args!("{sched_dis:#010x}"),
        preempt = format_args!("{preempt:#010x}"),
        pfifo_intr = format_args!("{pfifo_intr:#010x}"),
        pfifo_en = format_args!("{pfifo_en:#010x}"),
        pccsr_inst = format_args!("{pccsr_inst:#010x}"),
        pccsr_chan = format_args!("{pccsr_chan:#010x}"),
        pbdma0_intr = format_args!("{pbdma0_intr:#010x}"),
        pbdma0_hce = format_args!("{pbdma0_hce:#010x}"),
        pbdma1_intr = format_args!("{pbdma1_intr:#010x}"),
        pbdma0_idle = format_args!("{pbdma0_idle:#010x}"),
        pbdma1_idle = format_args!("{pbdma1_idle:#010x}"),
        engn0_status = format_args!("{engn0_status:#010x}"),
        rl0_info = format_args!("{rl0_info:#010x}"),
        bind_err = format_args!("{bind_err:#010x}"),
        runl_submit_info = format_args!("{runl_submit_info:#010x}"),
        doorbell_test = format_args!("{doorbell_test:#010x}"),
        pbdma_map = format_args!("{pbdma_map:#010x}"),
        "PFIFO diagnostics"
    );

    let mut seq = 0_usize;
    for pid in 0..32_usize {
        if pbdma_map & (1 << pid) == 0 {
            continue;
        }
        let b = 0x040000 + pid * 0x2000;
        let rl_assign = r(0x2390 + seq * 4);
        tracing::debug!(
            pbdma = pid,
            seq,
            runlist = rl_assign,
            gp_base_hi = format_args!("{:#010x}", r(b + 0x44)),
            gp_base_lo = format_args!("{:#010x}", r(b + 0x40)),
            gp_put = format_args!("{:#010x}", r(b + 0x54)),
            gp_fetch = format_args!("{:#010x}", r(b + 0x48)),
            userd_hi = format_args!("{:#010x}", r(b + 0xD4)),
            userd_lo = format_args!("{:#010x}", r(b + 0xD0)),
            "PBDMA state"
        );
        seq += 1;
    }
}
