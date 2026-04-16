// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cryptographic validation and verification for crypto lock system
//!
//! **Deep Debt Evolution**: Now capability-based via Universal Adapter!
//! - Discovers security provider at runtime
//! - No hardcoded primal names
//! - Falls back to local validation if no provider available

mod types;
mod validators;

#[cfg(test)]
mod tests;

pub use types::*;
pub use validators::*;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use toadstool::error::ToadStoolResult;

use super::permissions::SecurityProviderPermission;
use crate::security_provider::SecurityProviderDispatch;
use crate::security_provider::provider::SecurityProvider;
use crate::security_provider::types::{
    PermissionScope as ProviderPermissionScope, ProviderMetadata,
    ResourceLimits as ProviderResourceLimits, SecurityPermission as ProviderPermission,
    SecurityProof as ProviderSecurityProof, SignatureAlgorithm,
};

/// Security Permission Validator - validates crypto permissions
///
/// **Deep Debt**: Uses Universal Adapter to discover security provider (no hardcoded primal)
pub struct SecurityPermissionValidator {
    /// Security provider (discovered at runtime via Universal Adapter)
    security_provider: Option<Arc<SecurityProviderDispatch>>,

    /// Security provider public keys for permission verification (fallback)
    security_provider_keys: HashMap<String, SecurityPublicKey>,
    /// Cryptographic signature validator (fallback)
    crypto_validator: CryptoValidator,
    /// Permission delegation chain validator
    delegation_validator: DelegationValidator,
    /// Permission revocation list
    revocation_list: PermissionRevocationList,
}

impl SecurityPermissionValidator {
    /// Create validator with runtime-discovered security provider
    ///
    /// **Deep Debt**: Discovers provider via Universal Adapter (no hardcoded Security!)
    pub async fn new() -> ToadStoolResult<Self> {
        // Try to discover security provider via Universal Adapter
        let security_provider = Self::discover_security_provider().await;

        Ok(Self {
            security_provider,
            security_provider_keys: HashMap::new(),
            crypto_validator: CryptoValidator::new(),
            delegation_validator: DelegationValidator::new(),
            revocation_list: PermissionRevocationList::new(),
        })
    }

    /// Register a public key for a provider (used when no external discovery available).
    pub fn register_key(&mut self, provider_id: String, key: SecurityPublicKey) {
        self.security_provider_keys.insert(provider_id, key);
    }

    /// Discover security provider via Universal Adapter
    ///
    /// **Deep Debt**: Runtime discovery, not hardcoded!
    async fn discover_security_provider() -> Option<Arc<SecurityProviderDispatch>> {
        // Try to discover security provider via Universal Adapter
        use toadstool_common::universal_adapter::{
            CapabilityType, SecurityFeature, TrustLevel, UniversalAdapter,
        };

        match UniversalAdapter::new().await {
            Ok(adapter) => {
                let request = CapabilityType::Security {
                    features: vec![SecurityFeature::Signing],
                    min_trust_level: TrustLevel::High,
                };

                match adapter.request_capability(request).await {
                    Ok(handle) => {
                        // Try to create provider from handle
                        use crate::security_provider::factory::SecurityProviderFactory;

                        (SecurityProviderFactory::create_from_handle(&handle).await).ok()
                    }
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Validate permission using discovered security provider
    ///
    /// **Deep Debt**: Uses SecurityProvider trait (not hardcoded Security!)
    pub async fn validate_permission(
        &self,
        permission: &SecurityProviderPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        // Check basic time validity first
        let now = SystemTime::now();
        if now < permission.valid_from {
            return Ok(PermissionValidationResult::Invalid);
        }
        if now > permission.valid_until {
            return Ok(PermissionValidationResult::Expired);
        }

        // Route to discovered provider when available; fall back to local time-only validation.
        if let Some(provider) = &self.security_provider {
            let provider_permission = to_provider_permission(permission);
            let provider_result = provider.validate_permission(&provider_permission).await?;
            Ok(match provider_result {
                crate::security_provider::provider::PermissionValidationResult::Valid => {
                    PermissionValidationResult::Valid
                }
                crate::security_provider::provider::PermissionValidationResult::InvalidSignature
                | crate::security_provider::provider::PermissionValidationResult::NotFound => {
                    PermissionValidationResult::Invalid
                }
                crate::security_provider::provider::PermissionValidationResult::Expired => {
                    PermissionValidationResult::Expired
                }
                crate::security_provider::provider::PermissionValidationResult::Revoked => {
                    PermissionValidationResult::Revoked
                }
            })
        } else {
            // No external provider — use local revocation check + crypto + delegation.
            if self.revocation_list.is_revoked(&permission.permission_id) {
                return Ok(PermissionValidationResult::Revoked);
            }
            if !self.crypto_validator.validate_signature(
                &permission.crypto_proof.signature,
                permission.crypto_proof.timestamp,
            ) {
                return Ok(PermissionValidationResult::Invalid);
            }
            if let Some(chain) = &permission.delegation_chain
                && !self.delegation_validator.validate_chain(chain)
            {
                return Ok(PermissionValidationResult::Invalid);
            }
            Ok(PermissionValidationResult::Valid)
        }
    }

    /// Validates a delegation proof by checking signature presence, timestamp
    /// freshness, and delegating to the discovered security provider when available.
    pub async fn validate_delegation_proof(&self, proof: &SecurityProof) -> ToadStoolResult<()> {
        if proof.signature.is_empty() {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof has empty signature".to_string(),
            ));
        }

        let elapsed = proof
            .timestamp
            .elapsed()
            .unwrap_or(std::time::Duration::MAX);
        const MAX_PROOF_AGE: std::time::Duration = std::time::Duration::from_secs(86_400);
        if elapsed > MAX_PROOF_AGE {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof expired (older than 24h)".to_string(),
            ));
        }

        if proof.public_key_id.is_empty() {
            return Err(toadstool::error::ToadStoolError::validation(
                "delegation proof missing public key id".to_string(),
            ));
        }

        if let Some(provider) = &self.security_provider {
            provider
                .verify(&proof.signature, &proof.signature, &proof.public_key_id)
                .await?;
        }

        Ok(())
    }
}

/// Convert a `SecurityProviderPermission` (crypto_lock domain type) into the
/// provider-agnostic `SecurityPermission` expected by the `SecurityProvider` trait.
fn to_provider_permission(p: &SecurityProviderPermission) -> ProviderPermission {
    let holder_id = match &p.holder {
        super::permissions::PermissionHolder::Individual { user_id, .. } => user_id.clone(),
        super::permissions::PermissionHolder::Organization { org_id, .. } => org_id.clone(),
        super::permissions::PermissionHolder::Delegated { delegated_to, .. } => {
            delegated_to.clone()
        }
    };

    // ExternalTarget is unified - same type in both domains
    let target = p.external_target.clone();

    let scope = ProviderPermissionScope {
        operations: p.scope.feature_restrictions.clone(),
        resource_limits: ProviderResourceLimits::default(),
        geo_restrictions: p.scope.geographic_limits.clone(),
    };

    let proof = ProviderSecurityProof {
        signature: p.crypto_proof.signature.clone(),
        algorithm: SignatureAlgorithm::Ed25519,
        public_key_id: p.crypto_proof.public_key_id.clone(),
        signed_at: p.crypto_proof.timestamp,
    };

    let provider_metadata = ProviderMetadata {
        provider_id: "crypto-lock".to_string(),
        provider_type: "local".to_string(),
        provider_version: "0.1.0".to_string(),
        metadata: HashMap::new(),
    };

    ProviderPermission {
        permission_id: p.permission_id,
        holder_id,
        target,
        scope,
        valid_from: p.valid_from,
        valid_until: p.valid_until,
        proof,
        provider_metadata,
    }
}
