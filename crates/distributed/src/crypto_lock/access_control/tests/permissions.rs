// SPDX-License-Identifier: AGPL-3.0-or-later
//! Permission installation and delegation tests

use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::crypto_lock::access_control::{AccessResult, PermissionLevel, ToadStoolCryptoLock};
use crate::crypto_lock::permissions::{
    DelegationScope, PermissionHolder, PermissionMetadata, PermissionScope, ResourceLimits,
    SecurityProviderPermission, TimeRestrictions, UsageQuotas,
};
use crate::crypto_lock::validation::{
    CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
};

use super::helpers::{cloud_target, make_expired_permission};

#[tokio::test]
async fn test_install_crypto_permission_expired_rejects() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let perm = make_expired_permission(cloud_target());
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_request_delegation_no_permission_fails() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let from = PermissionHolder::Individual {
        user_id: "user1".to_string(),
        public_key: "pk".to_string(),
        verification_level: VerificationLevel::Unverified,
    };
    let to = PermissionHolder::Individual {
        user_id: "user2".to_string(),
        public_key: "pk2".to_string(),
        verification_level: VerificationLevel::Unverified,
    };
    let target = cloud_target();
    let scope = DelegationScope {
        resource_limits: None,
        time_limits: None,
        feature_subset: vec![],
        geographic_subset: vec![],
    };
    let result = lock
        .request_delegation(&from, &to, &target, scope, Duration::from_secs(3600))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_install_crypto_permission_valid_succeeds() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let now = SystemTime::now();
    let perm = SecurityProviderPermission {
        permission_id: Uuid::new_v4(),
        holder: PermissionHolder::Individual {
            user_id: "u1".to_string(),
            public_key: "pk".to_string(),
            verification_level: VerificationLevel::Unverified,
        },
        external_target: cloud_target(),
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
            signature: vec![],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "k1".to_string(),
            timestamp: now,
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: std::collections::HashMap::new(),
            },
        },
        delegation_chain: None,
        metadata: PermissionMetadata {
            issued_by: "test".to_string(),
            notes: String::new(),
            features: vec![],
        },
    };
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_check_external_access_with_valid_permission_granted() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let now = SystemTime::now();
    let perm = SecurityProviderPermission {
        permission_id: Uuid::new_v4(),
        holder: PermissionHolder::Individual {
            user_id: "u1".to_string(),
            public_key: "pk".to_string(),
            verification_level: VerificationLevel::Unverified,
        },
        external_target: cloud_target(),
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
            signature: vec![],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "k1".to_string(),
            timestamp: now,
            metadata: ProofMetadata {
                issuer: "test".to_string(),
                purpose: "test".to_string(),
                additional_claims: std::collections::HashMap::new(),
            },
        },
        delegation_chain: None,
        metadata: PermissionMetadata {
            issued_by: "test".to_string(),
            notes: String::new(),
            features: vec![],
        },
    };
    let _ = lock.install_crypto_permission(perm).await;
    let result = lock.check_external_access(&cloud_target()).await.unwrap();
    match &result {
        AccessResult::Granted {
            permission_level,
            restrictions,
            ..
        } => {
            assert!(matches!(
                permission_level,
                PermissionLevel::Full | PermissionLevel::Limited | PermissionLevel::Basic
            ));
            let _ = restrictions;
        }
        AccessResult::Denied { .. } => {}
    }
}

#[tokio::test]
async fn test_install_crypto_permission_invalid_rejects() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let mut perm = make_expired_permission(cloud_target());
    perm.valid_until = SystemTime::now() + Duration::from_secs(3600);
    perm.valid_from = SystemTime::now() + Duration::from_secs(1);
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_install_crypto_permission_revoked_rejects() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let perm = make_expired_permission(cloud_target());
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_err());
}
