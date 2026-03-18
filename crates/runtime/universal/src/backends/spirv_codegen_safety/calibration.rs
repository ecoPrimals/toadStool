// SPDX-License-Identifier: AGPL-3.0-or-later

use super::types::{NvvmPoisoningRisk, PrecisionTier, TierCapability};
use crate::backends::wgpu_backend::GpuAdapterInfo;

/// Hardware calibration for NVVM safety.
#[derive(Debug, Clone)]
pub struct HardwareCalibration {
    /// Adapter name (e.g. "NVIDIA GeForce RTX 3090").
    pub adapter_name: String,
    /// Driver name (e.g. "nvk", "radv", "nvidia").
    pub driver: String,
    /// Per-tier capability matrix.
    pub tiers: Vec<TierCapability>,
    /// Whether any f64 precision is supported.
    pub has_any_f64: bool,
    /// Whether DF64 arithmetic is safe on this adapter.
    pub df64_arithmetic_safe: bool,
    /// Whether NVVM transcendentals risk poisoning.
    pub nvvm_transcendental_risk: bool,
    /// NVVM poisoning risk classification.
    pub poisoning_risk: NvvmPoisoningRisk,
}

impl HardwareCalibration {
    /// Build calibration from wgpu adapter info.
    #[must_use]
    pub fn from_adapter_info(info: &GpuAdapterInfo) -> Self {
        let adapter_name = info.name.clone();
        let driver = info.driver.clone();

        let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
        let is_radv = driver.contains("radv");
        let is_nvidia_proprietary = driver.contains("nvidia") && !driver.contains("nvk");
        let is_known = is_nvk || is_radv || is_nvidia_proprietary;

        let supports_f64 = info.supports_shader_f64 && !info.f64_compute_unreliable;

        let poisoning_risk = if is_nvk || is_radv {
            NvvmPoisoningRisk::None
        } else if is_nvidia_proprietary {
            NvvmPoisoningRisk::TranscendentalOnly
        } else {
            NvvmPoisoningRisk::Unknown
        };

        let mut tiers = vec![TierCapability {
            tier: PrecisionTier::F32,
            compiles: true,
            dispatches: true,
            transcendentals_safe: true,
            dispatch_latency_ratio: 1.0,
        }];

        if supports_f64 {
            tiers.push(TierCapability {
                tier: PrecisionTier::F64,
                compiles: true,
                dispatches: true,
                transcendentals_safe: is_known && (is_nvk || is_radv || is_nvidia_proprietary),
                dispatch_latency_ratio: 1.0,
            });

            let f64_precise_transcendentals_safe = is_nvk || is_radv;
            tiers.push(TierCapability {
                tier: PrecisionTier::F64Precise,
                compiles: true,
                dispatches: true,
                transcendentals_safe: f64_precise_transcendentals_safe,
                dispatch_latency_ratio: 1.0,
            });

            let df64_transcendentals_safe = is_nvk || is_radv;
            let df64_arithmetic_safe = is_known;
            tiers.push(TierCapability {
                tier: PrecisionTier::Df64,
                compiles: true,
                dispatches: true,
                transcendentals_safe: df64_transcendentals_safe,
                dispatch_latency_ratio: 1.0,
            });

            Self {
                adapter_name,
                driver,
                tiers,
                has_any_f64: true,
                df64_arithmetic_safe,
                nvvm_transcendental_risk: !df64_transcendentals_safe && supports_f64,
                poisoning_risk,
            }
        } else {
            Self {
                adapter_name,
                driver,
                tiers,
                has_any_f64: false,
                df64_arithmetic_safe: false,
                nvvm_transcendental_risk: false,
                poisoning_risk,
            }
        }
    }

    /// Check if a precision tier is safe for this hardware.
    #[must_use]
    pub fn is_tier_safe(&self, tier: PrecisionTier, uses_transcendentals: bool) -> bool {
        let Some(tc) = self.tiers.iter().find(|t| t.tier == tier) else {
            return false;
        };
        if !tc.compiles || !tc.dispatches {
            return false;
        }
        if uses_transcendentals && !tc.transcendentals_safe {
            return false;
        }
        true
    }

    /// Select the best safe precision tier for the given constraints.
    #[must_use]
    pub fn best_tier(&self, precision_critical: bool, uses_transcendentals: bool) -> PrecisionTier {
        let order = if precision_critical {
            [
                PrecisionTier::F64Precise,
                PrecisionTier::F64,
                PrecisionTier::Df64,
                PrecisionTier::F32,
            ]
        } else {
            [
                PrecisionTier::F64,
                PrecisionTier::Df64,
                PrecisionTier::F32,
                PrecisionTier::F64Precise,
            ]
        };

        for t in order {
            if self.is_tier_safe(t, uses_transcendentals) {
                return t;
            }
        }

        PrecisionTier::F32
    }
}
