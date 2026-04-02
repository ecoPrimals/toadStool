// SPDX-License-Identifier: AGPL-3.0-only
//! Access control result types

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::super::permissions::{
    DelegationChain, ExpiringPermission, ExternalTarget, PermissionStatus,
};

/// Access result (granted or denied)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessResult {
    /// Access was granted with optional expiry and restrictions.
    Granted {
        /// Human-readable grant reason.
        reason: String,
        /// Coarse permission tier.
        permission_level: PermissionLevel,
        /// When the grant expires, if applicable.
        expires_at: Option<SystemTime>,
        /// Additional restriction strings (e.g. IP, scope).
        restrictions: Vec<String>,
    },
    /// Access was denied.
    Denied {
        /// Denial reason.
        reason: String,
        /// Hint for obtaining access (e.g. request workflow).
        how_to_get_access: String,
    },
}

/// Permission level for access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Minimal access.
    Basic,
    /// Partial access with constraints.
    Limited,
    /// Full access within scope.
    Full,
}

/// Crypto lock status report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoLockStatus {
    /// Whether pure-Rust features are unlocked without external permission.
    pub pure_rust_unlocked: bool,
    /// Current permission status per external target.
    pub external_permissions: HashMap<ExternalTarget, PermissionStatus>,
    /// Active delegation chains.
    pub delegation_chains: Vec<DelegationChain>,
    /// Permissions nearing expiry.
    pub expiring_permissions: Vec<ExpiringPermission>,
}

/// Access control policies governing who can access what.
///
/// Specifies which capabilities require elevated permissions and which
/// are freely available. Default: all pure-Rust capabilities are open.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccessPolicies {
    /// Capabilities that require explicit permission grants.
    #[serde(default)]
    pub restricted_capabilities: Vec<String>,
    /// Maximum delegation depth for permission forwarding.
    #[serde(default = "default_max_delegation_depth")]
    pub max_delegation_depth: usize,
    /// Whether to allow access when no security provider is discovered.
    #[serde(default = "default_allow_without_provider")]
    pub allow_without_provider: bool,
}

impl Default for AccessPolicies {
    fn default() -> Self {
        Self {
            restricted_capabilities: Vec::new(),
            max_delegation_depth: 3,
            allow_without_provider: true,
        }
    }
}

fn default_max_delegation_depth() -> usize {
    3
}

fn default_allow_without_provider() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{AccessPolicies, AccessResult, CryptoLockStatus, PermissionLevel};
    use crate::crypto_lock::permissions::{
        DelegationChain, ExpiringPermission, PermissionHolder, PermissionScope, PermissionStatus,
        ResourceLimits, TimeRestrictions, UsageQuotas,
    };
    use crate::crypto_lock::validation::VerificationLevel;
    use crate::security_provider::types::{CloudProvider, ExternalTarget};
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};
    use uuid::Uuid;

    fn sample_target() -> ExternalTarget {
        ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec!["us-east-1".to_string()],
            services: vec![],
        }
    }

    fn sample_permission_status() -> PermissionStatus {
        PermissionStatus {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            valid_until: SystemTime::UNIX_EPOCH + Duration::from_secs(3_600),
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
        }
    }

    #[test]
    fn access_policies_default_clone_debug() {
        let a = AccessPolicies::default();
        let b = a.clone();
        assert_eq!(a, b);
        assert!(a.allow_without_provider);
        assert_eq!(a.max_delegation_depth, 3);
        assert!(a.restricted_capabilities.is_empty());
        let dbg = format!("{a:?}");
        assert!(dbg.contains("AccessPolicies"));
    }

    #[test]
    fn permission_level_debug_clone_eq() {
        for level in [
            PermissionLevel::Basic,
            PermissionLevel::Limited,
            PermissionLevel::Full,
        ] {
            let c = level.clone();
            assert_eq!(level, c);
            let dbg = format!("{level:?}");
            assert!(!dbg.is_empty());
        }
    }

    #[test]
    fn access_result_granted_denied_clone_debug_partial_eq() {
        let granted = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            restrictions: vec!["a".to_string()],
        };
        assert_eq!(granted.clone(), granted);
        let denied = AccessResult::Denied {
            reason: "no".to_string(),
            how_to_get_access: "ask".to_string(),
        };
        assert_eq!(denied.clone(), denied);
        assert_ne!(granted, denied);
    }

    #[test]
    fn crypto_lock_status_clone_debug_serde_json_roundtrip_empty_external_map() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: HashMap::new(),
            delegation_chains: vec![DelegationChain {
                original_holder: PermissionHolder::Individual {
                    user_id: "o".to_string(),
                    public_key: "pk2".to_string(),
                    verification_level: VerificationLevel::Unverified,
                },
                delegations: vec![],
                delegation_level: 0,
                max_delegation_depth: 2,
            }],
            expiring_permissions: vec![ExpiringPermission {
                permission_id: Uuid::nil(),
                target: sample_target(),
                expires_in: Duration::from_secs(120),
            }],
        };
        let json = serde_json::to_string(&status).expect("serialize CryptoLockStatus");
        let back: CryptoLockStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.pure_rust_unlocked, status.pure_rust_unlocked);
        assert_eq!(
            back.external_permissions.len(),
            status.external_permissions.len()
        );
        assert_eq!(back.delegation_chains.len(), status.delegation_chains.len());
        assert_eq!(
            back.expiring_permissions.len(),
            status.expiring_permissions.len()
        );
        let dbg = format!("{status:?}");
        assert!(dbg.contains("CryptoLockStatus"));
    }

    #[test]
    fn crypto_lock_status_with_external_permissions_clone_and_debug() {
        let mut ext = HashMap::new();
        ext.insert(sample_target(), sample_permission_status());
        let status = CryptoLockStatus {
            pure_rust_unlocked: false,
            external_permissions: ext,
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        let c = status.clone();
        assert_eq!(c.pure_rust_unlocked, status.pure_rust_unlocked);
        assert_eq!(c.external_permissions.len(), 1);
        assert!(!format!("{status:?}").is_empty());
    }

    #[test]
    fn serde_json_roundtrip_permission_level() {
        for level in [
            PermissionLevel::Basic,
            PermissionLevel::Limited,
            PermissionLevel::Full,
        ] {
            let json = serde_json::to_string(&level).expect("serialize PermissionLevel");
            let parsed: PermissionLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, level);
        }
    }

    #[test]
    fn serde_json_roundtrip_access_policies() {
        let p = AccessPolicies::default();
        let json = serde_json::to_string(&p).expect("serialize AccessPolicies");
        let parsed: AccessPolicies = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, p);
    }

    #[test]
    fn serde_json_roundtrip_access_result_variants() {
        let cases = vec![
            AccessResult::Granted {
                reason: "r".to_string(),
                permission_level: PermissionLevel::Limited,
                expires_at: None,
                restrictions: vec![],
            },
            AccessResult::Granted {
                reason: "r2".to_string(),
                permission_level: PermissionLevel::Basic,
                expires_at: Some(SystemTime::UNIX_EPOCH),
                restrictions: vec!["x".to_string()],
            },
            AccessResult::Denied {
                reason: "d".to_string(),
                how_to_get_access: "h".to_string(),
            },
        ];
        for original in cases {
            let json = serde_json::to_string(&original).expect("serialize AccessResult");
            let parsed: AccessResult = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, original);
        }
    }

    proptest! {
        #[test]
        fn permission_level_serde_roundtrip_prop(
            idx in 0usize..3usize
        ) {
            let level = match idx {
                0 => PermissionLevel::Basic,
                1 => PermissionLevel::Limited,
                _ => PermissionLevel::Full,
            };
            let json = serde_json::to_string(&level).expect("serialize");
            let parsed: PermissionLevel = serde_json::from_str(&json).expect("deserialize");
            prop_assert_eq!(parsed, level);
        }
    }
}
