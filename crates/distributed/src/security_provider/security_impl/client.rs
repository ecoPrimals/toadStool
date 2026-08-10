// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security SecurityProvider Implementation
//!
//! Implements the generic SecurityProvider trait using the crypto integration client.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, RwLock};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::interned_strings::capabilities;

use crate::crypto_integration::{
    CryptoServiceClient, CryptoServiceConfig, CryptoServiceDiscovery,
    encryption_algorithm_from_wire,
    types::{CryptoOperation, CryptoRequest, SecurityLevel},
};
use crate::security_provider::{
    EncryptionOptions, SigningOptions,
    provider::{PermissionValidationResult, ProviderHealth, SecurityCapability, SecurityProvider},
    types::{
        DecryptionMetadata, DecryptionResult, EncryptionMetadata, EncryptionResult,
        PermissionRequest, ProviderMetadata, SecurityPermission, SecurityProof, SignatureAlgorithm,
        SignatureResult, VerificationResult,
    },
};

/// Security implementation of SecurityProvider
///
/// This wraps the crypto integration client and adapts it to the SecurityProvider trait.
/// This allows the distributed crypto provider to be used interchangeably with other security providers.
pub struct DistributedSecurityProvider {
    /// Underlying crypto client (wrapped in Arc for sharing)
    client: Arc<RwLock<Option<Arc<CryptoServiceClient>>>>,

    /// Discovery configuration (for future reconnection logic)
    config: CryptoServiceConfig,

    /// Provider metadata
    metadata: ProviderMetadata,

    /// Cached capabilities
    capabilities: Vec<SecurityCapability>,
}

impl DistributedSecurityProvider {
    /// Create a new distributed crypto security provider
    ///
    /// This performs runtime discovery to find a crypto service.
    /// If not found, the provider will operate in degraded mode.
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(CryptoServiceConfig::default()).await
    }

    /// Create with custom configuration
    pub async fn with_config(config: CryptoServiceConfig) -> ToadStoolResult<Self> {
        let _discovery = CryptoServiceDiscovery::new(config.clone()).await?;

        let client = Self::connect_client(&config).await.ok();

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
            config,
            metadata,
            capabilities,
        })
    }

    async fn connect_client(
        config: &CryptoServiceConfig,
    ) -> ToadStoolResult<Arc<CryptoServiceClient>> {
        #[cfg(unix)]
        {
            if let Ok(socket_path) =
                toadstool_common::primal_sockets::discover_crypto_socket().await
            {
                return CryptoServiceClient::from_local_socket(&socket_path).map(Arc::new);
            }
        }

        let discovery = CryptoServiceDiscovery::new(config.clone()).await?;
        let services = discovery.discover().await?;
        let service = services.first().ok_or_else(|| {
            ToadStoolError::not_found(
                "crypto service not found - security provider unavailable".to_string(),
            )
        })?;

        CryptoServiceClient::new(service).map(Arc::new)
    }

    /// Get or create client connection
    async fn get_client(&self) -> ToadStoolResult<Arc<CryptoServiceClient>> {
        {
            let client_lock = self.client.read().unwrap_or_else(|e| e.into_inner());
            if let Some(client) = client_lock.as_ref() {
                return Ok(Arc::clone(client));
            }
        }

        let client = Self::connect_client(&self.config).await?;

        {
            let mut client_lock = self.client.write().unwrap_or_else(|e| e.into_inner());
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

            let request = CryptoRequest {
                request_id: uuid::Uuid::new_v4(),
                operation: CryptoOperation::Encrypt,
                data: data.to_vec(),
                key_id: None,
                algorithm: Some(encryption_algorithm_from_wire("AES-256-GCM")),
                security_level: SecurityLevel::Standard,
                metadata: serde_json::Value::Null,
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

            let request = CryptoRequest {
                request_id: uuid::Uuid::new_v4(),
                operation: CryptoOperation::Decrypt,
                data: ciphertext.to_vec(),
                key_id: Some(metadata.key_id.clone()),
                algorithm: Some(encryption_algorithm_from_wire(&metadata.algorithm)),
                security_level: SecurityLevel::Standard,
                metadata: serde_json::Value::Null,
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

            let response = client.sign(data).await?;

            Ok(SignatureResult {
                signature: response.signature,
                algorithm: SignatureAlgorithm::EcdsaP256,
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
                    public_key_id: "crypto-permission-key".to_string(),
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

            client.revoke_permission(permission_id, reason).await
        }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_ {
        async {
            let client = {
                let client_opt = self.client.read().unwrap_or_else(|e| e.into_inner());
                match client_opt.as_ref() {
                    Some(c) => Arc::clone(c),
                    None => return Ok(ProviderHealth::Unhealthy),
                }
            };

            match client.health_check().await {
                Ok(true) => Ok(ProviderHealth::Healthy),
                Ok(false) => Ok(ProviderHealth::Degraded),
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
        // This may fail if crypto service is not running, which is expected
        let result = DistributedSecurityProvider::new().await;

        // Provider creation should succeed even if crypto service is not available
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
