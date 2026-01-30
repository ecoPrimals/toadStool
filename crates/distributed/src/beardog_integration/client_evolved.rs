//! Security Service Client - Capability-based discovery (Evolved)
//!
//! **DEEP DEBT EVOLUTION**: This is the evolved version of the BearDog client.
//! It discovers security providers by capability, not by hardcoded "beardog" name.
//!
//! **Design Philosophy**:
//! - **Pure Rust**: Unix sockets, no HTTP/TLS
//! - **Async-first**: Non-blocking operations with tokio
//! - **Local IPC**: Fast, secure primal-to-primal communication
//! - **Zero hardcoding**: Discovers "who provides security?" not "where is beardog?"
//! - **Capability-based**: Works with ANY security provider (beardog, vault, etc.)

use toadstool_common::capability_provider::{CapabilityProvider, CapabilityError};
use toadstool_common::primal_identity::Capability;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors for security service client
#[derive(Debug, thiserror::Error)]
pub enum SecurityClientError {
    #[error("No security provider found")]
    NoProvider,
    
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Signature failed: {0}")]
    SignatureFailed(String),
    
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Key management failed: {0}")]
    KeyManagementFailed(String),
    
    #[error("Token validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SecurityClientError>;

/// Encryption request
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionRequest {
    pub data: Vec<u8>,
    pub algorithm: String,
    pub key_id: Option<String>,
}

/// Encryption response
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionResponse {
    pub encrypted_data: Vec<u8>,
    pub key_id: String,
    pub algorithm: String,
}

/// Decryption request
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionRequest {
    pub encrypted_data: Vec<u8>,
    pub key_id: String,
}

/// Decryption response
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptionResponse {
    pub data: Vec<u8>,
}

/// Signature request
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureRequest {
    pub data: Vec<u8>,
    pub algorithm: String,
    pub key_id: Option<String>,
}

/// Signature response
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureResponse {
    pub signature: Vec<u8>,
    pub key_id: String,
    pub algorithm: String,
}

/// Verification request
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
    pub key_id: String,
}

/// Verification response
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResponse {
    pub valid: bool,
    pub reason: Option<String>,
}

/// Token validation request
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenValidationRequest {
    pub token: String,
}

/// Token validation response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenValidationResponse {
    pub valid: bool,
    pub user_id: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<i64>,
}

/// Security service client with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs security services
/// 2. **Runtime discovery**: Finds provider by capability
/// 3. **Proper errors**: No unwrap(), all errors handled
/// 4. **Agnostic**: Doesn't care which primal provides security
///
/// # Evolution from Legacy
///
/// Before: `BearDogClient` hardcoded to "/primal/beardog"
/// After: `SecurityClient` discovers "who provides security?"
pub struct SecurityClient {
    /// Security provider (discovered at runtime)
    provider: Arc<RwLock<Option<CapabilityProvider>>>,
}

impl SecurityClient {
    /// Create new security client
    pub fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Get or discover security provider
    ///
    /// Discovers by capability: "Who provides security services?"
    async fn get_provider(&self) -> Result<CapabilityProvider> {
        let mut provider_lock = self.provider.write().await;
        
        if provider_lock.is_none() {
            // Use Crypto capability for security services
            use toadstool_common::primal_identity::CryptoCapability;
            let capability = Capability::Crypto(CryptoCapability::Encryption);
            
            let discovered = CapabilityProvider::discover(capability)
                .await
                .map_err(|e| match e {
                    CapabilityError::NoProviderFound(_) => SecurityClientError::NoProvider,
                    other => SecurityClientError::Capability(other),
                })?;
            
            *provider_lock = Some(discovered);
        }
        
        Ok(provider_lock.as_ref().unwrap().clone())
    }
    
    /// Encrypt data
    ///
    /// # Deep Debt Evolution
    ///
    /// Before: `call_rpc("/primal/beardog", "beardog.encrypt", ...)`
    /// After: `provider.call("security.encrypt", ...)`
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable or encryption fails
    pub async fn encrypt(&self, request: EncryptionRequest) -> Result<EncryptionResponse> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "data": request.data,
            "algorithm": request.algorithm,
            "key_id": request.key_id,
        });
        
        let response = provider.call("security.encrypt", params)
            .await
            .map_err(|e| SecurityClientError::EncryptionFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(SecurityClientError::Json)
    }
    
    /// Decrypt data
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails
    pub async fn decrypt(&self, request: DecryptionRequest) -> Result<DecryptionResponse> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "encrypted_data": request.encrypted_data,
            "key_id": request.key_id,
        });
        
        let response = provider.call("security.decrypt", params)
            .await
            .map_err(|e| SecurityClientError::DecryptionFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(SecurityClientError::Json)
    }
    
    /// Sign data
    ///
    /// # Errors
    ///
    /// Returns error if signing fails
    pub async fn sign(&self, request: SignatureRequest) -> Result<SignatureResponse> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "data": request.data,
            "algorithm": request.algorithm,
            "key_id": request.key_id,
        });
        
        let response = provider.call("security.sign", params)
            .await
            .map_err(|e| SecurityClientError::SignatureFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(SecurityClientError::Json)
    }
    
    /// Verify signature
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub async fn verify(&self, request: VerificationRequest) -> Result<VerificationResponse> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "data": request.data,
            "signature": request.signature,
            "key_id": request.key_id,
        });
        
        let response = provider.call("security.verify", params)
            .await
            .map_err(|e| SecurityClientError::VerificationFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(SecurityClientError::Json)
    }
    
    /// Validate token
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub async fn validate_token(&self, request: TokenValidationRequest) -> Result<TokenValidationResponse> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "token": request.token,
        });
        
        let response = provider.call("security.validate_token", params)
            .await
            .map_err(|e| SecurityClientError::ValidationFailed(e.to_string()))?;
        
        serde_json::from_value(response)
            .map_err(SecurityClientError::Json)
    }
    
    /// Generate new key
    ///
    /// # Errors
    ///
    /// Returns error if key generation fails
    pub async fn generate_key(&self, algorithm: String) -> Result<String> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "algorithm": algorithm,
        });
        
        let response = provider.call("security.generate_key", params)
            .await
            .map_err(|e| SecurityClientError::KeyManagementFailed(e.to_string()))?;
        
        let key_id = response["key_id"]
            .as_str()
            .ok_or_else(|| SecurityClientError::Capability(
                CapabilityError::InvalidResponse("No key_id in response".into())
            ))?;
        
        Ok(key_id.to_string())
    }
    
    /// Delete key
    ///
    /// # Errors
    ///
    /// Returns error if key deletion fails
    pub async fn delete_key(&self, key_id: &str) -> Result<()> {
        let provider = self.get_provider().await?;
        
        let params = json!({
            "key_id": key_id,
        });
        
        provider.call("security.delete_key", params)
            .await
            .map_err(|e| SecurityClientError::KeyManagementFailed(e.to_string()))?;
        
        Ok(())
    }
    
    /// List available keys
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_keys(&self) -> Result<Vec<String>> {
        let provider = self.get_provider().await?;
        
        let response = provider.call("security.list_keys", json!({}))
            .await
            .map_err(|e| SecurityClientError::KeyManagementFailed(e.to_string()))?;
        
        let keys = response["keys"]
            .as_array()
            .ok_or_else(|| SecurityClientError::Capability(
                CapabilityError::InvalidResponse("No keys array in response".into())
            ))?;
        
        Ok(keys.iter()
            .filter_map(|k| k.as_str().map(String::from))
            .collect::<Vec<_>>())
    }
    
    /// Check if security provider is available
    pub async fn is_available(&self) -> bool {
        self.get_provider().await.is_ok()
    }
    
    /// Get provider info (for debugging only!)
    pub async fn provider_info(&self) -> Option<String> {
        let provider_lock = self.provider.read().await;
        provider_lock.as_ref().map(|p| p.service_name().to_string())
    }
    
    /// Force rediscovery of provider
    ///
    /// Use this if you suspect the provider has changed or is no longer available
    pub async fn rediscover(&self) -> Result<()> {
        let mut provider_lock = self.provider.write().await;
        *provider_lock = None;
        drop(provider_lock);
        
        // Trigger discovery
        self.get_provider().await?;
        Ok(())
    }
}

impl Default for SecurityClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_security_client_creation() {
        let client = SecurityClient::new();
        let provider_lock = client.provider.read().await;
        assert!(provider_lock.is_none());
    }
    
    #[test]
    fn test_error_messages() {
        let err = SecurityClientError::NoProvider;
        assert!(err.to_string().contains("No security provider found"));
        
        let err = SecurityClientError::EncryptionFailed("test error".into());
        assert!(err.to_string().contains("test error"));
    }
    
    #[tokio::test]
    async fn test_rediscover() {
        let client = SecurityClient::new();
        // Should fail since no Songbird is running
        let result = client.rediscover().await;
        assert!(result.is_err());
    }
}
