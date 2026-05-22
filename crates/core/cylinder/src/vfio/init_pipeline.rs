// SPDX-License-Identifier: AGPL-3.0-or-later
//! Thin cross-vendor probe/verify surface.
//!
//! Full NVIDIA sovereign init goes through [`sovereign_init`](super::sovereign_init)
//! and [`SovereignStrategy`](super::sovereign_strategy::SovereignStrategy), the production
//! orchestrator with 10+ stages, warm handoff, golden-state replay, PFIFO channel setup,
//! and tier classification.
//!
//! [`InitPipeline`] exists as a *lightweight abstraction* for:
//!
//! 1. **Cross-vendor probing** — a single `probe → devinit → engine_init → verify`
//!    contract that Kepler, Volta, and future AMD/NPU implementations share.
//! 2. **Multi-die coordination** — `DeviceInit` / `DieInfo` model the K80 dual-die
//!    topology where each die runs an independent pipeline instance.
//! 3. **Experimentation** — fast four-phase pipeline for new hardware bring-up
//!    before wiring a full `SovereignStrategy`.
//!
//! The vendor-agnostic [`BootPipeline`](crate::hardware::BootPipeline) trait
//! operates over `&dyn RegisterAccess` (no `MappedBar` required), making it
//! usable from pure unit tests and emulated BAR0 backends.
//!
//! # Pipeline stages
//!
//! ```text
//! probe     → BAR0 chip identification, warm/cold detection
//! devinit   → VBIOS interpreter or PMU FALCON, VRAM training
//! engine_init → falcon boot, GR ungating, FECS context setup
//! verify    → PTIMER, PRAMIN sentinel, engine health
//! ```

use crate::error::DriverError;
use crate::nv::gr_init::ChipFamily;
use crate::vfio::device::MappedBar;

/// Result of the probe phase.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Raw BOOT0 register value.
    pub boot0: u32,
    /// Decoded chip ID from BOOT0 bits [28:20].
    pub chip_id: u32,
    /// SM version for this chip.
    pub sm_version: u32,
    /// Whether the GPU is in a warm state (PRAMIN accessible, PMC populated).
    pub warm: bool,
    /// PMC_ENABLE register value at probe time.
    pub pmc_enable: u32,
}

/// Result of the devinit phase.
#[derive(Debug, Clone)]
pub struct DevinitResult {
    /// Whether VRAM is confirmed alive after devinit.
    pub vram_alive: bool,
    /// How devinit was performed (or skipped).
    pub method: DevinitMethod,
    /// Number of MMIO writes applied (for VBIOS interpreter path).
    pub writes_applied: usize,
}

/// How device initialization was performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevinitMethod {
    /// GPU was warm — devinit skipped.
    WarmSkip,
    /// Host-side VBIOS script interpreter (Kepler path).
    VbiosInterpreter,
    /// PMU FALCON firmware upload and execution (GM200+).
    PmuFalcon,
    /// HBM2 typestate training controller (Volta, Ampere datacenter).
    Hbm2Training,
    /// GSP-RM firmware path (Ampere+ consumer, Hopper, Blackwell).
    GspRm,
}

/// Result of the engine_init phase.
#[derive(Debug, Clone)]
pub struct EngineResult {
    /// Whether FECS is running and responsive.
    pub fecs_running: bool,
    /// FECS CPUCTL register value after init.
    pub fecs_cpuctl: u32,
    /// FECS MAILBOX0 value after init.
    pub fecs_mailbox0: u32,
    /// How falcon boot was achieved.
    pub method: EngineInitMethod,
}

/// How engine initialization was performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineInitMethod {
    /// FECS warm-preserved from driver handoff.
    WarmPreserved,
    /// Direct PIO firmware upload (Kepler, no ACR).
    PioUpload,
    /// ACR secure boot chain (Volta+).
    AcrBoot,
    /// Skipped — engines gated, no firmware provider.
    WarmGated,
}

/// Result of the verify phase.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// PTIMER is running (non-zero).
    pub ptimer_alive: bool,
    /// PRAMIN sentinel test passed.
    pub vram_ok: bool,
    /// PMC_ENABLE value at verify time.
    pub pmc_enable: u32,
    /// Combined detail string.
    pub detail: String,
}

/// Per-generation GPU initialization pipeline.
///
/// Each GPU architecture family implements this trait to encode its
/// specific initialization strategy. The pipeline phases are called
/// in order: `probe` → `devinit` → `engine_init` → `verify`.
///
/// Implementations wrap the existing functions in `sovereign_stages.rs`
/// rather than duplicating logic.
pub trait InitPipeline: Send + Sync {
    /// GPU architecture family this pipeline targets.
    fn chip_family(&self) -> ChipFamily;

    /// Phase 1: Probe BAR0, identify chip, detect warm/cold state.
    fn probe(&self, bar0: &MappedBar) -> Result<ProbeResult, DriverError>;

    /// Phase 2: Device initialization (VRAM training).
    ///
    /// For warm GPUs, this should return `Ok` with `DevinitMethod::WarmSkip`.
    /// For cold GPUs, this runs the appropriate training path.
    fn devinit(
        &self,
        bar0: &MappedBar,
        probe: &ProbeResult,
    ) -> Result<DevinitResult, DriverError>;

    /// Phase 3: Engine initialization (falcon boot, GR setup).
    fn engine_init(
        &self,
        bar0: &MappedBar,
        probe: &ProbeResult,
    ) -> Result<EngineResult, DriverError>;

    /// Phase 4: Final verification (PTIMER, VRAM sentinel).
    fn verify(&self, bar0: &MappedBar) -> Result<VerifyResult, DriverError>;
}

/// Select the appropriate `InitPipeline` implementation for a given chip family.
///
/// Returns `None` for families that don't yet have an `InitPipeline`
/// implementation. Callers should fall back to the full `sovereign_init`
/// pipeline via `SovereignStrategy` for unsupported families.
pub fn pipeline_for_family(family: ChipFamily) -> Option<Box<dyn InitPipeline>> {
    match family {
        ChipFamily::Kepler => Some(Box::new(super::init_kepler::KeplerInit::new())),
        ChipFamily::Volta => Some(Box::new(super::init_volta::VoltaInit::new())),
        _ => None,
    }
}

// ── Multi-die DeviceInit ───────────────────────────────────────────────

/// Describes a physical GPU device that may contain multiple compute dies.
///
/// The K80 has two independent GK210 dies behind a PLX PCIe switch, each
/// with its own VRAM, PMU, FECS, and BDF address. Single-die GPUs (Titan V,
/// RTX 5060) have exactly one die. The `DeviceInit` model handles both by
/// treating each die as an independent `InitPipeline` invocation while
/// coordinating shared resources (VBIOS, PLX bridge health).
#[derive(Debug)]
pub struct DeviceInit {
    /// Human-readable device name (e.g. "Tesla K80", "Titan V").
    pub name: String,
    /// Per-die initialization info, ordered by BDF.
    pub dies: Vec<DieInfo>,
    /// Shared VBIOS ROM bytes (K80 shares VBIOS across dies).
    pub shared_vbios: Option<Vec<u8>>,
}

/// Information about a single compute die within a multi-die device.
#[derive(Debug, Clone)]
pub struct DieInfo {
    /// PCI BDF address for this die.
    pub bdf: String,
    /// Die index within the device (0-based).
    pub die_index: usize,
    /// Chip family for this die.
    pub chip_family: ChipFamily,
}

/// Result of initializing a multi-die device.
#[derive(Debug)]
pub struct DeviceInitResult {
    /// Per-die results, in the same order as `DeviceInit::dies`.
    pub die_results: Vec<DieInitResult>,
    /// Whether all dies reached compute-ready.
    pub all_ready: bool,
}

/// Result for a single die within a multi-die init.
#[derive(Debug)]
pub struct DieInitResult {
    /// BDF of this die.
    pub bdf: String,
    /// Die index.
    pub die_index: usize,
    /// Probe result.
    pub probe: Option<ProbeResult>,
    /// Devinit result.
    pub devinit: Option<DevinitResult>,
    /// Engine init result.
    pub engine: Option<EngineResult>,
    /// Verify result.
    pub verify: Option<VerifyResult>,
    /// Error if any phase failed.
    pub error: Option<String>,
}

impl DeviceInit {
    /// Create a single-die device description.
    pub fn single(name: impl Into<String>, bdf: impl Into<String>, family: ChipFamily) -> Self {
        Self {
            name: name.into(),
            dies: vec![DieInfo {
                bdf: bdf.into(),
                die_index: 0,
                chip_family: family,
            }],
            shared_vbios: None,
        }
    }

    /// Create a dual-die device description (e.g. Tesla K80).
    pub fn dual_die(
        name: impl Into<String>,
        bdf0: impl Into<String>,
        bdf1: impl Into<String>,
        family: ChipFamily,
    ) -> Self {
        Self {
            name: name.into(),
            dies: vec![
                DieInfo {
                    bdf: bdf0.into(),
                    die_index: 0,
                    chip_family: family,
                },
                DieInfo {
                    bdf: bdf1.into(),
                    die_index: 1,
                    chip_family: family,
                },
            ],
            shared_vbios: None,
        }
    }

    /// Attach a shared VBIOS ROM (parsed once, used by all dies).
    pub fn with_vbios(mut self, rom: Vec<u8>) -> Self {
        self.shared_vbios = Some(rom);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devinit_method_equality() {
        assert_eq!(DevinitMethod::WarmSkip, DevinitMethod::WarmSkip);
        assert_ne!(DevinitMethod::WarmSkip, DevinitMethod::VbiosInterpreter);
    }

    #[test]
    fn engine_init_method_equality() {
        assert_eq!(EngineInitMethod::PioUpload, EngineInitMethod::PioUpload);
        assert_ne!(EngineInitMethod::WarmPreserved, EngineInitMethod::AcrBoot);
    }

    #[test]
    fn probe_result_fields() {
        let p = ProbeResult {
            boot0: 0x0F22D0A1,
            chip_id: 0x0F2,
            sm_version: 35,
            warm: false,
            pmc_enable: 0,
        };
        assert_eq!(p.chip_id, 0x0F2);
        assert!(!p.warm);
    }

    #[test]
    fn single_die_device() {
        let dev = DeviceInit::single("Titan V", "0000:02:00.0", ChipFamily::Volta);
        assert_eq!(dev.dies.len(), 1);
        assert_eq!(dev.dies[0].die_index, 0);
        assert_eq!(dev.dies[0].chip_family, ChipFamily::Volta);
    }

    #[test]
    fn dual_die_device() {
        let dev = DeviceInit::dual_die(
            "Tesla K80",
            "0000:4b:00.0",
            "0000:4c:00.0",
            ChipFamily::Kepler,
        );
        assert_eq!(dev.dies.len(), 2);
        assert_eq!(dev.dies[0].bdf, "0000:4b:00.0");
        assert_eq!(dev.dies[1].bdf, "0000:4c:00.0");
        assert_eq!(dev.dies[1].die_index, 1);
    }

    #[test]
    fn device_with_shared_vbios() {
        let dev = DeviceInit::dual_die(
            "Tesla K80",
            "0000:4b:00.0",
            "0000:4c:00.0",
            ChipFamily::Kepler,
        )
        .with_vbios(vec![0x55, 0xAA]);
        assert!(dev.shared_vbios.is_some());
        assert_eq!(dev.shared_vbios.as_ref().unwrap().len(), 2);
    }
}
