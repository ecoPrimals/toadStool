// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::crypto_lock::permissions::{
    PermissionHolder, PermissionMetadata, PermissionScope, ResourceLimits,
    SecurityProviderPermission, TimeRestrictions, UsageQuotas,
};
use crate::security_provider::types::ExternalTarget;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

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
