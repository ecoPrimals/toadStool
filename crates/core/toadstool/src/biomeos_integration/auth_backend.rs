//! Authentication backend traits and implementations for BiomeOS/BearDog integration
//!
//! This module defines the trait interface for authentication backends and provides
//! production and test implementations using proper dependency injection.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{ToadStoolError, ToadStoolResult};

// Re-export types from auth module
pub use super::auth::{AuthenticationToken, TokenRefreshRequest, TokenRequest};

/// Trait defining the interface for authentication backends
///
/// This allows dependency injection of different authentication implementations
/// (production BearDog backend, in-memory test backend, etc.) without relying
/// on feature flags or conditional compilation.
#[async_trait]
pub trait AuthBackend: Send + Sync {
    /// Initialize/test connection to authentication backend
    ///
    /// For network backends (BearDog), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    async fn initialize(&self) -> ToadStoolResult<()> {
        Ok(()) // Default implementation is no-op
    }

    /// Request a new authentication token
    async fn request_token(&self, request: &TokenRequest) -> ToadStoolResult<AuthenticationToken>;

    /// Refresh an authentication token
    async fn refresh_token(
        &self,
        request: &TokenRefreshRequest,
    ) -> ToadStoolResult<AuthenticationToken>;

    /// Validate a token (optional, default implementation)
    fn validate_token(&self, token: &AuthenticationToken) -> ToadStoolResult<()> {
        // Check expiration
        if token.expires_at <= chrono::Utc::now() {
            return Err(ToadStoolError::runtime(
                "Token is already expired".to_string(),
            ));
        }

        // Check issuer
        if token.issuer != "beardog" {
            return Err(ToadStoolError::runtime(format!(
                "Invalid token issuer: {}",
                token.issuer
            )));
        }

        // Check token type
        if token.token_type != "Bearer" && token.token_type != "Ed25519" {
            return Err(ToadStoolError::runtime(format!(
                "Unsupported token type: {}",
                token.token_type
            )));
        }

        Ok(())
    }
}

/// Production implementation using BearDog Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct BearDogBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

impl BearDogBackend {
    /// Create crypto auth backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers crypto service by capability, not name.
    /// Works with ANY service providing crypto.authentication capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    pub async fn new_async() -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY crypto service (not hardcoded "beardog")
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .map_err(|e| ToadStoolError::configuration(format!(
                "No crypto service discovered: {}. Ensure a crypto provider is running.",
                e
            )))?;

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        })
    }

    /// Create a new crypto authentication backend with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    #[must_use]
    #[deprecated(since = "0.3.0", note = "Use new_async() for capability-based discovery")]
    #[allow(deprecated)]
    pub fn new(_endpoint: impl Into<String>) -> Self {
        // LEGACY: Uses primal name for backward compatibility
        let socket_path = toadstool_common::primal_sockets::get_socket_path_for_service("beardog");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        }
    }
}

#[async_trait]
impl AuthBackend for BearDogBackend {
    async fn initialize(&self) -> ToadStoolResult<()> {
        // Health check via JSON-RPC over unix socket
        let _health: serde_json::Value = self
            .rpc_client
            .call("beardog.health", serde_json::json!({}))
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to connect to BearDog: {e}")))?;

        tracing::info!("Successfully connected to BearDog via unix socket");
        Ok(())
    }

    async fn request_token(&self, request: &TokenRequest) -> ToadStoolResult<AuthenticationToken> {
        let params = serde_json::to_value(request)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize request: {e}")))?;

        let token: AuthenticationToken = self
            .rpc_client
            .call_typed("beardog.request_token", params)
            .await
            .map_err(|e| {
                ToadStoolError::runtime(format!("Failed to request token from BearDog: {e}"))
            })?;

        // Validate token
        self.validate_token(&token)?;

        Ok(token)
    }

    async fn refresh_token(
        &self,
        request: &TokenRefreshRequest,
    ) -> ToadStoolResult<AuthenticationToken> {
        let params = serde_json::to_value(request)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize request: {e}")))?;

        let token: AuthenticationToken = self
            .rpc_client
            .call_typed("beardog.refresh_token", params)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to refresh token: {e}")))?;

        Ok(token)
    }
}

/// In-memory test backend for testing without external dependencies
///
/// This is a proper test implementation, not a mock. It generates valid
/// tokens for testing purposes without requiring a real BearDog service.
pub struct InMemoryAuthBackend {
    tokens: Arc<Mutex<HashMap<String, AuthenticationToken>>>,
}

impl InMemoryAuthBackend {
    /// Create a new in-memory authentication backend for testing
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a test token
    fn generate_test_token(&self, requesting_primal: &str) -> AuthenticationToken {
        let token_id = format!("test-token-{}", requesting_primal);
        AuthenticationToken {
            id: token_id,
            token_type: "Bearer".to_string(),
            token: format!("test-token-value-{}", requesting_primal),
            public_key: "test-public-key".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            issued_at: chrono::Utc::now(),
            issuer: "beardog".to_string(),
            audience: vec![
                "songbird".to_string(),
                "nestgate".to_string(),
                "squirrel".to_string(),
            ],
            scope: vec!["cross-primal".to_string(), "propagation".to_string()],
            claims: HashMap::new(),
        }
    }
}

impl Default for InMemoryAuthBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthBackend for InMemoryAuthBackend {
    async fn request_token(&self, request: &TokenRequest) -> ToadStoolResult<AuthenticationToken> {
        let token = self.generate_test_token(&request.requesting_primal);

        // Store token for potential refresh
        let mut tokens = self.tokens.lock().await;
        tokens.insert(token.id.clone(), token.clone());

        tracing::debug!("Generated test token for {}", request.requesting_primal);
        Ok(token)
    }

    async fn refresh_token(
        &self,
        request: &TokenRefreshRequest,
    ) -> ToadStoolResult<AuthenticationToken> {
        // Generate a refreshed token
        let token = AuthenticationToken {
            id: format!("test-refreshed-token-{}", request.requesting_primal),
            token_type: "Bearer".to_string(),
            token: format!("test-refreshed-value-{}", request.requesting_primal),
            public_key: "test-public-key".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            issued_at: chrono::Utc::now(),
            issuer: "beardog".to_string(),
            audience: vec!["songbird".to_string(), "nestgate".to_string()],
            scope: vec!["cross-primal".to_string()],
            claims: HashMap::new(),
        };

        // Store refreshed token
        let mut tokens = self.tokens.lock().await;
        tokens.insert(token.id.clone(), token.clone());

        tracing::debug!("Refreshed test token for {}", request.requesting_primal);
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_auth_backend_request() {
        let backend = InMemoryAuthBackend::new();
        let request = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["cross-primal".to_string()],
            audience: vec!["songbird".to_string()],
            timestamp: chrono::Utc::now(),
        };

        let result = backend.request_token(&request).await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.issuer, "beardog");
        assert!(token.expires_at > chrono::Utc::now());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_auth_backend_refresh() {
        let backend = InMemoryAuthBackend::new();
        let request = TokenRefreshRequest {
            requesting_primal: "toadstool".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let result = backend.refresh_token(&request).await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(token.id.contains("refreshed"));
        assert_eq!(token.issuer, "beardog");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_token_validation() {
        let backend = InMemoryAuthBackend::new();
        let request = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["cross-primal".to_string()],
            audience: vec!["songbird".to_string()],
            timestamp: chrono::Utc::now(),
        };

        let token = backend.request_token(&request).await.unwrap();
        let validation_result = backend.validate_token(&token);
        assert!(validation_result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_expired_token_validation() {
        let backend = InMemoryAuthBackend::new();
        let mut token = AuthenticationToken {
            id: "test-expired".to_string(),
            token_type: "Bearer".to_string(),
            token: "test-value".to_string(),
            public_key: "test-key".to_string(),
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1), // Expired!
            issued_at: chrono::Utc::now() - chrono::Duration::hours(2),
            issuer: "beardog".to_string(),
            audience: vec!["test".to_string()],
            scope: vec!["test".to_string()],
            claims: HashMap::new(),
        };

        let result = backend.validate_token(&token);
        assert!(result.is_err());

        // Fix expiration
        token.expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = backend.validate_token(&token);
        assert!(result.is_ok());
    }
}
