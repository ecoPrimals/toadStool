// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ephemeral seed types from the security service

use crate::types::{EntropyMixing, EntropySource};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

const DEFAULT_SEED_FRESHNESS: Duration = Duration::from_mins(5);

/// Quality score for entropy
///
/// Indicates confidence in entropy source and mixing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeedQuality {
    /// Overall quality score (0.0 - 1.0)
    pub score: f32,
    /// Machine entropy quality (0.0 - 1.0)
    pub machine_quality: f32,
    /// Human entropy quality (0.0 - 1.0)
    pub human_quality: f32,
}

impl SeedQuality {
    /// Create new quality score
    #[must_use]
    pub const fn new(score: f32, machine: f32, human: f32) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            machine_quality: machine.clamp(0.0, 1.0),
            human_quality: human.clamp(0.0, 1.0),
        }
    }

    /// Check if quality is acceptable for cryptographic use
    #[must_use]
    pub fn is_cryptographic(&self) -> bool {
        self.score >= 0.9
    }

    /// Check if quality is acceptable for ML/simulation use
    #[must_use]
    pub fn is_sufficient(&self) -> bool {
        self.score >= 0.7
    }
}

/// Ephemeral seed from the security service
///
/// High-quality, human-mixed entropy for random number generation.
/// "Ephemeral" means it's single-use and time-limited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralSeed {
    /// Seed data (cryptographic-quality bytes)
    pub seed_data: Vec<u8>,
    /// Timestamp when seed was generated
    pub timestamp: SystemTime,
    /// Entropy source type
    pub source: EntropySource,
    /// Entropy mixing configuration
    pub mixing: EntropyMixing,
    /// Quality assessment
    pub quality: SeedQuality,
}

impl EphemeralSeed {
    /// Create new ephemeral seed
    #[must_use]
    pub fn new(
        seed_data: Vec<u8>,
        source: EntropySource,
        mixing: EntropyMixing,
        quality: SeedQuality,
    ) -> Self {
        Self {
            seed_data,
            timestamp: SystemTime::now(),
            source,
            mixing,
            quality,
        }
    }

    /// Check if seed is still fresh (not expired)
    ///
    /// Seeds should be used within reasonable time (default: 5 minutes)
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        self.is_fresh_within(DEFAULT_SEED_FRESHNESS)
    }

    /// Check if seed is fresh within specified duration
    #[must_use]
    pub fn is_fresh_within(&self, duration: std::time::Duration) -> bool {
        self.timestamp
            .elapsed()
            .is_ok_and(|elapsed| elapsed < duration)
    }

    /// Get seed as u64 for simple use cases
    ///
    /// Uses first 8 bytes of seed data.
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        if self.seed_data.len() >= 8 {
            u64::from_le_bytes([
                self.seed_data[0],
                self.seed_data[1],
                self.seed_data[2],
                self.seed_data[3],
                self.seed_data[4],
                self.seed_data[5],
                self.seed_data[6],
                self.seed_data[7],
            ])
        } else {
            0
        }
    }

    /// Validate seed meets quality requirements
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.seed_data.is_empty()
            && self.is_fresh()
            && self.quality.is_sufficient()
            && self.mixing.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_quality() {
        let quality = SeedQuality::new(0.95, 0.9, 0.8);
        assert!(quality.is_cryptographic());
        assert!(quality.is_sufficient());

        let low_quality = SeedQuality::new(0.6, 0.5, 0.7);
        assert!(!low_quality.is_cryptographic());
        assert!(!low_quality.is_sufficient());
    }

    #[test]
    fn test_ephemeral_seed() {
        let seed_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let quality = SeedQuality::new(0.95, 0.9, 0.8);
        let mixing = EntropyMixing::security_standard();

        let seed = EphemeralSeed::new(seed_data, EntropySource::Mixed, mixing, quality);

        assert!(seed.is_valid());
        assert!(seed.is_fresh());
        assert_eq!(seed.source, EntropySource::Mixed);
    }

    #[test]
    fn test_seed_as_u64() {
        let seed_data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let quality = SeedQuality::new(0.95, 0.9, 0.8);
        let seed = EphemeralSeed::new(
            seed_data,
            EntropySource::Mixed,
            EntropyMixing::default(),
            quality,
        );

        let value = seed.as_u64();
        assert_ne!(value, 0);
    }
}
