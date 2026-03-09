// SPDX-License-Identifier: AGPL-3.0-only
//! Security tier definitions and required feature mapping.
//!
//! Security tiers map to required capabilities: Basic (encryption only),
//! Standard (encryption + audit), High (encryption + audit + isolation).

use crate::cloud::types::SecurityFeature;

/// Security tier levels that map to required features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityTier {
    /// Basic: encryption at rest/transit only.
    Basic,
    /// Standard: encryption + audit logging.
    Standard,
    /// High: encryption + audit + resource isolation (dedicated, network segmentation).
    High,
}

impl SecurityTier {
    /// Required security features for this tier.
    pub fn required_features(self) -> &'static [SecurityFeature] {
        match self {
            SecurityTier::Basic => &[SecurityFeature::Encryption],
            SecurityTier::Standard => &[SecurityFeature::Encryption, SecurityFeature::Compliance],
            SecurityTier::High => &[
                SecurityFeature::Encryption,
                SecurityFeature::Compliance,
                SecurityFeature::NetworkSecurity,
            ],
        }
    }
}
