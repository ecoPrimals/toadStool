// SPDX-License-Identifier: AGPL-3.0-or-later

use super::calibration::HardwareCalibration;
use super::types::{PrecisionHint, PrecisionTier};

const DEFAULT_F64_THROTTLE_RATIO: f64 = 8.0;

/// Domain-aware precision routing brain (absorbed from hotSpring v0.6.25).
#[derive(Debug, Clone)]
pub struct PrecisionBrain {
    calibration: HardwareCalibration,
    route_table: [PrecisionTier; 4],
}

impl PrecisionBrain {
    /// Create a precision brain from hardware calibration.
    #[must_use]
    pub fn new(calibration: HardwareCalibration, f64_throttle_ratio: Option<f64>) -> Self {
        let threshold = f64_throttle_ratio.unwrap_or(DEFAULT_F64_THROTTLE_RATIO);
        let f64_throttled = Self::detect_f64_throttle(&calibration, threshold);
        let route_table = Self::build_route_table(&calibration, f64_throttled);

        Self {
            calibration,
            route_table,
        }
    }

    /// Route a precision hint to the best safe tier.
    #[must_use]
    pub const fn route(&self, hint: PrecisionHint) -> PrecisionTier {
        self.route_table[hint as usize]
    }

    /// Check if transcendentals are safe for the given hint.
    #[must_use]
    pub fn transcendentals_safe(&self, hint: PrecisionHint) -> bool {
        self.calibration.is_tier_safe(self.route(hint), true)
    }

    /// Get the hardware calibration.
    #[must_use]
    pub const fn calibration(&self) -> &HardwareCalibration {
        &self.calibration
    }

    /// Get the adapter name.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.calibration.adapter_name
    }

    fn detect_f64_throttle(cal: &HardwareCalibration, threshold: f64) -> bool {
        let f32_cap = cal.tiers.iter().find(|t| t.tier == PrecisionTier::F32);
        let f64_cap = cal.tiers.iter().find(|t| t.tier == PrecisionTier::F64);
        match (f32_cap, f64_cap) {
            (Some(f32_t), Some(f64_t)) if f32_t.dispatches && f64_t.dispatches => {
                f64_t.dispatch_latency_ratio > threshold
            }
            _ => false,
        }
    }

    fn build_route_table(cal: &HardwareCalibration, f64_throttled: bool) -> [PrecisionTier; 4] {
        [
            Self::first_safe(
                cal,
                &[
                    PrecisionTier::F64Precise,
                    PrecisionTier::F64,
                    PrecisionTier::Df64,
                    PrecisionTier::F32,
                ],
            ),
            Self::first_safe(
                cal,
                &[PrecisionTier::F64, PrecisionTier::Df64, PrecisionTier::F32],
            ),
            if f64_throttled {
                Self::first_safe(
                    cal,
                    &[PrecisionTier::Df64, PrecisionTier::F64, PrecisionTier::F32],
                )
            } else {
                Self::first_safe(
                    cal,
                    &[PrecisionTier::F64, PrecisionTier::Df64, PrecisionTier::F32],
                )
            },
            PrecisionTier::F32,
        ]
    }

    fn first_safe(cal: &HardwareCalibration, order: &[PrecisionTier]) -> PrecisionTier {
        for &tier in order {
            if cal.is_tier_safe(tier, false) {
                return tier;
            }
        }
        PrecisionTier::F32
    }
}
