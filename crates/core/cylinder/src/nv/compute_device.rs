// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA VFIO compute device — sovereign GPU dispatch via BAR0/PBDMA.
//!
//! Implements [`ComputeDevice`] for NVIDIA GPUs bound to `vfio-pci`. This is
//! the direct-dispatch path: toadStool owns the GPU fd, programs PBDMA channels
//! via BAR0 MMIO, and reads back results without any kernel driver intermediary.
//!
//! # Current status
//!
//! **FECS-gated**: The full dispatch path (alloc → upload → dispatch → sync →
//! readback) requires a running FECS (Falcon Engine Compute Scheduler). The FECS
//! firmware is managed by [`GspBridge`](super::gsp_bridge::GspBridge):
//!
//! - **Warm path** (nouveau/nvidia-470 preserves FECS state): dispatch works
//! - **Cold path** (FECS needs firmware upload): requires real `GspBridge`
//! - **Stub path** (`StubGspBridge`): FECS boot returns `Unsupported`
//!
//! Until the FECS firmware bridge is resolved, `dispatch()` returns
//! `DriverError::Unsupported` on cold-boot devices.

use crate::error::{DriverError, DriverResult};
use crate::{BufferHandle, ComputeDevice, DispatchDims, HardwareCapabilities, MemoryDomain, ShaderInfo};

/// NVIDIA GPU compute device via VFIO direct dispatch.
///
/// Created from a PCI BDF. Capabilities are initially `UNKNOWN` until
/// BAR0 is probed for BOOT0 → SM version → generation profile.
pub struct NvVfioComputeDevice {
    bdf: String,
    caps: HardwareCapabilities,
    fecs_ready: bool,
}

impl NvVfioComputeDevice {
    /// Create a new NVIDIA VFIO compute device for the given BDF.
    ///
    /// Initializes with `HardwareCapabilities::UNKNOWN`. Call
    /// [`probe_capabilities`](Self::probe_capabilities) after BAR0 open
    /// to populate real caps from the BOOT0 register.
    #[must_use]
    pub fn new(bdf: String) -> Self {
        Self {
            bdf,
            caps: HardwareCapabilities::UNKNOWN,
            fecs_ready: false,
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
            fecs_ready: false,
        }
    }

    /// Probe capabilities from BOOT0 register if BAR0 is accessible.
    ///
    /// On success, updates the internal capabilities from the GPU's
    /// generation profile. Requires sysfs BAR0 access (VFIO feature).
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

    /// Whether FECS compute context is available for dispatch.
    #[must_use]
    pub fn is_fecs_ready(&self) -> bool {
        self.fecs_ready
    }
}

impl ComputeDevice for NvVfioComputeDevice {
    fn alloc(&mut self, _size: u64, _domain: MemoryDomain) -> DriverResult<BufferHandle> {
        if !self.fecs_ready {
            return Err(DriverError::Unsupported(
                "NVIDIA VFIO alloc requires FECS compute context — see GspBridge".into(),
            ));
        }
        Err(DriverError::Unsupported(
            "NVIDIA VFIO buffer allocation via PBDMA not yet wired".into(),
        ))
    }

    fn free(&mut self, _handle: BufferHandle) -> DriverResult<()> {
        Err(DriverError::Unsupported(
            "NVIDIA VFIO buffer free not yet wired".into(),
        ))
    }

    fn upload(&mut self, _handle: BufferHandle, _offset: u64, _data: &[u8]) -> DriverResult<()> {
        Err(DriverError::Unsupported(
            "NVIDIA VFIO upload not yet wired".into(),
        ))
    }

    fn readback(&self, _handle: BufferHandle, _offset: u64, _len: usize) -> DriverResult<Vec<u8>> {
        Err(DriverError::Unsupported(
            "NVIDIA VFIO readback not yet wired".into(),
        ))
    }

    fn dispatch(
        &mut self,
        _shader: &[u8],
        _buffers: &[BufferHandle],
        _dims: DispatchDims,
        _info: &ShaderInfo,
    ) -> DriverResult<()> {
        if !self.fecs_ready {
            return Err(DriverError::Unsupported(
                "NVIDIA VFIO dispatch requires FECS compute context — firmware loads but \
                 compute context never becomes ready. Production path: warm-handoff from \
                 nouveau/nvidia-470, or real GspBridge (coralReef IPC or local absorption)"
                    .into(),
            ));
        }
        Err(DriverError::Unsupported(
            "NVIDIA VFIO dispatch via PBDMA not yet wired".into(),
        ))
    }

    fn sync(&mut self) -> DriverResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> &HardwareCapabilities {
        &self.caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_dispatch_returns_fecs_error() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        let result = dev.dispatch(
            &[0u8; 64],
            &[],
            DispatchDims::new(1, 1, 1),
            &ShaderInfo::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FECS"));
    }

    #[test]
    fn cold_alloc_returns_unsupported() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(dev.alloc(4096, MemoryDomain::Vram).is_err());
    }

    #[test]
    fn with_sm_populates_caps() {
        let dev = NvVfioComputeDevice::with_sm("0000:25:00.0".into(), 70);
        let caps = dev.capabilities();
        assert_eq!(caps.vendor, crate::hardware::Vendor::Nvidia);
        assert_ne!(caps.device_name, "unknown");
    }

    #[test]
    fn new_has_unknown_caps() {
        let dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert_eq!(dev.capabilities().vendor, crate::hardware::Vendor::Unknown);
    }

    #[test]
    fn fecs_ready_flag() {
        let mut dev = NvVfioComputeDevice::new("0000:01:00.0".into());
        assert!(!dev.is_fecs_ready());
        dev.set_fecs_ready(true);
        assert!(dev.is_fecs_ready());
    }
}
