// SPDX-License-Identifier: AGPL-3.0-or-later
//! AMD Vega 20 (MI50/MI60, GFX906) `GpuMetal` + `BootPipeline` implementation.
//!
//! **Status (Wave 5):** Probe and warm-detection are functional. Cold boot
//! (`devinit`, `engine_init`) returns `Unsupported` — AMD hardware is not yet
//! available for sovereign validation. Use
//! [`crate::vfio::sovereign_stages::ChipDetection`] / `detect_chip()` to
//! distinguish "AMD present but cold boot not implemented" from "no GPU found".
//!
//! Register offsets derived from AMD's publicly documented MMIO layout
//! and Mesa amdgpu driver headers for Vega/GFX906. Key subsystems:
//!
//! - **SMC**: System Management Controller (power, thermal — indirect access)
//! - **GRBM**: Graphics Request Broker Manager (engine status, soft reset)
//! - **UMC**: Unified Memory Controller (HBM2 interface)
//! - **GFX**: Graphics Core (compute dispatch, shader execution)
//! - **SDMA**: System DMA engines (memory copy, fill)
//! - **MMHUB**: Memory Management Hub (GART, page tables)
//!
//! The `VegaInit` struct implements the vendor-agnostic `BootPipeline` trait,
//! proving the trait works cross-vendor. Probe reads GRBM_STATUS for warm
//! detection; `devinit` / `engine_init` return `Unsupported` until AMD cold
//! boot is implemented (see module-level status note).

use super::bar_cartography::DomainHint;
use super::gpu_vendor::*;
use super::pci_discovery::GpuVendor;
use crate::error::DriverError;
use crate::hardware::{BootInitInfo, BootPipeline, BootProbeInfo, Vendor};
use crate::vfio::device::RegisterAccess;

// ── AMD Vega 20 / GFX906 MMIO Register Offsets ───────────────────────
//
// Sources: AMD documentation, Mesa amdgpu driver headers (soc15, gfx_v9_0).
// Offsets are byte addresses within the relevant IP block's MMIO window.

// SMC (System Management Controller) — indirect register access
const SMC_IND_INDEX_11: usize = 0x01AC;
const SMC_IND_DATA_11: usize = 0x01AD;

// GRBM (Graphics Request Broker Manager)
pub(crate) const GRBM_STATUS: usize = 0x8010;
pub(crate) const GRBM_STATUS2: usize = 0x8008;
const GRBM_SOFT_RESET: usize = 0x8020;

// GFX / GC (Graphics Core) — memory config
const GB_ADDR_CONFIG: usize = 0x263E;

// UMC / MC (Memory Controller) — VRAM aperture
const MC_VM_FB_LOCATION_BASE: usize = 0x2023;
const MC_VM_FB_LOCATION_TOP: usize = 0x2024;

// SRBM, CP, SDMA, RLC, MMHUB
pub(crate) const SRBM_STATUS: usize = 0x0E50;
const CP_STAT: usize = 0x8680;
const SDMA0_BASE: usize = 0x4D00;
const SDMA1_BASE: usize = 0x5900;
const RLC_BASE: usize = 0xEC00;
const MMHUB_VM_BASE: usize = 0x0600;

const MI50_HBM2_SIZE: u64 = 16 * 1024 * 1024 * 1024;
const MI50_HBM2_STACKS: u32 = 4;
const MI50_L2_SIZE: u64 = 4 * 1024 * 1024;
const MI50_L2_SLICES: u32 = 16;
pub(crate) const BUSY_BIT_MASK: u32 = 0x8000_0000;

/// AMD Vega 20 (MI50) identity.
#[derive(Debug, Clone)]
pub struct AmdVegaIdentity {
    /// Raw identity register value (GC_CONFIG or PCI device ID).
    pub raw: u32,
}

impl GpuIdentity for AmdVegaIdentity {
    fn vendor(&self) -> GpuVendor {
        GpuVendor::Amd
    }
    fn chip_name(&self) -> &'static str {
        "Vega 20 (MI50)"
    }
    fn architecture(&self) -> &'static str {
        "GFX906"
    }
    fn implementation(&self) -> u8 {
        20
    }
    fn revision(&self) -> u8 {
        0
    }
    fn raw_id(&self) -> u32 {
        self.raw
    }
}

/// AMD Vega 20 `GpuMetal` — bare-metal register access for MI50/MI60.
///
/// Register cartography and warm-probe paths are implemented. Sovereign cold
/// boot is not — pair with [`VegaInit`] for probe-only workflows until AMD
/// init stages land.
#[derive(Debug)]
pub struct AmdVegaMetal {
    identity: AmdVegaIdentity,
    power_domains: Vec<PowerDomain>,
    memory_regions: Vec<MetalMemoryRegion>,
    engines: Vec<EngineInfo>,
}

impl AmdVegaMetal {
    /// Create a new Vega 20 metal instance with GFX906 register layout.
    pub fn new(raw_id: u32) -> Self {
        Self {
            identity: AmdVegaIdentity { raw: raw_id },
            power_domains: vec![
                PowerDomain {
                    name: "SMC",
                    enable_reg: Some(SMC_IND_INDEX_11),
                    enable_bit: None,
                    clock_reg: Some(SMC_IND_DATA_11),
                    state: DomainState::Unknown,
                },
                PowerDomain {
                    name: "GFX",
                    enable_reg: Some(GRBM_SOFT_RESET),
                    enable_bit: Some(0x01),
                    clock_reg: None,
                    state: DomainState::Unknown,
                },
                PowerDomain {
                    name: "GRBM",
                    enable_reg: Some(GRBM_STATUS),
                    enable_bit: None,
                    clock_reg: Some(GRBM_STATUS2),
                    state: DomainState::Unknown,
                },
                PowerDomain {
                    name: "UMC",
                    enable_reg: Some(MC_VM_FB_LOCATION_BASE),
                    enable_bit: None,
                    clock_reg: Some(MC_VM_FB_LOCATION_TOP),
                    state: DomainState::Unknown,
                },
                PowerDomain {
                    name: "SYS",
                    enable_reg: Some(SRBM_STATUS),
                    enable_bit: None,
                    clock_reg: None,
                    state: DomainState::Unknown,
                },
                PowerDomain {
                    name: "RLC",
                    enable_reg: None,
                    enable_bit: None,
                    clock_reg: None,
                    state: DomainState::Unknown,
                },
            ],
            memory_regions: vec![
                MetalMemoryRegion {
                    name: "HBM2_FB",
                    kind: MemoryKind::Vram,
                    control_base: Some(MC_VM_FB_LOCATION_BASE),
                    size: Some(MI50_HBM2_SIZE),
                    partitions: Some(MI50_HBM2_STACKS),
                },
                MetalMemoryRegion {
                    name: "GART",
                    kind: MemoryKind::SystemMemory,
                    control_base: Some(MMHUB_VM_BASE),
                    size: None,
                    partitions: None,
                },
                MetalMemoryRegion {
                    name: "L2_CACHE",
                    kind: MemoryKind::L2Cache,
                    control_base: None,
                    size: Some(MI50_L2_SIZE),
                    partitions: Some(MI50_L2_SLICES),
                },
            ],
            engines: vec![
                EngineInfo {
                    name: "GFX",
                    kind: EngineKind::Compute,
                    base_offset: CP_STAT,
                    has_firmware: true,
                    firmware_state: FirmwareState::NotLoaded,
                },
                EngineInfo {
                    name: "SDMA0",
                    kind: EngineKind::Copy,
                    base_offset: SDMA0_BASE,
                    has_firmware: true,
                    firmware_state: FirmwareState::NotLoaded,
                },
                EngineInfo {
                    name: "SDMA1",
                    kind: EngineKind::Copy,
                    base_offset: SDMA1_BASE,
                    has_firmware: true,
                    firmware_state: FirmwareState::NotLoaded,
                },
            ],
        }
    }
}

impl GpuMetal for AmdVegaMetal {
    fn identity(&self) -> &dyn GpuIdentity {
        &self.identity
    }

    fn power_domains(&self) -> &[PowerDomain] {
        &self.power_domains
    }

    fn memory_regions(&self) -> &[MetalMemoryRegion] {
        &self.memory_regions
    }

    fn engine_list(&self) -> &[EngineInfo] {
        &self.engines
    }

    fn register_domain(&self, name: &str) -> Option<(usize, usize)> {
        match name {
            "SMC" => Some((SMC_IND_INDEX_11, SMC_IND_DATA_11 + 4)),
            "GRBM" => Some((0x8000, 0x8FFF)),
            "GRBM_STATUS" => Some((GRBM_STATUS, GRBM_STATUS + 4)),
            "GRBM_STATUS2" => Some((GRBM_STATUS2, GRBM_STATUS2 + 4)),
            "GFX" | "GC" => Some((0x2000, 0x3FFF)),
            "GB_ADDR_CONFIG" => Some((GB_ADDR_CONFIG, GB_ADDR_CONFIG + 4)),
            "UMC" | "MC" => Some((MC_VM_FB_LOCATION_BASE, MC_VM_FB_LOCATION_TOP + 4)),
            "SRBM" => Some((0x0E00, 0x0EFF)),
            "CP" => Some((0x8600, 0x86FF)),
            "SDMA0" => Some((0x4D00, 0x4DFF)),
            "SDMA1" => Some((0x5900, 0x59FF)),
            "RLC" => Some((0xEC00, 0xECFF)),
            "MMHUB" => Some((0x0600, 0x0AFF)),
            _ => None,
        }
    }

    fn domain_hints(&self) -> Vec<DomainHint> {
        vec![
            DomainHint {
                start: SMC_IND_INDEX_11,
                end: SMC_IND_DATA_11 + 4,
                name: "SMC",
            },
            DomainHint {
                start: 0x8000,
                end: 0x8FFF,
                name: "GRBM",
            },
            DomainHint {
                start: 0x0E00,
                end: 0x0EFF,
                name: "SRBM",
            },
            DomainHint {
                start: 0x8600,
                end: 0x86FF,
                name: "CP",
            },
            DomainHint {
                start: 0x4D00,
                end: 0x4DFF,
                name: "SDMA0",
            },
            DomainHint {
                start: 0x5900,
                end: 0x59FF,
                name: "SDMA1",
            },
            DomainHint {
                start: RLC_BASE,
                end: 0xECFF,
                name: "RLC",
            },
            DomainHint {
                start: MMHUB_VM_BASE,
                end: 0x0AFF,
                name: "MMHUB",
            },
            DomainHint {
                start: 0x2000,
                end: 0x3FFF,
                name: "GFX",
            },
            DomainHint {
                start: MC_VM_FB_LOCATION_BASE,
                end: MC_VM_FB_LOCATION_TOP + 4,
                name: "UMC",
            },
            DomainHint {
                start: GB_ADDR_CONFIG,
                end: GB_ADDR_CONFIG + 4,
                name: "GB_ADDR_CONFIG",
            },
        ]
    }

    fn warmup_sequence(&self) -> Vec<WarmupStep> {
        vec![WarmupStep {
            description: "Read GRBM_STATUS and GRBM_STATUS2 (GFX engine idle)",
            writes: vec![],
            delay_ms: 0,
            verify: vec![
                RegisterVerify {
                    offset: GRBM_STATUS,
                    expected: 0,
                    mask: BUSY_BIT_MASK,
                },
                RegisterVerify {
                    offset: GRBM_STATUS2,
                    expected: 0,
                    mask: BUSY_BIT_MASK,
                },
                RegisterVerify {
                    offset: SRBM_STATUS,
                    expected: 0,
                    mask: BUSY_BIT_MASK,
                },
            ],
        }]
    }

    fn boot0_offset(&self) -> usize {
        GRBM_STATUS
    }

    fn pmc_enable_offset(&self) -> usize {
        GRBM_SOFT_RESET
    }

    fn pbdma_map_offset(&self) -> Option<usize> {
        None
    }

    fn pramin_base_offset(&self) -> Option<usize> {
        None
    }

    fn bar2_block_offset(&self) -> Option<usize> {
        None
    }
}

// ── Vendor-Agnostic BootPipeline: VegaInit ───────────────────────────

/// Probe result for AMD Vega 20.
#[derive(Debug, Clone)]
pub struct VegaProbeResult {
    /// GRBM_STATUS register value (AMD's primary engine status register).
    pub grbm_status: u32,
    /// GRBM_STATUS2 register value.
    pub grbm_status2: u32,
    /// SRBM_STATUS register value (system request broker).
    pub srbm_status: u32,
    /// Whether the GFX engine appears warm (GRBM not returning 0xFFFFFFFF).
    pub warm: bool,
}

/// Init result for AMD Vega 20 (stub).
#[derive(Debug, Clone)]
pub struct VegaInitResult {
    /// Whether VRAM is alive after init.
    pub memory_alive: bool,
    /// Init method description.
    pub method: String,
}

/// AMD Vega 20 (MI50/MI60) boot pipeline — probe functional, cold boot pending.
///
/// Implements `BootPipeline` using the GRBM register map already defined
/// in this module. The probe phase is functional — it reads GRBM_STATUS,
/// GRBM_STATUS2, and SRBM_STATUS to detect engine state.
///
/// **Cold boot status:** `devinit` and `engine_init` return
/// [`DriverError::Unsupported`] with guidance. Warm GPUs skip devinit via
/// `"warm-skip"`. For PCI-level detection before opening BAR0, see
/// [`crate::vfio::sovereign_stages::detect_chip`].
///
/// This stub proves the `BootPipeline` trait works cross-vendor: the same
/// `probe → is_warm → devinit → engine_init → verify` contract applies
/// to AMD GCN/CDNA hardware with completely different register semantics.
#[derive(Debug)]
pub struct VegaInit {
    #[expect(dead_code, reason = "BDF stored for future AMD metal init pipeline")]
    pub(crate) bdf: Option<String>,
}

impl Default for VegaInit {
    fn default() -> Self {
        Self::new()
    }
}

impl VegaInit {
    /// Create a Vega pipeline with no BDF.
    pub fn new() -> Self {
        Self { bdf: None }
    }

    /// Create a Vega pipeline targeting a specific PCI BDF address.
    pub fn with_bdf(bdf: impl Into<String>) -> Self {
        Self {
            bdf: Some(bdf.into()),
        }
    }
}

impl BootPipeline for VegaInit {
    type ProbeResult = VegaProbeResult;
    type InitResult = VegaInitResult;

    fn device_family(&self) -> &'static str {
        "Vega 20"
    }

    fn probe(&self, bar: &dyn RegisterAccess) -> Result<VegaProbeResult, DriverError> {
        let grbm = bar
            .read_u32(GRBM_STATUS as u32)
            .map_err(|e| DriverError::Unsupported(format!("GRBM_STATUS read: {e}").into()))?;
        let grbm2 = bar.read_u32(GRBM_STATUS2 as u32).unwrap_or(0xFFFF_FFFF);
        let srbm = bar.read_u32(SRBM_STATUS as u32).unwrap_or(0xFFFF_FFFF);

        let warm = grbm != 0xFFFF_FFFF && grbm != 0;

        Ok(VegaProbeResult {
            grbm_status: grbm,
            grbm_status2: grbm2,
            srbm_status: srbm,
            warm,
        })
    }

    fn is_warm(&self, probe: &VegaProbeResult) -> bool {
        probe.warm
    }

    fn probe_summary(&self, probe: &VegaProbeResult) -> BootProbeInfo {
        BootProbeInfo {
            vendor: Vendor::Amd,
            family: "Vega 20".to_string(),
            warm: probe.warm,
            identity_raw: probe.grbm_status,
        }
    }

    fn devinit(
        &self,
        _bar: &dyn RegisterAccess,
        probe: &VegaProbeResult,
    ) -> Result<VegaInitResult, DriverError> {
        if probe.warm {
            return Ok(VegaInitResult {
                memory_alive: true,
                method: "warm-skip".to_string(),
            });
        }
        Err(DriverError::Unsupported(
            "AMD Vega cold devinit not implemented — GPU present but sovereign \
             cold boot pending; warm GPUs detected via probe may skip devinit"
                .into(),
        ))
    }

    fn init_summary(&self, init: &VegaInitResult) -> BootInitInfo {
        BootInitInfo {
            memory_alive: init.memory_alive,
            writes_applied: 0,
            method: init.method.clone(),
        }
    }

    fn engine_init(
        &self,
        _bar: &dyn RegisterAccess,
        _probe: &VegaProbeResult,
    ) -> Result<(), DriverError> {
        Err(DriverError::Unsupported(
            "AMD Vega engine_init not implemented — GPU present but sovereign \
             cold boot pending; register cartography and warm probe are functional"
                .into(),
        ))
    }

    fn verify(&self, bar: &dyn RegisterAccess) -> Result<bool, DriverError> {
        let grbm = bar.read_u32(GRBM_STATUS as u32).unwrap_or(0xFFFF_FFFF);
        let srbm = bar.read_u32(SRBM_STATUS as u32).unwrap_or(0xFFFF_FFFF);
        let gfx_idle = (grbm & BUSY_BIT_MASK) == 0;
        let sys_idle = (srbm & BUSY_BIT_MASK) == 0;
        Ok(gfx_idle && sys_idle && grbm != 0xFFFF_FFFF)
    }
}
