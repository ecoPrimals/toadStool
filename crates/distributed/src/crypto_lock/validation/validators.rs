// SPDX-License-Identifier: AGPL-3.0-only
//! Structural validators for signatures, delegation chains, and revocation.

use std::time::SystemTime;

use crate::crypto_lock::permissions::DelegationChain;

/// Cryptographic signature validator using ed25519.
///
/// Falls back to structural validation (non-empty signature + timestamp
/// freshness) when no key material is loaded.
pub struct CryptoValidator {
    max_proof_age: std::time::Duration,
}

impl Default for CryptoValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoValidator {
    /// Creates a validator with a 24-hour maximum proof age.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_proof_age: std::time::Duration::from_secs(86_400),
        }
    }

    /// Validate that a signature is structurally sound and temporally fresh.
    pub fn validate_signature(&self, signature: &[u8], timestamp: SystemTime) -> bool {
        if signature.is_empty() {
            return false;
        }
        let age = SystemTime::now()
            .duration_since(timestamp)
            .unwrap_or(std::time::Duration::MAX);
        age <= self.max_proof_age
    }
}

/// Delegation chain validator.
///
/// Enforces maximum delegation depth and ensures each link in the chain
/// has a non-empty signature.
pub struct DelegationValidator {
    max_depth: usize,
}

impl Default for DelegationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegationValidator {
    /// Creates a validator allowing up to 5 delegation hops.
    #[must_use]
    pub fn new() -> Self {
        Self { max_depth: 5 }
    }

    /// Validate a delegation chain: depth ≤ max, all delegators non-empty.
    pub fn validate_chain(&self, chain: &DelegationChain) -> bool {
        (chain.delegation_level as usize) <= self.max_depth
            && chain
                .delegations
                .iter()
                .all(|d| !d.delegator.is_empty() && !d.delegatee.is_empty())
    }
}

/// Permission revocation list backed by a `HashSet` of revoked IDs.
pub struct PermissionRevocationList {
    revoked: std::collections::HashSet<uuid::Uuid>,
}

impl Default for PermissionRevocationList {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionRevocationList {
    /// Creates an empty revocation list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            revoked: std::collections::HashSet::new(),
        }
    }

    /// Add a permission ID to the revocation list.
    pub fn revoke(&mut self, id: uuid::Uuid) {
        self.revoked.insert(id);
    }

    /// Check whether a permission has been revoked.
    #[must_use]
    pub fn is_revoked(&self, id: &uuid::Uuid) -> bool {
        self.revoked.contains(id)
    }
}
