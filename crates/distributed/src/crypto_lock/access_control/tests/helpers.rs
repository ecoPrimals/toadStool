// SPDX-License-Identifier: AGPL-3.0-only
//! Test helpers for access control tests

use std::time::{Duration, SystemTime};
use uuid::Uuid;

use crate::crypto_lock::permissions::{
    PermissionHolder, PermissionMetadata, PermissionScope, ResourceLimits,
    SecurityProviderPermission, TimeRestrictions, UsageQuotas,
};
use crate::crypto_lock::validation::{
    CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
};
use crate::security_provider::types::{CloudProvider, ExternalTarget};

pub fn pure_rust_target() -> ExternalTarget {
    ExternalTarget::ExternalTool {
        tool_name: "toadstool".to_string(),
        api_endpoints: vec![],
        feature_set: vec!["ecosystem_native".to_string()],
    }
}

pub fn trusted_target() -> ExternalTarget {
    ExternalTarget::ExternalTool {
        tool_name: "nestgate".to_string(),
        api_endpoints: vec![],
        feature_set: vec!["trusted".to_string()],
    }
}

pub fn primal_toadstool_target() -> ExternalTarget {
    ExternalTarget::ExternalTool {
        tool_name: "tool".to_string(),
        api_endpoints: vec![],
        feature_set: vec!["primal:toadstool".to_string()],
    }
}

pub fn cloud_target() -> ExternalTarget {
    ExternalTarget::CloudProvider {
        provider: CloudProvider::AWS,
        regions: vec!["us-east-1".to_string()],
        services: vec![],
    }
}

pub fn external_tool_target_no_trust() -> ExternalTarget {
    ExternalTarget::ExternalTool {
        tool_name: "external-api".to_string(),
        api_endpoints: vec!["https://api.example.com".to_string()],
        feature_set: vec![],
    }
}

pub fn make_expired_permission(target: ExternalTarget) -> SecurityProviderPermission {
    let now = SystemTime::now();
    let valid_from = now - Duration::from_secs(3600);
    let valid_until = now - Duration::from_secs(1);
    SecurityProviderPermission {
        permission_id: Uuid::new_v4(),
        holder: PermissionHolder::Individual {
            user_id: "user1".to_string(),
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
        valid_from,
        valid_until,
        crypto_proof: SecurityProof {
            signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
            algorithm: CryptoAlgorithm::Ed25519,
            public_key_id: "key1".to_string(),
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
