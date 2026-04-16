// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentication backend traits and implementations for BiomeOS / security-service integration
//!
//! This module defines the trait interface for authentication backends and provides
//! production and test implementations using proper dependency injection.
//!
//! **Connection hints**: prefer capability-based discovery (`SecurityBackend::new_async`).
//! Environment fallbacks are centralized in `toadstool_common::primal_sockets::SocketPathEnv`
//! (`security_connection_hint` and `resolve_capability_socket_fallback`), not direct `BEARDOG_*`
//! env lookups — see `auth::AuthManager::discover`.

#![expect(
    deprecated,
    reason = "IPC addressing requires well-known legacy names during migration"
)]

#[cfg(any(test, feature = "test-mocks"))]
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
#[cfg(any(test, feature = "test-mocks"))]
use std::sync::Arc;
#[cfg(any(test, feature = "test-mocks"))]
use std::time::Duration;
use std::time::SystemTime;
#[cfg(any(test, feature = "test-mocks"))]
use tokio::sync::Mutex;

use toadstool_common::constants::ecosystem::well_known;
use toadstool_common::constants::primal_identity::{PRIMAL_NAME, audience};

use crate::{ToadStoolError, ToadStoolResult};

// Re-export types from auth module
pub use super::auth::{AuthenticationToken, TokenRefreshRequest, TokenRequest};

/// Trait defining the interface for authentication backends
///
/// This allows dependency injection of different authentication implementations
/// (production security-service backend, in-memory test backend when `cfg(test)` or the
/// `test-mocks` crate feature is enabled).
// NOTE(async-dyn): async methods return `Pin<Box<dyn Future>>` — native async fn in trait is not dyn-compatible
pub trait AuthBackend: Send + Sync {
    /// Initialize/test connection to authentication backend
    ///
    /// For network backends (security service), this tests connectivity.
    /// For local backends (in-memory), this is typically a no-op.
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    /// Request a new authentication token
    fn request_token<'a>(
        &'a self,
        request: &'a TokenRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>>;

    /// Refresh an authentication token
    fn refresh_token<'a>(
        &'a self,
        request: &'a TokenRefreshRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>>;

    /// Sign a payload and return a signature string.
    ///
    /// Production backends delegate to the security / crypto provider via JSON-RPC (`crypto.sign`).
    /// The in-memory backend returns mock signatures for testing.
    ///
    /// # Errors
    ///
    /// Returns an error if no signing capability is available.
    fn sign_payload<'a>(
        &'a self,
        _payload: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>> {
        Box::pin(async move {
            Err(ToadStoolError::configuration(
                "Signing not available. Ensure a crypto / security provider is running.",
            ))
        })
    }

    /// Export the public key for signature verification, if available.
    ///
    /// Returns `None` when no signing key is configured or the backend
    /// does not support local key export.
    fn public_key(&self) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
        Box::pin(async { None })
    }

    /// Validate a token (optional, default implementation)
    ///
    /// # Errors
    ///
    /// Returns an error if the token is expired, has invalid issuer/audience, or unsupported type.
    fn validate_token(&self, token: &AuthenticationToken) -> ToadStoolResult<()> {
        // Check expiration
        if token.expires_at <= SystemTime::now() {
            return Err(ToadStoolError::runtime(
                "Token is already expired".to_string(),
            ));
        }

        // Check issuer (security / crypto service; well-known integration id)
        if token.issuer != well_known::BEARDOG {
            return Err(ToadStoolError::runtime(format!(
                "Invalid token issuer: {}",
                token.issuer
            )));
        }

        // Accept tokens with audience matching self or platform
        let acceptable = token
            .audience
            .iter()
            .any(|a| a == PRIMAL_NAME || a == audience::PLATFORM_AUDIENCE);
        if !acceptable {
            return Err(ToadStoolError::runtime(format!(
                "Token audience {:?} does not include {} or {}",
                token.audience,
                PRIMAL_NAME,
                audience::PLATFORM_AUDIENCE
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

/// Production implementation using the security / crypto service Unix socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct SecurityBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

/// Legacy alias for [`SecurityBackend`].
#[deprecated(since = "0.3.0", note = "Use SecurityBackend directly")]
pub type BearDogBackend = SecurityBackend;

impl SecurityBackend {
    /// Create crypto auth backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers crypto service by capability, not name.
    /// Works with ANY service providing crypto.authentication capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no crypto service can be found.
    pub async fn new_async() -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY crypto service by capability
        let socket_path = toadstool_common::primal_sockets::discover_crypto_socket()
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "No crypto service discovered: {e}. Ensure a crypto provider is running."
                ))
            })?;

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
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    pub fn new(_endpoint: impl Into<String>) -> Self {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("crypto");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        }
    }
}

impl AuthBackend for SecurityBackend {
    fn sign_payload<'a>(
        &'a self,
        payload: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>> {
        Box::pin(async move {
            let params = serde_json::json!({ "payload": payload });
            self.rpc_client
                .call_typed::<String>("crypto.sign", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("security/crypto service sign failed: {e}"))
                })
        })
    }

    fn public_key(&self) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
        Box::pin(async move {
            let value: serde_json::Value = self
                .rpc_client
                .call("crypto.public_key", serde_json::json!({}))
                .await
                .ok()?;
            value
                .get("public_key")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
    }

    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            // Health check via JSON-RPC over unix socket
            let _health: serde_json::Value = self
                .rpc_client
                .call("crypto.health", serde_json::json!({}))
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to connect to security/crypto service: {e}"
                    ))
                })?;

            tracing::info!("Successfully connected to security/crypto service via unix socket");
            Ok(())
        })
    }

    fn request_token<'a>(
        &'a self,
        request: &'a TokenRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>> {
        Box::pin(async move {
            let params = serde_json::to_value(request).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to serialize request: {e}"))
            })?;

            let token: AuthenticationToken = self
                .rpc_client
                .call_typed("crypto.request_token", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to request token from security/crypto service: {e}"
                    ))
                })?;

            // Validate token
            self.validate_token(&token)?;

            Ok(token)
        })
    }

    fn refresh_token<'a>(
        &'a self,
        request: &'a TokenRefreshRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>> {
        Box::pin(async move {
            let params = serde_json::to_value(request).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to serialize request: {e}"))
            })?;

            let token: AuthenticationToken = self
                .rpc_client
                .call_typed("crypto.refresh_token", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to refresh token: {e}")))?;

            Ok(token)
        })
    }
}

/// In-memory test backend for testing without external dependencies
///
/// This is a proper test implementation, not a mock. It generates valid
/// tokens for testing purposes without requiring a real security service.
#[cfg(any(test, feature = "test-mocks"))]
pub struct InMemoryAuthBackend {
    tokens: Arc<Mutex<HashMap<String, AuthenticationToken>>>,
}

#[cfg(any(test, feature = "test-mocks"))]
impl InMemoryAuthBackend {
    /// Create a new in-memory authentication backend for testing
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a test token
    fn generate_test_token(requesting_primal: &str) -> AuthenticationToken {
        let token_id = format!("test-token-{requesting_primal}");
        AuthenticationToken {
            id: token_id,
            token_type: "Bearer".to_string(),
            token: format!("test-token-value-{requesting_primal}"),
            public_key: "test-public-key".to_string(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            issued_at: SystemTime::now(),
            issuer: well_known::BEARDOG.to_string(),
            audience: vec![
                PRIMAL_NAME.to_string(),
                audience::PLATFORM_AUDIENCE.to_string(),
            ],
            scope: vec!["cross-primal".to_string(), "propagation".to_string()],
            claims: HashMap::new(),
        }
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl Default for InMemoryAuthBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl AuthBackend for InMemoryAuthBackend {
    fn sign_payload<'a>(
        &'a self,
        payload: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>> {
        Box::pin(async move {
            use base64::{Engine as _, engine::general_purpose};
            tracing::warn!("INSECURE: In-memory mock signature. Acceptable ONLY in tests.");
            Ok(format!(
                "ed25519:mock:{}",
                general_purpose::STANDARD.encode(payload.as_bytes())
            ))
        })
    }

    fn public_key(&self) -> Pin<Box<dyn Future<Output = Option<String>> + Send + '_>> {
        Box::pin(async move { Some("test-public-key".to_string()) })
    }

    fn request_token<'a>(
        &'a self,
        request: &'a TokenRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>> {
        Box::pin(async move {
            let token = Self::generate_test_token(&request.requesting_primal);

            // Store token for potential refresh
            self.tokens
                .lock()
                .await
                .insert(token.id.clone(), token.clone());

            tracing::debug!("Generated test token for {}", request.requesting_primal);
            Ok(token)
        })
    }

    fn refresh_token<'a>(
        &'a self,
        request: &'a TokenRefreshRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<AuthenticationToken>> + Send + 'a>> {
        Box::pin(async move {
            // Generate a refreshed token
            let token = AuthenticationToken {
                id: format!("test-refreshed-token-{}", request.requesting_primal),
                token_type: "Bearer".to_string(),
                token: format!("test-refreshed-value-{}", request.requesting_primal),
                public_key: "test-public-key".to_string(),
                expires_at: SystemTime::now() + Duration::from_secs(3600),
                issued_at: SystemTime::now(),
                issuer: well_known::BEARDOG.to_string(),
                audience: vec![
                    PRIMAL_NAME.to_string(),
                    audience::PLATFORM_AUDIENCE.to_string(),
                ],
                scope: vec!["cross-primal".to_string()],
                claims: HashMap::new(),
            };

            // Store refreshed token
            self.tokens
                .lock()
                .await
                .insert(token.id.clone(), token.clone());

            tracing::debug!("Refreshed test token for {}", request.requesting_primal);
            Ok(token)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_auth_backend_request() {
        use toadstool_common::interned_strings::capabilities;
        let backend = InMemoryAuthBackend::new();
        let request = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["cross-primal".to_string()],
            audience: vec![capabilities::COORDINATION.to_string()],
            timestamp: SystemTime::now(),
        };

        let result = backend.request_token(&request).await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.issuer, well_known::BEARDOG);
        assert!(token.expires_at > SystemTime::now());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_auth_backend_refresh() {
        let backend = InMemoryAuthBackend::new();
        let request = TokenRefreshRequest {
            requesting_primal: "toadstool".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = backend.refresh_token(&request).await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(token.id.contains("refreshed"));
        assert_eq!(token.issuer, well_known::BEARDOG);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_token_validation() {
        use toadstool_common::interned_strings::capabilities;
        let backend = InMemoryAuthBackend::new();
        let request = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["cross-primal".to_string()],
            audience: vec![capabilities::COORDINATION.to_string()],
            timestamp: SystemTime::now(),
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
            expires_at: SystemTime::now() - Duration::from_secs(3600), // Expired!
            issued_at: SystemTime::now() - Duration::from_secs(7200),
            issuer: well_known::BEARDOG.to_string(),
            audience: vec![PRIMAL_NAME.to_string()],
            scope: vec!["test".to_string()],
            claims: HashMap::new(),
        };

        let result = backend.validate_token(&token);
        assert!(result.is_err());

        // Fix expiration
        token.expires_at = SystemTime::now() + Duration::from_secs(3600);
        let result = backend.validate_token(&token);
        assert!(result.is_ok());
    }
}
