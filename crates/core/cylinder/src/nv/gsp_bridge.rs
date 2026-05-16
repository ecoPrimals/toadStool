// SPDX-License-Identifier: AGPL-3.0-or-later
//! Firmware bridge trait — abstracts coralReef GSP/ACR operations.
//!
//! [`GspBridge`] is the trait boundary between toadStool's hardware lifecycle
//! (cylinder) and coralReef's firmware domain. Functions that need firmware
//! operations (FECS boot, ACR chain, PGOB disable, GR BAR0 init) call through
//! this trait rather than importing `vfio_compute` directly.
//!
//! This keeps the wire-only principle intact: toadStool does hardware,
//! coralReef does firmware/compiler. The bridge can be implemented:
//! - Locally (when vfio_compute modules are fully absorbed into cylinder)
//! - Via IPC (JSON-RPC call to coralReef's `compute.firmware.*` methods)
//! - As [`StubGspBridge`] (sentinel for hardware without firmware needs,
//!   or before coralReef connection is established)
//!
//! **hotSpring validation (May 2026):** Warm VFIO open and sovereign init
//! stages 1-3 work with `StubGspBridge`. FECS compute context init (stage 4)
//! requires a real bridge — either warm-handoff or coralReef IPC.

use crate::error::DriverResult;
use crate::vfio::device::MappedBar;

/// Result of a falcon boot attempt.
#[derive(Debug, Clone)]
pub struct FalconBootResult {
    /// Post-boot CPUCTL register value.
    pub cpuctl_after: u32,
    /// MAILBOX0 register value after boot.
    pub mailbox0: u32,
    /// MAILBOX1 register value after boot.
    pub mailbox1: u32,
    /// Whether the falcon is running (CPUCTL not halted, mailbox non-zero).
    pub running: bool,
}

/// Result of an ACR boot solver attempt.
#[derive(Debug, Clone)]
pub struct AcrBootResult {
    /// Whether this strategy succeeded.
    pub success: bool,
    /// Name of the boot strategy used.
    pub strategy: String,
    /// Diagnostic notes from the attempt.
    pub notes: Vec<String>,
}

/// Result of a PGOB disable operation.
#[derive(Debug, Clone)]
pub struct PgobResult {
    /// Number of GPC domains detected as alive after ungating.
    pub gpc_alive: u32,
}

/// Firmware bridge — abstracts operations that require GSP/ACR/vfio_compute.
///
/// Implementors provide the actual firmware upload, ACR chain execution,
/// and GR initialization logic. toadStool's sovereign init stages call
/// through this trait boundary.
///
/// # Capability queries
///
/// Default methods (`supports_acr`, `supports_pgob`, `supports_pmu`)
/// let callers discover what the bridge can do before calling. The
/// defaults return `false`; real implementations override the ones they
/// support. This lets `sovereign_init` skip stages cleanly without
/// external `BootStrategy` matching.
pub trait GspBridge: Send + Sync {
    // ── capability queries ──────────────────────────────────────────

    /// Whether this bridge can run the ACR secure-boot chain (SEC2 DMA
    /// → ACR → FECS release). Volta+ with signed firmware.
    fn supports_acr(&self) -> bool {
        false
    }

    /// Whether this bridge can manage PGOB (Power-Gated Off Block)
    /// domains — i.e., ungate GPC compute partitions.
    fn supports_pgob(&self) -> bool {
        false
    }

    /// Whether this bridge can bootstrap the PMU falcon (unsigned
    /// PIO upload for Kepler-class GPUs).
    fn supports_pmu(&self) -> bool {
        false
    }

    /// Whether this bridge can apply GR BAR0 init writes.
    fn supports_gr_init(&self) -> bool {
        false
    }

    // ── operations ──────────────────────────────────────────────────

    /// Bootstrap the PMU falcon (unsigned PIO upload, start, mailbox
    /// handshake). Returns `Unsupported` by default.
    fn pmu_boot(
        &self,
        bar0: &MappedBar,
        imem: &[u8],
        dmem: &[u8],
    ) -> DriverResult<crate::nv::pmu_init::PmuBootResult> {
        let _ = (bar0, imem, dmem);
        Err(crate::DriverError::Unsupported(
            "PMU boot requires firmware provider (GspBridge)".into(),
        ))
    }

    /// Apply GR BAR0 initialization writes (engine enable, nonctx, dynamic).
    fn apply_gr_bar0_init(&self, bar0: &MappedBar, sm_version: u32) -> DriverResult<()>;

    /// Run the ACR boot solver for the given GPU generation.
    fn acr_boot(
        &self,
        bar0: &MappedBar,
        sm_version: u32,
        chip: &str,
        dma: Option<crate::vfio::device::DmaBackend>,
    ) -> DriverResult<Vec<AcrBootResult>>;

    /// Boot GR falcons (FECS + GPCCS) via direct PIO upload.
    fn boot_gr_falcons(&self, bar0: &MappedBar, chip: &str) -> DriverResult<FalconBootResult>;

    /// Boot FECS only (for GR init after falcon_boot).
    fn boot_fecs(&self, bar0: &MappedBar, chip: &str) -> DriverResult<FalconBootResult>;

    /// Run PGOB diagnostic (log GPC power state).
    fn pgob_diagnostic(&self, bar0: &MappedBar, label: &str);

    /// Disable PGOB (ungate GPC compute domains).
    fn pgob_disable(&self, bar0: &MappedBar) -> DriverResult<PgobResult>;
}

/// Sentinel `GspBridge` — null-object default when no firmware provider
/// is available.
///
/// This is **not** a test mock. It is the complete implementation of the
/// "no firmware provider" state. Non-firmware stages (bar0_probe, pmc_enable,
/// memory training, warm detection) run normally. Firmware-dependent stages
/// (ACR boot, FECS/GPCCS PIO upload, GR init via `boot_fecs`) return
/// [`DriverError::Unsupported`] with guidance to connect a `GspBridge`
/// implementor.
///
/// **Hardware validation note (hotSpring May 2026):** Dispatch readback
/// fails at FECS compute context init because `StubGspBridge` cannot
/// upload FECS firmware. The production path is either:
/// - coralReef provides a real `GspBridge` impl via IPC
/// - toadStool absorbs `vfio_compute` with a local `NvGspBridge` impl
/// - Warm-handoff from nouveau/nvidia-470 preserves FECS state
#[derive(Debug, Default)]
pub struct StubGspBridge;

impl GspBridge for StubGspBridge {
    fn apply_gr_bar0_init(&self, _bar0: &MappedBar, _sm_version: u32) -> DriverResult<()> {
        tracing::warn!("GspBridge stub: apply_gr_bar0_init skipped (no firmware provider)");
        Ok(())
    }

    fn acr_boot(
        &self,
        _bar0: &MappedBar,
        _sm_version: u32,
        _chip: &str,
        _dma: Option<crate::vfio::device::DmaBackend>,
    ) -> DriverResult<Vec<AcrBootResult>> {
        Err(crate::DriverError::Unsupported(
            "ACR boot requires firmware provider (GspBridge)".into(),
        ))
    }

    fn boot_gr_falcons(&self, _bar0: &MappedBar, _chip: &str) -> DriverResult<FalconBootResult> {
        Err(crate::DriverError::Unsupported(
            "falcon boot requires firmware provider (GspBridge)".into(),
        ))
    }

    fn boot_fecs(&self, _bar0: &MappedBar, _chip: &str) -> DriverResult<FalconBootResult> {
        Err(crate::DriverError::Unsupported(
            "FECS boot requires firmware provider (GspBridge)".into(),
        ))
    }

    fn pgob_diagnostic(&self, _bar0: &MappedBar, label: &str) {
        tracing::debug!(label, "GspBridge stub: pgob_diagnostic skipped");
    }

    fn pgob_disable(&self, _bar0: &MappedBar) -> DriverResult<PgobResult> {
        tracing::warn!("GspBridge stub: pgob_disable skipped (no firmware provider)");
        Ok(PgobResult { gpc_alive: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_bridge_gr_init_is_ok() {
        let bridge = StubGspBridge;
        // SAFETY: MappedBar cannot be created without real hardware,
        // so we only test the bridge trait object dispatch here.
        let _: Box<dyn GspBridge> = Box::new(bridge);
    }

    #[test]
    fn stub_bridge_capabilities_all_false() {
        let bridge = StubGspBridge;
        assert!(!bridge.supports_acr());
        assert!(!bridge.supports_pgob());
        assert!(!bridge.supports_pmu());
        assert!(!bridge.supports_gr_init());
    }

    #[test]
    fn falcon_boot_result_fields() {
        let result = FalconBootResult {
            cpuctl_after: 0x10,
            mailbox0: 0xCAFE,
            mailbox1: 0xBEEF,
            running: true,
        };
        assert!(result.running);
        assert_eq!(result.cpuctl_after, 0x10);
    }

    #[test]
    fn acr_boot_result_fields() {
        let result = AcrBootResult {
            success: false,
            strategy: "pio_fallback".into(),
            notes: vec!["no WPR".into()],
        };
        assert!(!result.success);
        assert_eq!(result.strategy, "pio_fallback");
    }

    #[test]
    fn pgob_result_fields() {
        let result = PgobResult { gpc_alive: 6 };
        assert_eq!(result.gpc_alive, 6);
    }
}
