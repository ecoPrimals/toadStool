// SPDX-License-Identifier: AGPL-3.0-or-later
//! Open VFIO device and create PFIFO channel for PBDMA dispatch.

use crate::error::DriverResult;
use crate::nv::gsp_bridge::GspBridge;
use crate::nv::registers::{gpc, pgraph, pmc};
use crate::vfio::VfioDevice;

use super::super::generation::{PageTableFormat, GenerationProfile};
use super::super::nv_gsp_bridge::NvGspBridge;
use super::channel_init::{alloc_semaphore_buffer, build_dispatch_state, init_channel_buffers};
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

        let init = init_channel_buffers(
            &dma_backend,
            &bar0,
            &profile,
            is_kepler,
            self.fecs_ready,
            &self.bdf,
            "",
        )?;

        tracing::info!(
            bdf = %self.bdf,
            channel_id = init.channel.id(),
            fecs_ready = self.fecs_ready,
            generation = profile.name,
            doorbell = ?init.doorbell,
            "VFIO PBDMA dispatch state initialized"
        );

        let target_pbdma_base =
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
                &profile,
                &self.bdf,
                fecs_hs_booted,
            )?;
        }

        // Catalyst path: RM firmware booted FECS/TPCs but RM's FECS idle loop
        // doesn't process our context-switch protocol.
        if catalyst_mode && !is_kepler && init.gr_ctx.is_some() {
            handle_catalyst_path(
                &bar0,
                &dma_backend,
                &init.channel,
                &profile,
                &self.bdf,
            );
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
            &profile,
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
        &profile,
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
