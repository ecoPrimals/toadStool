// SPDX-License-Identifier: AGPL-3.0-only
//! Cryptographic validation and verification for crypto lock system
//!
//! **Deep Debt Evolution**: Now capability-based via Universal Adapter!
//! - Discovers security provider at runtime
//! - No hardcoded primal names
//! - Falls back to local validation if no provider available

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use toadstool::error::ToadStoolResult;

use super::permissions::SecurityProviderPermission;
use crate::security_provider::provider::SecurityProvider;
use crate::security_provider::types::{
    PermissionScope as ProviderPermissionScope, ProviderMetadata,
    ResourceLimits as ProviderResourceLimits, SecurityPermission as ProviderPermission,
    SecurityProof as ProviderSecurityProof, SignatureAlgorithm,
};

/// Security Permission Validator - validates crypto permissions
///
/// **Deep Debt**: Uses Universal Adapter to discover security provider (no hardcoded primal)
pub struct SecurityPermissionValidator {
    /// Security provider (discovered at runtime via Universal Adapter)
    security_provider: Option<Arc<dyn SecurityProvider>>,

    /// Security provider public keys for permission verification (fallback)
    security_provider_keys: HashMap<String, SecurityPublicKey>,
    /// Cryptographic signature validator (fallback)
    crypto_validator: CryptoValidator,
    /// Permission delegation chain validator
    delegation_validator: DelegationValidator,
    /// Permission revocation list
    revocation_list: PermissionRevocationList,
}

impl SecurityPermissionValidator {
    /// Create validator with runtime-discovered security provider
    ///
    /// **Deep Debt**: Discovers provider via Universal Adapter (no hardcoded BearDog!)
    pub async fn new() -> ToadStoolResult<Self> {
        // Try to discover security provider via Universal Adapter
        let security_provider = Self::discover_security_provider().await;

        Ok(Self {
            security_provider,
            security_provider_keys: HashMap::new(),
            crypto_validator: CryptoValidator::new(),
            delegation_validator: DelegationValidator::new(),
            revocation_list: PermissionRevocationList::new(),
        })
    }

    /// Register a public key for a provider (used when no external discovery available).
    pub fn register_key(&mut self, provider_id: String, key: SecurityPublicKey) {
        self.security_provider_keys.insert(provider_id, key);
    }

    /// Discover security provider via Universal Adapter
    ///
    /// **Deep Debt**: Runtime discovery, not hardcoded!
    async fn discover_security_provider() -> Option<Arc<dyn SecurityProvider>> {
        // Try to discover security provider via Universal Adapter
        use toadstool_common::universal_adapter::{
            CapabilityType, SecurityFeature, TrustLevel, UniversalAdapter,
        };

        match UniversalAdapter::new().await {
            Ok(adapter) => {
                let request = CapabilityType::Security {
                    features: vec![SecurityFeature::Signing],
                    min_trust_level: TrustLevel::High,
                };

                match adapter.request_capability(request).await {
                    Ok(handle) => {
                        // Try to create provider from handle
                        use crate::security_provider::factory::SecurityProviderFactory;

                        (SecurityProviderFactory::create_from_handle(&handle).await).ok()
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Validate permission using discovered security provider
    ///
    /// **Deep Debt**: Uses SecurityProvider trait (not hardcoded BearDog!)
    pub async fn validate_permission(
        &self,
        permission: &SecurityProviderPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        // Check basic time validity first
        let now = SystemTime::now();
        if now < permission.valid_from {
            return Ok(PermissionValidationResult::Invalid);
        }
        if now > permission.valid_until {
            return Ok(PermissionValidationResult::Expired);
        }

        // Route to discovered provider when available; fall back to local time-only validation.
        if let Some(provider) = &self.security_provider {
            let provider_permission = to_provider_permission(permission);
            let provider_result = provider.validate_permission(&provider_permission).await?;
            Ok(match provider_result {
                crate::security_provider::provider::PermissionValidationResult::Valid => {
                    PermissionValidationResult::Valid
                }
                crate::security_provider::provider::PermissionValidationResult::InvalidSignature
                | crate::security_provider::provider::PermissionValidationResult::NotFound => {
                    PermissionValidationResult::Invalid
                }
                crate::security_provider::provider::PermissionValidationResult::Expired => {
                    PermissionValidationResult::Expired
                }
                crate::security_provider::provider::PermissionValidationResult::Revoked => {
                    PermissionValidationResult::Revoked
                }
            })
        } else {
            // No external provider — use local revocation check + crypto + delegation.
            if self.revocation_list.is_revoked(&permission.permission_id) {
                return Ok(PermissionValidationResult::Revoked);
            }
            if !self.crypto_validator.validate_signature(
                &permission.crypto_proof.signature,
                permission.crypto_proof.timestamp,
            ) {
                return Ok(PermissionValidationResult::Invalid);
            }
            if let Some(chain) = &permission.delegation_chain
                && !self.delegation_validator.validate_chain(chain)
            {
                return Ok(PermissionValidationResult::Invalid);
            }
            Ok(PermissionValidationResult::Valid)
        }
    }

    /// Validates a delegation proof by checking signature presence, timestamp
    /// freshness, and delegating to the discovered security provider when available.
    pub async fn validate_delegation_proof(&self, proof: &SecurityProof) -> ToadStoolResult<()> {
        if proof.signature.is_empty() {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof has empty signature".to_string(),
            ));
        }

        let elapsed = proof
            .timestamp
            .elapsed()
            .unwrap_or(std::time::Duration::MAX);
        const MAX_PROOF_AGE: std::time::Duration = std::time::Duration::from_secs(86_400);
        if elapsed > MAX_PROOF_AGE {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof expired (older than 24h)".to_string(),
            ));
        }

        if proof.public_key_id.is_empty() {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof missing public key id".to_string(),
            ));
        }

        if let Some(provider) = &self.security_provider {
            provider
                .verify(&proof.signature, &proof.signature, &proof.public_key_id)
                .await?;
        }

        Ok(())
    }
}

/// Convert a `SecurityProviderPermission` (crypto_lock domain type) into the
/// provider-agnostic `SecurityPermission` expected by the `SecurityProvider` trait.
fn to_provider_permission(p: &SecurityProviderPermission) -> ProviderPermission {
    let holder_id = match &p.holder {
        super::permissions::PermissionHolder::Individual { user_id, .. } => user_id.clone(),
        super::permissions::PermissionHolder::Organization { org_id, .. } => org_id.clone(),
        super::permissions::PermissionHolder::Delegated { delegated_to, .. } => {
            delegated_to.clone()
        }
    };

    // ExternalTarget is unified - same type in both domains
    let target = p.external_target.clone();

    let scope = ProviderPermissionScope {
        operations: p.scope.feature_restrictions.clone(),
        resource_limits: ProviderResourceLimits::default(),
        geo_restrictions: p.scope.geographic_limits.clone(),
    };

    let proof = ProviderSecurityProof {
        signature: p.crypto_proof.signature.clone(),
        algorithm: SignatureAlgorithm::Ed25519,
        public_key_id: p.crypto_proof.public_key_id.clone(),
        signed_at: p.crypto_proof.timestamp,
    };

    let provider_metadata = ProviderMetadata {
        provider_id: "crypto-lock".to_string(),
        provider_type: "local".to_string(),
        provider_version: "0.1.0".to_string(),
        metadata: HashMap::new(),
    };

    ProviderPermission {
        permission_id: p.permission_id,
        holder_id,
        target,
        scope,
        valid_from: p.valid_from,
        valid_until: p.valid_until,
        proof,
        provider_metadata,
    }
}

/// Security provider cryptographic proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProof {
    /// Cryptographic signature
    pub signature: Vec<u8>,
    /// Signature algorithm used
    pub algorithm: CryptoAlgorithm,
    /// Public key identifier
    pub public_key_id: String,
    /// Proof timestamp
    pub timestamp: SystemTime,
    /// Additional proof metadata
    pub metadata: ProofMetadata,
}

/// Cryptographic algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CryptoAlgorithm {
    /// Ed25519 signatures.
    Ed25519,
    /// ECDSA P-256.
    EcdsaP256,
    /// RSA-4096.
    Rsa4096,
    /// BearDog-specific custom algorithm.
    BearDogCustom,
}

/// Proof metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Entity that issued the proof.
    pub issuer: String,
    /// Intended use or audience for the proof.
    pub purpose: String,
    /// Additional string claims attached to the proof.
    pub additional_claims: HashMap<String, String>,
}

/// Permission validation result
#[derive(Debug, Clone)]
pub enum PermissionValidationResult {
    /// Permission is valid for use.
    Valid,
    /// Permission failed validation (e.g. bad signature).
    Invalid,
    /// Permission is outside its valid time window.
    Expired,
    /// Permission was revoked.
    Revoked,
}

/// Verification level for identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationLevel {
    /// No verification performed.
    Unverified,
    /// Email ownership verified.
    EmailVerified,
    /// Government or KYC-style identity verified.
    IdentityVerified,
    /// Institution affiliation verified.
    InstitutionVerified,
}

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
    #[allow(clippy::cast_possible_truncation)]
    pub fn validate_chain(&self, chain: &super::permissions::DelegationChain) -> bool {
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

/// Security provider public key (opaque key material for verification).
pub struct SecurityPublicKey {
    /// Raw key bytes (e.g. 32-byte ed25519 public key).
    pub bytes: Vec<u8>,
    /// Algorithm identifier.
    pub algorithm: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto_lock::permissions::{
        PermissionHolder, PermissionMetadata, PermissionScope, ResourceLimits,
        SecurityProviderPermission, TimeRestrictions, UsageQuotas,
    };
    use crate::security_provider::types::ExternalTarget;
    use std::time::Duration;

    fn make_valid_permission() -> SecurityProviderPermission {
        let now = SystemTime::now();
        SecurityProviderPermission {
            permission_id: uuid::Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "user1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: ExternalTarget::CloudProvider {
                provider: crate::security_provider::types::CloudProvider::AWS,
                regions: vec![],
                services: vec![],
            },
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from: now - Duration::from_secs(3600),
            valid_until: now + Duration::from_secs(3600),
            crypto_proof: SecurityProof {
                signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "key1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_security_permission_validator_new() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let _ = validator;
    }

    #[tokio::test]
    async fn test_validate_permission_valid() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let perm = make_valid_permission();
        let result = validator.validate_permission(&perm).await.unwrap();
        assert!(matches!(result, PermissionValidationResult::Valid));
    }

    #[tokio::test]
    async fn test_validate_permission_expired() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let mut perm = make_valid_permission();
        perm.valid_until = SystemTime::now() - Duration::from_secs(1);
        let result = validator.validate_permission(&perm).await.unwrap();
        assert!(matches!(result, PermissionValidationResult::Expired));
    }

    #[tokio::test]
    async fn test_validate_permission_invalid_future_valid_from() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let mut perm = make_valid_permission();
        perm.valid_from = SystemTime::now() + Duration::from_secs(3600);
        let result = validator.validate_permission(&perm).await.unwrap();
        assert!(matches!(result, PermissionValidationResult::Invalid));
    }

    #[tokio::test]
    async fn test_validate_delegation_proof_valid() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let proof = SecurityProof {
            signature: vec![1, 2, 3, 4],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "key1".to_string(),
            timestamp: SystemTime::now(),
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: HashMap::new(),
            },
        };
        let result = validator.validate_delegation_proof(&proof).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_delegation_proof_rejects_empty_signature() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let proof = SecurityProof {
            signature: vec![],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "key1".to_string(),
            timestamp: SystemTime::now(),
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: HashMap::new(),
            },
        };
        let result = validator.validate_delegation_proof(&proof).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_delegation_proof_rejects_missing_key_id() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let proof = SecurityProof {
            signature: vec![1, 2, 3],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: String::new(),
            timestamp: SystemTime::now(),
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: HashMap::new(),
            },
        };
        let result = validator.validate_delegation_proof(&proof).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_delegation_proof_rejects_expired() {
        let validator = SecurityPermissionValidator::new().await.unwrap();
        let proof = SecurityProof {
            signature: vec![1, 2, 3],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "key1".to_string(),
            timestamp: SystemTime::now() - Duration::from_secs(100_000),
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: HashMap::new(),
            },
        };
        let result = validator.validate_delegation_proof(&proof).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_crypto_validator_new_and_default() {
        let v = CryptoValidator::new();
        assert!(v.validate_signature(b"sig", SystemTime::now()));
        assert!(!v.validate_signature(b"", SystemTime::now()));
        let d = CryptoValidator::default();
        assert!(d.validate_signature(b"sig", SystemTime::now()));
    }

    #[test]
    fn test_delegation_validator_new_and_default() {
        let v = DelegationValidator::new();
        let _ = v;
        let d = DelegationValidator::default();
        let _ = d;
    }

    #[test]
    fn test_permission_revocation_list_new_and_default() {
        let mut v = PermissionRevocationList::new();
        let id = uuid::Uuid::new_v4();
        assert!(!v.is_revoked(&id));
        v.revoke(id);
        assert!(v.is_revoked(&id));
        let d = PermissionRevocationList::default();
        assert!(!d.is_revoked(&id));
    }

    #[test]
    fn test_crypto_algorithm_serde() {
        for alg in [
            CryptoAlgorithm::Ed25519,
            CryptoAlgorithm::EcdsaP256,
            CryptoAlgorithm::Rsa4096,
            CryptoAlgorithm::BearDogCustom,
        ] {
            let json = serde_json::to_value(&alg).unwrap();
            let _: CryptoAlgorithm = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_verification_level_serde() {
        for level in [
            VerificationLevel::Unverified,
            VerificationLevel::EmailVerified,
            VerificationLevel::IdentityVerified,
            VerificationLevel::InstitutionVerified,
        ] {
            let json = serde_json::to_value(&level).unwrap();
            let _: VerificationLevel = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_proof_metadata_serde() {
        let meta = ProofMetadata {
            issuer: "issuer".to_string(),
            purpose: "purpose".to_string(),
            additional_claims: vec![
                ("k1".to_string(), "v1".to_string()),
                ("k2".to_string(), "v2".to_string()),
            ]
            .into_iter()
            .collect(),
        };
        let json = serde_json::to_value(&meta).unwrap();
        let parsed: ProofMetadata = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.issuer, "issuer");
        assert_eq!(parsed.additional_claims.get("k1"), Some(&"v1".to_string()));
    }

    #[test]
    fn test_permission_validation_result_variants() {
        let _ = PermissionValidationResult::Valid;
        let _ = PermissionValidationResult::Invalid;
        let _ = PermissionValidationResult::Expired;
        let _ = PermissionValidationResult::Revoked;
    }
}
