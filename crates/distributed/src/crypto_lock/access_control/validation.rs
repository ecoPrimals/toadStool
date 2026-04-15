// SPDX-License-Identifier: AGPL-3.0-or-later
//! Delegation and resource-limit validation helpers for crypto lock access control.

use std::time::{Duration, SystemTime};

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::super::permissions::{
    DelegationChain, DelegationScope, PermissionHolder, PermissionScope, SecurityProviderPermission,
};

/// Compares two [`PermissionHolder`] values for delegation (identity of the delegator).
pub(super) fn permission_holder_matches(a: &PermissionHolder, b: &PermissionHolder) -> bool {
    match (a, b) {
        (
            PermissionHolder::Individual {
                user_id: u1,
                public_key: pk1,
                ..
            },
            PermissionHolder::Individual {
                user_id: u2,
                public_key: pk2,
                ..
            },
        ) => u1 == u2 && pk1 == pk2,
        (
            PermissionHolder::Organization { org_id: o1, .. },
            PermissionHolder::Organization { org_id: o2, .. },
        ) => o1 == o2,
        (
            PermissionHolder::Delegated {
                original_holder: oh1,
                delegated_to: d1,
                delegation_scope: s1,
            },
            PermissionHolder::Delegated {
                original_holder: oh2,
                delegated_to: d2,
                delegation_scope: s2,
            },
        ) => d1 == d2 && s1 == s2 && permission_holder_matches(oh1, oh2),
        _ => false,
    }
}

pub(super) fn validate_delegation_resource_limits(
    base_scope: &PermissionScope,
    delegation: &DelegationScope,
) -> ToadStoolResult<()> {
    let Some(dlimits) = &delegation.resource_limits else {
        return Ok(());
    };
    let b = &base_scope.resource_limits;
    fn check(opt_del: Option<f64>, opt_base: Option<f64>) -> ToadStoolResult<()> {
        match (opt_del, opt_base) {
            (Some(d), Some(bv)) if d > bv => Err(ToadStoolError::security(
                "Delegated resource limit exceeds base permission",
            )),
            _ => Ok(()),
        }
    }
    check(dlimits.max_cpu_cores, b.max_cpu_cores)?;
    check(dlimits.max_memory_gb, b.max_memory_gb)?;
    check(dlimits.max_storage_gb, b.max_storage_gb)?;
    check(dlimits.max_network_bandwidth, b.max_network_bandwidth)?;
    Ok(())
}

/// Validates structural depth constraints on a delegation chain (sync checks only).
pub(super) fn validate_delegation_chain_depth(chain: &DelegationChain) -> ToadStoolResult<()> {
    if chain.delegation_level > chain.max_delegation_depth {
        return Err(ToadStoolError::configuration("Delegation chain too deep"));
    }
    Ok(())
}

pub(super) fn validate_delegation_request(
    from_holder: &PermissionHolder,
    permission: &SecurityProviderPermission,
    scope: &DelegationScope,
    delegation_duration: Duration,
) -> ToadStoolResult<()> {
    if !permission_holder_matches(from_holder, &permission.holder) {
        return Err(ToadStoolError::security(
            "Delegator does not hold the base permission for this target",
        ));
    }

    if let Some(chain) = &permission.delegation_chain
        && chain.delegation_level >= chain.max_delegation_depth
    {
        return Err(ToadStoolError::security(
            "Maximum delegation depth reached for this permission",
        ));
    }

    let now = SystemTime::now();
    let expiry = now
        .checked_add(delegation_duration)
        .ok_or_else(|| ToadStoolError::security("Delegation duration overflow"))?;
    if expiry > permission.valid_until {
        return Err(ToadStoolError::security(
            "Delegation would outlive the base permission validity",
        ));
    }

    if let Some(limit) = scope.time_limits
        && delegation_duration > limit
    {
        return Err(ToadStoolError::security(
            "Delegation duration exceeds the delegation scope time limit",
        ));
    }

    if !scope.feature_subset.is_empty() && !permission.scope.feature_restrictions.is_empty() {
        for f in &scope.feature_subset {
            if !permission.scope.feature_restrictions.contains(f) {
                return Err(ToadStoolError::security(
                    "Delegation feature not permitted by base permission scope",
                ));
            }
        }
    }

    if !scope.geographic_subset.is_empty() && !permission.scope.geographic_limits.is_empty() {
        for g in &scope.geographic_subset {
            if !permission.scope.geographic_limits.contains(g) {
                return Err(ToadStoolError::security(
                    "Delegation geography not permitted by base permission scope",
                ));
            }
        }
    }

    validate_delegation_resource_limits(&permission.scope, scope)?;

    Ok(())
}
