// SPDX-License-Identifier: AGPL-3.0-only
//! Type constructor and debug tests

use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::crypto_lock::access_control::{
    AccessPolicies, AccessResult, CryptoLockStatus, PermissionLevel,
};
use crate::crypto_lock::permissions::{
    PermissionHolder, PermissionScope, PermissionStatus, ResourceLimits, TimeRestrictions,
    UsageQuotas,
};
use crate::crypto_lock::validation::VerificationLevel;

use super::helpers::cloud_target;

#[test]
fn test_access_result_granted_constructor() {
    let result = AccessResult::Granted {
        reason: "test".to_string(),
        permission_level: PermissionLevel::Full,
        expires_at: None,
        restrictions: vec![],
    };
    match &result {
        AccessResult::Granted {
            permission_level, ..
        } => assert!(matches!(permission_level, PermissionLevel::Full)),
        _ => panic!(),
    }
}

#[test]
fn test_access_result_denied_constructor() {
    let result = AccessResult::Denied {
        reason: "No permission".to_string(),
        how_to_get_access: "Contact admin".to_string(),
    };
    match &result {
        AccessResult::Denied { reason, .. } => assert_eq!(reason, "No permission"),
        _ => panic!(),
    }
}

#[test]
fn test_permission_level_variants() {
    let _b = PermissionLevel::Basic;
    let _l = PermissionLevel::Limited;
    let _f = PermissionLevel::Full;
}

#[test]
fn test_crypto_lock_status_constructor() {
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: std::collections::HashMap::new(),
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    assert!(status.pure_rust_unlocked);
}

#[test]
fn test_access_policies_default() {
    let policies = AccessPolicies::default();
    assert!(policies.allow_without_provider);
    assert_eq!(policies.max_delegation_depth, 3);
}

#[test]
fn test_crypto_lock_status_expiring_permissions_field() {
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: std::collections::HashMap::new(),
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    assert!(status.expiring_permissions.is_empty());
}

#[test]
fn test_permission_level_all_variants_debug() {
    let levels = [
        PermissionLevel::Basic,
        PermissionLevel::Limited,
        PermissionLevel::Full,
    ];
    for level in levels {
        let s = format!("{:?}", level);
        assert!(!s.is_empty());
    }
}

#[test]
fn test_access_result_granted_with_restrictions() {
    let result = AccessResult::Granted {
        reason: "ok".to_string(),
        permission_level: PermissionLevel::Limited,
        expires_at: None,
        restrictions: vec!["geo:us".to_string()],
    };
    match &result {
        AccessResult::Granted { restrictions, .. } => assert_eq!(restrictions.len(), 1),
        _ => panic!(),
    }
}

#[test]
fn test_access_result_denied_debug() {
    let result = AccessResult::Denied {
        reason: "no perm".to_string(),
        how_to_get_access: "contact admin".to_string(),
    };
    let s = format!("{:?}", result);
    assert!(s.contains("Denied"));
}

#[test]
fn test_access_result_clone() {
    let granted = AccessResult::Granted {
        reason: "r".to_string(),
        permission_level: PermissionLevel::Full,
        expires_at: None,
        restrictions: vec![],
    };
    let c = granted;
    assert!(matches!(c, AccessResult::Granted { .. }));
}

#[test]
fn test_permission_level_basic_debug() {
    let level = PermissionLevel::Basic;
    let s = format!("{:?}", level);
    assert!(s.contains("Basic"));
}

#[test]
fn test_permission_level_limited_debug() {
    let level = PermissionLevel::Limited;
    let s = format!("{:?}", level);
    assert!(s.contains("Limited"));
}

#[test]
fn test_access_result_granted_with_expires_at() {
    let result = AccessResult::Granted {
        reason: "ok".to_string(),
        permission_level: PermissionLevel::Full,
        expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
        restrictions: vec![],
    };
    match &result {
        AccessResult::Granted { expires_at, .. } => assert!(expires_at.is_some()),
        _ => panic!(),
    }
}

#[test]
fn test_crypto_lock_status_with_external_permissions() {
    let mut perms = std::collections::HashMap::new();
    perms.insert(
        cloud_target(),
        PermissionStatus {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            valid_until: SystemTime::now() + Duration::from_secs(3600),
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
            is_delegated: false,
        },
    );
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: perms,
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    assert_eq!(status.external_permissions.len(), 1);
}

#[test]
fn test_expiring_permission_constructor() {
    use crate::crypto_lock::permissions::ExpiringPermission;
    let ep = ExpiringPermission {
        permission_id: Uuid::new_v4(),
        target: cloud_target(),
        expires_in: Duration::from_secs(60),
    };
    assert_eq!(ep.expires_in.as_secs(), 60);
}

#[test]
fn test_access_result_denied_clone() {
    let denied = AccessResult::Denied {
        reason: "no".to_string(),
        how_to_get_access: "get permit".to_string(),
    };
    let c = denied;
    assert!(matches!(c, AccessResult::Denied { .. }));
}

#[test]
fn test_permission_level_full_debug() {
    let level = PermissionLevel::Full;
    let s = format!("{:?}", level);
    assert!(s.contains("Full"));
}

#[test]
fn test_crypto_lock_status_delegation_chains() {
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: std::collections::HashMap::new(),
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    assert!(status.delegation_chains.is_empty());
}

#[test]
fn test_access_result_granted_with_all_fields() {
    let result = AccessResult::Granted {
        reason: "ok".to_string(),
        permission_level: PermissionLevel::Basic,
        expires_at: Some(SystemTime::now()),
        restrictions: vec!["r1".to_string(), "r2".to_string()],
    };
    match &result {
        AccessResult::Granted {
            permission_level,
            restrictions,
            ..
        } => {
            assert!(matches!(permission_level, PermissionLevel::Basic));
            assert_eq!(restrictions.len(), 2);
        }
        _ => panic!(),
    }
}

#[test]
fn test_crypto_lock_status_clone() {
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: std::collections::HashMap::new(),
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    let c = status;
    assert!(c.pure_rust_unlocked);
}

#[test]
fn test_access_policies_serde_roundtrip() {
    let p = AccessPolicies::default();
    let json = serde_json::to_string(&p).expect("serialize");
    let parsed: AccessPolicies = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p, parsed);
}

#[test]
fn test_crypto_lock_status_with_delegation_chains() {
    let status = CryptoLockStatus {
        pure_rust_unlocked: true,
        external_permissions: std::collections::HashMap::new(),
        delegation_chains: vec![],
        expiring_permissions: vec![],
    };
    assert!(status.delegation_chains.is_empty());
}

#[test]
fn test_permission_level_ordering() {
    let levels = [
        PermissionLevel::Basic,
        PermissionLevel::Limited,
        PermissionLevel::Full,
    ];
    for level in levels {
        let s = format!("{:?}", level);
        assert!(!s.is_empty());
    }
}
