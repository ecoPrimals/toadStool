// SPDX-License-Identifier: AGPL-3.0-only
//! Authentication management for cross-Primal token propagation

mod permissions;
mod tokens;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use toadstool_common::constants::primal_identity::{PRIMAL_NAME, audience};
use toadstool_common::constants::timeouts::{TIMESTAMP_VALIDATION_WINDOW, TOKEN_REFRESH_INTERVAL};

use super::auth_backend::AuthBackend;
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
    /// BearDog crypto service endpoint (legacy; prefer capability discovery).
    #[serde(default)]
    pub beardog_endpoint: String,
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
            beardog_endpoint: String::new(),
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
    backend: Arc<dyn AuthBackend>,
    refresh_task: Option<tokio::task::JoinHandle<()>>,
}

impl AuthenticationManager {
    /// Creates a new auth manager with config and backend.
    #[must_use]
    pub fn new(config: AuthManagerConfig, backend: Arc<dyn AuthBackend>) -> Self {
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

        if let Ok(endpoint) = std::env::var("BEARDOG_ENDPOINT")
            .or_else(|_| std::env::var("TOADSTOOL_SECURITY_ENDPOINT"))
        {
            tracing::info!("Discovered crypto service via environment: {}", endpoint);
            let mut config = config;
            config.beardog_endpoint = endpoint;
            #[allow(deprecated)]
            return Ok(Self::with_beardog(config));
        }

        if !config.beardog_endpoint.is_empty() {
            #[allow(deprecated)]
            return Ok(Self::with_beardog(config));
        }

        tracing::warn!(
            "No crypto provider discovered, using in-memory backend. \
             Ensure a crypto provider is running or set BEARDOG_ENDPOINT."
        );
        Ok(Self::with_inmemory(config))
    }

    /// # Errors
    ///
    /// Returns an error if crypto service discovery fails or the backend cannot be initialized.
    pub async fn with_crypto_service(config: AuthManagerConfig) -> ToadStoolResult<Self> {
        let backend = super::auth_backend::BearDogBackend::new_async().await?;
        Ok(Self {
            config,
            current_token: None,
            backend: Arc::new(backend),
            refresh_task: None,
        })
    }

    /// Creates auth manager with BearDog backend (deprecated).
    #[must_use]
    #[deprecated(since = "0.3.0", note = "Use with_crypto_service() or discover()")]
    #[allow(deprecated)]
    pub fn with_beardog(config: AuthManagerConfig) -> Self {
        let backend = super::auth_backend::BearDogBackend::new(config.beardog_endpoint.clone());
        Self {
            config,
            current_token: None,
            backend: Arc::new(backend),
            refresh_task: None,
        }
    }

    /// Creates auth manager with in-memory backend (no crypto).
    #[must_use]
    pub fn with_inmemory(config: AuthManagerConfig) -> Self {
        let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
        Self {
            config,
            current_token: None,
            backend: Arc::new(backend),
            refresh_task: None,
        }
    }

    /// # Errors
    ///
    /// Returns an error if the backend connection cannot be established.
    pub async fn initialize_beardog_connection(&self) -> ToadStoolResult<()> {
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
    pub fn sign_token_request(
        &self,
        token: &AuthenticationToken,
        target_primal: &str,
    ) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
        let payload = format!(
            "{}:{}:{}",
            token.id,
            target_primal,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        );
        self.sign_payload(&payload)
    }

    /// Sign a verification request payload for the given primal.
    ///
    /// # Errors
    ///
    /// Returns an error if signing fails (e.g., invalid key configuration).
    pub fn sign_verification_request(&self, primal_name: &str) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
        let payload = format!(
            "verify:{}:{}",
            primal_name,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        );
        self.sign_payload(&payload)
    }

    fn sign_payload(&self, payload: &str) -> ToadStoolResult<String> {
        use base64::{Engine as _, engine::general_purpose};
        use ed25519_dalek::Signer;

        if let Some(ref seed_b64) = self.config.signing_key_seed {
            let seed_bytes = general_purpose::STANDARD.decode(seed_b64).map_err(|e| {
                crate::ToadStoolError::configuration(format!(
                    "Invalid signing key seed (base64 decode error): {e}"
                ))
            })?;
            if seed_bytes.len() != 32 {
                return Err(crate::ToadStoolError::configuration(format!(
                    "Invalid signing key seed length: expected 32 bytes, got {}",
                    seed_bytes.len()
                )));
            }
            let seed: [u8; 32] = seed_bytes.try_into().map_err(|_: Vec<u8>| {
                crate::ToadStoolError::configuration(
                    "seed byte conversion failed (length invariant violated)",
                )
            })?;
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
            let signature = signing_key.sign(payload.as_bytes());
            Ok(format!(
                "ed25519:{}",
                general_purpose::STANDARD.encode(signature.to_bytes())
            ))
        } else {
            #[cfg(any(test, feature = "dev-mock-auth"))]
            {
                #[cfg(all(feature = "dev-mock-auth", not(debug_assertions)))]
                compile_error!(
                    "dev-mock-auth feature must not be enabled in release builds! \
                     Use TOADSTOOL_SIGNING_KEY_SEED environment variable for production."
                );
                tracing::warn!(
                    "⚠️ INSECURE: No signing key configured, using mock signature. \
                     This is acceptable ONLY in tests."
                );
                Ok(format!(
                    "ed25519:mock:{}",
                    general_purpose::STANDARD.encode(payload.as_bytes())
                ))
            }
            #[cfg(not(any(test, feature = "dev-mock-auth")))]
            {
                Err(crate::ToadStoolError::configuration(
                    "No signing key configured. Set TOADSTOOL_SIGNING_KEY_SEED or configure signing_key_seed in auth config.",
                ))
            }
        }
    }

    /// Returns the public key for signature verification, if a signing key is configured.
    #[must_use]
    pub fn get_public_key(&self) -> Option<String> {
        use base64::{Engine as _, engine::general_purpose};
        let seed_b64 = self.config.signing_key_seed.as_ref()?;
        let seed_bytes = general_purpose::STANDARD.decode(seed_b64).ok()?;
        if seed_bytes.len() != 32 {
            return None;
        }
        let seed: [u8; 32] = seed_bytes.try_into().ok()?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        Some(general_purpose::STANDARD.encode(verifying_key.as_bytes()))
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
