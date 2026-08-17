// SPDX-License-Identifier: AGPL-3.0-or-later
//! Open VFIO device and create PFIFO channel for PBDMA dispatch.

use crate::error::DriverResult;
use crate::vfio::VfioDevice;
use crate::vfio::channel::pfifo::validate_gr_runlist;

use super::super::generation::PageTableFormat;
use super::super::nv_gsp_bridge::NvGspBridge;
use super::NvVfioComputeDevice;
use super::channel_init::{
    self, alloc_semaphore_buffer, build_dispatch_state, init_channel_buffers,
};
use super::gr_falcon_boot::boot_gpccs_fecs_deferred;
use super::pbdma::find_target_pbdma;

use super::open_vfio_catalyst::handle_catalyst_path;
use super::open_vfio_fecs_probe::probe_fecs_for_deferred_boot;
use super::open_vfio_pfifo_recovery::{
    log_post_pgraph_pfifo_state, pfifo_truly_broken, recover_broken_pfifo,
};
use super::open_vfio_pgraph::handle_pgraph_ungating_and_fecs_setup;
use super::open_vfio_readiness::log_channel_readiness;

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
        let is_kepler = matches!(profile.page_table_format, PageTableFormat::V1TwoLevel);

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

        let mut target_pbdma_base = find_target_pbdma(&bar0, &init.channel, init.doorbell, "");

        // PTOP engine-runlist validation: verify the channel's runlist ID
        // matches the GR engine's hardware-assigned runlist. Prevents
        // wrong-slot failure where runlist writes land on a different
        // engine's PRI domain (RCA failure mode #2).
        if !is_kepler
            && let Some(hw_rl) =
                validate_gr_runlist(&bar0, profile, init.channel.runlist_id_hint(), &self.bdf)
            && hw_rl != init.channel.runlist_id_hint()
        {
            tracing::warn!(
                bdf = %self.bdf,
                correcting_from = init.channel.runlist_id_hint(),
                correcting_to = hw_rl,
                "forcing channel to hardware-verified GR runlist"
            );
            init.channel.force_runlist(hw_rl);
        }

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
            let broken = pfifo_truly_broken(&bar0, &init.channel);
            log_post_pgraph_pfifo_state(&bar0, &self.bdf, &init.channel, broken);

            if broken {
                (init, target_pbdma_base) = recover_broken_pfifo(
                    &bar0,
                    &dma_backend,
                    init,
                    profile,
                    is_kepler,
                    &self.bdf,
                    target_pbdma_base,
                );
            }
        }

        // Catalyst path: RM firmware booted FECS/TPCs but RM's FECS idle loop
        // doesn't process our context-switch protocol.
        if catalyst_mode && !is_kepler && init.gr_ctx.is_some() {
            handle_catalyst_path(&bar0, &dma_backend, &init.channel, profile, &self.bdf);

            // Exp 229 Phase A: If sovereign channel is still PENDING after
            // catalyst path, try adopting the RM channel directly.
            let pccsr_val = bar0
                .read_u32(crate::vfio::channel::registers::pccsr::channel(
                    init.channel.id(),
                ))
                .unwrap_or(0);
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
                    &dma_backend,
                    &bar0,
                    profile,
                    &self.bdf,
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

        log_channel_readiness(&bar0, &self.bdf, &init, self.fecs_ready);

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
