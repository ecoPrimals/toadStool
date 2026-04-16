// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentication management for cross-Primal token propagation

mod permissions;
mod tokens;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use toadstool_common::constants::primal_identity::{PRIMAL_NAME, audience};
use toadstool_common::constants::timeouts::{TIMESTAMP_VALIDATION_WINDOW, TOKEN_REFRESH_INTERVAL};

use super::auth_backend::{AuthBackend, AuthBackendDispatch};
use crate::ToadStoolResult;

pub use permissions::{
    PrimalTypeConfig, PropagationResult, TokenPropagationRequest, TokenPropagationStatus,
    VerificationResult,
};
pub use tokens::{
    AuthenticationToken, TokenRefreshRequest, TokenRequest, TokenVerificationRequest,
    TokenVerificationResponse, TokenVerificationStatus,
};

/// Configuration for authentication manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthManagerConfig {
    /// Crypto/security service endpoint (legacy; prefer capability discovery).
    #[serde(default, alias = "security_endpoint")]
    pub security_endpoint: String,
    /// Interval between token refresh attempts.
    pub token_refresh_interval: Duration,
    /// Whether to validate token signatures.
    pub signature_validation: bool,
    /// Allowed timestamp skew for token validation.
    pub timestamp_window: Duration,
    /// Whether to enforce replay protection.
    pub replay_protection: bool,
    /// Base64-encoded 32-byte Ed25519 signing key seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_key_seed: Option<String>,
    /// Primals this node is allowed to request tokens for.
    ///
    /// Defaults to the full peer set discovered from environment variable
    /// `TOADSTOOL_AUTH_AUDIENCE` (comma-separated) or the canonical
    /// cross-primal peer list. Override via config to narrow token scope.
    #[serde(default)]
    pub token_audience: Vec<String>,
}

impl Default for AuthManagerConfig {
    fn default() -> Self {
        Self {
            security_endpoint: String::new(),
            token_refresh_interval: TOKEN_REFRESH_INTERVAL,
            signature_validation: true,
            timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
            replay_protection: true,
            signing_key_seed: None,
            token_audience: default_token_audience(),
        }
    }
}

/// Resolve the token audience at runtime: env override → config defaults.
///
/// Reads `TOADSTOOL_AUTH_AUDIENCE` (comma-separated primal names).
/// Falls back to self + platform audience when the variable is absent.
/// ToadStool only knows itself; external peers are discovered at runtime.
fn default_token_audience() -> Vec<String> {
    if let Ok(val) = std::env::var("TOADSTOOL_AUTH_AUDIENCE") {
        let list: Vec<String> = val
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if !list.is_empty() {
            return list;
        }
    }
    // Self-knowledge only: tokens valid for this primal and the platform
    vec![
        PRIMAL_NAME.to_string(),
        audience::PLATFORM_AUDIENCE.to_string(),
    ]
}

/// Authentication manager for cross-Primal token propagation
pub struct AuthenticationManager {
    config: AuthManagerConfig,
    current_token: Option<AuthenticationToken>,
    backend: Arc<AuthBackendDispatch>,
    refresh_task: Option<tokio::task::JoinHandle<()>>,
}

impl AuthenticationManager {
    /// Creates a new auth manager with config and backend.
    #[must_use]
    pub fn new(config: AuthManagerConfig, backend: Arc<AuthBackendDispatch>) -> Self {
        Self {
            config,
            current_token: None,
            backend,
            refresh_task: None,
        }
    }

    /// Discover auth manager via capability lookup or env fallbacks.
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no crypto provider can be configured.
    pub async fn discover() -> crate::ToadStoolResult<Self> {
        Self::discover_with_config(AuthManagerConfig::default()).await
    }

    /// Discover auth manager with custom config via capability lookup or env fallbacks.
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no crypto provider can be configured.
    pub async fn discover_with_config(config: AuthManagerConfig) -> crate::ToadStoolResult<Self> {
        match Self::with_crypto_service(config.clone()).await {
            Ok(manager) => {
                tracing::info!("Discovered crypto service via capability-based discovery");
                return Ok(manager);
            }
            Err(e) => tracing::debug!("Capability discovery failed: {}, trying fallbacks", e),
        }

        let socket_env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
        if let Some(endpoint) = socket_env.security_connection_hint {
            tracing::info!("Discovered crypto service via environment: {}", endpoint);
            let mut config = config;
            config.security_endpoint = endpoint;
            #[expect(deprecated)]
            return Ok(Self::with_security(config));
        }

        if !config.security_endpoint.is_empty() {
            #[expect(deprecated)]
            return Ok(Self::with_security(config));
        }

        Err(crate::ToadStoolError::configuration(
            "No crypto provider discovered. Set TOADSTOOL_SECURITY_ENDPOINT or SECURITY_ENDPOINT, \
             configure security_endpoint in the auth manager config, or ensure a crypto/security \
             service is running.",
        ))
    }

    /// # Errors
    ///
    /// Returns an error if crypto service discovery fails or the backend cannot be initialized.
    pub async fn with_crypto_service(config: AuthManagerConfig) -> ToadStoolResult<Self> {
        let backend = super::auth_backend::SecurityBackend::new_async().await?;
        Ok(Self {
            config,
            current_token: None,
            backend: Arc::new(AuthBackendDispatch::Security(backend)),
            refresh_task: None,
        })
    }

    /// Creates auth manager with legacy security backend (deprecated).
    #[must_use]
    #[deprecated(since = "0.3.0", note = "Use with_crypto_service() or discover()")]
    #[expect(deprecated, reason = "calls deprecated SecurityBackend constructor")]
    pub fn with_security(config: AuthManagerConfig) -> Self {
        let backend = super::auth_backend::SecurityBackend::new(config.security_endpoint.clone());
        Self {
            config,
            current_token: None,
            backend: Arc::new(AuthBackendDispatch::Security(backend)),
            refresh_task: None,
        }
    }

    /// Creates auth manager with in-memory backend (no crypto).
    ///
    /// Only available when compiling tests or with the `test-mocks` feature.
    #[must_use]
    #[cfg(any(test, feature = "test-mocks"))]
    pub fn with_inmemory(config: AuthManagerConfig) -> Self {
        let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
        Self {
            config,
            current_token: None,
            backend: Arc::new(AuthBackendDispatch::InMemory(backend)),
            refresh_task: None,
        }
    }

    /// # Errors
    ///
    /// Returns an error if the backend connection cannot be established.
    pub async fn initialize_security_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// # Errors
    ///
    /// Returns an error if token request or refresh fails.
    pub async fn get_current_token(&self) -> ToadStoolResult<AuthenticationToken> {
        if let Some(token) = &self.current_token {
            if token.expires_at > SystemTime::now() + Duration::from_secs(30) {
                return Ok(token.clone());
            }
        }
        self.request_new_token().await
    }

    async fn request_new_token(&self) -> ToadStoolResult<AuthenticationToken> {
        let token_request = TokenRequest {
            // Self-knowledge: this primal's identity is fixed and knowable without discovery.
            requesting_primal: env!("CARGO_PKG_NAME").to_string(),
            scope: vec!["cross-primal".to_string(), "propagation".to_string()],
            // Audience sourced from config (env TOADSTOOL_AUTH_AUDIENCE or defaults).
            audience: self.config.token_audience.clone(),
            timestamp: SystemTime::now(),
        };
        self.backend.request_token(&token_request).await
    }

    /// Sign a token propagation request for the target primal.
    ///
    /// # Errors
    ///
    /// Returns an error if signing fails (e.g., invalid key configuration).
    pub async fn sign_token_request(
        &self,
        token: &AuthenticationToken,
        target_primal: &str,
    ) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }
        let payload = format!(
            "{}:{}:{}",
            token.id,
            target_primal,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        self.sign_payload(&payload).await
    }

    /// Sign a verification request payload for the given primal.
    ///
    /// # Errors
    ///
    /// Returns an error if signing fails (e.g., invalid key configuration).
    pub async fn sign_verification_request(&self, primal_name: &str) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }
        let payload = format!(
            "verify:{}:{}",
            primal_name,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        self.sign_payload(&payload).await
    }

    async fn sign_payload(&self, payload: &str) -> ToadStoolResult<String> {
        self.backend.sign_payload(payload).await
    }

    /// Returns the public key for signature verification, if available.
    #[must_use]
    pub async fn get_public_key(&self) -> Option<String> {
        self.backend.public_key().await
    }

    /// # Errors
    ///
    /// Returns an error if the refresh task cannot be spawned.
    pub fn start_token_refresh(&mut self) -> ToadStoolResult<()> {
        let refresh_interval = self.config.token_refresh_interval;
        let backend = Arc::clone(&self.backend);
        let refresh_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            loop {
                interval.tick().await;
                tracing::debug!("Refreshing authentication token");
                let refresh_request = TokenRefreshRequest {
                    requesting_primal: PRIMAL_NAME.to_string(),
                    timestamp: SystemTime::now(),
                };
                match backend.refresh_token(&refresh_request).await {
                    Ok(_) => tracing::info!("Authentication token refreshed successfully"),
                    Err(e) => tracing::error!("Failed to refresh authentication token: {}", e),
                }
            }
        });
        self.refresh_task = Some(refresh_task);
        Ok(())
    }

    /// Stops the background token refresh task.
    pub fn stop_token_refresh(&mut self) {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests;
