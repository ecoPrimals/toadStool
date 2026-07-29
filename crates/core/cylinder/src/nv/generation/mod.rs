// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA GPU generation profiles — single source of truth for per-generation knowledge.
//!
//! Every property that varies by GPU generation (QMD version, channel class,
//! compute class, local memory window, completion strategy, boot strategy,
//! launch method) is consolidated into [`GenerationProfile`]. All code that
//! previously branched on raw SM numbers now consults [`profile_for_sm`].
//!
//! Adding a new generation = one new `const GenerationProfile`, zero new
//! match arms scattered across the codebase.

mod ada;
mod ampere;
mod blackwell;
mod hopper;
mod kepler;
mod maxwell;
mod pascal;
mod turing;
mod volta;

use std::ops::RangeInclusive;

use crate::nv::registers::pmc::InterruptProfile;

/// QMD (Queue Management Descriptor) version for this generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QmdVersion {
    /// 256-byte (64-word) — Pascal and earlier.
    V21,
    /// 256-byte (64-word) — Volta, Turing.
    V22,
    /// 256-byte (64-word) — Ampere (GA100/GA10x). NVK/CUDA confirmed v2.3.
    V23,
    /// 256-byte (64-word) — Ada, Hopper.
    V30,
    /// 384-byte (96-word) — Blackwell and later.
    V50,
}

/// Push buffer launch method for compute dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMethod {
    /// `SEND_SIGNALING_PCAS_B` (0x02BC) — Volta, Turing.
    Pcas,
    /// `SEND_SIGNALING_PCAS2_B` (0x02C0) — Ampere and later.
    Pcas2,
}

/// GPFIFO completion tracking strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionStrategy {
    /// Poll USERD GP_GET until it matches GP_PUT — pre-Blackwell.
    GpGetPoll,
    /// Semaphore release via compute engine — Blackwell+ (GP_GET removed from USERD).
    SemaphoreFence,
}

/// Sovereign boot strategy for VFIO / direct hardware init.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStrategy {
    /// Kepler: no ACR/WPR, direct PIO falcon upload.
    NoAcr,
    /// Volta/Pascal/Turing/Ampere/Ada: SEC2 DMA → ACR chain → FECS release.
    AcrSec2,
    /// Blackwell+: kernel module `GPU_PROMOTE_CTX` for GR context buffers.
    KmodPromote,
    /// Generation exists but sovereign boot path is untested.
    Untested,
}

/// Power safety profile for sovereign boot PMC_ENABLE sequencing.
///
/// Older GPUs (Kepler, Maxwell) have no firmware-managed power sequencing.
/// Writing 0xFFFF_FFFF to PMC_ENABLE on a cold GPU can instantly ungate
/// all engine clock domains, causing inrush current spikes that exceed
/// the VRM's capacity — especially on high-TDP multi-die cards (K80).
///
/// This profile controls how aggressively the sovereign pipeline enables
/// engine clock domains during cold boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerSafetyProfile {
    /// PMC_ENABLE mask for initial engine bring-up (stage 2).
    /// Only these engines will be clocked before VBIOS devinit.
    /// Bits: 0=PPCI, 1=PBUS, 4=PTIMER, 5=PFB, 6=PGRAPH, etc.
    pub initial_pmc_mask: u32,
    /// Whether full PMC_ENABLE (0xFFFF_FFFF) is safe after devinit.
    /// True for Volta+ where firmware manages power rails.
    /// False for Kepler/Maxwell where devinit IS the power sequencer.
    pub full_enable_after_devinit: bool,
    /// Whether PMC_ENABLE must be rolled back on devinit failure.
    /// True for all pre-GSP generations.
    pub rollback_on_devinit_failure: bool,
}

/// Conservative mask: PPCI + PBUS + PTIMER + PFIFO + PMC essentials.
/// Does NOT enable PGRAPH, CE, NVDEC, or memory controller engines.
const PMC_MASK_CONSERVATIVE: u32 = 0xC000_2030;

/// Full ungating — firmware-managed generations only.
const PMC_MASK_FULL: u32 = 0xFFFF_FFFF;

impl PowerSafetyProfile {
    /// Pre-firmware generations: only enable minimal engines before devinit.
    pub const PRE_FIRMWARE: Self = Self {
        initial_pmc_mask: PMC_MASK_CONSERVATIVE,
        full_enable_after_devinit: false,
        rollback_on_devinit_failure: true,
    };

    /// Firmware-managed generations: safe to enable all engines.
    pub const FIRMWARE_MANAGED: Self = Self {
        initial_pmc_mask: PMC_MASK_FULL,
        full_enable_after_devinit: true,
        rollback_on_devinit_failure: false,
    };
}

pub use crate::hardware::MemoryType;

/// Source of `@builtin(num_workgroups)` in compiled shaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NctaidSource {
    /// S2R NCTAID_X/Y/Z system registers (pre-Blackwell).
    SystemRegister,
    /// LDC c\[7\]\[0/4/8\] driver constant buffer (Blackwell+, S2R NCTAID broken).
    DriverCbuf7,
}

/// GPU MMU page table format used by this generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTableFormat {
    /// Fermi/Kepler 2-level (PD + PT) with 40-bit physical address.
    /// PDE: 8 bytes, `[2:0]=target, [4]=present`. PTE: 8 bytes, `[0]=present, [2:1]=target`.
    /// 128 TB VA space = 1 PD level + 1 PT level. Pages = 4 KiB small / 128 KiB big.
    V1TwoLevel,
    /// GP100+ 5-level (PD3→PD2→PD1→PD0→PT) with V2 PDE/PTE encoding.
    /// PDEs: `addr >> 4 | aperture | VOL`. PD0 dual-entry (16 bytes: small + large).
    /// 128 TB VA space. Pages = 4 KiB small / 64 KiB big.
    V2FiveLevel,
}

/// Instance block layout for the PFIFO channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceBlockFormat {
    /// Kepler/Maxwell: RAMFC + simple PDB at 0x200. No subcontext array.
    /// `PAGE_DIR_BASE` at RAMIN offset 0x200, `ADDR_LIMIT` at 0x208.
    Simple,
    /// Volta+: RAMFC + PDB + 64-subcontext array.
    /// `SC_PDB_VALID` at 0x298, `SC0_PDB` at 0x2A0, etc.
    Subcontexted,
}

/// Runlist entry format and register layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunlistFormat {
    /// GK104/GK110: 8-byte channel entries, global RUNLIST_BASE/SUBMIT at 0x2270/0x2274.
    /// Entry: `[31:12]=INST_PTR, [9:8]=INST_TARGET, [0]=ENABLE`.
    Gk104Global,
    /// GV100+: 16-byte TSG header + 16-byte channel entries, per-runlist
    /// BASE/SUBMIT at stride 0x10 from 0x2270.
    Gv100PerRunlist,
}

/// Consolidated per-generation GPU knowledge.
///
/// Every property that varies by NVIDIA GPU generation is collected here.
/// Use [`profile_for_sm`] to look up the profile for a given SM version.
#[derive(Debug, Clone)]
pub struct GenerationProfile {
    /// Human-readable generation name.
    pub name: &'static str,
    /// SM architecture range this profile covers.
    pub sm_range: RangeInclusive<u32>,
    /// QMD layout version.
    pub qmd_version: QmdVersion,
    /// QMD size in u32 words (64 for V21-V30, 96 for V50).
    pub qmd_word_count: usize,
    /// RM channel class for GPFIFO allocation.
    pub channel_class: u32,
    /// RM compute engine class.
    pub compute_class: u32,
    /// Push buffer launch method (PCAS vs PCAS2).
    pub launch_method: LaunchMethod,
    /// Shader local memory window base address.
    pub local_mem_window: u64,
    /// GPFIFO completion tracking strategy.
    pub completion: CompletionStrategy,
    /// Sovereign VFIO boot strategy.
    pub boot_strategy: BootStrategy,
    /// GPU memory type.
    pub memory_type: MemoryType,
    /// Whether MUFU.RCP64H works in hardware (false on Blackwell).
    pub has_hardware_f64_rcp: bool,
    /// Source of `@builtin(num_workgroups)`.
    pub nctaid_source: NctaidSource,
    /// Whether USERD contains GP_GET (false on Blackwell).
    pub userd_gp_get: bool,
    /// Chip codename for firmware directory lookup.
    pub firmware_chip: &'static str,
    /// Whether this generation has full-rate FP64 (1:2 ratio with FP32).
    /// True for HPC variants (Kepler GK110/210, Pascal GP100, Volta GV100,
    /// Ampere GA100, Hopper GH100, Blackwell GB100).
    pub has_full_rate_fp64: bool,
    /// Recommended workgroup size (threads) for compute dispatch.
    pub recommended_workgroup_size: u32,
    /// Maximum concurrent CTAs (workgroups) per SM.
    pub max_cta_per_sm: u32,
    /// GPU MMU page table format.
    pub page_table_format: PageTableFormat,
    /// Instance block layout (simple vs subcontexted).
    pub instance_block_format: InstanceBlockFormat,
    /// Runlist entry format and register programming.
    pub runlist_format: RunlistFormat,
    /// Power safety profile for PMC_ENABLE sequencing.
    pub power_safety: PowerSafetyProfile,

    // ── Sovereign tier classification offsets ────────────────────────
    // These drive `classify_tier_for_profile()` so that tier classification
    // is data-driven rather than hardcoded per generation.
    /// BAR0 offset for FECS program counter (used for FECS liveness check).
    /// Kepler: 0x409624, Volta+: 0x409624 (same register, same falcon base).
    pub fecs_pc_offset: u32,
    /// BAR0 offset for GPC broadcast status (GPC power-gate detection).
    /// Kepler: 0x41A004, Volta+: 0x41A004.
    pub gpc_broadcast_offset: u32,
    /// BAR0 offset for CE0 base register (CE power-gate detection).
    /// Kepler: 0x104000, Volta+: 0x104000.
    pub ce0_base_offset: u32,
    /// BAR0 offset for PGRAPH status register.
    pub pgraph_status_offset: u32,
    /// CE DMA class for pushbuffer construction.
    /// Kepler: 0xA0B5 (KEPLER_DMA_COPY_A), Volta: 0xC3B5 (VOLTA_DMA_COPY_A).
    pub ce_class: u32,
    /// Interrupt register semantics (direct-write vs SET/CLEAR pair).
    pub interrupt_profile: InterruptProfile,

    // ── PFIFO / PTOP discovery offsets ──────────────────────────────
    // Drive `discover_ce_runlist()` / `find_pbdma_for_runlist()` instead of
    // hardcoded GV100 BAR0 addresses.
    /// BAR0 base for PTOP engine topology table (DEVICE_INFO walk).
    pub ptop_device_info_base: u32,
    /// BAR0 base for RUNLIST_PBDMA_MAP (indexed by runlist ID).
    pub runlist_pbdma_map_base: u32,
}

pub(crate) const LOCAL_MEM_WINDOW_LEGACY: u64 = 0xFF00_0000;
pub(crate) const LOCAL_MEM_WINDOW_VOLTA: u64 = 0xFF00_0000_0000_0000;

/// Standard FECS program counter offset (same across Kepler–Blackwell).
pub(crate) const FECS_PC: u32 = 0x0040_9624;
/// Standard GPC broadcast status offset.
pub(crate) const GPC_BROADCAST: u32 = 0x0041_A004;
/// Standard CE0 base offset.
pub(crate) const CE0_BASE: u32 = 0x0010_4000;
/// Standard PGRAPH status offset.
pub(crate) const PGRAPH_STATUS: u32 = 0x0040_0700;
/// PTOP engine topology table — GK104 uses the same region as GV100 V2 entries.
pub(crate) const PTOP_DEVICE_INFO: u32 = 0x0002_2700;
/// RUNLIST_PBDMA_MAP — PBDMA bitmask per runlist ID (Kepler through Blackwell).
pub(crate) const RUNLIST_PBDMA_MAP: u32 = 0x0000_2390;

pub use ada::ADA;
pub use ampere::{AMPERE_A, AMPERE_B};
pub use blackwell::{BLACKWELL_A, BLACKWELL_B};
pub use hopper::HOPPER;
pub use kepler::KEPLER;
pub use maxwell::MAXWELL;
pub use pascal::PASCAL;
pub use turing::TURING;
pub use volta::VOLTA;

/// All known generation profiles, ordered by SM range.
const ALL_PROFILES: &[&GenerationProfile] = &[
    &KEPLER,
    &MAXWELL,
    &PASCAL,
    &VOLTA,
    &TURING,
    &AMPERE_A,
    &AMPERE_B,
    &ADA,
    &HOPPER,
    &BLACKWELL_A,
    &BLACKWELL_B,
];

/// Look up the generation profile for a given SM version.
///
/// This is the ONE lookup — all code that previously branched on raw SM
/// numbers should call this instead.
///
/// Falls back to Volta for unrecognized SM versions (matching the existing
/// default behavior across the codebase).
#[must_use]
pub fn profile_for_sm(sm: u32) -> &'static GenerationProfile {
    for profile in ALL_PROFILES {
        if profile.sm_range.contains(&sm) {
            return profile;
        }
    }
    &VOLTA
}

/// Check whether a profile represents a Kepler-class GPU.
#[must_use]
pub const fn is_kepler(profile: &GenerationProfile) -> bool {
    matches!(profile.boot_strategy, BootStrategy::NoAcr)
}

/// Check whether a profile uses semaphore-based GPFIFO completion.
#[must_use]
pub const fn uses_semaphore_fence(profile: &GenerationProfile) -> bool {
    matches!(profile.completion, CompletionStrategy::SemaphoreFence)
}

impl GenerationProfile {
    /// Build vendor-agnostic `HardwareCapabilities` from this NVIDIA profile.
    #[must_use]
    pub fn to_capabilities(&self) -> crate::HardwareCapabilities {
        use crate::hardware::{CompletionStyle, Vendor, WaveSize};
        crate::HardwareCapabilities {
            vendor: Vendor::Nvidia,
            device_name: self.name,
            generation_name: self.name,
            has_hardware_f64: true,
            has_hardware_f64_rcp: self.has_hardware_f64_rcp,
            has_full_rate_fp64: self.has_full_rate_fp64,
            native_wave_size: WaveSize::Wave32,
            memory_type: self.memory_type,
            completion_style: match self.completion {
                CompletionStrategy::GpGetPoll => CompletionStyle::RegisterPoll,
                CompletionStrategy::SemaphoreFence => CompletionStyle::DeviceFence,
            },
            max_shared_mem_bytes: 49152,
        }
    }
}

#[cfg(test)]
mod tests;
