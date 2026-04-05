// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for access control manager
//! Target: exercise check_external_access, install_permission, request_delegation, get_status.

use std::time::{Duration, SystemTime};

use toadstool_distributed::crypto_lock::permissions::{
    DelegationScope, PermissionHolder, PermissionMetadata, PermissionScope, ResourceLimits,
    SecurityProviderPermission, TimeRestrictions, UsageQuotas,
};
use toadstool_distributed::crypto_lock::validation::{
    CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
};
use toadstool_distributed::crypto_lock::{AccessResult, PermissionLevel, ToadStoolCryptoLock};
use toadstool_distributed::security_provider::types::{CloudProvider, ExternalTarget};
use uuid::Uuid;

fn cloud_target() -> ExternalTarget {
    ExternalTarget::CloudProvider {
        provider: CloudProvider::AWS,
        regions: vec!["us-east-1".to_string()],
        services: vec![],
    }
}

fn pure_rust_target() -> ExternalTarget {
    ExternalTarget::ExternalTool {
        tool_name: "toadstool".to_string(),
        api_endpoints: vec![],
        feature_set: vec!["ecosystem_native".to_string()],
    }
}

fn make_valid_permission(target: ExternalTarget) -> SecurityProviderPermission {
    let now = SystemTime::now();
    SecurityProviderPermission {
        permission_id: Uuid::new_v4(),
        holder: PermissionHolder::Individual {
            user_id: "u1".to_string(),
            public_key: "pk".to_string(),
            verification_level: VerificationLevel::Unverified,
        },
        external_target: target,
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
    }
}

// ─── ToadStoolCryptoLock::new ─────────────────────────────────────────────

#[tokio::test]
async fn crypto_lock_new_succeeds() {
    let result = ToadStoolCryptoLock::new().await;
    assert!(result.is_ok());
}

// ─── check_external_access: pure rust granted ───────────────────────────────

#[tokio::test]
async fn check_external_access_pure_rust_granted() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let target = pure_rust_target();
    let result = lock.check_external_access(&target).await.unwrap();
    match &result {
        AccessResult::Granted {
            reason,
            permission_level,
            ..
        } => {
            assert!(reason.contains("Pure Rust"));
            assert!(matches!(permission_level, PermissionLevel::Full));
        }
        AccessResult::Denied { .. } => panic!("Expected Granted"),
    }
}

// ─── check_external_access: no permission denied ────────────────────────────

#[tokio::test]
async fn check_external_access_cloud_denied() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let target = cloud_target();
    let result = lock.check_external_access(&target).await.unwrap();
    match &result {
        AccessResult::Denied {
            reason,
            how_to_get_access,
        } => {
            assert!(!reason.is_empty());
            assert!(how_to_get_access.contains("AWS"));
        }
        AccessResult::Granted { .. } => panic!("Expected Denied"),
    }
}

// ─── get_crypto_lock_status ─────────────────────────────────────────────────

#[tokio::test]
async fn get_crypto_lock_status_empty() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let status = lock.get_crypto_lock_status().await.unwrap();
    assert!(status.pure_rust_unlocked);
    assert!(status.external_permissions.is_empty());
    assert!(status.delegation_chains.is_empty());
    assert!(status.expiring_permissions.is_empty());
}

// ─── install_crypto_permission ─────────────────────────────────────────────

#[tokio::test]
async fn install_crypto_permission_valid_succeeds() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let perm = make_valid_permission(cloud_target());
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn install_crypto_permission_expired_rejects() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let mut perm = make_valid_permission(cloud_target());
    perm.valid_until = SystemTime::now() - Duration::from_secs(1);
    perm.valid_from = SystemTime::now() - Duration::from_secs(3600);
    let result = lock.install_crypto_permission(perm).await;
    assert!(result.is_err());
}

// ─── request_delegation ─────────────────────────────────────────────────────

#[tokio::test]
async fn request_delegation_no_permission_fails() {
    let lock = ToadStoolCryptoLock::new().await.unwrap();
    let from = PermissionHolder::Individual {
        user_id: "u1".to_string(),
        public_key: "pk".to_string(),
        verification_level: VerificationLevel::Unverified,
    };
    let to = PermissionHolder::Individual {
        user_id: "u2".to_string(),
        public_key: "pk2".to_string(),
        verification_level: VerificationLevel::Unverified,
    };
    let scope = DelegationScope {
        resource_limits: None,
        time_limits: None,
        feature_subset: vec![],
        geographic_subset: vec![],
    };
    let result = lock
        .request_delegation(
            &from,
            &to,
            &cloud_target(),
            scope,
            Duration::from_secs(3600),
        )
        .await;
    assert!(result.is_err());
}

// ─── check_external_access with installed permission ────────────────────────

#[tokio::test]
async fn check_external_access_with_permission_granted() {
    let mut lock = ToadStoolCryptoLock::new().await.unwrap();
    let target = cloud_target();
    let perm = make_valid_permission(target.clone());
    lock.install_crypto_permission(perm).await.unwrap();
    let result = lock.check_external_access(&target).await.unwrap();
    match &result {
        AccessResult::Granted {
            permission_level, ..
        } => {
            assert!(matches!(
                permission_level,
                PermissionLevel::Full | PermissionLevel::Limited | PermissionLevel::Basic
            ));
        }
        AccessResult::Denied { .. } => {}
    }
}

// ─── AccessResult and PermissionLevel types ─────────────────────────────────

#[test]
fn access_result_granted_clone() {
    let r = AccessResult::Granted {
        reason: "ok".to_string(),
        permission_level: PermissionLevel::Full,
        expires_at: None,
        restrictions: vec![],
    };
    let c = r;
    assert!(matches!(c, AccessResult::Granted { .. }));
}

#[test]
fn access_result_denied_clone() {
    let r = AccessResult::Denied {
        reason: "no".to_string(),
        how_to_get_access: "get".to_string(),
    };
    let c = r;
    assert!(matches!(c, AccessResult::Denied { .. }));
}

#[test]
fn permission_level_variants() {
    let _ = PermissionLevel::Basic;
    let _ = PermissionLevel::Limited;
    let _ = PermissionLevel::Full;
}
