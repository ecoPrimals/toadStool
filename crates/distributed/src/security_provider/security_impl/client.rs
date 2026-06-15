// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security SecurityProvider Implementation
//!
//! Implements the generic SecurityProvider trait using Security primal.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::interned_strings::capabilities;

use crate::security_provider::{
    EncryptionOptions, SigningOptions,
    provider::{PermissionValidationResult, ProviderHealth, SecurityCapability, SecurityProvider},
    types::{
        DecryptionMetadata, DecryptionResult, EncryptionMetadata, EncryptionResult,
        PermissionRequest, ProviderMetadata, SecurityPermission, SecurityProof, SignatureAlgorithm,
        SignatureResult, VerificationResult,
    },
};

use crate::security::{SecurityClient, SecurityConfig, SecurityDiscovery};

/// Security implementation of SecurityProvider
///
/// This wraps the existing SecurityClient and adapts it to the SecurityProvider trait.
/// This allows Security to be used interchangeably with other security providers.
pub struct DistributedSecurityProvider {
    /// Underlying Security client (wrapped in Arc for sharing)
    client: Arc<RwLock<Option<Arc<SecurityClient>>>>,

    /// Discovery mechanism (for future reconnection logic)
    _discovery: SecurityDiscovery,

    /// Provider metadata
    metadata: ProviderMetadata,

    /// Cached capabilities
    capabilities: Vec<SecurityCapability>,
}

impl DistributedSecurityProvider {
    /// Create a new Security security provider
    ///
    /// This performs runtime discovery to find security service.
    /// If not found, the provider will operate in degraded mode.
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(SecurityConfig::default()).await
    }

    /// Create with custom configuration
    pub async fn with_config(config: SecurityConfig) -> ToadStoolResult<Self> {
        let discovery = SecurityDiscovery::new(config.clone());

        let client = match SecurityClient::new_async(config.clone()).await {
            Ok(client) => {
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
    async fn get_client(&self) -> ToadStoolResult<Arc<SecurityClient>> {
        let client_lock = self.client.read().await;

        if let Some(client) = &*client_lock {
            return Ok(Arc::clone(client));
        }

        drop(client_lock);

        let client = Arc::new(
            SecurityClient::new_async(SecurityConfig::default())
                .await
                .map_err(|e| {
                    ToadStoolError::not_found(format!(
                        "security service not found - security provider unavailable: {e}"
                    ))
                })?,
        );

        let endpoints = client.discover().await?;
        if endpoints.is_empty() {
            return Err(ToadStoolError::not_found(
                "security service not found - security provider unavailable".to_string(),
            ));
        }

        {
            let mut client_lock = self.client.write().await;
            *client_lock = Some(Arc::clone(&client));
        }

        Ok(client)
    }
}

impl SecurityProvider for DistributedSecurityProvider {
    fn capabilities(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<SecurityCapability>>> + Send + '_ {
        let caps = self.capabilities.clone();
        async move { Ok(caps) }
    }

    fn metadata(&self) -> impl Future<Output = ToadStoolResult<ProviderMetadata>> + Send + '_ {
        let meta = self.metadata.clone();
        async move { Ok(meta) }
    }

    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        _options: Option<EncryptionOptions>,
    ) -> impl Future<Output = ToadStoolResult<EncryptionResult>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to encrypt
            use crate::security::types::{EncryptionOperation, EncryptionRequest, SecurityLevel};

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
    }

    fn decrypt<'a>(
        &'a self,
        ciphertext: &'a [u8],
        metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<DecryptionResult>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to decrypt
            use crate::security::types::{EncryptionOperation, EncryptionRequest, SecurityLevel};

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
    }

    fn sign<'a>(
        &'a self,
        data: &'a [u8],
        _options: Option<SigningOptions>,
    ) -> impl Future<Output = ToadStoolResult<SignatureResult>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to sign
            let response = client.sign(data).await?;

            Ok(SignatureResult {
                signature: response.signature,
                algorithm: SignatureAlgorithm::EcdsaP256, // Security default
                key_id: response.key_id,
                signed_at: std::time::SystemTime::now(),
            })
        }
    }

    fn verify<'a>(
        &'a self,
        data: &'a [u8],
        signature: &'a [u8],
        public_key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VerificationResult>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to verify
            let is_valid = client.verify(data, signature, public_key_id).await?;

            Ok(if is_valid {
                VerificationResult::Valid
            } else {
                VerificationResult::Invalid
            })
        }
    }

    fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> impl Future<Output = ToadStoolResult<SecurityPermission>> + Send + '_ {
        async move {
            let client = self.get_client().await?;

            // Use Security client to create permission
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
                    public_key_id: "security-permission-key".to_string(),
                    signed_at: now,
                },
                provider_metadata: self.metadata.clone(),
            })
        }
    }

    fn validate_permission<'a>(
        &'a self,
        permission: &'a SecurityPermission,
    ) -> impl Future<Output = ToadStoolResult<PermissionValidationResult>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to validate
            let is_valid = client.validate_permission(permission).await?;

            Ok(if is_valid {
                PermissionValidationResult::Valid
            } else {
                PermissionValidationResult::InvalidSignature
            })
        }
    }

    fn revoke_permission<'a>(
        &'a self,
        permission_id: &'a uuid::Uuid,
        reason: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            let client = self.get_client().await?;

            // Use Security client to revoke
            client.revoke_permission(permission_id, reason).await
        }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_ {
        async {
            let client_opt = self.client.read().await;
            let client = match client_opt.as_ref() {
                Some(c) => Arc::clone(c),
                None => return Ok(ProviderHealth::Unhealthy),
            };
            drop(client_opt);

            // Call Security health_check to verify client is responsive
            match client.health_check().await {
                Ok(endpoints) if !endpoints.is_empty() => Ok(ProviderHealth::Healthy),
                Ok(_) => Ok(ProviderHealth::Degraded),
                Err(_) => Ok(ProviderHealth::Unhealthy),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_provider_creation() {
        // This may fail if Security is not running, which is expected
        let result = DistributedSecurityProvider::new().await;

        // Provider creation should succeed even if Security is not available
        // (it will operate in degraded mode)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_security_provider_capabilities() {
        let provider = DistributedSecurityProvider::new().await.unwrap();
        let caps = provider.capabilities().await.unwrap();

        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
    }

    #[tokio::test]
    async fn test_security_provider_metadata() {
        let provider = DistributedSecurityProvider::new().await.unwrap();
        let metadata = provider.metadata().await.unwrap();

        assert_eq!(metadata.provider_type, "crypto");
    }
}
