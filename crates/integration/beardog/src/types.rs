// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common types for bearDog entropy integration

use serde::{Deserialize, Serialize};

/// Entropy source type
///
/// Describes the origin of entropy for transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropySource {
    /// Pure machine entropy (hardware RNG, /dev/urandom, etc.)
    Machine,
    /// Pure human entropy (touch, accelerometer, audio, biometric)
    Human,
    /// Mixed entropy (bearDog's signature: 60% machine + 40% human)
    Mixed,
}

/// Entropy mixing configuration
///
/// Defines how machine and human entropy are combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyMixing {
    /// Machine entropy weight (0.0 - 1.0)
    pub machine_weight: f32,
    /// Human entropy weight (0.0 - 1.0)
    pub human_weight: f32,
    /// Mixing algorithm (e.g., "SHA3-512")
    pub algorithm: String,
}

impl Default for EntropyMixing {
    fn default() -> Self {
        Self {
            machine_weight: 0.6,
            human_weight: 0.4,
            algorithm: "SHA3-512".to_string(),
        }
    }
}

impl EntropyMixing {
    /// Create bearDog standard mixing (60% machine + 40% human)
    #[must_use]
    pub fn beardog_standard() -> Self {
        Self::default()
    }

    /// Validate mixing weights
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (self.machine_weight + self.human_weight - 1.0).abs() < 0.01
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mixing() {
        let mixing = EntropyMixing::default();
        assert!(mixing.is_valid());
        assert!((mixing.machine_weight - 0.6).abs() < f32::EPSILON);
        assert!((mixing.human_weight - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_beardog_standard() {
        let mixing = EntropyMixing::beardog_standard();
        assert!(mixing.is_valid());
        assert_eq!(mixing.algorithm, "SHA3-512");
    }
}
