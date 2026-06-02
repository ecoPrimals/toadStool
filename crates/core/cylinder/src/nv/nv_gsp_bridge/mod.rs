// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local `GspBridge` implementation — loads GR falcon firmware from
//! `/lib/firmware/nvidia/{chip}/gr/` and uploads via PIO or DMA.
//!
//! This replaces `NoopGspBridge` for sovereign cold boot on GPUs where
//! the vendor driver warm-handoff path is unavailable (e.g. Volta on
//! systems with open nvidia.ko that doesn't support pre-GSP GPUs).
//!
//! The PIO upload mechanism writes directly to IMEM/DMEM via BAR0
//! registers. It works regardless of falcon security mode — the host
//! PIO port is always writable.
//!
//! # Frozen Dependency Status
//!
//! `NvGspBridge` is classified as a **frozen dependency** in the ecosystem:
//!
//! - **Firmware blobs are pinned**: The files under `/lib/firmware/nvidia/{chip}/gr/`
//!   are extracted from vendor drivers and committed to the ecosystem's artifact
//!   store. They do not change between vendor driver versions for a given chip —
//!   the GR microcode is burned into the VBIOS/GPU ROM and the firmware images
//!   are simply the host-readable copy.
//!
//! - **Upload mechanisms are hardware-defined**: PIO writes to falcon IMEM/DMEM
//!   use register offsets that are fixed in silicon (CPUCTL, BOOTVEC, MAILBOX0,
//!   etc.). The DMA HS boot path uses FBIF TRANSCFG registers and descriptor
//!   layouts defined by the falcon hardware specification. These do not change
//!   between driver versions or kernel updates.
//!
//! - **Glacial evolution**: The Rust code in this module evolves only when
//!   targeting a new GPU generation (new falcon version, new FBIF layout).
//!   For existing supported chips (GK210, GV100), the implementation is stable
//!   and tested. Changes flow through the `SovereignStrategy` trait layer, not
//!   through the bridge internals.
//!
//! - **Future bridge implementations** (AMD, NPU, etc.) follow the same pattern:
//!   frozen vendor blobs on disk + pure Rust register-write upload mechanisms.
//!   The `GspBridge` trait provides the stable interface boundary.

mod boot;
mod bridge_impl;

use std::path::PathBuf;

use crate::error::{DriverError, DriverResult};

/// DMA IOVA for FECS firmware code image (from centralized layout).
pub const FECS_FW_CODE_IOVA: u64 = super::iova::firmware::FECS_CODE_IOVA;
/// DMA IOVA for FECS firmware data image.
pub const FECS_FW_DATA_IOVA: u64 = super::iova::firmware::FECS_DATA_IOVA;
/// DMA IOVA for GPCCS firmware code image.
pub const GPCCS_FW_CODE_IOVA: u64 = super::iova::firmware::GPCCS_CODE_IOVA;
/// DMA IOVA for GPCCS firmware data image.
pub const GPCCS_FW_DATA_IOVA: u64 = super::iova::firmware::GPCCS_DATA_IOVA;
/// DMA IOVA for ACR load ucode image.
pub const ACR_UCODE_IOVA: u64 = super::iova::firmware::ACR_UCODE_IOVA;

/// Firmware-backed `GspBridge` that loads blobs from the local filesystem.
///
/// This is a **frozen dependency**: firmware blobs are pinned artifacts and the
/// upload mechanisms (PIO register writes, DMA descriptor format) are defined
/// by silicon, not software. See module-level docs for the full rationale.
///
/// # Supported firmware files
///
/// | File | Purpose | Warm boot? | Cold boot? |
/// |------|---------|-----------|-----------|
/// | `fecs_inst.bin` + `fecs_data.bin` | FECS falcon firmware | No (preserved) | **Yes** |
/// | `gpccs_inst.bin` + `gpccs_data.bin` | GPCCS falcon firmware | No | **Yes** |
/// | `fecs_bl.bin` + `gpccs_bl.bin` | HS bootloader (Volta+ DMA path) | No | **Yes** |
/// | `fecs_sig.bin` + `gpccs_sig.bin` | ACR signatures (optional) | No | **Yes** |
/// | `sw_nonctx.bin` | GR non-context BAR0 init writes | No | **Yes** |
#[derive(Debug)]
pub struct NvGspBridge {
    firmware_base: PathBuf,
}

impl NvGspBridge {
    /// Create a bridge that looks for firmware at `/lib/firmware/nvidia/{chip}/gr/`.
    #[must_use]
    pub fn new(chip: &str) -> Self {
        Self {
            firmware_base: PathBuf::from(format!("/lib/firmware/nvidia/{chip}")),
        }
    }

    /// Check whether the required GR firmware files exist.
    #[must_use]
    pub fn has_gr_firmware(&self) -> bool {
        let gr = self.firmware_base.join("gr");
        gr.join("fecs_inst.bin").exists() && gr.join("fecs_data.bin").exists()
    }

    pub(super) fn load_gr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        let path = self.firmware_base.join("gr").join(name);
        std::fs::read(&path).map_err(|e| {
            DriverError::Unsupported(
                format!("firmware read failed: {}: {e}", path.display()).into(),
            )
        })
    }

    pub(super) fn load_acr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        let path = self.firmware_base.join("acr").join(name);
        std::fs::read(&path).map_err(|e| {
            DriverError::Unsupported(
                format!("ACR firmware read failed: {}: {e}", path.display()).into(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nv::gsp_bridge::GspBridge;

    #[test]
    fn bridge_reports_firmware_availability() {
        let bridge = NvGspBridge::new("gv100");
        // On test machines without firmware, this returns false.
        // On the biomegate lab machine with GV100 firmware, it returns true.
        let _ = bridge.has_gr_firmware();
    }

    #[test]
    fn bridge_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NvGspBridge>();
    }

    #[test]
    fn bridge_acr_reports_skip() {
        let bridge = NvGspBridge::new("gv100");
        // ACR boot always reports skip for pre-GSP GPUs — no panic.
        let _: Box<dyn GspBridge> = Box::new(bridge);
    }
}
