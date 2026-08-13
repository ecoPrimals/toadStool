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
    /// Accepts both `.bin` and `.bin.zst` (kernel firmware compression).
    #[must_use]
    pub fn has_gr_firmware(&self) -> bool {
        let gr = self.firmware_base.join("gr");
        let has = |name: &str| gr.join(name).exists() || gr.join(format!("{name}.zst")).exists();
        has("fecs_inst.bin") && has("fecs_data.bin")
    }

    pub(super) fn load_gr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        Self::load_blob(&self.firmware_base.join("gr"), name)
    }

    pub(super) fn load_acr_blob(&self, name: &str) -> DriverResult<Vec<u8>> {
        Self::load_blob(&self.firmware_base.join("acr"), name)
    }

    /// Load a firmware blob, transparently decompressing `.zst` if the
    /// uncompressed file doesn't exist. Linux kernel 6.2+ ships firmware
    /// as zstd-compressed by default.
    fn load_blob(dir: &std::path::Path, name: &str) -> DriverResult<Vec<u8>> {
        let plain = dir.join(name);
        if plain.exists() {
            return std::fs::read(&plain).map_err(|e| {
                DriverError::Unsupported(
                    format!("firmware read failed: {}: {e}", plain.display()).into(),
                )
            });
        }

        let zst = dir.join(format!("{name}.zst"));
        if zst.exists() {
            let compressed = std::fs::read(&zst).map_err(|e| {
                DriverError::Unsupported(
                    format!("firmware read failed: {}: {e}", zst.display()).into(),
                )
            })?;
            let mut decoder =
                ruzstd::decoding::StreamingDecoder::new(compressed.as_slice()).map_err(|e| {
                    DriverError::Unsupported(
                        format!("zstd init failed for {}: {e}", zst.display()).into(),
                    )
                })?;
            let mut decompressed = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut decompressed).map_err(|e| {
                DriverError::Unsupported(
                    format!("zstd decompress failed for {}: {e}", zst.display()).into(),
                )
            })?;
            tracing::debug!(
                file = %zst.display(),
                compressed = compressed.len(),
                decompressed = decompressed.len(),
                "firmware blob decompressed from .zst"
            );
            return Ok(decompressed);
        }

        // Follow symlinks for cross-chip shared firmware (e.g. gpccs_bl → gp107)
        let zst_link = dir.join(format!("{name}.zst"));
        Err(DriverError::Unsupported(
            format!(
                "firmware not found: {} (tried {} and {})",
                name,
                plain.display(),
                zst_link.display()
            )
            .into(),
        ))
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
