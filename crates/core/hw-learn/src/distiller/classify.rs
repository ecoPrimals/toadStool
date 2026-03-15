// SPDX-License-Identifier: AGPL-3.0-only
//! Classify trace events by register function.
//!
//! Uses known register offset ranges from envytools (NVIDIA),
//! amd-gfx headers (AMD), and intel-gfx specs (Intel) to tag
//! each register write with its functional purpose.
//!
//! Supports multi-architecture classification for NVIDIA GPUs:
//! Maxwell, Pascal, Volta, Turing, and Ampere.

use crate::distiller::RegFunction;
use crate::observer::{TraceEvent, TraceEventKind};

/// GPU architecture for register classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuGen {
    /// Maxwell (GM200, GM204, GM206, `GM20b`)
    Maxwell,
    /// Pascal (GP100, GP102, GP104, GP106, GP107, `GP10b`)
    Pascal,
    /// Volta (GV100)
    Volta,
    /// Turing (TU102, TU104, TU106, TU116, TU117)
    Turing,
    /// Ampere (GA102, GA104, GA106, GA107)
    Ampere,
    /// Unknown / generic fallback
    Unknown,
}

impl GpuGen {
    /// Detect generation from chip codename.
    #[must_use]
    pub fn from_chip(chip: &str) -> Self {
        let lower = chip.to_lowercase();
        if lower.starts_with("gm") {
            return Self::Maxwell;
        }
        if lower.starts_with("gp") {
            return Self::Pascal;
        }
        if lower.starts_with("gv") {
            return Self::Volta;
        }
        if lower.starts_with("tu") {
            return Self::Turing;
        }
        if lower.starts_with("ga") {
            return Self::Ampere;
        }
        Self::Unknown
    }
}

/// A classified trace event with its inferred function.
#[derive(Debug, Clone)]
pub struct ClassifiedEvent {
    pub event: TraceEvent,
    pub function: RegFunction,
}

/// Classify a stream of trace events.
///
/// When `chip` is provided, uses architecture-specific register ranges
/// for better classification. Otherwise falls back to common NVIDIA ranges.
pub fn classify_events(events: &[TraceEvent], chip: Option<&str>) -> Vec<ClassifiedEvent> {
    let gen = chip.map_or(GpuGen::Unknown, GpuGen::from_chip);
    events
        .iter()
        .map(|e| {
            let function = classify_single(e, gen);
            ClassifiedEvent {
                event: e.clone(),
                function,
            }
        })
        .collect()
}

fn classify_single(event: &TraceEvent, gen: GpuGen) -> RegFunction {
    match &event.kind {
        TraceEventKind::RegisterWrite { offset, .. } => classify_register_for_gen(*offset, gen),
        TraceEventKind::FirmwareLoad { .. } => RegFunction::PowerGate,
        TraceEventKind::IoctlCall { ioctl_nr, .. } => classify_ioctl(*ioctl_nr),
        _ => RegFunction::Unknown,
    }
}

/// Classify a register write by offset and GPU generation.
///
/// When generation is `Unknown`, falls back to common ranges shared
/// across all NVIDIA architectures.
#[must_use]
pub fn classify_register_for_gen(offset: u64, gen: GpuGen) -> RegFunction {
    // Try generation-specific ranges first
    if let Some(func) = classify_gen_specific(offset, gen) {
        return func;
    }
    // Fall back to common NVIDIA ranges
    classify_common_nvidia(offset)
        .or_else(|| classify_amd(offset))
        .unwrap_or(RegFunction::Unknown)
}

/// Original function preserved for backward compatibility.
#[must_use]
pub fn classify_register(offset: u64) -> RegFunction {
    classify_register_for_gen(offset, GpuGen::Unknown)
}

fn classify_gen_specific(offset: u64, gen: GpuGen) -> Option<RegFunction> {
    match gen {
        GpuGen::Volta => classify_volta(offset),
        GpuGen::Turing => classify_turing(offset),
        GpuGen::Ampere => classify_ampere(offset),
        GpuGen::Maxwell | GpuGen::Pascal | GpuGen::Unknown => None,
    }
}

/// Volta (GV100) register ranges.
fn classify_volta(offset: u64) -> Option<RegFunction> {
    match offset {
        // NV_PMC (Master Control), NV_PBUS, NV_PUNITS, NV_PMU
        0x0000_0000..=0x0000_0FFF
        | 0x0000_9000..=0x0000_9FFF
        | 0x0014_0000..=0x0014_0FFF
        | 0x0006_0000..=0x0006_0FFF
        | 0x001C_0000..=0x001F_FFFF => Some(RegFunction::PowerGate),
        // NV_PTIMER
        0x0002_0000..=0x0002_0FFF => Some(RegFunction::ClockEnable),
        // NV_PPCI (PCI config), NV_PFB (Framebuffer), NV_PLTCG (L2 cache global), NV_PLTC (L2 cache per slice)
        0x0008_8000..=0x0008_8FFF
        | 0x0010_0000..=0x0010_0FFF
        | 0x0010_4000..=0x0010_4FFF
        | 0x0014_1000..=0x0014_7FFF => Some(RegFunction::MemoryConfig),
        // NV_PGRAPH (Graphics/Compute), NV_PCE (Copy Engine)
        0x0040_0000..=0x0041_FFFF | 0x0080_0000..=0x0080_0FFF => Some(RegFunction::EngineReset),
        _ => None,
    }
}

/// Turing (TU102) register ranges — same base as Volta with additions.
fn classify_turing(offset: u64) -> Option<RegFunction> {
    // Turing-specific: NV_PNVDEC (Video decoder)
    if (0x0050_0000..=0x0050_FFFF).contains(&offset) {
        return Some(RegFunction::EngineReset);
    }
    // Turing-specific: NV_PSEC2 (Security engine)
    if (0x0084_0000..=0x0084_0FFF).contains(&offset) {
        return Some(RegFunction::EngineReset);
    }
    // NV_PGRAPH_PRI_FE/GPC
    if (0x0041_8000..=0x0041_FFFF).contains(&offset) {
        return Some(RegFunction::EngineReset);
    }
    // Shared with Volta
    classify_volta(offset)
}

/// Ampere (GA102) register ranges — `NET_img` address space.
fn classify_ampere(offset: u64) -> Option<RegFunction> {
    // Ampere-specific: NV_PGRAPH_PRI_PPC
    if (0x0041_A000..=0x0041_AFFF).contains(&offset) {
        return Some(RegFunction::EngineReset);
    }
    // Ampere-specific: NV_PBUS (config space)
    if (0x0016_0000..=0x0016_3FFF).contains(&offset) {
        return Some(RegFunction::MemoryConfig);
    }
    // Ampere-specific: NV_PCFG
    if (0x00B8_0000..=0x00B8_0FFF).contains(&offset) {
        return Some(RegFunction::MemoryConfig);
    }
    // Shared with Turing
    classify_turing(offset)
}

/// Common NVIDIA ranges shared across all architectures.
fn classify_common_nvidia(offset: u64) -> Option<RegFunction> {
    match offset {
        // NVIDIA PMC / power management
        0x0002_0000..=0x0002_0FFF => Some(RegFunction::PowerGate),
        // NVIDIA PTIMER / clock
        0x0006_0000..=0x0006_0FFF => Some(RegFunction::ClockEnable),
        // NVIDIA PFB / memory
        0x0010_0000..=0x0010_0FFF => Some(RegFunction::MemoryConfig),
        // NVIDIA PGRAPH compute engine
        0x0040_0000..=0x004F_FFFF => Some(RegFunction::EngineReset),
        // NVIDIA interrupts
        0x0000_0100..=0x0000_01FF => Some(RegFunction::InterruptEnable),
        _ => None,
    }
}

/// AMD register ranges (from amdgpu headers).
fn classify_amd(offset: u64) -> Option<RegFunction> {
    match offset {
        // AMD GC (graphics compute) register range
        0x0000_2000..=0x0000_2FFF => Some(RegFunction::EngineReset),
        // AMD power/SMN
        0x0000_D000..=0x0000_DFFF => Some(RegFunction::PowerGate),
        // AMD thermal
        0x0001_6000..=0x0001_6FFF => Some(RegFunction::ThermalConfig),
        _ => None,
    }
}

fn classify_ioctl(ioctl_nr: u64) -> RegFunction {
    // nouveau new UAPI ioctl numbers (from drm/nouveau_drm.h)
    let cmd = (ioctl_nr & 0xFF) as u8;
    match cmd {
        0x00 => RegFunction::ContextAlloc, // VM_INIT
        0x01 => RegFunction::MemoryConfig, // VM_BIND
        0x02 => RegFunction::ChannelBind,  // CHANNEL_ALLOC (EXEC)
        _ => RegFunction::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_nvidia_power() {
        assert_eq!(classify_register(0x0002_0010), RegFunction::PowerGate);
    }

    #[test]
    fn classify_nvidia_clock() {
        assert_eq!(classify_register(0x0006_0000), RegFunction::ClockEnable);
    }

    #[test]
    fn classify_nvidia_pgraph() {
        assert_eq!(classify_register(0x0040_0000), RegFunction::EngineReset);
    }

    #[test]
    fn classify_unknown_offset() {
        assert_eq!(classify_register(0xFFFF_FFFF), RegFunction::Unknown);
    }

    #[test]
    fn classify_amd_gc() {
        assert_eq!(classify_register(0x0000_2100), RegFunction::EngineReset);
    }

    #[test]
    fn classify_ampere_pgraph() {
        assert_ne!(
            classify_register_for_gen(0x0041_A000, GpuGen::Ampere),
            RegFunction::Unknown
        );
    }

    #[test]
    fn classify_turing_nvdec() {
        assert_ne!(
            classify_register_for_gen(0x0050_0000, GpuGen::Turing),
            RegFunction::Unknown
        );
    }

    #[test]
    fn chip_detection() {
        assert_eq!(GpuGen::from_chip("gv100"), GpuGen::Volta);
        assert_eq!(GpuGen::from_chip("GA102"), GpuGen::Ampere);
        assert_eq!(GpuGen::from_chip("TU102"), GpuGen::Turing);
        assert_eq!(GpuGen::from_chip("GP100"), GpuGen::Pascal);
        assert_eq!(GpuGen::from_chip("GM200"), GpuGen::Maxwell);
        assert_eq!(GpuGen::from_chip("unknown"), GpuGen::Unknown);
    }

    #[test]
    fn backward_compat_classify_register() {
        // Ensure the original function still works
        assert_eq!(classify_register(0x0002_0010), RegFunction::PowerGate);
        assert_eq!(classify_register(0x0040_0000), RegFunction::EngineReset);
    }
}
