// SPDX-License-Identifier: AGPL-3.0-or-later
//! Open VFIO dispatch state from pre-existing anchor/ember fds.

use crate::error::DriverResult;
use crate::vfio::VfioDevice;

use super::super::generation::PageTableFormat;
use super::channel_init::{alloc_semaphore_buffer, build_dispatch_state, init_channel_buffers};
use super::gr_falcon_boot::{boot_gpccs_fecs_catalyst, fecs_setup_channel};
use super::gr_ungating::{ungate_gr_engine, UngatingLog};
use super::pbdma::find_target_pbdma;
use super::NvVfioComputeDevice;

impl NvVfioComputeDevice {
    /// Open VFIO dispatch state using pre-existing fds from an anchor/ember.
    ///
    /// Identical to [`open_vfio`](Self::open_vfio) except the `VfioDevice` is reconstructed
    /// from received fds (via `VfioDevice::from_received`) instead of
    /// opening `/dev/vfio/{group}` directly. This avoids the EBUSY conflict
    /// when ember already holds the VFIO group.
    pub fn open_vfio_from_received(
        &mut self,
        fds: crate::vfio::ReceivedVfioFds,
    ) -> DriverResult<()> {
        let profile = super::super::generation::profile_for_sm(self.sm);
        let is_kepler = matches!(
            profile.page_table_format,
            PageTableFormat::V1TwoLevel
        );

        let device = VfioDevice::from_received(&self.bdf, fds)?;
        let bar0 = device.map_bar(0)?;
        let dma_backend = device.dma_backend();

        let fecs_running = if !is_kepler && self.fecs_ready {
            use crate::vfio::channel::registers::falcon;
            let fecs_alias = bar0
                .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
                .unwrap_or(0xDEAD);
            let fecs_pc = bar0
                .read_u32(falcon::FECS_BASE + falcon::PC)
                .unwrap_or(0xDEAD);
            let halted = fecs_alias & falcon::CPUCTL_HALTED != 0;
            let in_hreset = fecs_alias & falcon::CPUCTL_HRESET != 0;
            let running = !halted && !in_hreset;
            tracing::info!(
                bdf = %self.bdf,
                fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
                fecs_pc = format_args!("{fecs_pc:#010x}"),
                running,
                "adopt_anchor: FECS state check"
            );
            running
        } else {
            false
        };

        let init = init_channel_buffers(
            &dma_backend,
            &bar0,
            &profile,
            is_kepler,
            self.fecs_ready,
            &self.bdf,
            "anchor adopt",
        )?;

        // Full GPC ungating before FECS method protocol.
        if self.catalyst_warm && !is_kepler && init.gr_ctx.is_some() {
            handle_anchor_catalyst_ungating(
                &bar0,
                &dma_backend,
                &init.channel,
                &profile,
                &self.bdf,
            );
        } else if fecs_running && !is_kepler {
            handle_anchor_fecs_running_ungating(&bar0, &init.channel, &profile, &self.bdf)?;
        } else if fecs_running {
            fecs_setup_channel(&bar0, &init.channel)?;
        }

        tracing::info!(
            bdf = %self.bdf,
            channel_id = init.channel.id(),
            fecs_ready = self.fecs_ready,
            fecs_running,
            generation = profile.name,
            doorbell = ?init.doorbell,
            "VFIO PBDMA dispatch state initialized (from anchor fds)"
        );

        let target_pbdma_base =
            find_target_pbdma(&bar0, &init.channel, init.doorbell, " (anchor)");

        let semaphore =
            alloc_semaphore_buffer(&dma_backend, profile.completion, &self.bdf, "anchor adopt")?;

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

fn handle_anchor_catalyst_ungating(
    bar0: &crate::vfio::device::MappedBar,
    dma_backend: &crate::vfio::device::DmaBackend,
    channel: &crate::vfio::channel::VfioChannel,
    profile: &super::super::generation::GenerationProfile,
    bdf: &str,
) {
    tracing::info!(
        bdf = %bdf,
        "anchor catalyst: PRI ungating + nouveau FECS boot"
    );

    let bridge = super::super::nv_gsp_bridge::NvGspBridge::new(profile.firmware_chip);

    let _ = ungate_gr_engine(
        bar0,
        &bridge,
        &profile,
        UngatingLog {
            prefix: "anchor catalyst",
            log_phases: false,
            ack_pmc_intr: false,
            verbose_mmu: false,
        },
    );

    boot_gpccs_fecs_catalyst(bar0, &bridge, dma_backend, bdf, "anchor catalyst");

    std::thread::sleep(std::time::Duration::from_millis(100));
    if crate::vfio::channel::fecs::fecs_is_alive(bar0) {
        let _ = fecs_setup_channel(bar0, channel);
    }
}

fn handle_anchor_fecs_running_ungating(
    bar0: &crate::vfio::device::MappedBar,
    channel: &crate::vfio::channel::VfioChannel,
    profile: &super::super::generation::GenerationProfile,
    bdf: &str,
) -> DriverResult<()> {
    let bridge = super::super::nv_gsp_bridge::NvGspBridge::new(profile.firmware_chip);

    let result = ungate_gr_engine(
        bar0,
        &bridge,
        &profile,
        UngatingLog {
            prefix: "anchor",
            log_phases: true,
            ack_pmc_intr: false,
            verbose_mmu: false,
        },
    );
    tracing::info!(bdf = %bdf, changes = result.cg_changes, "anchor: CG sweep");
    tracing::info!(bdf = %bdf, alive = result.pri_alive, "anchor: PRI recovery");

    fecs_setup_channel(bar0, channel)
}
