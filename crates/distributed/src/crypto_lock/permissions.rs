// SPDX-License-Identifier: AGPL-3.0-or-later
//! Permission types and data structures for crypto lock system

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::validation::{SecurityProof, VerificationLevel};

/// `Security` Crypto Permission - cryptographic proof of access rights
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
    /// Valid time range (inclusive start)
    pub valid_from: SystemTime,
    /// Valid time range (inclusive end)
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

/// Permission holder identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionHolder {
    /// Individual user.
    Individual {
        /// User identifier.
        user_id: String,
        /// Public key for verification.
        public_key: String,
        /// Verification level.
        verification_level: VerificationLevel,
    },
    /// Organization (university, company, etc.).
    Organization {
        /// Organization identifier.
        org_id: String,
        /// Organization type.
        org_type: OrganizationType,
        /// Authorized user IDs.
        authorized_users: Vec<String>,
    },
    /// Delegated permission (someone lending access).
    Delegated {
        /// Original permission holder.
        original_holder: Box<Self>,
        /// User ID delegated to.
        delegated_to: String,
        /// Scope of delegation.
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

/// Organization types for permission holders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrganizationType {
    /// University.
    University,
    /// Research institution.
    Research,
    /// Non-profit.
    NonProfit,
    /// Commercial entity.
    Commercial,
    /// Government.
    Government,
}

/// Delegation scope for permission lending.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DelegationScope {
    /// Resource limits for delegation.
    pub resource_limits: Option<ResourceLimits>,
    /// Time limit for delegation.
    pub time_limits: Option<Duration>,
    /// Subset of features allowed.
    pub feature_subset: Vec<String>,
    /// Geographic subset allowed.
    pub geographic_subset: Vec<String>,
}

/// Resource limits for crypto lock permissions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// Max CPU cores.
    pub max_cpu_cores: Option<f64>,
    /// Max memory in GB.
    pub max_memory_gb: Option<f64>,
    /// Max storage in GB.
    pub max_storage_gb: Option<f64>,
    /// Max network bandwidth.
    pub max_network_bandwidth: Option<f64>,
}

/// Time restrictions for permission validity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    /// Allowed hours (0–23).
    pub allowed_hours: Option<Vec<u8>>,
    /// Allowed days (0–6, Sunday–Saturday).
    pub allowed_days: Option<Vec<u8>>,
    /// Timezone for time checks.
    pub timezone: Option<String>,
}

/// Usage quotas for permission limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageQuotas {
    /// Max requests per hour.
    pub max_requests_per_hour: Option<u64>,
    /// Max data transfer in GB.
    pub max_data_transfer_gb: Option<f64>,
    /// Max compute hours.
    pub max_compute_hours: Option<f64>,
}

/// Permission metadata for audit and display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionMetadata {
    /// Issuer identifier.
    pub issued_by: String,
    /// Notes.
    pub notes: String,
    /// Enabled features.
    pub features: Vec<String>,
}

/// Delegation request for permission lending.
#[derive(Debug, Clone)]
pub struct DelegationRequest {
    /// Request ID.
    pub request_id: Uuid,
    /// Base permission to delegate.
    pub base_permission_id: Uuid,
    /// Current holder delegating.
    pub from_holder: PermissionHolder,
    /// Target holder to receive.
    pub to_holder: PermissionHolder,
    /// Target external integration.
    pub target: ExternalTarget,
    /// Delegation scope.
    pub delegation_scope: DelegationScope,
    /// Delegation duration.
    pub duration: Duration,
    /// Request timestamp.
    pub requested_at: SystemTime,
    /// Request status.
    pub status: DelegationStatus,
}

/// Delegation request status.
#[derive(Debug, Clone)]
pub enum DelegationStatus {
    /// Pending approval.
    Pending,
    /// Approved.
    Approved,
    /// Denied.
    Denied,
    /// Expired.
    Expired,
}

/// Notification for expiring permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiringPermission {
    /// Permission ID.
    pub permission_id: Uuid,
    /// Target integration.
    pub target: ExternalTarget,
    /// Time until expiry.
    pub expires_in: Duration,
}

/// Permission status for display/audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionStatus {
    /// Permission ID.
    pub permission_id: Uuid,
    /// Current holder.
    pub holder: PermissionHolder,
    /// Valid until timestamp.
    pub valid_until: SystemTime,
    /// Permission scope.
    pub scope: PermissionScope,
    /// Whether permission was delegated.
    pub is_delegated: bool,
}

// Cloud providers and external services - re-export from security_provider
pub use crate::security_provider::types::{
    CloudProvider, ContainerPlatform, HPCScheduler, QuantumProvider, ServiceTier,
};
