// SPDX-License-Identifier: AGPL-3.0-only
// Hardware register names (GRBM_STATUS, CP_MEC_*, mmRLC_*) are standard identifiers
#![allow(clippy::doc_markdown)]
//! AMD amdgpu compute init canonical pattern — the gold-standard reference recipe.
//!
//! The amdgpu kernel driver is fully open-source, making its compute init sequence
//! the most observable and well-documented GPU compute initialization in existence.
//! This module encodes that sequence as a reference recipe that informs all
//! vendor-neutral learning.
//!
//! ## Source
//!
//! Derived from `drivers/gpu/drm/amd/amdgpu/` in the Linux kernel:
//! - `amdgpu_device.c` — top-level init
//! - `gfx_v10_0.c` / `gfx_v11_0.c` — GFX/compute engine init (RDNA2/RDNA3)
//! - `gmc_v10_0.c` / `gmc_v11_0.c` — memory controller init
//! - `smu_v13_0.c` — System Management Unit (power)
//! - `mes_v11_0.c` — Micro Engine Scheduler (GFX11+)
//!
//! ## The Universal Compute Init Pattern
//!
//! Across all GPU vendors, compute init follows the same 7-step skeleton.
//! AMD's implementation is the reference for each step:
//!
//! 1. **Probe** — IP discovery, firmware version check
//! 2. **Firmware** — Load microcode (if needed: SMU, MES, PFP, ME, CE)
//! 3. **Power** — Clock gating, DPM (Dynamic Power Management), voltage
//! 4. **Memory** — GMC (Graphics Memory Controller), VRAM, page tables
//! 5. **Engine** — GFX/compute engine reset, CP (Command Processor) init
//! 6. **Context** — Ring buffer alloc, compute queue setup, doorbell
//! 7. **Verify** — Submit trivial dispatch, readback, confirm

use crate::distiller::{
    DriverKind, Engine, GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck,
};

/// The canonical register offsets for AMD GFX10 (RDNA2) compute init.
///
/// These are documented in the kernel headers:
/// - `drivers/gpu/drm/amd/include/asic_reg/gc/gc_10_3_0_offset.h`
/// - `drivers/gpu/drm/amd/include/asic_reg/gc/gc_10_3_0_sh_mask.h`
pub mod gfx10_registers {
    /// GRBM_STATUS — engine idle/active status.
    pub const GRBM_STATUS: u64 = 0x8010;
    /// GRBM_SOFT_RESET — GFX engine soft reset control.
    pub const GRBM_SOFT_RESET: u64 = 0x8020;
    /// GRBM_GFX_CNTL — ME/pipe/queue selection for register access.
    pub const GRBM_GFX_CNTL: u64 = 0x8000;

    /// CP_MEC_ME1_UCODE_ADDR — MEC microcode load address.
    pub const CP_MEC_ME1_UCODE_ADDR: u64 = 0x8188;
    /// CP_MEC_ME1_UCODE_DATA — MEC microcode data port.
    pub const CP_MEC_ME1_UCODE_DATA: u64 = 0x818C;
    /// CP_HQD_ACTIVE — Hardware Queue Descriptor active flag.
    pub const CP_HQD_ACTIVE: u64 = 0xC91C;
    /// CP_HQD_PQ_CONTROL — Ring buffer size and control.
    pub const CP_HQD_PQ_CONTROL: u64 = 0xC938;
    /// CP_HQD_PQ_RPTR — Read pointer for compute queue.
    pub const CP_HQD_PQ_RPTR: u64 = 0xC940;
    /// CP_HQD_PQ_WPTR — Write pointer for compute queue.
    pub const CP_HQD_PQ_WPTR: u64 = 0xC950;
    /// CP_HQD_PQ_BASE — Ring buffer base address (low 32 bits).
    pub const CP_HQD_PQ_BASE: u64 = 0xC930;
    /// CP_HQD_PQ_BASE_HI — Ring buffer base address (high 32 bits).
    pub const CP_HQD_PQ_BASE_HI: u64 = 0xC934;

    /// SRBM_STATUS — system-level engine status.
    pub const SRBM_STATUS: u64 = 0x0E60;

    /// RLC_CNTL — Run List Controller enable.
    pub const RLC_CNTL: u64 = 0xB100;
    /// RLC_CSIB_ADDR_LO — Clear State Image Block address (low).
    pub const RLC_CSIB_ADDR_LO: u64 = 0xB118;
    /// RLC_CSIB_ADDR_HI — Clear State Image Block address (high).
    pub const RLC_CSIB_ADDR_HI: u64 = 0xB11C;
    /// RLC_CSIB_LENGTH — Clear State Image Block size.
    pub const RLC_CSIB_LENGTH: u64 = 0xB120;

    /// SPI_CONFIG_CNTL — Shader Processor Interrupt config.
    pub const SPI_CONFIG_CNTL: u64 = 0x9100;
    /// SPI_CONFIG_CNTL_1 — Additional SPI config.
    pub const SPI_CONFIG_CNTL_1: u64 = 0x9104;
    /// COMPUTE_DISPATCH_INITIATOR — Dispatch packet initiator.
    pub const COMPUTE_DISPATCH_INITIATOR: u64 = 0xA400;
    /// COMPUTE_NUM_THREAD_X — Dispatch thread count X dimension.
    pub const COMPUTE_NUM_THREAD_X: u64 = 0xA404;
    /// COMPUTE_NUM_THREAD_Y — Dispatch thread count Y dimension.
    pub const COMPUTE_NUM_THREAD_Y: u64 = 0xA408;
    /// COMPUTE_NUM_THREAD_Z — Dispatch thread count Z dimension.
    pub const COMPUTE_NUM_THREAD_Z: u64 = 0xA40C;
    /// COMPUTE_PGM_LO — Shader program address (low).
    pub const COMPUTE_PGM_LO: u64 = 0xA410;
    /// COMPUTE_PGM_HI — Shader program address (high).
    pub const COMPUTE_PGM_HI: u64 = 0xA414;
    /// COMPUTE_PGM_RSRC1 — Shader resource descriptor 1.
    pub const COMPUTE_PGM_RSRC1: u64 = 0xA418;
    /// COMPUTE_PGM_RSRC2 — Shader resource descriptor 2.
    pub const COMPUTE_PGM_RSRC2: u64 = 0xA41C;
    /// COMPUTE_TMPRING_SIZE — LDS/temp ring size for wavefronts.
    pub const COMPUTE_TMPRING_SIZE: u64 = 0xA424;
    /// COMPUTE_USER_DATA_0 — First user-data slot for shader.
    pub const COMPUTE_USER_DATA_0: u64 = 0xA500;
}

/// Build the canonical AMD GFX10 (RDNA2) compute init recipe.
///
/// This is the gold-standard reference: what a fully-working open-source
/// driver does to initialize GPU compute. Every other vendor's recipe
/// should follow this same abstract skeleton.
#[must_use]
pub fn amd_gfx10_compute_init() -> InitRecipe {
    use gfx10_registers::{
        CP_HQD_ACTIVE, CP_HQD_PQ_BASE, CP_HQD_PQ_BASE_HI, CP_HQD_PQ_CONTROL, CP_HQD_PQ_RPTR,
        CP_HQD_PQ_WPTR, GRBM_GFX_CNTL, GRBM_SOFT_RESET, GRBM_STATUS, RLC_CNTL, RLC_CSIB_ADDR_HI,
        RLC_CSIB_ADDR_LO, RLC_CSIB_LENGTH, SPI_CONFIG_CNTL,
    };

    let steps = vec![
        // ── Step 1: Probe ──
        // GRBM_STATUS: verify GFX engine is responsive
        InitStep::Verify {
            check: VerifyCheck::RegisterMatch {
                offset: GRBM_STATUS,
                expected: 0,
                mask: 0x8000_0000, // GUI_ACTIVE bit should be 0 (idle)
            },
        },
        // ── Step 2: Firmware ──
        // Load MEC (Micro Engine Compute) microcode
        // The kernel loads PFP, ME, CE, MEC1, MEC2 firmware via CP registers
        InitStep::FirmwareLoad {
            engine: Engine::Custom("MEC".into()),
            path: "/lib/firmware/amdgpu/navi21_mec.bin".into(),
        },
        // ── Step 3: Power ──
        // Enable clock gating for GFX (reduce idle power)
        InitStep::RegisterWrite {
            offset: RLC_CNTL,
            value: 0x1, // RLC_ENABLE
            function: RegFunction::ClockEnable,
        },
        // ── Step 4: Memory ──
        // Set up RLC CSIB (Clear State Image Block) for context save/restore
        InitStep::RegisterWrite {
            offset: RLC_CSIB_ADDR_LO,
            value: 0, // placeholder — actual address set at runtime
            function: RegFunction::MemoryConfig,
        },
        InitStep::RegisterWrite {
            offset: RLC_CSIB_ADDR_HI,
            value: 0,
            function: RegFunction::MemoryConfig,
        },
        InitStep::RegisterWrite {
            offset: RLC_CSIB_LENGTH,
            value: 4096,
            function: RegFunction::MemoryConfig,
        },
        // ── Step 5: Engine ──
        // Soft-reset the GFX engine to clean state
        InitStep::RegisterWrite {
            offset: GRBM_SOFT_RESET,
            value: 0x1, // GFX_RESET
            function: RegFunction::EngineReset,
        },
        InitStep::Delay { us: 50 },
        // Clear the reset bit
        InitStep::RegisterWrite {
            offset: GRBM_SOFT_RESET,
            value: 0x0,
            function: RegFunction::EngineReset,
        },
        InitStep::Delay { us: 50 },
        // Select pipe 0, queue 0, ME 1 for compute (via GRBM_GFX_CNTL)
        InitStep::RegisterWrite {
            offset: GRBM_GFX_CNTL,
            value: 0x0100_0000, // MEID=1, PIPEID=0, QUEUEID=0
            function: RegFunction::ChannelBind,
        },
        // ── Step 6: Context ──
        // Set up Hardware Queue Descriptor (HQD) for compute queue
        // Deactivate HQD first
        InitStep::RegisterWrite {
            offset: CP_HQD_ACTIVE,
            value: 0,
            function: RegFunction::ContextAlloc,
        },
        // Set ring buffer base address (page-aligned GPU virtual address)
        InitStep::RegisterWrite {
            offset: CP_HQD_PQ_BASE,
            value: 0, // placeholder — set at runtime
            function: RegFunction::ContextAlloc,
        },
        InitStep::RegisterWrite {
            offset: CP_HQD_PQ_BASE_HI,
            value: 0,
            function: RegFunction::ContextAlloc,
        },
        // Configure ring size and control
        // Bits: [5:0] = ring size log2 (e.g., 12 = 4096 entries)
        // [8] = rptr writeback enable
        InitStep::RegisterWrite {
            offset: CP_HQD_PQ_CONTROL,
            value: 0x0000_010C, // size=12 (4K entries), rptr_writeback=1
            function: RegFunction::ContextAlloc,
        },
        // Initialize read and write pointers to 0
        InitStep::RegisterWrite {
            offset: CP_HQD_PQ_RPTR,
            value: 0,
            function: RegFunction::ContextAlloc,
        },
        InitStep::RegisterWrite {
            offset: CP_HQD_PQ_WPTR,
            value: 0,
            function: RegFunction::ContextAlloc,
        },
        // Activate the queue
        InitStep::RegisterWrite {
            offset: CP_HQD_ACTIVE,
            value: 1,
            function: RegFunction::ContextAlloc,
        },
        // Enable interrupts for compute completion
        InitStep::RegisterWrite {
            offset: SPI_CONFIG_CNTL,
            value: 0x1, // enable compute dispatch
            function: RegFunction::InterruptEnable,
        },
        // ── Step 7: Verify ──
        // Submit a trivial compute dispatch and verify readback
        InitStep::Verify {
            check: VerifyCheck::ComputeReadback,
        },
    ];

    InitRecipe {
        source_arch: GpuArch {
            vendor: Vendor::Amd,
            generation: "RDNA2".into(),
            chip: "Navi21".into(),
            compute_class: "gfx1030".into(),
        },
        source_driver: DriverKind::Amdgpu,
        target_arch: GpuArch {
            vendor: Vendor::Amd,
            generation: "RDNA2".into(),
            chip: "Navi21".into(),
            compute_class: "gfx1030".into(),
        },
        steps,
        confidence: 1.0, // Gold standard — derived from kernel source
        description: "AMD GFX10 (RDNA2/Navi21) canonical compute init — \
                      7-step universal pattern derived from amdgpu kernel driver"
            .into(),
    }
}

/// The universal 7-step compute init skeleton.
///
/// This is the vendor-neutral abstraction that all GPU compute init
/// follows, regardless of vendor. AMD's implementation is the reference.
#[derive(Debug, Clone, Copy)]
pub enum UniversalInitPhase {
    /// 1. Probe hardware identity and firmware versions.
    Probe,
    /// 2. Load firmware (if required: PMU/GSP/GuC/MEC).
    Firmware,
    /// 3. Initialize power management (clocks, voltage, thermal).
    Power,
    /// 4. Configure memory (page tables, VRAM, context save areas).
    Memory,
    /// 5. Reset and initialize compute engine.
    Engine,
    /// 6. Allocate context (channel, ring buffer, queue, doorbell).
    Context,
    /// 7. Verify: submit trivial compute, readback result.
    Verify,
}

impl UniversalInitPhase {
    /// Map a `RegFunction` to its universal init phase.
    #[must_use]
    pub const fn from_reg_function(f: RegFunction) -> Self {
        match f {
            RegFunction::ClockEnable | RegFunction::PowerGate | RegFunction::ThermalConfig => {
                Self::Power
            }
            RegFunction::MemoryConfig => Self::Memory,
            RegFunction::EngineReset => Self::Engine,
            RegFunction::ContextAlloc | RegFunction::ChannelBind | RegFunction::InterruptEnable => {
                Self::Context
            }
            RegFunction::Unknown => Self::Probe,
        }
    }

    /// All phases in order.
    pub const ALL: [Self; 7] = [
        Self::Probe,
        Self::Firmware,
        Self::Power,
        Self::Memory,
        Self::Engine,
        Self::Context,
        Self::Verify,
    ];
}

impl std::fmt::Display for UniversalInitPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Probe => write!(f, "1. Probe"),
            Self::Firmware => write!(f, "2. Firmware"),
            Self::Power => write!(f, "3. Power"),
            Self::Memory => write!(f, "4. Memory"),
            Self::Engine => write!(f, "5. Engine"),
            Self::Context => write!(f, "6. Context"),
            Self::Verify => write!(f, "7. Verify"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp, reason = "test asserts exact constructed values")]
    fn baseline_recipe_has_all_phases() {
        let recipe = amd_gfx10_compute_init();
        assert_eq!(recipe.confidence, 1.0);
        assert!(recipe.steps.len() >= 15);
        assert_eq!(recipe.source_arch.vendor, Vendor::Amd);

        let has_engine_reset = recipe.steps.iter().any(|s| {
            matches!(
                s,
                InitStep::RegisterWrite {
                    function: RegFunction::EngineReset,
                    ..
                }
            )
        });
        assert!(has_engine_reset);

        let has_context = recipe.steps.iter().any(|s| {
            matches!(
                s,
                InitStep::RegisterWrite {
                    function: RegFunction::ContextAlloc,
                    ..
                }
            )
        });
        assert!(has_context);

        let has_verify = recipe.steps.iter().any(|s| {
            matches!(
                s,
                InitStep::Verify {
                    check: VerifyCheck::ComputeReadback
                }
            )
        });
        assert!(has_verify);
    }

    #[test]
    fn universal_phases_cover_all_reg_functions() {
        for func in [
            RegFunction::ClockEnable,
            RegFunction::PowerGate,
            RegFunction::ThermalConfig,
            RegFunction::MemoryConfig,
            RegFunction::EngineReset,
            RegFunction::ContextAlloc,
            RegFunction::ChannelBind,
            RegFunction::InterruptEnable,
            RegFunction::Unknown,
        ] {
            let _phase = UniversalInitPhase::from_reg_function(func);
        }
    }

    #[test]
    fn universal_phases_display() {
        assert_eq!(UniversalInitPhase::Probe.to_string(), "1. Probe");
        assert_eq!(UniversalInitPhase::Verify.to_string(), "7. Verify");
    }

    #[test]
    fn all_phases_count() {
        assert_eq!(UniversalInitPhase::ALL.len(), 7);
    }

    #[test]
    fn gfx10_register_offsets_documented() {
        assert_eq!(gfx10_registers::GRBM_STATUS, 0x8010);
        assert_eq!(gfx10_registers::GRBM_SOFT_RESET, 0x8020);
        assert_eq!(gfx10_registers::CP_HQD_ACTIVE, 0xC91C);
        assert_eq!(gfx10_registers::COMPUTE_DISPATCH_INITIATOR, 0xA400);
    }
}
