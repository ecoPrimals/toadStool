// SPDX-License-Identifier: AGPL-3.0-only
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

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use toadstool_common::capability_provider::{CapabilityError, CapabilityProvider};
use toadstool_common::primal_identity::Capability;
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

            let discovered =
                CapabilityProvider::discover(capability)
                    .await
                    .map_err(|e| match e {
                        CapabilityError::NoProviderFound(_) => SecurityClientError::NoProvider,
                        other => SecurityClientError::Capability(other),
                    })?;

            *provider_lock = Some(discovered);
        }

        provider_lock
            .as_ref()
            .cloned()
            .ok_or(SecurityClientError::NoProvider)
    }

    /// Encrypt data
    ///
    /// # Deep Debt Evolution
    ///
    /// Before: `call_rpc("/primal/beardog", "crypto.encrypt", ...)`
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

        let response = provider
            .call("security.encrypt", params)
            .await
            .map_err(|e| SecurityClientError::EncryptionFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(SecurityClientError::Json)
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

        let response = provider
            .call("security.decrypt", params)
            .await
            .map_err(|e| SecurityClientError::DecryptionFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(SecurityClientError::Json)
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

        let response = provider
            .call("security.sign", params)
            .await
            .map_err(|e| SecurityClientError::SignatureFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(SecurityClientError::Json)
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

        let response = provider
            .call("security.verify", params)
            .await
            .map_err(|e| SecurityClientError::VerificationFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(SecurityClientError::Json)
    }

    /// Validate token
    ///
    /// # Errors
    ///
    /// Returns error if validation fails
    pub async fn validate_token(
        &self,
        request: TokenValidationRequest,
    ) -> Result<TokenValidationResponse> {
        let provider = self.get_provider().await?;

        let params = json!({
            "token": request.token,
        });

        let response = provider
            .call("security.validate_token", params)
            .await
            .map_err(|e| SecurityClientError::ValidationFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(SecurityClientError::Json)
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

        let response = provider
            .call("security.generate_key", params)
            .await
            .map_err(|e| SecurityClientError::KeyManagementFailed(e.to_string()))?;

        let key_id = response["key_id"].as_str().ok_or_else(|| {
            SecurityClientError::Capability(CapabilityError::InvalidResponse(
                "No key_id in response".into(),
            ))
        })?;

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

        provider
            .call("security.delete_key", params)
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

        let response = provider
            .call("security.list_keys", json!({}))
            .await
            .map_err(|e| SecurityClientError::KeyManagementFailed(e.to_string()))?;

        let keys = response["keys"].as_array().ok_or_else(|| {
            SecurityClientError::Capability(CapabilityError::InvalidResponse(
                "No keys array in response".into(),
            ))
        })?;

        Ok(keys
            .iter()
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

    #[test]
    fn test_security_client_default() {
        let client = SecurityClient::default();
        assert!(std::mem::size_of_val(&client) > 0);
    }

    #[tokio::test]
    async fn test_provider_info_none_when_no_provider() {
        let client = SecurityClient::new();
        let info = client.provider_info().await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_is_available_fails_without_provider() {
        let client = SecurityClient::new();
        let available = client.is_available().await;
        assert!(!available);
    }

    #[test]
    fn test_encryption_request_construction() {
        let req = EncryptionRequest {
            data: vec![1, 2, 3],
            algorithm: "AES-256-GCM".to_string(),
            key_id: Some("key-1".to_string()),
        };
        assert_eq!(req.data.len(), 3);
        assert_eq!(req.algorithm, "AES-256-GCM");
        assert_eq!(req.key_id.as_deref(), Some("key-1"));
    }

    #[test]
    fn test_decryption_request_construction() {
        let req = DecryptionRequest {
            encrypted_data: vec![0xaa, 0xbb],
            key_id: "key-xyz".to_string(),
        };
        assert_eq!(req.encrypted_data.len(), 2);
        assert_eq!(req.key_id, "key-xyz");
    }

    #[test]
    fn test_signature_request_construction() {
        let req = SignatureRequest {
            data: vec![1, 2, 3, 4, 5],
            algorithm: "ECDSA".to_string(),
            key_id: None,
        };
        assert_eq!(req.data.len(), 5);
        assert_eq!(req.algorithm, "ECDSA");
    }

    #[test]
    fn test_verification_request_construction() {
        let req = VerificationRequest {
            data: vec![1, 2, 3],
            signature: vec![0x11, 0x22],
            key_id: "sig-key".to_string(),
        };
        assert_eq!(req.signature.len(), 2);
        assert_eq!(req.key_id, "sig-key");
    }

    #[test]
    fn test_token_validation_request_construction() {
        let req = TokenValidationRequest {
            token: "jwt-token-123".to_string(),
        };
        assert_eq!(req.token, "jwt-token-123");
    }

    #[test]
    fn test_encryption_response_serde() {
        let resp = EncryptionResponse {
            encrypted_data: vec![0xde, 0xad, 0xbe, 0xef],
            key_id: "k1".to_string(),
            algorithm: "AES".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key_id, resp.key_id);
        assert_eq!(parsed.encrypted_data, resp.encrypted_data);
    }

    #[test]
    fn test_decryption_response_serde() {
        let resp = DecryptionResponse {
            data: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: DecryptionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, resp.data);
    }

    #[test]
    fn test_verification_response_construction() {
        let resp = VerificationResponse {
            valid: true,
            reason: None,
        };
        assert!(resp.valid);
        assert!(resp.reason.is_none());
    }

    #[test]
    fn test_token_validation_response_construction() {
        let resp = TokenValidationResponse {
            valid: true,
            user_id: Some("user-42".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
            expires_at: Some(1234567890),
        };
        assert!(resp.valid);
        assert_eq!(resp.user_id.as_deref(), Some("user-42"));
        assert_eq!(resp.scopes.len(), 2);
    }

    #[test]
    fn test_security_client_error_all_variants() {
        let _ = SecurityClientError::NoProvider;
        let _ = SecurityClientError::EncryptionFailed("e".into());
        let _ = SecurityClientError::DecryptionFailed("e".into());
        let _ = SecurityClientError::SignatureFailed("e".into());
        let _ = SecurityClientError::VerificationFailed("e".into());
        let _ = SecurityClientError::KeyManagementFailed("e".into());
        let _ = SecurityClientError::ValidationFailed("e".into());
        let err = SecurityClientError::Json(
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err(),
        );
        assert!(err.to_string().contains("expected"));
    }

    #[tokio::test]
    async fn test_encrypt_fails_without_provider() {
        let client = SecurityClient::new();
        let req = EncryptionRequest {
            data: vec![1, 2, 3],
            algorithm: "AES-256-GCM".to_string(),
            key_id: None,
        };
        let result = client.encrypt(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_fails_without_provider() {
        let client = SecurityClient::new();
        let req = DecryptionRequest {
            encrypted_data: vec![0xaa, 0xbb],
            key_id: "key-1".to_string(),
        };
        let result = client.decrypt(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sign_fails_without_provider() {
        let client = SecurityClient::new();
        let req = SignatureRequest {
            data: vec![1, 2, 3],
            algorithm: "ECDSA".to_string(),
            key_id: None,
        };
        let result = client.sign(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_fails_without_provider() {
        let client = SecurityClient::new();
        let req = VerificationRequest {
            data: vec![1, 2, 3],
            signature: vec![0x11, 0x22],
            key_id: "key-1".to_string(),
        };
        let result = client.verify(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_token_fails_without_provider() {
        let client = SecurityClient::new();
        let req = TokenValidationRequest {
            token: "jwt-token".to_string(),
        };
        let result = client.validate_token(req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_key_fails_without_provider() {
        let client = SecurityClient::new();
        let result = client.generate_key("AES-256".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_key_fails_without_provider() {
        let client = SecurityClient::new();
        let result = client.delete_key("key-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_keys_fails_without_provider() {
        let client = SecurityClient::new();
        let result = client.list_keys().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_response_serde() {
        let resp = SignatureResponse {
            signature: vec![0xde, 0xad],
            key_id: "k1".to_string(),
            algorithm: "ECDSA".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SignatureResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key_id, resp.key_id);
    }

    #[test]
    fn test_decryption_request_serde() {
        let req = DecryptionRequest {
            encrypted_data: vec![1, 2, 3],
            key_id: "key".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: DecryptionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key_id, req.key_id);
    }

    #[test]
    fn test_verification_request_serde() {
        let req = VerificationRequest {
            data: vec![1],
            signature: vec![2],
            key_id: "k".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: VerificationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key_id, req.key_id);
    }

    #[test]
    fn test_capability_error_conversion() {
        use toadstool_common::capability_provider::CapabilityError;
        use toadstool_common::primal_identity::{Capability, CryptoCapability};
        let cap_err =
            CapabilityError::NoProviderFound(Capability::Crypto(CryptoCapability::Encryption));
        let sec_err: SecurityClientError = cap_err.into();
        assert!(matches!(
            sec_err,
            SecurityClientError::Capability(CapabilityError::NoProviderFound(_))
        ));
    }
}
