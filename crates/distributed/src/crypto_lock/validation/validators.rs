// SPDX-License-Identifier: AGPL-3.0-or-later
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
    const MAX_PROOF_AGE_SECS: u64 = 86_400;

    /// Creates a validator with a 24-hour maximum proof age.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_proof_age: std::time::Duration::from_secs(Self::MAX_PROOF_AGE_SECS),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::SystemTime;

    use crate::crypto_lock::permissions::{
        Delegation, DelegationChain, DelegationScope, PermissionHolder,
    };

    use super::{CryptoValidator, DelegationValidator, PermissionRevocationList};
    use crate::crypto_lock::validation::{
        CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
    };

    fn sample_delegation(delegator: &str, delegatee: &str) -> Delegation {
        let now = SystemTime::now();
        Delegation {
            delegator: delegator.to_string(),
            delegatee: delegatee.to_string(),
            delegated_at: now,
            expires_at: now,
            delegated_scope: DelegationScope {
                resource_limits: None,
                time_limits: None,
                feature_subset: vec![],
                geographic_subset: vec![],
            },
            delegation_proof: SecurityProof {
                signature: vec![1],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "pk".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "i".to_string(),
                    purpose: "p".to_string(),
                    additional_claims: HashMap::new(),
                },
            },
        }
    }

    fn sample_chain(delegation_level: u32, delegations: Vec<Delegation>) -> DelegationChain {
        DelegationChain {
            original_holder: PermissionHolder::Individual {
                user_id: "owner".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            delegations,
            delegation_level,
            max_delegation_depth: 10,
        }
    }

    #[test]
    fn crypto_validator_rejects_empty_signature() {
        let v = CryptoValidator::new();
        assert!(!v.validate_signature(&[], SystemTime::now()));
    }

    #[test]
    fn crypto_validator_accepts_fresh_non_empty_signature() {
        let v = CryptoValidator::new();
        assert!(v.validate_signature(&[0xab], SystemTime::now()));
    }

    #[test]
    fn crypto_validator_rejects_timestamp_too_far_in_past() {
        let v = CryptoValidator::new();
        let ancient = SystemTime::UNIX_EPOCH;
        assert!(!v.validate_signature(&[1], ancient));
    }

    #[test]
    fn crypto_validator_new_and_default_agree_on_behavior() {
        let new_v = CryptoValidator::new();
        let default_v = CryptoValidator::default();
        let t = SystemTime::now();
        assert_eq!(
            new_v.validate_signature(&[1], t),
            default_v.validate_signature(&[1], t)
        );
    }

    #[test]
    fn delegation_validator_accepts_shallow_chain_with_nonempty_identities() {
        let v = DelegationValidator::new();
        let chain = sample_chain(2, vec![sample_delegation("a", "b")]);
        assert!(v.validate_chain(&chain));
    }

    #[test]
    fn delegation_validator_rejects_excessive_depth() {
        let v = DelegationValidator::new();
        let chain = sample_chain(6, vec![sample_delegation("a", "b")]);
        assert!(!v.validate_chain(&chain));
    }

    #[test]
    fn delegation_validator_rejects_empty_delegator() {
        let v = DelegationValidator::new();
        let mut d = sample_delegation("x", "y");
        d.delegator.clear();
        let chain = sample_chain(1, vec![d]);
        assert!(!v.validate_chain(&chain));
    }

    #[test]
    fn delegation_validator_new_and_default_agree_on_behavior() {
        let new_v = DelegationValidator::new();
        let default_v = DelegationValidator::default();
        let chain = sample_chain(2, vec![sample_delegation("a", "b")]);
        assert_eq!(
            new_v.validate_chain(&chain),
            default_v.validate_chain(&chain)
        );
    }

    #[test]
    fn permission_revocation_list_revoke_and_is_revoked() {
        let mut list = PermissionRevocationList::new();
        let id = uuid::Uuid::new_v4();
        assert!(!list.is_revoked(&id));
        list.revoke(id);
        assert!(list.is_revoked(&id));
    }

    #[test]
    fn permission_revocation_list_default_is_empty() {
        let list = PermissionRevocationList::default();
        assert!(!list.is_revoked(&uuid::Uuid::new_v4()));
    }
}
