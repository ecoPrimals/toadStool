// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA VFIO compute device — sovereign GPU dispatch via BAR0/PBDMA.
//!
//! Implements [`ComputeDevice`] for NVIDIA GPUs bound to `vfio-pci`. This is
//! the direct-dispatch path: toadStool owns the GPU fd, programs PBDMA channels
//! via BAR0 MMIO, and reads back results without any kernel driver intermediary.
//!
//! # FECS gate
//!
//! The full dispatch path (alloc → upload → dispatch → sync → readback)
//! requires a running FECS (Falcon Engine Compute Scheduler):
//!
//! - **Warm path** (nouveau/nvidia-470 preserves FECS state): dispatch works
//! - **Cold path** (FECS needs firmware upload): requires real `GspBridge`
//! - **Noop path** (`NoopGspBridge`): FECS boot returns `Unsupported`
//!
//! # PBDMA dispatch
//!
//! After [`open_vfio`](NvVfioComputeDevice::open_vfio), the device holds a live
//! PFIFO channel with GPFIFO ring and USERD page. `alloc`/`upload`/`readback`
//! map through [`DmaBuffer`](crate::vfio::dma::DmaBuffer), and `dispatch`
//! submits a pushbuffer via GPFIFO + doorbell.

mod compute;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
mod channel_init;
#[cfg(target_os = "linux")]
mod dispatch_state;
#[cfg(target_os = "linux")]
mod gr_falcon_boot;
#[cfg(target_os = "linux")]
mod gr_ungating;
#[cfg(target_os = "linux")]
mod open_anchor;
#[cfg(target_os = "linux")]
mod open_vfio;
#[cfg(target_os = "linux")]
mod open_vfio_catalyst;
#[cfg(target_os = "linux")]
mod open_vfio_fecs_probe;
#[cfg(target_os = "linux")]
mod open_vfio_pfifo_recovery;
#[cfg(target_os = "linux")]
mod open_vfio_pgraph;
#[cfg(target_os = "linux")]
mod open_vfio_readiness;
#[cfg(target_os = "linux")]
mod pbdma;
#[cfg(target_os = "linux")]
mod warm_probe;

use crate::error::DriverResult;
use crate::HardwareCapabilities;

use super::iova;

pub(crate) const GPFIFO_IOVA: u64 = iova::dispatch::GPFIFO_IOVA;
pub(crate) const USERD_IOVA: u64 = iova::dispatch::USERD_IOVA;
pub(crate) const GR_CTX_IOVA: u64 = iova::dispatch::GR_CTX_IOVA;
pub(crate) const GR_CTX_SIZE: usize = iova::dispatch::GR_CTX_SIZE;
pub(crate) const USER_BUFFER_BASE_IOVA: u64 = iova::dispatch::USER_BUFFER_BASE_IOVA;
/// GPFIFO entry count (4 KiB / 8 bytes per entry = 512).
pub(crate) const GPFIFO_ENTRIES: u32 = 512;
pub(crate) const IOVA_LIMIT: u64 = iova::IOVA_LIMIT;
pub(crate) const PAGE_SIZE: u64 = iova::PAGE_SIZE;

#[cfg(target_os = "linux")]
pub(crate) use dispatch_state::{DoorbellKind, VfioDispatchState};

/// NVIDIA GPU compute device via VFIO direct dispatch.
///
/// Created from a PCI BDF. Capabilities are initially `UNKNOWN` until
/// BAR0 is probed for BOOT0 → SM version → generation profile.
pub struct NvVfioComputeDevice {
    pub(crate) bdf: String,
    pub(crate) caps: HardwareCapabilities,
    pub(crate) sm: u32,
    pub(crate) fecs_ready: bool,
    /// Post-catalyst state: RM firmware booted FECS/TPC, now under VFIO.
    /// When true, `open_vfio` skips destructive PRI ring recovery and
    /// pgraph reset to preserve the catalyst-established hardware state.
    pub(crate) catalyst_warm: bool,
    /// Exp 229: RM-allocated channel ID (from RmChannelEvidence).
    /// Used by Phase A fallback to adopt the RM channel if Phase B fails.
    pub(crate) rm_channel_id: Option<u32>,
    #[cfg(target_os = "linux")]
    pub(crate) vfio_state: Option<VfioDispatchState>,
}

impl NvVfioComputeDevice {
    /// Create a new NVIDIA VFIO compute device for the given BDF.
    ///
    /// Initializes with `HardwareCapabilities::UNKNOWN`. Call
    /// [`probe_capabilities`](Self::probe_capabilities) after BAR0 open
    /// to populate real caps from the BOOT0 register.
    #[must_use]
    pub fn new(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            caps: HardwareCapabilities::UNKNOWN,
            sm: 0,
            fecs_ready: false,
            catalyst_warm: false,
            rm_channel_id: None,
            #[cfg(target_os = "linux")]
            vfio_state: None,
        }
    }

    /// Create a device with known SM version (from prior BAR0 probe or
    /// warm handoff detection).
    #[must_use]
    pub fn with_sm(bdf: String, sm: u32) -> Self {
        let profile = super::generation::profile_for_sm(sm);
        Self {
            bdf,
            caps: profile.to_capabilities(),
            sm,
            fecs_ready: false,
            catalyst_warm: false,
            rm_channel_id: None,
            #[cfg(target_os = "linux")]
            vfio_state: None,
        }
    }

    /// Probe capabilities from BOOT0 register if BAR0 is accessible.
    ///
    /// On success, updates the internal capabilities from the GPU's
    /// generation profile. Requires sysfs BAR0 access (VFIO feature).
    #[cfg(feature = "vfio")]
    pub fn probe_capabilities(&mut self) -> DriverResult<()> {
        const BAR0_MIN_SIZE: usize = 0x1000;
        let bar0 = crate::vfio::sysfs_bar0::SysfsBar0::open(&self.bdf, BAR0_MIN_SIZE)
            .map_err(|e| DriverError::Unsupported(format!("BAR0 open failed: {e}").into()))?;
        let boot0 = bar0.read_u32(0);
        if let Some(sm) = super::identity::boot0_to_sm(boot0) {
            let profile = super::generation::profile_for_sm(sm);
            self.caps = profile.to_capabilities();
            self.sm = sm;
            tracing::info!(
                bdf = %self.bdf, sm, chip = super::identity::chip_name(sm),
                "NVIDIA VFIO: probed capabilities from BOOT0"
            );
        }
        Ok(())
    }

    /// Mark FECS as ready (warm-preserved or firmware booted).
    pub fn set_fecs_ready(&mut self, ready: bool) {
        self.fecs_ready = ready;
    }

    /// BDF address of this device.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// SM version detected from BOOT0 (0 if not yet probed).
    #[must_use]
    pub fn sm_version(&self) -> u32 {
        self.sm
    }

    /// Whether FECS compute context is available for dispatch.
    #[must_use]
    pub fn is_fecs_ready(&self) -> bool {
        self.fecs_ready
    }

    /// Whether the VFIO dispatch path is initialized.
    #[must_use]
    pub fn is_vfio_open(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.vfio_state.is_some()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

use crate::error::DriverError;
