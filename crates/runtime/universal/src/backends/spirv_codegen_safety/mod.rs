// SPDX-License-Identifier: AGPL-3.0-or-later
//! SPIR-V codegen safety — transcendental poisoning defense.
//!
//! Root cause: naga SPIR-V codegen (not NVVM) produces incorrect transcendental
//! operations on proprietary NVIDIA drivers. Renamed from `nvvm_safety` per
//! hotSpring v0.6.30 root-cause clarification.
//!
//! Absorbed from hotSpring v0.6.25 handoff: proprietary NVIDIA drivers may
//! poison the device when running shaders that use DF64/F64Precise
//! transcendentals (exp, log) through naga-generated SPIR-V. NVK/Mesa and
//! AMD radv handle all tiers correctly.
//!
//! This module provides driver-aware calibration so callers can avoid
//! transcendental compilation on risky drivers without probing (probing risks
//! poisoning).

#[allow(unused_imports)]
pub use super::nvk_zero_guard::*;

mod calibration;
mod precision_brain;
mod types;

#[cfg(feature = "hardware-learning")]
pub mod fleet;

#[cfg(test)]
mod tests;

pub use calibration::HardwareCalibration;
pub use precision_brain::PrecisionBrain;
pub use types::{NvvmPoisoningRisk, PrecisionHint, PrecisionTier, TierCapability};
