// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security Service Client - Capability-based discovery (Evolved)
//!
//! **DEEP DEBT EVOLUTION**: This is the evolved version of the Security client.
//! It discovers security providers by capability, not by hardcoded "security" name.
//!
//! **Design Philosophy**:
//! - **Pure Rust**: Unix sockets, no HTTP/TLS
//! - **Async-first**: Non-blocking operations with tokio
//! - **Local IPC**: Fast, secure primal-to-primal communication
//! - **Zero hardcoding**: Discovers "who provides security?" not "where is security?"
//! - **Capability-based**: Works with ANY security provider (security, vault, etc.)

pub mod errors;
pub mod protocol;

pub use errors::*;
pub use protocol::*;

use serde::{Deserialize, Serialize};
#[cfg(unix)]
use serde_json::json;
use std::sync::Arc;
#[cfg(unix)]
use toadstool_common::capability_provider::CapabilityError;
use toadstool_common::capability_provider::CapabilityProvider;
#[cfg(unix)]
use toadstool_common::primal_identity::Capability;
use tokio::sync::RwLock;

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
/// Before: `SecurityClient` hardcoded to "/primal/security"
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
        #[cfg(not(unix))]
        {
            return Err(SecurityClientError::NoProvider);
        }
        #[cfg(unix)]
        {
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
    }

    /// Call a security RPC method with typed request/response.
    async fn rpc<Req, Resp>(
        &self,
        method: &str,
        request: &Req,
        map_err: fn(String) -> SecurityClientError,
    ) -> Result<Resp>
    where
        Req: Serialize + Sync,
        Resp: for<'de> Deserialize<'de>,
    {
        #[cfg(not(unix))]
        {
            let _ = (method, request, map_err);
            return Err(SecurityClientError::NoProvider);
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(request).map_err(SecurityClientError::Json)?;
            let provider = self.get_provider().await?;
            let response = provider
                .call(method, params)
                .await
                .map_err(|e| map_err(e.to_string()))?;
            serde_json::from_value(response).map_err(SecurityClientError::Json)
        }
    }

    /// Encrypt data
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable or encryption fails
    pub async fn encrypt(&self, request: EncryptionRequest) -> Result<EncryptionResponse> {
        self.rpc(
            "security.encrypt",
            &request,
            SecurityClientError::EncryptionFailed,
        )
        .await
    }

    /// Decrypt data
    ///
    /// # Errors
    ///
    /// Returns error if decryption fails
    pub async fn decrypt(&self, request: DecryptionRequest) -> Result<DecryptionResponse> {
        self.rpc(
            "security.decrypt",
            &request,
            SecurityClientError::DecryptionFailed,
        )
        .await
    }

    /// Sign data
    ///
    /// # Errors
    ///
    /// Returns error if signing fails
    pub async fn sign(&self, request: SignatureRequest) -> Result<SignatureResponse> {
        self.rpc(
            "security.sign",
            &request,
            SecurityClientError::SignatureFailed,
        )
        .await
    }

    /// Verify signature
    ///
    /// # Errors
    ///
    /// Returns error if verification fails
    pub async fn verify(&self, request: VerificationRequest) -> Result<VerificationResponse> {
        self.rpc(
            "security.verify",
            &request,
            SecurityClientError::VerificationFailed,
        )
        .await
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
        self.rpc(
            "security.validate_token",
            &request,
            SecurityClientError::ValidationFailed,
        )
        .await
    }

    /// Generate new key
    ///
    /// # Errors
    ///
    /// Returns error if key generation fails
    pub async fn generate_key(&self, algorithm: String) -> Result<String> {
        #[cfg(not(unix))]
        {
            let _ = algorithm;
            return Err(SecurityClientError::NoProvider);
        }
        #[cfg(unix)]
        {
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
    }

    /// Delete key
    ///
    /// # Errors
    ///
    /// Returns error if key deletion fails
    pub async fn delete_key(&self, key_id: &str) -> Result<()> {
        #[cfg(not(unix))]
        {
            let _ = key_id;
            return Err(SecurityClientError::NoProvider);
        }
        #[cfg(unix)]
        {
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
    }

    /// List available keys
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_keys(&self) -> Result<Vec<String>> {
        #[cfg(not(unix))]
        {
            return Err(SecurityClientError::NoProvider);
        }
        #[cfg(unix)]
        {
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
    /// Same as [`SecurityClient::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod client_evolved_tests;
