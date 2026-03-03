// SPDX-License-Identifier: AGPL-3.0-or-later
//! Permission types and data structures for crypto lock system

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::validation::{SecurityProof, VerificationLevel};

/// `BearDog` Crypto Permission - cryptographic proof of access rights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProviderPermission {
    /// Permission ID
    pub permission_id: Uuid,
    /// Permission holder (who has access)
    pub holder: PermissionHolder,
    /// What external integration this unlocks
    pub external_target: ExternalTarget,
    /// Permission scope and limits
    pub scope: PermissionScope,
    /// Valid time range
    pub valid_from: SystemTime,
    pub valid_until: SystemTime,
    /// Security provider cryptographic proof
    pub crypto_proof: SecurityProof,
    /// Delegation chain (if this was delegated)
    pub delegation_chain: Option<DelegationChain>,
    /// Permission metadata
    pub metadata: PermissionMetadata,
}

/// Re-export ExternalTarget from security_provider for backward compatibility
pub use crate::security_provider::types::ExternalTarget;

/// Permission holder identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionHolder {
    /// Individual user
    Individual {
        user_id: String,
        public_key: String,
        verification_level: VerificationLevel,
    },
    /// Organization (university, company, etc.)
    Organization {
        org_id: String,
        org_type: OrganizationType,
        authorized_users: Vec<String>,
    },
    /// Delegated permission (someone lending access)
    Delegated {
        original_holder: Box<PermissionHolder>,
        delegated_to: String,
        delegation_scope: DelegationScope,
    },
}

/// Permission scope and limitations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionScope {
    /// Maximum resource limits
    pub resource_limits: ResourceLimits,
    /// Time-based restrictions
    pub time_restrictions: TimeRestrictions,
    /// Usage quotas
    pub usage_quotas: UsageQuotas,
    /// Geographic restrictions
    pub geographic_limits: Vec<String>,
    /// Feature restrictions
    pub feature_restrictions: Vec<String>,
}

/// Delegation chain for permission lending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationChain {
    /// Original permission holder
    pub original_holder: PermissionHolder,
    /// Chain of delegations
    pub delegations: Vec<Delegation>,
    /// Current delegation level
    pub delegation_level: u32,
    /// Maximum delegation depth allowed
    pub max_delegation_depth: u32,
}

/// Individual delegation in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    /// Who delegated the permission
    pub delegator: String,
    /// Who received the delegated permission
    pub delegatee: String,
    /// When the delegation was created
    pub delegated_at: SystemTime,
    /// Delegation expiry
    pub expires_at: SystemTime,
    /// Scope of delegated permission
    pub delegated_scope: DelegationScope,
    /// Cryptographic proof of delegation
    pub delegation_proof: SecurityProof,
}

/// Organization types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrganizationType {
    University,
    Research,
    NonProfit,
    Commercial,
    Government,
}

/// Delegation scope for permission lending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationScope {
    pub resource_limits: Option<ResourceLimits>,
    pub time_limits: Option<Duration>,
    pub feature_subset: Vec<String>,
    pub geographic_subset: Vec<String>,
}

/// Resource limits for permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_cores: Option<f64>,
    pub max_memory_gb: Option<f64>,
    pub max_storage_gb: Option<f64>,
    pub max_network_bandwidth: Option<f64>,
}

/// Time restrictions for permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    pub allowed_hours: Option<Vec<u8>>, // Hours 0-23
    pub allowed_days: Option<Vec<u8>>,  // Days 0-6 (Sunday-Saturday)
    pub timezone: Option<String>,
}

/// Usage quotas for permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuotas {
    pub max_requests_per_hour: Option<u64>,
    pub max_data_transfer_gb: Option<f64>,
    pub max_compute_hours: Option<f64>,
}

/// Permission metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionMetadata {
    pub issued_by: String,
    pub notes: String,
    pub features: Vec<String>,
}

/// Delegation request (for permission lending)
#[derive(Debug, Clone)]
pub struct DelegationRequest {
    pub request_id: Uuid,
    pub base_permission_id: Uuid,
    pub from_holder: PermissionHolder,
    pub to_holder: PermissionHolder,
    pub target: ExternalTarget,
    pub delegation_scope: DelegationScope,
    pub duration: Duration,
    pub requested_at: SystemTime,
    pub status: DelegationStatus,
}

/// Delegation status
#[derive(Debug, Clone)]
pub enum DelegationStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

/// Expiring permission notification
#[derive(Debug, Clone)]
pub struct ExpiringPermission {
    pub permission_id: Uuid,
    pub target: ExternalTarget,
    pub expires_in: Duration,
}

/// Permission status information
#[derive(Debug, Clone)]
pub struct PermissionStatus {
    pub permission_id: Uuid,
    pub holder: PermissionHolder,
    pub valid_until: SystemTime,
    pub scope: PermissionScope,
    pub is_delegated: bool,
}

// Cloud providers and external services - re-export from security_provider
pub use crate::security_provider::types::{
    CloudProvider, ContainerPlatform, HPCScheduler, QuantumProvider, ServiceTier,
};
