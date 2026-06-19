// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO recovery after PGRAPH ungating when scheduler/runlist PRI domains fault.

use crate::nv::registers::pmc;
use crate::vfio::device::{DmaBackend, MappedBar};

use super::super::generation::GenerationProfile;
use super::channel_init::ChannelInitResult;
use super::gr_falcon_boot::fecs_setup_channel;
use super::pbdma::find_target_pbdma;
use super::{GPFIFO_ENTRIES, GPFIFO_IOVA, USERD_IOVA};

/// Check whether PFIFO is in a truly broken state (not merely SCHED_EN faulted on GV100).
pub(super) fn pfifo_truly_broken(
    bar0: &MappedBar,
    channel: &crate::vfio::channel::VfioChannel,
) -> bool {
    use crate::vfio::channel::registers::pfifo;

    let rl_base = bar0
        .read_u32(pfifo::runlist_base(channel.runlist_id_hint()))
        .unwrap_or(0);
    let sched_dis = bar0.read_u32(pfifo::SCHED_DISABLE).unwrap_or(0);
    let sched_dis_faulted = sched_dis & 0xBAD0_0000 == 0xBAD0_0000;
    rl_base == 0 || sched_dis_faulted
}

/// Log post-PGRAPH PFIFO register state for diagnostics.
pub(super) fn log_post_pgraph_pfifo_state(
    bar0: &MappedBar,
    bdf: &str,
    channel: &crate::vfio::channel::VfioChannel,
    pfifo_truly_broken: bool,
) {
    use crate::vfio::channel::registers::pfifo;

    let rl_base = bar0
        .read_u32(pfifo::runlist_base(channel.runlist_id_hint()))
        .unwrap_or(0);
    let sched_en = bar0.read_u32(pfifo::SCHED_EN).unwrap_or(0);
    let sched_dis = bar0.read_u32(pfifo::SCHED_DISABLE).unwrap_or(0);
    let pccsr_status = bar0.read_u32(0x800C).unwrap_or(0) & 0xF;

    tracing::info!(
        bdf = %bdf,
        runlist_base = format_args!("{rl_base:#010x}"),
        sched_en = format_args!("{sched_en:#010x}"),
        sched_disable = format_args!("{sched_dis:#010x}"),
        pccsr_status,
        pfifo_truly_broken,
        "post-PGRAPH PFIFO state check"
    );
}

/// Full PFIFO re-init and channel recreation when scheduler/runlist PRI domains are dead.
pub(super) fn recover_broken_pfifo(
    bar0: &MappedBar,
    dma_backend: &DmaBackend,
    init: ChannelInitResult,
    profile: &GenerationProfile,
    is_kepler: bool,
    bdf: &str,
    target_pbdma_base: Option<usize>,
) -> (ChannelInitResult, Option<usize>) {
    use crate::vfio::channel::registers::pfifo;

    let sched_en = bar0.read_u32(pfifo::SCHED_EN).unwrap_or(0);

    tracing::info!(
        bdf = %bdf,
        "PFIFO truly broken — full PFIFO re-init + channel recreation"
    );

    // PFIFO-only reset + PRI ring re-enumerate.
    // DO NOT reset PGRAPH (bit 12) — FECS was just set up by
    // fecs_setup_channel() and would be killed.
    // SCHED_EN (0x2504) is permanently PRI-faulted on GV100
    // VFIO — this is expected. The functional scheduler control
    // is SCHED_DISABLE (0x2630).

    let pmc_cur = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);

    // Step 1: Reset PRIV_RING (PMC bit 3) to reinitialize PRI bus.
    // Without this, PRI faults persist and GPCs/scheduler stay dead.
    let priv_ring_bit = 1u32 << 3;
    let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_cur & !priv_ring_bit);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_cur | priv_ring_bit);
    std::thread::sleep(std::time::Duration::from_millis(20));

    // Step 2: PMC reset PFIFO (bit 8).
    let pfifo_bit = 1u32 << 8;
    let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_cur & !pfifo_bit);
    std::thread::sleep(std::time::Duration::from_millis(20));
    let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_cur | pfifo_bit);
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Step 3: PRI ring master — enumerate + ACK all satellites.
    for round in 0..5u32 {
        let _ = bar0.write_u32(0x12_0004, 0x04);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let rm_stat = bar0.read_u32(0x12_0058).unwrap_or(0);
        if rm_stat != 0 && rm_stat & 0xBAD0_0000 != 0xBAD0_0000 {
            let _ = bar0.write_u32(0x12_004C, 0x02);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let rm_after = bar0.read_u32(0x12_0058).unwrap_or(0);
        tracing::info!(
            bdf = %bdf,
            round,
            rm_before = format_args!("{rm_stat:#010x}"),
            rm_after = format_args!("{rm_after:#010x}"),
            "PRI ring enumerate + ACK (recovery)"
        );
        if rm_after == 0 {
            break;
        }
        if rm_after & 0xBAD0_0000 == 0xBAD0_0000 && round >= 2 {
            break;
        }
    }
    let _ = bar0.write_u32(0x12_2058, 0xFFFF_FFFF);
    let _ = bar0.write_u32(0x12_8058, 0xFFFF_FFFF);
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Step 3: Verify scheduler is accessible after reset.
    let sched_en_after = bar0.read_u32(pfifo::SCHED_EN).unwrap_or(0);
    let sched_dis_after = bar0.read_u32(pfifo::SCHED_DISABLE).unwrap_or(0);
    let rl_test = bar0.read_u32(pfifo::runlist_base(1)).unwrap_or(0xDEAD);
    tracing::info!(
        bdf = %bdf,
        sched_en_before = format_args!("{sched_en:#010x}"),
        sched_en_after = format_args!("{sched_en_after:#010x}"),
        sched_dis_after = format_args!("{sched_dis_after:#010x}"),
        runlist_base_test = format_args!("{rl_test:#010x}"),
        "PMC PFIFO-only reset + PRI enumerate applied"
    );

    let mut init = init;
    let mut target_pbdma_base = target_pbdma_base;

    // Recreate channel with cold PFIFO config (warm_handoff=false)
    // since we just did a PMC PFIFO reset.
    //
    // After PGRAPH reset on GV100, per-runlist registers for
    // runlist IDs > 0 are in a faulted PRI domain. Only runlist 0
    // accepts writes. Override the target runlist to 0.
    match super::channel_init::init_channel_buffers_with_pfifo_config(
        dma_backend,
        bar0,
        profile,
        is_kepler,
        true,
        false,
        bdf,
        "post-pgraph",
    ) {
        Ok(mut new_init) => {
            // Force runlist 0 if runlist 1+ writes don't stick.
            // On GV100 after PGRAPH reset, per-runlist PRI domains
            // for RL 1+ are faulted. Also reassign PBDMA[1] to RL 0.
            let rl_test = bar0
                .read_u32(pfifo::runlist_base(new_init.channel.runlist_id_hint()))
                .unwrap_or(0);
            if rl_test == 0 {
                tracing::info!(
                    bdf = %bdf,
                    original_runlist = new_init.channel.runlist_id_hint(),
                    "runlist write failed — forcing channel to runlist 0 + PBDMA reassign"
                );
                new_init.channel.force_runlist(0);

                // Reassign PBDMA[1] from runlist 1 → runlist 0.
                // GV100 PBDMA→runlist table at 0x2390, sequential
                // by PBDMA presence in PBDMA_MAP (0x2004).
                let rl_map_pre = bar0.read_u32(0x2390).unwrap_or(0xDEAD);
                let _ = bar0.write_u32(0x2390, 0); // seq[0] = PBDMA[1] → RL 0
                std::thread::sleep(std::time::Duration::from_millis(5));
                let rl_map_post = bar0.read_u32(0x2390).unwrap_or(0xDEAD);
                tracing::info!(
                    bdf = %bdf,
                    rl_map_pre = format_args!("{rl_map_pre:#010x}"),
                    rl_map_post = format_args!("{rl_map_post:#010x}"),
                    "PBDMA→runlist reassign (0x2390): wrote 0, readback"
                );
                if rl_map_post != 0 {
                    tracing::warn!(
                        bdf = %bdf,
                        "PBDMA→runlist write did NOT stick — register is read-only"
                    );
                }

                if let Err(e) = new_init.channel.resubmit_runlist(bar0) {
                    tracing::warn!(
                        bdf = %bdf,
                        error = %e,
                        "runlist 0 submission failed"
                    );
                }
            }
            tracing::info!(
                bdf = %bdf,
                channel_id = new_init.channel.id(),
                runlist = new_init.channel.runlist_id_hint(),
                "channel recreated after PFIFO reset"
            );

            // Re-run FECS method protocol on the new channel.
            // FECS is still alive (we only reset PFIFO, not PGRAPH).
            let fecs_ok = crate::vfio::channel::fecs::fecs_is_alive(bar0);
            if fecs_ok {
                if let Err(e) = fecs_setup_channel(bar0, &new_init.channel) {
                    tracing::warn!(
                        bdf = %bdf,
                        error = %e,
                        "FECS setup on recreated channel failed"
                    );
                }
            } else {
                tracing::warn!(
                    bdf = %bdf,
                    "FECS not alive after PFIFO reset — skipping channel setup"
                );
            }

            init = new_init;

            // Re-discover PBDMA for the new channel's runlist
            // (may have switched from runlist 1 → 0).
            let pbdma_map_check = bar0
                .read_u32(crate::vfio::channel::registers::pfifo::PBDMA_MAP)
                .unwrap_or(0);
            let rl_map_0 = bar0.read_u32(0x2390).unwrap_or(0xDEAD);
            tracing::info!(
                bdf = %bdf,
                runlist = init.channel.runlist_id_hint(),
                pbdma_map = format_args!("{pbdma_map_check:#010x}"),
                rl_map_0 = format_args!("{rl_map_0:#010x}"),
                doorbell = ?init.doorbell,
                "post-recovery: about to re-discover PBDMA"
            );
            target_pbdma_base =
                find_target_pbdma(bar0, &init.channel, init.doorbell, " (post-recovery)");

            // On GV100 VFIO cold boot, no PBDMA serves runlist 0
            // (hardware maps PBDMAs to engine runlists only).
            // Force-program PBDMA 1 with our channel's GPFIFO/USERD
            // and clear all latched interrupts. Direct GP_PUT writes
            // bypass the scheduler/runlist mechanism entirely.
            if target_pbdma_base.is_none() {
                target_pbdma_base = force_program_pbdma_for_runlist0(bar0, bdf, &init);
            }
        }
        Err(e) => {
            tracing::warn!(
                bdf = %bdf,
                error = %e,
                "channel recreation after PFIFO reset failed"
            );
        }
    }

    (init, target_pbdma_base)
}

/// Force-program PBDMA 1 when no PBDMA serves runlist 0 on GV100 VFIO cold boot.
fn force_program_pbdma_for_runlist0(
    bar0: &MappedBar,
    bdf: &str,
    init: &ChannelInitResult,
) -> Option<usize> {
    use crate::vfio::channel::registers::{pbdma, pccsr, ramfc};

    let pb = pbdma::base(1);
    let w = |off: usize, val: u32| bar0.write_u32(pb + off, val).ok();
    let gpfifo_iova = GPFIFO_IOVA;
    let userd_iova = USERD_IOVA;
    let channel_id = init.channel.id();
    let limit2 = GPFIFO_ENTRIES.ilog2();
    let userd_val = (userd_iova as u32 & 0xFFFF_FE00) | 0x02;
    let gpbase_hi = (gpfifo_iova >> 32) as u32 | (limit2 << 16);

    // Clear all PBDMA interrupts first
    w(0x100, 0xFFFF_FFFF);
    w(0x108, 0xFFFF_FFFF);
    w(0x148, 0xFFFF_FFFF);
    std::thread::sleep(std::time::Duration::from_millis(5));

    // Program DIRECT registers
    w(pbdma::GP_BASE_LO, gpfifo_iova as u32);
    w(pbdma::GP_BASE_HI, gpbase_hi);
    w(pbdma::USERD_LO, userd_val);
    w(pbdma::USERD_HI, (userd_iova >> 32) as u32);
    w(pbdma::SIGNATURE, 0x0000_FACE);
    w(pbdma::CHANNEL_INFO, 0x0300_0000 | channel_id);
    w(pbdma::GP_FETCH, 0);
    w(pbdma::GP_STATE, 0);
    w(pbdma::GP_PUT, 0);

    // Program CTX registers (scheduler save/restore mirror)
    w(pbdma::CTX_USERD_LO, userd_val);
    w(pbdma::CTX_USERD_HI, (userd_iova >> 32) as u32);
    w(pbdma::CTX_SIGNATURE, 0x0000_FACE);
    w(pbdma::CTX_ACQUIRE, 0x7FFF_F902);
    w(pbdma::CTX_GP_BASE_LO, gpfifo_iova as u32);
    w(pbdma::CTX_GP_BASE_HI, gpbase_hi);
    w(pbdma::CTX_GP_PUT, 0);
    w(pbdma::CTX_GP_FETCH, 0);

    // RAMFC fields
    w(ramfc::PB_HEADER, 0x2040_0000);
    w(ramfc::SUBDEVICE, 0x3000_0000 | 0xFFF);
    w(ramfc::ACQUIRE, 0x7FFF_F902);
    w(ramfc::DMA_LIMIT_REF, 0x003F_6078);

    // Clear interrupts again after programming
    w(0x100, 0xFFFF_FFFF);
    w(0x108, 0xFFFF_FFFF);
    w(0x148, 0xFFFF_FFFF);
    std::thread::sleep(std::time::Duration::from_millis(5));

    // Clear PCCSR faults and re-enable channel
    let _ = bar0.write_u32(
        pccsr::channel(channel_id),
        pccsr::PBDMA_FAULTED_RESET | pccsr::ENG_FAULTED_RESET,
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
    let _ = bar0.write_u32(pccsr::channel(channel_id), pccsr::CHANNEL_ENABLE_SET);

    // Ring doorbell
    let _ = bar0.write_u32(
        crate::vfio::channel::registers::usermode::NOTIFY_CHANNEL_PENDING,
        channel_id,
    );
    std::thread::sleep(std::time::Duration::from_millis(50));

    let intr_after = bar0.read_u32(pb + 0x100).unwrap_or(0xDEAD);
    let gp_state = bar0.read_u32(pb + pbdma::GP_STATE).unwrap_or(0xDEAD);
    tracing::info!(
        bdf = %bdf,
        intr_after = format_args!("{intr_after:#010x}"),
        gp_state = format_args!("{gp_state:#010x}"),
        "PBDMA 1 re-force-programmed for runlist 0 channel"
    );

    Some(pb)
}
