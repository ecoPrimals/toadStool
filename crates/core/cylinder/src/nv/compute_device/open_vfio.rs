// SPDX-License-Identifier: AGPL-3.0-or-later
//! Open VFIO device and create PFIFO channel for PBDMA dispatch.

use crate::error::DriverResult;
use crate::nv::gsp_bridge::GspBridge;
use crate::nv::registers::{gpc, pgraph, pmc};
use crate::vfio::VfioDevice;

use super::super::generation::{PageTableFormat, GenerationProfile};
use super::super::nv_gsp_bridge::NvGspBridge;
use super::channel_init::{self, alloc_semaphore_buffer, build_dispatch_state, init_channel_buffers};
use super::gr_falcon_boot::{
    boot_gpccs_fecs_catalyst, boot_gpccs_fecs_deferred, fecs_setup_channel, reboot_fecs_after_reset,
};
use super::gr_ungating::{force_pri_enumerate, ungate_gr_engine, UngatingLog};
use super::pbdma::find_target_pbdma;
use super::NvVfioComputeDevice;

impl NvVfioComputeDevice {
    /// Open the VFIO device and create a PFIFO channel for PBDMA dispatch.
    ///
    /// After this call, `alloc`/`upload`/`readback`/`dispatch`/`sync` use
    /// real DMA buffers and GPFIFO submission instead of returning
    /// `Unsupported`.
    ///
    /// Uses warm handoff channel creation if FECS is already ready
    /// (preserves falcon engine state from nouveau/nvidia-470).
    ///
    /// # Errors
    ///
    /// Returns error if VFIO device open, BAR0 map, DMA buffer allocation,
    /// or channel creation fails.
    pub fn open_vfio(&mut self) -> DriverResult<()> {
        let profile = super::super::generation::profile_for_sm(self.sm);
        let is_kepler = matches!(
            profile.page_table_format,
            PageTableFormat::V1TwoLevel
        );

        let device = VfioDevice::open(&self.bdf)?;
        let bar0 = device.map_bar(0)?;
        let dma_backend = device.dma_backend();

        let mut fecs_hs_booted = false;
        let mut fecs_bridge: Option<NvGspBridge> = None;
        let mut pmc_was_cold = false;
        let catalyst_mode = self.catalyst_warm;

        if catalyst_mode {
            tracing::info!(
                bdf = %self.bdf,
                "catalyst_warm: skipping destructive FECS boot path — \
                 trusting catalyst-established hardware state"
            );
        }

        // Probe FECS state and prepare for deferred boot (after channel creation).
        if !is_kepler && self.fecs_ready && !catalyst_mode {
            probe_fecs_for_deferred_boot(
                &bar0,
                &self.bdf,
                profile,
                &mut pmc_was_cold,
                &mut fecs_bridge,
            );
        }

        // When PMC was cold (post-SBR/FLR), PFIFO needs cold initialization
        // even if fecs_ready is flagged. The PFIFO toggle and runlist flush
        // are required to bring per-runlist PRI domains online.
        let pfifo_warm = self.fecs_ready && !pmc_was_cold;
        let mut init = if pfifo_warm == self.fecs_ready {
            init_channel_buffers(
                &dma_backend,
                &bar0,
                profile,
                is_kepler,
                self.fecs_ready,
                &self.bdf,
                "",
            )?
        } else {
            channel_init::init_channel_buffers_with_pfifo_config(
                &dma_backend,
                &bar0,
                profile,
                is_kepler,
                self.fecs_ready,
                pfifo_warm,
                &self.bdf,
                "cold-pfifo",
            )?
        };

        tracing::info!(
            bdf = %self.bdf,
            channel_id = init.channel.id(),
            fecs_ready = self.fecs_ready,
            generation = profile.name,
            doorbell = ?init.doorbell,
            "VFIO PBDMA dispatch state initialized"
        );

        let mut target_pbdma_base =
            find_target_pbdma(&bar0, &init.channel, init.doorbell, "");

        // Deferred GR falcon boot: now that PFIFO + channel infrastructure
        // exists, boot GPCCS first, then FECS, then send INIT_CTXSW.
        if let Some(bridge) = fecs_bridge {
            fecs_hs_booted = boot_gpccs_fecs_deferred(&bar0, &bridge, &dma_backend, &self.bdf);
        }

        // After deferred boot (or on warm handoff), send FECS method protocol
        // to register our channel for context switching.
        if !is_kepler && init.gr_ctx.is_some() {
            let _ = handle_pgraph_ungating_and_fecs_setup(
                &bar0,
                &dma_backend,
                &init.channel,
                profile,
                &self.bdf,
                fecs_hs_booted,
            )?;

            // After PGRAPH ungating + FECS reboot, PFIFO may be in a broken
            // PRI-faulted state (all PBDMA/scheduler registers return 0xbad00200).
            // Detect this and do a full PFIFO re-init: since FECS was rebooted
            // from firmware, we no longer need to preserve its warm state.
            //
            // IMPORTANT: On GV100 VFIO, SCHED_EN (0x2504) is PERMANENTLY
            // PRI-faulted (0xbad00200) — this is normal, not a failure.
            // The functional scheduler control is SCHED_DISABLE (0x2630).
            // Only enter recovery when the runlist write actually failed
            // OR SCHED_DISABLE is also faulted (true PFIFO death).
            use crate::vfio::channel::registers::pfifo;
            let rl_base = bar0
                .read_u32(pfifo::runlist_base(init.channel.runlist_id_hint()))
                .unwrap_or(0);
            let sched_en = bar0.read_u32(pfifo::SCHED_EN).unwrap_or(0);
            let sched_dis = bar0.read_u32(pfifo::SCHED_DISABLE).unwrap_or(0);
            let pccsr_status = bar0.read_u32(0x800C).unwrap_or(0) & 0xF;
            let sched_dis_faulted = sched_dis & 0xBAD0_0000 == 0xBAD0_0000;
            let pfifo_truly_broken = rl_base == 0 || sched_dis_faulted;

            tracing::info!(
                bdf = %self.bdf,
                runlist_base = format_args!("{rl_base:#010x}"),
                sched_en = format_args!("{sched_en:#010x}"),
                sched_disable = format_args!("{sched_dis:#010x}"),
                pccsr_status,
                pfifo_truly_broken,
                "post-PGRAPH PFIFO state check"
            );

            if pfifo_truly_broken {
                tracing::info!(
                    bdf = %self.bdf,
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
                        bdf = %self.bdf,
                        round,
                        rm_before = format_args!("{rm_stat:#010x}"),
                        rm_after = format_args!("{rm_after:#010x}"),
                        "PRI ring enumerate + ACK (recovery)"
                    );
                    if rm_after == 0 { break; }
                    if rm_after & 0xBAD0_0000 == 0xBAD0_0000 && round >= 2 { break; }
                }
                let _ = bar0.write_u32(0x12_2058, 0xFFFF_FFFF);
                let _ = bar0.write_u32(0x12_8058, 0xFFFF_FFFF);
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Step 3: Verify scheduler is accessible after reset.
                let sched_en_after = bar0.read_u32(pfifo::SCHED_EN).unwrap_or(0);
                let sched_dis_after = bar0.read_u32(pfifo::SCHED_DISABLE).unwrap_or(0);
                let rl_test = bar0.read_u32(pfifo::runlist_base(1)).unwrap_or(0xDEAD);
                tracing::info!(
                    bdf = %self.bdf,
                    sched_en_before = format_args!("{sched_en:#010x}"),
                    sched_en_after = format_args!("{sched_en_after:#010x}"),
                    sched_dis_after = format_args!("{sched_dis_after:#010x}"),
                    runlist_base_test = format_args!("{rl_test:#010x}"),
                    "PMC PFIFO-only reset + PRI enumerate applied"
                );

                // Recreate channel with cold PFIFO config (warm_handoff=false)
                // since we just did a PMC PFIFO reset.
                //
                // After PGRAPH reset on GV100, per-runlist registers for
                // runlist IDs > 0 are in a faulted PRI domain. Only runlist 0
                // accepts writes. Override the target runlist to 0.
                match channel_init::init_channel_buffers_with_pfifo_config(
                    &dma_backend,
                    &bar0,
                    profile,
                    is_kepler,
                    true,
                    false,
                    &self.bdf,
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
                                bdf = %self.bdf,
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
                                bdf = %self.bdf,
                                rl_map_pre = format_args!("{rl_map_pre:#010x}"),
                                rl_map_post = format_args!("{rl_map_post:#010x}"),
                                "PBDMA→runlist reassign (0x2390): wrote 0, readback"
                            );
                            if rl_map_post != 0 {
                                tracing::warn!(
                                    bdf = %self.bdf,
                                    "PBDMA→runlist write did NOT stick — register is read-only"
                                );
                            }

                            if let Err(e) = new_init.channel.resubmit_runlist(&bar0) {
                                tracing::warn!(
                                    bdf = %self.bdf,
                                    error = %e,
                                    "runlist 0 submission failed"
                                );
                            }
                        }
                        tracing::info!(
                            bdf = %self.bdf,
                            channel_id = new_init.channel.id(),
                            runlist = new_init.channel.runlist_id_hint(),
                            "channel recreated after PFIFO reset"
                        );

                        // Re-run FECS method protocol on the new channel.
                        // FECS is still alive (we only reset PFIFO, not PGRAPH).
                        let fecs_ok = crate::vfio::channel::fecs::fecs_is_alive(&bar0);
                        if fecs_ok {
                            if let Err(e) = fecs_setup_channel(&bar0, &new_init.channel) {
                                tracing::warn!(
                                    bdf = %self.bdf,
                                    error = %e,
                                    "FECS setup on recreated channel failed"
                                );
                            }
                        } else {
                            tracing::warn!(
                                bdf = %self.bdf,
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
                            bdf = %self.bdf,
                            runlist = init.channel.runlist_id_hint(),
                            pbdma_map = format_args!("{pbdma_map_check:#010x}"),
                            rl_map_0 = format_args!("{rl_map_0:#010x}"),
                            doorbell = ?init.doorbell,
                            "post-recovery: about to re-discover PBDMA"
                        );
                        target_pbdma_base = find_target_pbdma(
                            &bar0,
                            &init.channel,
                            init.doorbell,
                            " (post-recovery)",
                        );

                        // On GV100 VFIO cold boot, no PBDMA serves runlist 0
                        // (hardware maps PBDMAs to engine runlists only).
                        // Force-program PBDMA 1 with our channel's GPFIFO/USERD
                        // and clear all latched interrupts. Direct GP_PUT writes
                        // bypass the scheduler/runlist mechanism entirely.
                        if target_pbdma_base.is_none() {
                            use crate::vfio::channel::registers::{pbdma, pccsr, ramfc};
                            let pb = pbdma::base(1);
                            let w = |off: usize, val: u32| bar0.write_u32(pb + off, val).ok();
                            let gpfifo_iova = super::GPFIFO_IOVA;
                            let userd_iova = super::USERD_IOVA;
                            let channel_id = init.channel.id();
                            let limit2 = super::GPFIFO_ENTRIES.ilog2();
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
                            let _ = bar0.write_u32(
                                pccsr::channel(channel_id),
                                pccsr::CHANNEL_ENABLE_SET,
                            );

                            // Ring doorbell
                            let _ = bar0.write_u32(
                                crate::vfio::channel::registers::usermode::NOTIFY_CHANNEL_PENDING,
                                channel_id,
                            );
                            std::thread::sleep(std::time::Duration::from_millis(50));

                            let intr_after = bar0.read_u32(pb + 0x100).unwrap_or(0xDEAD);
                            let gp_state = bar0.read_u32(pb + pbdma::GP_STATE).unwrap_or(0xDEAD);
                            tracing::info!(
                                bdf = %self.bdf,
                                intr_after = format_args!("{intr_after:#010x}"),
                                gp_state = format_args!("{gp_state:#010x}"),
                                "PBDMA 1 re-force-programmed for runlist 0 channel"
                            );

                            target_pbdma_base = Some(pb);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            bdf = %self.bdf,
                            error = %e,
                            "channel recreation after PFIFO reset failed"
                        );
                    }
                }
            }
        }

        // Catalyst path: RM firmware booted FECS/TPCs but RM's FECS idle loop
        // doesn't process our context-switch protocol.
        if catalyst_mode && !is_kepler && init.gr_ctx.is_some() {
            handle_catalyst_path(
                &bar0,
                &dma_backend,
                &init.channel,
                profile,
                &self.bdf,
            );

            // Exp 229 Phase A: If sovereign channel is still PENDING after
            // catalyst path, try adopting the RM channel directly.
            let pccsr_val = bar0.read_u32(
                crate::vfio::channel::registers::pccsr::channel(init.channel.id())
            ).unwrap_or(0);
            let status = crate::vfio::channel::registers::pccsr::status(pccsr_val);
            if status < 5 {
                tracing::info!(
                    bdf = %self.bdf,
                    sovereign_channel_id = init.channel.id(),
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "Phase B: sovereign channel still PENDING — attempting Phase A (RM channel adoption)"
                );
                match channel_init::adopt_rm_channel(
                    &dma_backend, &bar0, profile, &self.bdf,
                    self.rm_channel_id,
                ) {
                    Ok(Some(adopted_init)) => {
                        tracing::info!(
                            bdf = %self.bdf,
                            adopted_channel_id = adopted_init.channel.id(),
                            "Phase A: RM channel adopted — replacing sovereign channel"
                        );
                        init = adopted_init;
                    }
                    Ok(None) => {
                        tracing::info!(
                            bdf = %self.bdf,
                            "Phase A: no ACTIVE RM channels found — proceeding with PENDING sovereign channel"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            bdf = %self.bdf,
                            error = %e,
                            "Phase A: RM channel adoption failed — proceeding with PENDING sovereign channel"
                        );
                    }
                }
            } else {
                tracing::info!(
                    bdf = %self.bdf,
                    channel_id = init.channel.id(),
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "Phase B SUCCESS: sovereign channel is ACTIVE after catalyst path"
                );
            }
        }

        // Post-init diagnostic: verify channel readiness for dispatch.
        {
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
                bdf = %self.bdf,
                runlist_base = format_args!("{rl_base:#010x}"),
                pccsr_status,
                pccsr_status_name = pccsr::status_name(pccsr_val),
                fecs_alive,
                fecs_pc = format_args!("{fecs_pc:#010x}"),
                has_gr_ctx = init.gr_ctx.is_some(),
                dispatch_ready,
                "open_vfio: post-init channel readiness diagnostic"
            );

            if !dispatch_ready && self.fecs_ready {
                tracing::warn!(
                    bdf = %self.bdf,
                    "fecs_ready=true but channel NOT dispatch-ready — \
                     dispatch will proceed but may return zeros"
                );
            }
        }

        let semaphore = alloc_semaphore_buffer(&dma_backend, profile.completion, &self.bdf, "")?;

        self.vfio_state = Some(build_dispatch_state(
            device,
            bar0,
            init,
            dma_backend,
            semaphore,
            profile.completion,
            target_pbdma_base,
        ));

        Ok(())
    }
}

/// Probe FECS state and prepare for deferred boot (after channel creation).
fn probe_fecs_for_deferred_boot(
    bar0: &crate::vfio::device::MappedBar,
    bdf: &str,
    profile: &GenerationProfile,
    pmc_was_cold: &mut bool,
    fecs_bridge: &mut Option<NvGspBridge>,
) {
    use crate::vfio::channel::registers::falcon;

    let pmc_before = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
    if pmc_before.count_ones() < 8 {
        *pmc_was_cold = true;
        tracing::info!(
            bdf = %bdf,
            pmc_before = format_args!("{pmc_before:#010x}"),
            "PMC cold after VFIO FLR — enabling all engines"
        );
        let _ = bar0.write_u32(pmc::ENABLE as usize, 0xFFFF_FFFF);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let pmc_after = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
        tracing::info!(
            bdf = %bdf,
            pmc_after = format_args!("{pmc_after:#010x}"),
            popcount = pmc_after.count_ones(),
            "PMC engines enabled"
        );

        // PRI ring re-initialization after cold boot.
        // VFIO FLR leaves the PRI ring bus in a faulted state — GPCs,
        // PGRAPH, and scheduler registers return 0xbadfXXXX. Toggle the
        // PRIV_RING engine (PMC bit 3) to bring the ring hardware back,
        // then enumerate all satellites so GPC PRI domains are reachable.
        tracing::info!(bdf = %bdf, "PRI ring cold init: resetting PRIV_RING (PMC bit 3)");
        let priv_ring_bit = 1u32 << 3;
        let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_after & !priv_ring_bit);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_after | priv_ring_bit);
        std::thread::sleep(std::time::Duration::from_millis(20));

        // NV_PPRIV_SYS_MASTER_COMMAND = 0x12_0004 (0x4 = enumerate)
        for round in 0..5u32 {
            let _ = bar0.write_u32(0x12_0004, 0x04);
            std::thread::sleep(std::time::Duration::from_millis(20));

            // NV_PPRIV_SYS_MASTER_INTR_STATUS = 0x12_0058
            let rm_stat = bar0.read_u32(0x12_0058).unwrap_or(0xDEAD);
            // NV_PPRIV_SYS_MASTER_INTR_ACK = 0x12_004C
            if rm_stat != 0 && rm_stat & 0xBAD0_0000 != 0xBAD0_0000 {
                let _ = bar0.write_u32(0x12_004C, 0x02);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let rm_after = bar0.read_u32(0x12_0058).unwrap_or(0xDEAD);
            let is_clear = rm_after == 0;
            let is_pri_fault = rm_after & 0xBAD0_0000 == 0xBAD0_0000;
            tracing::info!(
                bdf = %bdf,
                round,
                rm_stat = format_args!("{rm_stat:#010x}"),
                rm_after = format_args!("{rm_after:#010x}"),
                is_clear,
                "PRI ring cold init: enumerate + ACK"
            );
            if is_clear { break; }
            if is_pri_fault && round >= 2 { break; }
        }

        // Clear GPC station interrupts to fully settle the ring.
        let _ = bar0.write_u32(0x12_2058, 0xFFFF_FFFF); // GPC master interrupt clear
        let _ = bar0.write_u32(0x12_8058, 0xFFFF_FFFF); // FBP station interrupt clear
        std::thread::sleep(std::time::Duration::from_millis(10));

        let gpc_enables = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0xDEAD);
        let is_gpc_fault = gpc_enables & 0xBAD0_0000 == 0xBAD0_0000;
        tracing::info!(
            bdf = %bdf,
            gpc_enables = format_args!("{gpc_enables:#010x}"),
            gpc_alive = !is_gpc_fault,
            "PRI ring cold init: GPC reachability after PRIV_RING reset"
        );
    }

    let fecs_alias = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    let fecs_pc = bar0.read_u32(falcon::FECS_BASE + falcon::PC).unwrap_or(0xDEAD);
    let is_bad_read = fecs_alias & 0xBADF_0000 == 0xBADF_0000;
    let fecs_in_hreset = !is_bad_read && (fecs_alias & falcon::CPUCTL_HRESET != 0);
    let fecs_running = !is_bad_read
        && !fecs_in_hreset
        && (fecs_alias & falcon::CPUCTL_HALTED == 0);
    let fecs_needs_boot = is_bad_read || fecs_in_hreset;

    let fecs_fw_wiped = *pmc_was_cold && fecs_pc < 0x100;

    if fecs_fw_wiped {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            pmc_was_cold = *pmc_was_cold,
            "FECS firmware wiped by VFIO FLR — need PIO reload"
        );
        let bridge = NvGspBridge::new(profile.firmware_chip);
        if bridge.has_gr_firmware() {
            tracing::info!(
                bdf = %bdf,
                chip = profile.firmware_chip,
                "FECS firmware available — deferring PIO boot to after channel creation"
            );
            *fecs_bridge = Some(bridge);
        } else {
            tracing::warn!(
                bdf = %bdf,
                chip = profile.firmware_chip,
                "No FECS firmware found on disk — FECS methods will fail"
            );
        }
    } else if fecs_running {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            "FECS already running (warm handoff preserved) — skipping boot"
        );
    } else if fecs_needs_boot {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            bad_read = is_bad_read,
            "FECS not alive — preparing deferred HS boot"
        );
        let bridge = NvGspBridge::new(profile.firmware_chip);
        if bridge.has_gr_firmware() {
            tracing::info!(
                bdf = %bdf,
                "FECS firmware available — deferring boot to after channel creation"
            );
            *fecs_bridge = Some(bridge);
        }
    }
}

/// Handle PGRAPH ungating and FECS channel setup after deferred boot.
fn handle_pgraph_ungating_and_fecs_setup(
    bar0: &crate::vfio::device::MappedBar,
    dma_backend: &crate::vfio::device::DmaBackend,
    channel: &crate::vfio::channel::VfioChannel,
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

/// Catalyst path: targeted PRI recovery + GPC ungating + nouveau falcon boot.
fn handle_catalyst_path(
    bar0: &crate::vfio::device::MappedBar,
    dma_backend: &crate::vfio::device::DmaBackend,
    channel: &crate::vfio::channel::VfioChannel,
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
