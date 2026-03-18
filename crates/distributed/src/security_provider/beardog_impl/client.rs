// SPDX-License-Identifier: AGPL-3.0-or-later
//! BearDog SecurityProvider Implementation
//!
//! Implements the generic SecurityProvider trait using BearDog primal.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::interned_strings::capabilities;

use crate::security_provider::{EncryptionOptions, SigningOptions, provider::*, types::*};

use crate::beardog_integration::{BearDogClient, BearDogConfig, BearDogDiscovery};

/// BearDog implementation of SecurityProvider
///
/// This wraps the existing BearDogClient and adapts it to the SecurityProvider trait.
/// This allows BearDog to be used interchangeably with other security providers.
pub struct BearDogSecurityProvider {
    /// Underlying BearDog client (wrapped in Arc for sharing)
    client: Arc<RwLock<Option<Arc<BearDogClient>>>>,

    /// Discovery mechanism (for future reconnection logic)
    _discovery: BearDogDiscovery,

    /// Provider metadata
    metadata: ProviderMetadata,

    /// Cached capabilities
    capabilities: Vec<SecurityCapability>,
}

impl BearDogSecurityProvider {
    /// Create a new BearDog security provider
    ///
    /// This performs runtime discovery to find BearDog service.
    /// If not found, the provider will operate in degraded mode.
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(BearDogConfig::default()).await
    }

    /// Create with custom configuration
    pub async fn with_config(config: BearDogConfig) -> ToadStoolResult<Self> {
        let discovery = BearDogDiscovery::new(config.clone());

        // Attempt to discover and create BearDog client
        let client = match BearDogClient::new(config.clone()) {
            Ok(client) => {
                // Verify we can discover endpoints
                match client.discover().await {
                    Ok(endpoints) if !endpoints.is_empty() => Some(Arc::new(client)),
                    _ => None,
                }
            }
            Err(_) => None,
        };

        let metadata = ProviderMetadata {
            provider_id: uuid::Uuid::new_v4().to_string(),
            provider_type: capabilities::CRYPTO.to_string(),
            provider_version: "2.0.0".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("capability".to_string(), capabilities::CRYPTO.to_string());
                m.insert("discovery".to_string(), "runtime".to_string());
                m
            },
        };

        let capabilities = vec![
            SecurityCapability::SymmetricEncryption,
            SecurityCapability::AsymmetricEncryption,
            SecurityCapability::DigitalSignatures,
            SecurityCapability::KeyManagement,
            SecurityCapability::PermissionIssuance,
            SecurityCapability::AuditLogging,
        ];

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            _discovery: discovery,
            metadata,
            capabilities,
        })
    }

    /// Get or create client connection
    async fn get_client(&self) -> ToadStoolResult<Arc<BearDogClient>> {
        let client_lock = self.client.read().await;

        if let Some(client) = &*client_lock {
            return Ok(Arc::clone(client));
        }

        drop(client_lock);

        // No client, try to discover and create
        let client = Arc::new(BearDogClient::new(BearDogConfig::default())?);

        // Verify we can discover endpoints
        let endpoints = client.discover().await?;
        if endpoints.is_empty() {
            return Err(ToadStoolError::not_found(
                "BearDog service not found - security provider unavailable".to_string(),
            ));
        }

        {
            let mut client_lock = self.client.write().await;
            *client_lock = Some(Arc::clone(&client));
        }

        Ok(client)
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl SecurityProvider for BearDogSecurityProvider {
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>> {
        Ok(self.capabilities.clone())
    }

    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata> {
        Ok(self.metadata.clone())
    }

    async fn encrypt(
        &self,
        data: &[u8],
        _options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult> {
        let client = self.get_client().await?;

        // Use BearDog client to encrypt
        use crate::beardog_integration::types::{
            EncryptionOperation, EncryptionRequest, SecurityLevel,
        };

        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Encrypt,
            data: data.to_vec(),
            key_id: None,
            algorithm: Some("AES-256-GCM".to_string()),
            security_level: SecurityLevel::Standard,
        };

        let response = client.encrypt(request).await?;

        // Extract IV and auth tag from metadata
        let metadata_obj = response.metadata.as_object();
        let iv = metadata_obj
            .and_then(|m| m.get("iv"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect()
            });

        let auth_tag = metadata_obj
            .and_then(|m| m.get("auth_tag"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect()
            });

        Ok(EncryptionResult {
            ciphertext: response.data,
            iv,
            auth_tag,
            metadata: EncryptionMetadata {
                algorithm: response.algorithm,
                key_id: response.key_id,
                encrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        let client = self.get_client().await?;

        // Use BearDog client to decrypt
        use crate::beardog_integration::types::{
            EncryptionOperation, EncryptionRequest, SecurityLevel,
        };

        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Decrypt,
            data: ciphertext.to_vec(),
            key_id: Some(metadata.key_id.clone()),
            algorithm: Some(metadata.algorithm.clone()),
            security_level: SecurityLevel::Standard,
        };

        let response = client.decrypt(request).await?;

        Ok(DecryptionResult {
            plaintext: response.data,
            metadata: DecryptionMetadata {
                key_id: metadata.key_id.clone(),
                decrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn sign(
        &self,
        data: &[u8],
        _options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        let client = self.get_client().await?;

        // Use BearDog client to sign
        let response = client.sign(data).await?;

        Ok(SignatureResult {
            signature: response.signature,
            algorithm: SignatureAlgorithm::EcdsaP256, // BearDog default
            key_id: response.key_id,
            signed_at: std::time::SystemTime::now(),
        })
    }

    async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        let client = self.get_client().await?;

        // Use BearDog client to verify
        let is_valid = client.verify(data, signature, public_key_id).await?;

        Ok(if is_valid {
            VerificationResult::Valid
        } else {
            VerificationResult::Invalid
        })
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        let client = self.get_client().await?;

        // Use BearDog client to create permission
        let response = client.create_permission(&request).await?;

        let now = std::time::SystemTime::now();

        Ok(SecurityPermission {
            permission_id: response.permission_id,
            holder_id: request.requester_id,
            target: request.target,
            scope: request.scope,
            valid_from: now,
            valid_until: now + request.validity_duration,
            proof: SecurityProof {
                signature: response.proof,
                algorithm: SignatureAlgorithm::EcdsaP256,
                public_key_id: "beardog-permission-key".to_string(),
                signed_at: now,
            },
            provider_metadata: self.metadata.clone(),
        })
    }

    async fn validate_permission(
        &self,
        permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        let client = self.get_client().await?;

        // Use BearDog client to validate
        let is_valid = client.validate_permission(permission).await?;

        Ok(if is_valid {
            PermissionValidationResult::Valid
        } else {
            PermissionValidationResult::InvalidSignature
        })
    }

    async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        let client = self.get_client().await?;

        // Use BearDog client to revoke
        client.revoke_permission(permission_id, reason).await
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        let client_opt = self.client.read().await;
        let client = match client_opt.as_ref() {
            Some(c) => Arc::clone(c),
            None => return Ok(ProviderHealth::Unhealthy),
        };
        drop(client_opt);

        // Call BearDog health_check to verify client is responsive
        match client.health_check().await {
            Ok(endpoints) if !endpoints.is_empty() => Ok(ProviderHealth::Healthy),
            Ok(_) => Ok(ProviderHealth::Degraded),
            Err(_) => Ok(ProviderHealth::Unhealthy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_beardog_provider_creation() {
        // This may fail if BearDog is not running, which is expected
        let result = BearDogSecurityProvider::new().await;

        // Provider creation should succeed even if BearDog is not available
        // (it will operate in degraded mode)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_beardog_provider_capabilities() {
        let provider = BearDogSecurityProvider::new().await.unwrap();
        let caps = provider.capabilities().await.unwrap();

        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
    }

    #[tokio::test]
    async fn test_beardog_provider_metadata() {
        let provider = BearDogSecurityProvider::new().await.unwrap();
        let metadata = provider.metadata().await.unwrap();

        assert_eq!(metadata.provider_type, "crypto");
    }
}
