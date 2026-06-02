// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO hardware channel creation for Volta+ via BAR0 MMIO.
//!
//! Creates a GPU command channel from scratch using direct register writes,
//! bypassing the kernel GPU driver. This is the bridge between VFIO BAR0/DMA
//! setup and actual GPU command dispatch — without a channel, the GPU's PFIFO
//! engine does not know our GPFIFO ring exists.
//!
//! # Channel creation sequence
//!
//! 1. Allocate DMA buffers for instance block, runlist, and V2 page tables
//! 2. Populate RAMFC (GPFIFO base, USERD pointer, channel ID, signature)
//! 3. Set up V2 MMU page tables (identity map for first 2 MiB of IOVA space)
//! 4. Build runlist with TSG header + channel entry (Volta RAMRL format)
//! 5. Bind instance block to channel via PCCSR registers
//! 6. Enable channel and submit runlist to PFIFO

pub mod devinit;
pub mod fecs;
pub mod glowplug;
pub mod hbm2_training;
mod bar2_init;
mod devinit_ops;
mod kepler_channel;
mod mmu;
#[expect(
    missing_docs,
    reason = "diagnostic oracle — struct fields are self-documenting"
)]
pub mod mmu_oracle;
pub mod nouveau_oracle;
pub mod oracle;
pub mod pri_monitor;
pub mod registers;

pub mod diagnostic;
pub mod mmu_fault;
mod page_tables;
pub(crate) mod pfifo;

pub use diagnostic::{
    ExperimentConfig, ExperimentOrdering, ExperimentResult, GpuCapabilities,
    build_experiment_matrix, build_metal_discovery_matrix, diagnostic_matrix,
    interpreter::{ProbeInterpreter, ProbeReport, memory_probe},
};
pub use pfifo::PfifoInitConfig;
pub use registers::ramuserd;

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::{DmaBackend, MappedBar};
use crate::vfio::dma::DmaBuffer;

use registers::*;

/// PFIFO hardware channel — owns all DMA resources for a single GPU channel.
///
/// Created during `NvVfioComputeDevice::open()` and held alive for
/// the device lifetime. Dropped automatically when the parent device drops,
/// releasing all DMA allocations.
pub struct VfioChannel {
    pub(super) instance: DmaBuffer,
    pub(super) runlist: DmaBuffer,
    pub(super) pd3: DmaBuffer,
    pub(super) pd2: DmaBuffer,
    pub(super) pd1: DmaBuffer,
    pub(super) pd0: DmaBuffer,
    pub(super) pt0: DmaBuffer,
    #[expect(dead_code, reason = "kept alive for DMA buffer lifecycle")]
    pub(super) fault_buf: DmaBuffer,
    #[expect(dead_code, reason = "IOMMU guard pages — prevent stale PBDMA IO_PAGE_FAULTs")]
    pub(super) guard_pages: Vec<DmaBuffer>,
    pub(super) channel_id: u32,
    pub(super) runlist_id: u32,
}

impl VfioChannel {
    /// Create a PFIFO channel using a `GenerationProfile` to drive
    /// generation-specific behavior.
    ///
    /// This is the unified entry point — callers pass the profile and
    /// a warm-handoff flag instead of choosing between `create_kepler`,
    /// `create_warm`, or `create`. Internally dispatches to the correct
    /// page table format, instance block layout, and runlist submission
    /// strategy.
    ///
    /// # Errors
    ///
    /// Returns error if any DMA allocation or BAR0 write fails.
    #[expect(
        clippy::too_many_arguments,
        reason = "hardware channel init requires all these distinct physical parameters"
    )]
    pub fn create_for_profile(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
        profile: &crate::nv::generation::GenerationProfile,
        warm_handoff: bool,
    ) -> DriverResult<Self> {
        use crate::nv::generation::PageTableFormat;

        match profile.page_table_format {
            PageTableFormat::V1TwoLevel => {
                let guard = crate::nv::hardware_guard::GuardedBar::new(bar0, 16)
                    .map_err(|e| DriverError::Unsupported(Cow::Owned(
                        format!("Kepler BAR0 guard init: {e}")
                    )))?;
                Self::create_kepler(
                    container,
                    &guard,
                    gpfifo_iova,
                    gpfifo_entries,
                    userd_iova,
                    channel_id,
                )
            }
            PageTableFormat::V2FiveLevel => {
                let config = pfifo::PfifoInitConfig::for_thermal_state(warm_handoff, warm_handoff);
                Self::create_with_config(
                    container,
                    bar0,
                    gpfifo_iova,
                    gpfifo_entries,
                    userd_iova,
                    channel_id,
                    &config,
                )
            }
        }
    }

    /// Create and activate a GPU PFIFO channel via BAR0 register programming.
    ///
    /// This performs the full channel lifecycle:
    /// 1. Allocate DMA buffers for instance block, runlist, and page tables
    /// 2. Populate RAMFC (GPFIFO base, USERD, channel ID)
    /// 3. Set up V2 MMU page tables (identity map for first 2 MiB)
    /// 4. Build runlist with TSG header + channel entry
    /// 5. Bind instance block and enable channel via PCCSR
    /// 6. Submit runlist to PFIFO
    ///
    /// # Errors
    ///
    /// Returns error if any DMA allocation or BAR0 write fails.
    pub fn create(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
    ) -> DriverResult<Self> {
        Self::create_with_config(
            container,
            bar0,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
            &pfifo::PfifoInitConfig::default(),
        )
    }

    /// Create a VFIO channel in warm handoff mode — preserves PFIFO/PMC
    /// state from nouveau so falcon engines (FECS/GPCCS) remain alive.
    pub fn create_warm(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
    ) -> DriverResult<Self> {
        Self::create_with_config(
            container,
            bar0,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
            &pfifo::PfifoInitConfig::warm_handoff(),
        )
    }

    /// Create a VFIO channel preserving a live FECS falcon — skips PMC PFIFO
    /// reset and PFIFO toggle that cascade into the GR engine on Volta.
    pub fn create_fecs_alive(
        container: DmaBackend,
        bar0: &MappedBar,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
        channel_id: u32,
    ) -> DriverResult<Self> {
        Self::create_with_config(
            container,
            bar0,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
            &pfifo::PfifoInitConfig::warm_fecs_alive(),
        )
    }

    /// Exp 229 Phase A: Adopt an existing RM channel from PCCSR.
    ///
    /// Uses `create_fecs_alive` to set up DMA infrastructure with the same
    /// channel_id as the RM-created channel. This creates our own IOVA
    /// mappings and page tables, then re-binds the channel in PCCSR to
    /// point at our instance block. The key insight: RM's channel already
    /// primed FECS, so when we re-bind with our instance block (which has
    /// our GPFIFO/USERD), FECS should already have valid ctx-switch state.
    ///
    /// Only called if Phase B (new sovereign channel) fails.
    pub fn adopt_existing(
        container: DmaBackend,
        bar0: &MappedBar,
        channel_id: u32,
        gpfifo_iova: u64,
        gpfifo_entries: u32,
        userd_iova: u64,
    ) -> DriverResult<Self> {
        tracing::info!(
            channel_id,
            "adopt_existing: reusing RM channel ID with sovereign DMA infrastructure"
        );

        // Use the warm+fecs-alive path — it preserves PFIFO state
        // (including RM's scheduler configuration) while setting up our
        // own instance block, page tables, and RAMFC.
        Self::create_fecs_alive(
            container,
            bar0,
            gpfifo_iova,
            gpfifo_entries,
            userd_iova,
            channel_id,
        )
    }

    /// Channel ID used for doorbell notification.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.channel_id
    }

    /// Runlist ID this channel was submitted on (for PBDMA discovery).
    #[must_use]
    pub const fn runlist_id_hint(&self) -> u32 {
        self.runlist_id
    }

    /// Override the target runlist ID. Use when the hardware rejects
    /// writes to the original runlist's per-RL registers (GV100 PRI
    /// domain fault after PGRAPH reset).
    pub fn force_runlist(&mut self, id: u32) {
        self.runlist_id = id;
    }

    /// BAR0 offset for the USERMODE doorbell register.
    #[must_use]
    pub const fn doorbell_offset() -> usize {
        usermode::NOTIFY_CHANNEL_PENDING
    }
}

impl std::fmt::Debug for VfioChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VfioChannel")
            .field("channel_id", &self.channel_id)
            .field("instance_iova", &format_args!("{INSTANCE_IOVA:#x}"))
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_info_constants() {
        assert_eq!(VfioChannel::doorbell_offset(), 0x81_0090);
    }
}
