// auth_backend_evolved.rs - Capability-based authentication backend
//
// DEEP DEBT EVOLUTION: This backend discovers security providers by capability
// at runtime, not by hardcoded primal names. It doesn't know or care if the
// provider is "beardog" - it just asks for "Who can manage tokens?"
//
// Migration from: auth_backend.rs (hardcoded "beardog")
// Evolution: Capability-based discovery, proper error handling

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use toadstool_common::capability_provider::{CapabilityError, CapabilityProvider};
use toadstool_common::primal_identity::Capability;
use tokio::sync::RwLock;

/// Errors for authentication backend
#[derive(Debug, thiserror::Error)]
pub enum AuthBackendError {
    #[error("Security provider not found")]
    NoSecurityProvider,

    #[error("Token request failed: {0}")]
    TokenRequestFailed(String),

    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AuthBackendError>;

/// Token structure (compatible with JWT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub issuer: String,
    pub audience: Vec<String>,
}

/// Token request parameters
#[derive(Debug, Serialize)]
pub struct TokenRequest {
    pub subject: String,
    pub audience: Vec<String>,
    pub scopes: Vec<String>,
    pub expires_in: Option<u64>,
}

/// Authentication backend with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs token management
/// 2. **Runtime discovery**: Finds provider by capability, not name
/// 3. **Proper errors**: No unwrap(), all errors handled
/// 4. **Agnostic**: Doesn't care which primal provides tokens
pub struct AuthBackend {
    /// Security provider (discovered at runtime)
    provider: Arc<RwLock<Option<CapabilityProvider>>>,
}

impl AuthBackend {
    /// Create new auth backend
    ///
    /// Provider is discovered lazily on first use
    pub fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or discover security provider
    ///
    /// This discovers the provider by capability:
    /// "Who can manage tokens?" not "Connect to beardog"
    async fn get_provider(&self) -> Result<CapabilityProvider> {
        let mut provider_lock = self.provider.write().await;

        if provider_lock.is_none() {
            // Discover security provider by capability
            use toadstool_common::primal_identity::AuthCapability;
            let capability = Capability::Authentication(AuthCapability::TokenManagement);

            let discovered =
                CapabilityProvider::discover(capability)
                    .await
                    .map_err(|e| match e {
                        CapabilityError::NoProviderFound(_) => AuthBackendError::NoSecurityProvider,
                        other => AuthBackendError::Capability(other),
                    })?;

            *provider_lock = Some(discovered);
        }

        Ok(provider_lock.as_ref().unwrap().clone())
    }

    /// Request a new token
    ///
    /// # Deep Debt Evolution
    ///
    /// Before: `call_rpc("/primal/beardog", "beardog.request_token", ...)`
    /// After: `provider.call("security.request_token", ...)`
    ///
    /// No hardcoded primal names!
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable or token request fails
    pub async fn request_token(&self, request: TokenRequest) -> Result<Token> {
        let provider = self.get_provider().await?;

        let params = json!({
            "subject": request.subject,
            "audience": request.audience,
            "scopes": request.scopes,
            "expires_in": request.expires_in.unwrap_or(3600),
        });

        // Call using semantic method name (wateringHole standard)
        let response = provider
            .call("security.request_token", params)
            .await
            .map_err(|e| AuthBackendError::TokenRequestFailed(e.to_string()))?;

        // Parse response
        serde_json::from_value(response).map_err(AuthBackendError::Json)
    }

    /// Validate a token
    ///
    /// # Errors
    ///
    /// Returns error if token is invalid or provider unavailable
    pub async fn validate_token(&self, token: &str) -> Result<bool> {
        let provider = self.get_provider().await?;

        let params = json!({
            "token": token,
        });

        let response = provider
            .call("security.validate_token", params)
            .await
            .map_err(|e| AuthBackendError::ValidationFailed(e.to_string()))?;

        Ok(response["valid"].as_bool().unwrap_or(false))
    }

    /// Refresh a token
    ///
    /// # Errors
    ///
    /// Returns error if refresh fails or provider unavailable
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<Token> {
        let provider = self.get_provider().await?;

        let params = json!({
            "refresh_token": refresh_token,
        });

        let response = provider
            .call("security.refresh_token", params)
            .await
            .map_err(|e| AuthBackendError::RefreshFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(AuthBackendError::Json)
    }

    /// Check if security provider is available
    ///
    /// Useful for health checks
    pub async fn is_available(&self) -> bool {
        self.get_provider().await.is_ok()
    }

    /// Get provider info (for debugging/logging only!)
    ///
    /// WARNING: Do NOT use provider name for logic decisions!
    pub async fn provider_info(&self) -> Option<String> {
        let provider_lock = self.provider.read().await;
        provider_lock.as_ref().map(|p| p.service_name().to_string())
    }
}

impl Default for AuthBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_backend_creation() {
        let backend = AuthBackend::new();

        // Provider starts as None
        let provider_lock = backend.provider.read().await;
        assert!(provider_lock.is_none());
    }

    #[test]
    fn test_token_serialization() {
        let token = Token {
            token: "test-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            issuer: "test-issuer".to_string(),
            audience: vec!["test-audience".to_string()],
        };

        let json = serde_json::to_value(&token).unwrap();
        assert_eq!(json["token"], "test-token");
        assert_eq!(json["expires_in"], 3600);
    }

    #[test]
    fn test_error_types() {
        let err = AuthBackendError::NoSecurityProvider;
        assert!(err.to_string().contains("Security provider not found"));

        let err = AuthBackendError::TokenRequestFailed("test".into());
        assert!(err.to_string().contains("Token request failed"));
    }
}
