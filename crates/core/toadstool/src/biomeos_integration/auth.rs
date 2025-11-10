//! Authentication management for cross-Primal token propagation
//!
//! This module handles authentication token management, propagation across Primals,
//! and integration with BearDog security services using trait-based dependency injection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::auth_backend::AuthBackend;
use super::types::{
    BearDogConfig, BiomeOSConfig, NestGateConfig, SongbirdConfig, SquirrelConfig, ToadStoolConfig,
};
use crate::ToadStoolResult;

/// Authentication manager for cross-Primal token propagation
///
/// Uses dependency injection via the `AuthBackend` trait for flexibility.
/// No conditional compilation or feature flags - the backend determines behavior.
pub struct AuthenticationManager {
    /// Configuration
    config: AuthManagerConfig,
    /// Current authentication token
    current_token: Option<AuthenticationToken>,
    /// Pluggable authentication backend (BearDog, in-memory, etc.)
    backend: Arc<dyn AuthBackend>,
    /// Token refresh task handle
    refresh_task: Option<tokio::task::JoinHandle<()>>,
}

/// Configuration for authentication manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthManagerConfig {
    /// BearDog endpoint URL
    pub beardog_endpoint: String,
    /// Token refresh interval
    pub token_refresh_interval: Duration,
    /// Require signature validation
    pub signature_validation: bool,
    /// Timestamp validation window
    pub timestamp_window: Duration,
    /// Enable replay attack protection
    pub replay_protection: bool,
}

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationToken {
    /// Unique token ID
    pub id: String,
    /// Token type
    pub token_type: String,
    /// Token value (encrypted)
    pub token: String,
    /// Public key for signature verification
    pub public_key: String,
    /// Token expiration time
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Token issued time
    pub issued_at: chrono::DateTime<chrono::Utc>,
    /// Issuing Primal (always "beardog")
    pub issuer: String,
    /// Target audiences (Primals)
    pub audience: Vec<String>,
    /// Token scope/permissions
    pub scope: Vec<String>,
    /// Additional claims
    pub claims: HashMap<String, serde_json::Value>,
}

/// Token propagation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationRequest {
    /// Authentication token to propagate
    pub token: AuthenticationToken,
    /// Source Primal (sender)
    pub source_primal: String,
    /// Target Primal (receiver)
    pub target_primal: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request signature for integrity
    pub signature: String,
}

/// Token verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationRequest {
    /// Primal name to verify token for
    pub primal_name: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request signature
    pub signature: String,
}

/// Token verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationResponse {
    /// Verification status
    pub status: TokenVerificationStatus,
    /// Token expiration time (if valid)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional verification details
    pub details: Option<String>,
}

/// Token verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenVerificationStatus {
    /// Token is valid
    Valid,
    /// Token is expired
    Expired,
    /// Token is invalid
    Invalid,
    /// Token not found
    NotFound,
    /// Verification error
    Error(String),
}

/// Token propagation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenPropagationStatus {
    /// Propagation successful
    Success,
    /// Propagation failed
    Failed(String),
    /// Propagation pending
    Pending,
    /// Propagation skipped
    Skipped(String),
}

/// Result of token propagation across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationResult {
    /// Total number of Primals
    pub total_primals: usize,
    /// Number of successful propagations
    pub successful_propagations: usize,
    /// Individual Primal results
    pub results: HashMap<String, TokenPropagationStatus>,
    /// Token ID that was propagated
    pub token_id: String,
    /// Propagation timestamp
    pub propagation_time: chrono::DateTime<chrono::Utc>,
}

/// Result of token verification across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Total number of Primals
    pub total_primals: usize,
    /// Number of valid tokens
    pub valid_tokens: usize,
    /// Individual Primal verification results
    pub results: HashMap<String, TokenVerificationStatus>,
    /// Verification timestamp
    pub verification_time: chrono::DateTime<chrono::Utc>,
}

/// Primal type configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimalTypeConfig {
    /// ToadStool configuration
    ToadStool(ToadStoolConfig),
    /// Songbird configuration
    Songbird(SongbirdConfig),
    /// BearDog configuration
    BearDog(BearDogConfig),
    /// NestGate configuration
    NestGate(NestGateConfig),
    /// Squirrel configuration
    Squirrel(SquirrelConfig),
    /// biomeOS configuration
    BiomeOS(BiomeOSConfig),
}

/// Token request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    /// Requesting Primal
    pub requesting_primal: String,
    /// Token scope
    pub scope: Vec<String>,
    /// Target audience
    pub audience: Vec<String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    /// Requesting Primal
    pub requesting_primal: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AuthenticationManager {
    /// Create a new authentication manager with custom backend
    #[must_use]
    pub fn new(config: AuthManagerConfig, backend: Arc<dyn AuthBackend>) -> Self {
        Self {
            config,
            current_token: None,
            backend,
            refresh_task: None,
        }
    }

    /// Create a new manager with BearDog production backend
    #[must_use]
    pub fn with_beardog(config: AuthManagerConfig) -> Self {
        let backend = super::auth_backend::BearDogBackend::new(config.beardog_endpoint.clone());
        Self {
            config,
            current_token: None,
            backend: Arc::new(backend),
            refresh_task: None,
        }
    }

    /// Create a new manager with in-memory test backend
    #[must_use]
    pub fn with_inmemory(config: AuthManagerConfig) -> Self {
        let backend = super::auth_backend::InMemoryAuthBackend::new();
        Self {
            config,
            current_token: None,
            backend: Arc::new(backend),
            refresh_task: None,
        }
    }

    /// Initialize connection to BearDog (or test backend)
    pub async fn initialize_beardog_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// Get current authentication token
    pub async fn get_current_token(&self) -> ToadStoolResult<AuthenticationToken> {
        // Check if we have a valid cached token
        if let Some(token) = &self.current_token {
            if token.expires_at > chrono::Utc::now() + chrono::Duration::seconds(30) {
                return Ok(token.clone());
            }
        }

        // Request new token
        self.request_new_token().await
    }

    /// Request new authentication token
    async fn request_new_token(&self) -> ToadStoolResult<AuthenticationToken> {
        let token_request = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["cross-primal".to_string(), "propagation".to_string()],
            audience: vec![
                "songbird".to_string(),
                "nestgate".to_string(),
                "squirrel".to_string(),
                "biomeos".to_string(),
            ],
            timestamp: chrono::Utc::now(),
        };

        self.backend.request_token(&token_request).await
    }

    /// Sign token propagation request
    pub async fn sign_token_request(
        &self,
        token: &AuthenticationToken,
        target_primal: &str,
    ) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }

        // Create signing payload
        let payload = format!(
            "{}:{}:{}",
            token.id,
            target_primal,
            chrono::Utc::now().timestamp()
        );

        // In a real implementation, this would use Ed25519 signing
        // For now, return a mock signature
        use base64::{engine::general_purpose, Engine as _};
        let signature = format!(
            "ed25519:{}",
            general_purpose::STANDARD.encode(payload.as_bytes())
        );

        Ok(signature)
    }

    /// Sign verification request
    pub async fn sign_verification_request(&self, primal_name: &str) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }

        // Create signing payload
        let payload = format!("verify:{}:{}", primal_name, chrono::Utc::now().timestamp());

        // In a real implementation, this would use Ed25519 signing
        use base64::{engine::general_purpose, Engine as _};
        let signature = format!(
            "ed25519:{}",
            general_purpose::STANDARD.encode(payload.as_bytes())
        );

        Ok(signature)
    }

    /// Start automatic token refresh
    pub async fn start_token_refresh(&mut self) -> ToadStoolResult<()> {
        let refresh_interval = self.config.token_refresh_interval;
        let backend = Arc::clone(&self.backend);

        let refresh_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);

            loop {
                interval.tick().await;

                tracing::debug!("Refreshing authentication token");

                let refresh_request = TokenRefreshRequest {
                    requesting_primal: "toadstool".to_string(),
                    timestamp: chrono::Utc::now(),
                };

                match backend.refresh_token(&refresh_request).await {
                    Ok(_) => {
                        tracing::info!("Authentication token refreshed successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh authentication token: {}", e);
                    }
                }
            }
        });

        self.refresh_task = Some(refresh_task);

        Ok(())
    }

    /// Stop automatic token refresh
    pub fn stop_token_refresh(&mut self) {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthManagerConfig {
        AuthManagerConfig {
            beardog_endpoint: "http://localhost:9090".to_string(),
            token_refresh_interval: Duration::from_secs(3600),
            signature_validation: true,
            timestamp_window: Duration::from_secs(300),
            replay_protection: true,
        }
    }

    #[tokio::test]
    async fn test_manager_with_inmemory_backend() {
        let config = test_config();
        let manager = AuthenticationManager::with_inmemory(config);

        let result = manager.get_current_token().await;
        assert!(result.is_ok());

        let token = result.unwrap();
        assert_eq!(token.issuer, "beardog");
        assert!(token.expires_at > chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_sign_token_request() {
        let config = test_config();
        let manager = AuthenticationManager::with_inmemory(config);
        let token = manager.get_current_token().await.unwrap();

        let signature = manager.sign_token_request(&token, "songbird").await;
        assert!(signature.is_ok());
        assert!(signature.unwrap().starts_with("ed25519:"));
    }
}
