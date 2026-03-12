// SPDX-License-Identifier: AGPL-3.0-only
//! Architecture register mapping — translate register offsets between GPU generations.
//!
//! When a recipe is learned on one GPU arch (e.g., Ada/AD104) and applied to another
//! (e.g., Volta/GV100), register offsets may differ. This module provides translation
//! tables built from envytools (NVIDIA), amd-gfx headers (AMD), and i915 specs (Intel).

use crate::distiller::{GpuArch, RegFunction, Vendor};
use std::collections::HashMap;

/// Register mapping between two architectures.
#[derive(Debug, Clone)]
pub struct ArchMapping {
    pub source: GpuArch,
    pub target: GpuArch,
    translations: HashMap<u64, u64>,
    confidence: f64,
}

impl ArchMapping {
    /// Create a new mapping between source and target architectures.
    pub fn new(source: GpuArch, target: GpuArch) -> Self {
        Self {
            source,
            target,
            translations: HashMap::new(),
            confidence: 0.0,
        }
    }

    /// Add a register offset translation.
    pub fn add_translation(&mut self, source_offset: u64, target_offset: u64) {
        self.translations.insert(source_offset, target_offset);
    }

    /// Translate a register offset from source to target arch.
    pub fn translate(&self, source_offset: u64) -> Option<u64> {
        self.translations.get(&source_offset).copied()
    }

    /// Whether this mapping has any translations.
    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn set_confidence(&mut self, confidence: f64) {
        self.confidence = confidence;
    }
}

/// Known register ranges that are stable within a vendor across generations.
///
/// These offsets tend to be at the same location regardless of generation,
/// which is what makes cross-generation learning feasible.
pub fn stable_registers(vendor: Vendor) -> Vec<(u64, RegFunction)> {
    match vendor {
        Vendor::Nvidia => vec![
            // PMC (Power Management Controller) — stable since Fermi
            (0x00000000, RegFunction::PowerGate), // PMC_BOOT_0
            (0x00000004, RegFunction::PowerGate), // PMC_BOOT_1
            (0x00000200, RegFunction::InterruptEnable), // PMC_INTR_EN
            // PTIMER — stable since NV04
            (0x00009400, RegFunction::ClockEnable), // PTIMER_TIME_0
            (0x00009410, RegFunction::ClockEnable), // PTIMER_TIME_1
        ],
        Vendor::Amd => vec![
            // GRBM (Graphics Register Bus Manager) — stable across GFX generations
            (0x00008010, RegFunction::EngineReset), // GRBM_STATUS
            (0x00008020, RegFunction::EngineReset), // GRBM_SOFT_RESET
            // SRBM (System Register Bus Manager)
            (0x00000E60, RegFunction::PowerGate), // SRBM_STATUS
        ],
        Vendor::Intel => vec![
            // Ring registers — stable across Gen9+
            (0x00002030, RegFunction::EngineReset), // RING_HEAD (render)
            (0x00002034, RegFunction::EngineReset), // RING_TAIL (render)
            (0x0000A090, RegFunction::PowerGate),   // FORCEWAKE
        ],
    }
}

/// Check if two architectures are in the same vendor family
/// and therefore likely to share register layout.
pub fn architectures_compatible(source: &GpuArch, target: &GpuArch) -> bool {
    source.vendor == target.vendor
}

/// Estimate how similar two architectures are for learning purposes.
///
/// Returns 0.0 (totally different) to 1.0 (same generation).
pub fn architecture_similarity(source: &GpuArch, target: &GpuArch) -> f64 {
    if source == target {
        return 1.0;
    }
    if source.vendor != target.vendor {
        return 0.1; // Cross-vendor: only universal patterns apply
    }
    if source.generation == target.generation {
        return 0.9; // Same generation, different chips
    }
    // Same vendor, different generation — moderate similarity
    0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distiller::Vendor;

    fn volta() -> GpuArch {
        GpuArch {
            vendor: Vendor::Nvidia,
            generation: "Volta".into(),
            chip: "GV100".into(),
            compute_class: "sm70".into(),
        }
    }

    fn ada() -> GpuArch {
        GpuArch {
            vendor: Vendor::Nvidia,
            generation: "Ada".into(),
            chip: "AD104".into(),
            compute_class: "sm89".into(),
        }
    }

    fn navi21() -> GpuArch {
        GpuArch {
            vendor: Vendor::Amd,
            generation: "RDNA2".into(),
            chip: "Navi21".into(),
            compute_class: "gfx1030".into(),
        }
    }

    #[test]
    fn same_arch_similarity_1() {
        assert_eq!(architecture_similarity(&volta(), &volta()), 1.0);
    }

    #[test]
    fn same_vendor_diff_gen_moderate() {
        let sim = architecture_similarity(&volta(), &ada());
        assert!((0.4..=0.6).contains(&sim));
    }

    #[test]
    fn cross_vendor_low_similarity() {
        let sim = architecture_similarity(&volta(), &navi21());
        assert!(sim < 0.2);
    }

    #[test]
    fn nvidia_stable_regs_exist() {
        let regs = stable_registers(Vendor::Nvidia);
        assert!(!regs.is_empty());
    }

    #[test]
    fn mapping_roundtrip() {
        let mut mapping = ArchMapping::new(ada(), volta());
        mapping.add_translation(0x100, 0x200);
        assert_eq!(mapping.translate(0x100), Some(0x200));
        assert_eq!(mapping.translate(0x999), None);
    }
}
