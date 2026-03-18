// SPDX-License-Identifier: AGPL-3.0-or-later
//! Access control result types

use std::collections::HashMap;
use std::time::SystemTime;

use super::super::permissions::{
    DelegationChain, ExpiringPermission, ExternalTarget, PermissionStatus,
};

/// Access result (granted or denied)
#[derive(Debug, Clone)]
pub enum AccessResult {
    Granted {
        reason: String,
        permission_level: PermissionLevel,
        expires_at: Option<SystemTime>,
        restrictions: Vec<String>,
    },
    Denied {
        reason: String,
        how_to_get_access: String,
    },
}

/// Permission level for access
#[derive(Debug, Clone)]
pub enum PermissionLevel {
    Basic,
    Limited,
    Full,
}

/// Crypto lock status report
#[derive(Debug, Clone)]
pub struct CryptoLockStatus {
    pub pure_rust_unlocked: bool,
    pub external_permissions: HashMap<ExternalTarget, PermissionStatus>,
    pub delegation_chains: Vec<DelegationChain>,
    pub expiring_permissions: Vec<ExpiringPermission>,
}

/// Access control policies
pub struct AccessPolicies;

impl Default for AccessPolicies {
    fn default() -> Self {
        Self
    }
}
