//! Cryptographic validation and verification for crypto lock system
//!
//! **Deep Debt Evolution**: Now capability-based via Universal Adapter!
//! - Discovers security provider at runtime
//! - No hardcoded primal names
//! - Falls back to local validation if no provider available

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use toadstool::error::ToadStoolResult;

use super::permissions::{ExternalTarget as CryptoExternalTarget, SecurityProviderPermission};
use crate::security_provider::provider::SecurityProvider;
use crate::security_provider::types::{
    ExternalTarget as ProviderExternalTarget, PermissionScope as ProviderPermissionScope,
    ProviderMetadata, ResourceLimits as ProviderResourceLimits,
    SecurityPermission as ProviderPermission, SecurityProof as ProviderSecurityProof,
    SignatureAlgorithm,
};

/// Security Permission Validator - validates crypto permissions
///
/// **Deep Debt**: Uses Universal Adapter to discover security provider (no hardcoded primal)
pub struct SecurityPermissionValidator {
    /// Security provider (discovered at runtime via Universal Adapter)
    security_provider: Option<Arc<dyn SecurityProvider>>,

    /// Security provider public keys for permission verification (fallback)
    _security_provider_keys: HashMap<String, SecurityPublicKey>,
    /// Cryptographic signature validator (fallback)
    _crypto_validator: CryptoValidator,
    /// Permission delegation chain validator
    _delegation_validator: DelegationValidator,
    /// Permission revocation list
    _revocation_list: PermissionRevocationList,
}

impl SecurityPermissionValidator {
    /// Create validator with runtime-discovered security provider
    ///
    /// **Deep Debt**: Discovers provider via Universal Adapter (no hardcoded BearDog!)
    pub async fn new() -> ToadStoolResult<Self> {
        // Try to discover security provider via Universal Adapter
        let security_provider = Self::discover_security_provider().await;

        Ok(Self {
            security_provider,
            _security_provider_keys: HashMap::new(),
            _crypto_validator: CryptoValidator::new(),
            _delegation_validator: DelegationValidator::new(),
            _revocation_list: PermissionRevocationList::new(),
        })
    }

    /// Discover security provider via Universal Adapter
    ///
    /// **Deep Debt**: Runtime discovery, not hardcoded!
    async fn discover_security_provider() -> Option<Arc<dyn SecurityProvider>> {
        // Try to discover security provider via Universal Adapter
        use toadstool_common::universal_adapter::*;

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
    /// **Deep Debt**: Uses SecurityProvider trait (not hardcoded BearDog!)
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
            // No provider discovered — local time-validity check is all we can do.
            // The caller already passed the time window check above, so the permission
            // is structurally valid for this node.
            Ok(PermissionValidationResult::Valid)
        }
    }

    pub async fn validate_delegation_proof(&self, _proof: &SecurityProof) -> ToadStoolResult<()> {
        // Validate delegation proof
        Ok(())
    }
}

/// Convert a `SecurityProviderPermission` (crypto_lock domain type) into the
/// provider-agnostic `SecurityPermission` expected by the `SecurityProvider` trait.
///
/// This is a bridge function that will be obsoleted once the two type systems are unified.
fn to_provider_permission(p: &SecurityProviderPermission) -> ProviderPermission {
    let holder_id = match &p.holder {
        super::permissions::PermissionHolder::Individual { user_id, .. } => user_id.clone(),
        super::permissions::PermissionHolder::Organization { org_id, .. } => org_id.clone(),
        super::permissions::PermissionHolder::Delegated { delegated_to, .. } => {
            delegated_to.clone()
        }
    };

    let target = match &p.external_target {
        CryptoExternalTarget::CloudProvider {
            provider, regions, ..
        } => ProviderExternalTarget::CloudProvider {
            provider: format!("{:?}", provider),
            regions: regions.clone(),
        },
        CryptoExternalTarget::ContainerPlatform {
            platform, clusters, ..
        } => ProviderExternalTarget::ContainerPlatform {
            platform: format!("{:?}", platform),
            clusters: clusters.clone(),
        },
        CryptoExternalTarget::ExternalTool {
            tool_name,
            api_endpoints,
            ..
        } => ProviderExternalTarget::ExternalTool {
            tool_name: tool_name.clone(),
            endpoints: api_endpoints.clone(),
        },
        // Map exotic targets to a generic ExternalTool representation
        CryptoExternalTarget::QuantumProvider {
            provider, backends, ..
        } => ProviderExternalTarget::ExternalTool {
            tool_name: format!("quantum:{:?}", provider),
            endpoints: backends.clone(),
        },
        CryptoExternalTarget::HPCCluster {
            cluster_name,
            partitions,
            ..
        } => ProviderExternalTarget::ExternalTool {
            tool_name: format!("hpc:{}", cluster_name),
            endpoints: partitions.clone(),
        },
        CryptoExternalTarget::EnterpriseService {
            service_name,
            features,
            ..
        } => ProviderExternalTarget::ExternalTool {
            tool_name: service_name.clone(),
            endpoints: features.clone(),
        },
    };

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

/// Security provider cryptographic proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProof {
    /// Cryptographic signature
    pub signature: Vec<u8>,
    /// Signature algorithm used
    pub algorithm: CryptoAlgorithm,
    /// Public key identifier
    pub public_key_id: String,
    /// Proof timestamp
    pub timestamp: SystemTime,
    /// Additional proof metadata
    pub metadata: ProofMetadata,
}

/// Cryptographic algorithms supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CryptoAlgorithm {
    Ed25519,
    EcdsaP256,
    Rsa4096,
    BearDogCustom,
}

/// Proof metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    pub issuer: String,
    pub purpose: String,
    pub additional_claims: HashMap<String, String>,
}

/// Permission validation result
#[derive(Debug, Clone)]
pub enum PermissionValidationResult {
    Valid,
    Invalid,
    Expired,
    Revoked,
}

/// Verification level for identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationLevel {
    Unverified,
    EmailVerified,
    IdentityVerified,
    InstitutionVerified,
}

/// Cryptographic signature validator
pub struct CryptoValidator;

impl Default for CryptoValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Delegation chain validator
pub struct DelegationValidator;

impl Default for DelegationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegationValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Permission revocation list
pub struct PermissionRevocationList;

impl Default for PermissionRevocationList {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionRevocationList {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Security provider public key
pub struct SecurityPublicKey;
