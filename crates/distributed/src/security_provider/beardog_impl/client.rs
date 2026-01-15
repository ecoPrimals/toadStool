//! BearDog SecurityProvider Implementation
//!
//! Implements the generic SecurityProvider trait using BearDog primal.

// Allow deprecated during migration phase
#[allow(deprecated)]

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::security_provider::{
    provider::*,
    types::*,
    EncryptionOptions,
    SigningOptions,
};

use crate::beardog_integration::{BearDogClient, BearDogConfig, BearDogDiscovery};

/// BearDog implementation of SecurityProvider
///
/// This wraps the existing BearDogClient and adapts it to the SecurityProvider trait.
/// This allows BearDog to be used interchangeably with other security providers.
pub struct BearDogSecurityProvider {
    /// Underlying BearDog client (wrapped in Arc for sharing)
    client: Arc<RwLock<Option<Arc<BearDogClient>>>>,
    
    /// Discovery mechanism (for future reconnection logic)
    #[allow(dead_code)]
    discovery: BearDogDiscovery,
    
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
            provider_type: "beardog".to_string(),
            provider_version: "2.0.0".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("primal".to_string(), "beardog".to_string());
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
            discovery,
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
        
        let mut client_lock = self.client.write().await;
        *client_lock = Some(Arc::clone(&client));
        
        Ok(client)
    }
}

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
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to encrypt once available
        // For now, return placeholder
        Ok(EncryptionResult {
            ciphertext: data.to_vec(), // Placeholder: not actually encrypted
            iv: Some(vec![0; 16]),
            auth_tag: Some(vec![0; 16]),
            metadata: EncryptionMetadata {
                algorithm: "AES-256-GCM".to_string(),
                key_id: "beardog-key-placeholder".to_string(),
                encrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to decrypt once available
        // For now, return placeholder
        Ok(DecryptionResult {
            plaintext: ciphertext.to_vec(), // Placeholder: not actually decrypted
            metadata: DecryptionMetadata {
                key_id: metadata.key_id.clone(),
                decrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn sign(
        &self,
        _data: &[u8],
        _options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to sign once available
        // For now, return placeholder
        Ok(SignatureResult {
            signature: vec![0xDE, 0xAD, 0xBE, 0xEF], // Placeholder signature
            algorithm: SignatureAlgorithm::EcdsaP256,
            key_id: "beardog-signing-key-placeholder".to_string(),
            signed_at: std::time::SystemTime::now(),
        })
    }

    async fn verify(
        &self,
        _data: &[u8],
        _signature: &[u8],
        _public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to verify once available
        // For now, return placeholder
        Ok(VerificationResult::Valid)
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to create permission once available
        // For now, return placeholder
        let now = std::time::SystemTime::now();
        
        Ok(SecurityPermission {
            permission_id: uuid::Uuid::new_v4(),
            holder_id: request.requester_id,
            target: request.target,
            scope: request.scope,
            valid_from: now,
            valid_until: now + request.validity_duration,
            proof: SecurityProof {
                signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
                algorithm: SignatureAlgorithm::EcdsaP256,
                public_key_id: "beardog-permission-key-placeholder".to_string(),
                signed_at: now,
            },
            provider_metadata: self.metadata.clone(),
        })
    }

    async fn validate_permission(
        &self,
        _permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        let _client = self.get_client().await?;
        
        // TODO: Use BearDog client to validate once available
        // For now, return placeholder
        Ok(PermissionValidationResult::Valid)
    }

    async fn revoke_permission(
        &self,
        _permission_id: &uuid::Uuid,
        _reason: &str,
    ) -> ToadStoolResult<()> {
        // TODO: Use BearDog client to revoke once available
        // For now, no-op
        Ok(())
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        let client_lock = self.client.read().await;
        
        match &*client_lock {
            Some(_client) => {
                // TODO: Check if client is responsive once health_check is available
                // For now, assume healthy if we have a client
                Ok(ProviderHealth::Healthy)
            }
            None => Ok(ProviderHealth::Unhealthy),
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
        
        assert_eq!(metadata.provider_type, "beardog");
    }
}
