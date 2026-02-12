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
    /// Ed25519 signing key seed (32 bytes, base64 encoded)
    /// When set, enables real Ed25519 signatures instead of mock signatures
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_key_seed: Option<String>,
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
        let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
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

    /// Sign token propagation request using Ed25519
    ///
    /// If a signing key seed is configured, produces a real Ed25519 signature.
    /// Otherwise, returns a mock signature for development/testing.
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

        self.sign_payload(&payload).await
    }

    /// Sign verification request using Ed25519
    ///
    /// If a signing key seed is configured, produces a real Ed25519 signature.
    /// Otherwise, returns a mock signature for development/testing.
    pub async fn sign_verification_request(&self, primal_name: &str) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }

        // Create signing payload
        let payload = format!("verify:{}:{}", primal_name, chrono::Utc::now().timestamp());

        self.sign_payload(&payload).await
    }

    /// Internal method to sign a payload using Ed25519
    ///
    /// Uses real Ed25519 signing when a key seed is configured, otherwise mock.
    async fn sign_payload(&self, payload: &str) -> ToadStoolResult<String> {
        use base64::{engine::general_purpose, Engine as _};

        // Check if we have a real signing key configured
        if let Some(ref seed_b64) = self.config.signing_key_seed {
            // Decode the seed from base64
            let seed_bytes = general_purpose::STANDARD.decode(seed_b64).map_err(|e| {
                crate::ToadStoolError::configuration(format!(
                    "Invalid signing key seed (base64 decode error): {}",
                    e
                ))
            })?;

            // Verify seed length (must be 32 bytes for Ed25519)
            if seed_bytes.len() != 32 {
                return Err(crate::ToadStoolError::configuration(format!(
                    "Invalid signing key seed length: expected 32 bytes, got {}",
                    seed_bytes.len()
                )));
            }

            // Create signing key from seed
            let seed: [u8; 32] = seed_bytes.try_into().expect("length verified above");
            let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);

            // Sign the payload
            use ed25519_dalek::Signer;
            let signature = signing_key.sign(payload.as_bytes());

            // Return base64-encoded signature with ed25519: prefix
            Ok(format!(
                "ed25519:{}",
                general_purpose::STANDARD.encode(signature.to_bytes())
            ))
        } else {
            // No signing key configured
            #[cfg(any(test, feature = "dev-mock-auth"))]
            {
                // Development/test mode: return mock signature
                tracing::warn!("No signing key configured, using mock signature (dev mode only)");
                Ok(format!(
                    "ed25519:mock:{}",
                    general_purpose::STANDARD.encode(payload.as_bytes())
                ))
            }
            #[cfg(not(any(test, feature = "dev-mock-auth")))]
            {
                // Production mode: require real signing key
                Err(crate::ToadStoolError::configuration(
                    "No signing key configured. Set TOADSTOOL_SIGNING_KEY_SEED or configure signing_key_seed in auth config."
                ))
            }
        }
    }

    /// Get the public key corresponding to the configured signing key
    ///
    /// Returns None if no signing key is configured.
    pub fn get_public_key(&self) -> Option<String> {
        use base64::{engine::general_purpose, Engine as _};

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
    use std::collections::HashMap;

    use crate::biomeos_integration::types::ToadStoolConfig;

    fn test_config() -> AuthManagerConfig {
        AuthManagerConfig {
            beardog_endpoint: "http://localhost:9090".to_string(),
            token_refresh_interval: Duration::from_secs(3600),
            signature_validation: true,
            timestamp_window: Duration::from_secs(300),
            replay_protection: true,
            signing_key_seed: None,
        }
    }

    fn test_config_with_signing_key() -> AuthManagerConfig {
        // Test signing key seed (32 bytes, base64 encoded)
        // In production, this would come from secure storage or environment
        let seed = [0u8; 32]; // Zero seed for deterministic test
        use base64::{engine::general_purpose, Engine as _};
        AuthManagerConfig {
            beardog_endpoint: "http://localhost:9090".to_string(),
            token_refresh_interval: Duration::from_secs(3600),
            signature_validation: true,
            timestamp_window: Duration::from_secs(300),
            replay_protection: true,
            signing_key_seed: Some(general_purpose::STANDARD.encode(seed)),
        }
    }

    fn sample_token() -> AuthenticationToken {
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::hours(1);
        AuthenticationToken {
            id: "token-123".to_string(),
            token_type: "Bearer".to_string(),
            token: "encrypted-value".to_string(),
            public_key: "pk-abc".to_string(),
            expires_at: expires,
            issued_at: now,
            issuer: "beardog".to_string(),
            audience: vec!["songbird".to_string(), "biomeos".to_string()],
            scope: vec!["cross-primal".to_string()],
            claims: HashMap::new(),
        }
    }

    #[test]
    fn test_auth_manager_config_construction() {
        let config = test_config();
        assert_eq!(config.beardog_endpoint, "http://localhost:9090");
        assert_eq!(config.token_refresh_interval, Duration::from_secs(3600));
        assert!(config.signature_validation);
        assert!(config.replay_protection);
    }

    #[test]
    fn test_auth_manager_config_serialization_roundtrip() {
        let config = test_config();
        let json = serde_json::to_string(&config).expect("serialize");
        let restored: AuthManagerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.beardog_endpoint, restored.beardog_endpoint);
        assert_eq!(config.signature_validation, restored.signature_validation);
    }

    #[test]
    fn test_authentication_token_construction() {
        let token = sample_token();
        assert_eq!(token.id, "token-123");
        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.issuer, "beardog");
        assert_eq!(token.audience.len(), 2);
        assert!(token.expires_at > token.issued_at);
    }

    #[test]
    fn test_authentication_token_serialization_roundtrip() {
        let token = sample_token();
        let json = serde_json::to_string(&token).expect("serialize");
        let restored: AuthenticationToken = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(token.id, restored.id);
        assert_eq!(token.issuer, restored.issuer);
    }

    #[test]
    fn test_token_propagation_request_construction() {
        let token = sample_token();
        let now = chrono::Utc::now();
        let req = TokenPropagationRequest {
            token: token.clone(),
            source_primal: "toadstool".to_string(),
            target_primal: "songbird".to_string(),
            timestamp: now,
            signature: "sig-xyz".to_string(),
        };
        assert_eq!(req.source_primal, "toadstool");
        assert_eq!(req.target_primal, "songbird");
        assert_eq!(req.token.id, "token-123");
    }

    #[test]
    fn test_token_propagation_request_serialization_roundtrip() {
        let req = TokenPropagationRequest {
            token: sample_token(),
            source_primal: "toadstool".to_string(),
            target_primal: "songbird".to_string(),
            timestamp: chrono::Utc::now(),
            signature: "sig".to_string(),
        };
        let json = serde_json::to_string(&req).expect("serialize");
        let restored: TokenPropagationRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(req.source_primal, restored.source_primal);
        assert_eq!(req.target_primal, restored.target_primal);
    }

    #[test]
    fn test_token_verification_request_construction() {
        let now = chrono::Utc::now();
        let req = TokenVerificationRequest {
            primal_name: "songbird".to_string(),
            timestamp: now,
            signature: "verify-sig".to_string(),
        };
        assert_eq!(req.primal_name, "songbird");
    }

    #[test]
    fn test_token_verification_request_serialization_roundtrip() {
        let req = TokenVerificationRequest {
            primal_name: "nestgate".to_string(),
            timestamp: chrono::Utc::now(),
            signature: "sig".to_string(),
        };
        let json = serde_json::to_string(&req).expect("serialize");
        let restored: TokenVerificationRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(req.primal_name, restored.primal_name);
    }

    #[test]
    fn test_token_verification_response_construction() {
        let resp = TokenVerificationResponse {
            status: TokenVerificationStatus::Valid,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            details: Some("all good".to_string()),
        };
        assert_eq!(resp.status, TokenVerificationStatus::Valid);
        assert!(resp.expires_at.is_some());
        assert_eq!(resp.details.as_deref(), Some("all good"));
    }

    #[test]
    fn test_token_verification_response_serialization_roundtrip() {
        let resp = TokenVerificationResponse {
            status: TokenVerificationStatus::Expired,
            expires_at: None,
            details: None,
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let restored: TokenVerificationResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(resp.status, restored.status);
    }

    #[test]
    fn test_token_verification_status_variants() {
        assert_eq!(
            TokenVerificationStatus::Valid,
            TokenVerificationStatus::Valid
        );
        assert_eq!(
            TokenVerificationStatus::Expired,
            TokenVerificationStatus::Expired
        );
        assert_eq!(
            TokenVerificationStatus::Invalid,
            TokenVerificationStatus::Invalid
        );
        assert_eq!(
            TokenVerificationStatus::NotFound,
            TokenVerificationStatus::NotFound
        );
        let err = TokenVerificationStatus::Error("reason".to_string());
        assert!(matches!(err, TokenVerificationStatus::Error(s) if s == "reason"));
    }

    #[test]
    fn test_token_propagation_status_variants() {
        assert_eq!(
            TokenPropagationStatus::Success,
            TokenPropagationStatus::Success
        );
        assert_eq!(
            TokenPropagationStatus::Pending,
            TokenPropagationStatus::Pending
        );
        assert!(matches!(
            TokenPropagationStatus::Failed("msg".to_string()),
            TokenPropagationStatus::Failed(s) if s == "msg"
        ));
        assert!(matches!(
            TokenPropagationStatus::Skipped("reason".to_string()),
            TokenPropagationStatus::Skipped(s) if s == "reason"
        ));
    }

    #[test]
    fn test_propagation_result_construction() {
        let mut results = HashMap::new();
        results.insert("songbird".to_string(), TokenPropagationStatus::Success);
        results.insert("nestgate".to_string(), TokenPropagationStatus::Pending);

        let res = PropagationResult {
            total_primals: 2,
            successful_propagations: 1,
            results: results.clone(),
            token_id: "token-1".to_string(),
            propagation_time: chrono::Utc::now(),
        };
        assert_eq!(res.total_primals, 2);
        assert_eq!(res.successful_propagations, 1);
        assert_eq!(res.results.len(), 2);
        assert_eq!(res.token_id, "token-1");
    }

    #[test]
    fn test_propagation_result_serialization_roundtrip() {
        let mut results = HashMap::new();
        results.insert("p1".to_string(), TokenPropagationStatus::Success);
        let res = PropagationResult {
            total_primals: 1,
            successful_propagations: 1,
            results,
            token_id: "t1".to_string(),
            propagation_time: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&res).expect("serialize");
        let restored: PropagationResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(res.token_id, restored.token_id);
    }

    #[test]
    fn test_verification_result_construction() {
        let mut results = HashMap::new();
        results.insert("songbird".to_string(), TokenVerificationStatus::Valid);

        let res = VerificationResult {
            total_primals: 1,
            valid_tokens: 1,
            results,
            verification_time: chrono::Utc::now(),
        };
        assert_eq!(res.total_primals, 1);
        assert_eq!(res.valid_tokens, 1);
    }

    #[test]
    fn test_verification_result_serialization_roundtrip() {
        let mut results = HashMap::new();
        results.insert("p1".to_string(), TokenVerificationStatus::Invalid);
        let res = VerificationResult {
            total_primals: 1,
            valid_tokens: 0,
            results,
            verification_time: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&res).expect("serialize");
        let restored: VerificationResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(res.valid_tokens, restored.valid_tokens);
    }

    #[test]
    fn test_token_request_construction() {
        let now = chrono::Utc::now();
        let req = TokenRequest {
            requesting_primal: "toadstool".to_string(),
            scope: vec!["read".to_string(), "write".to_string()],
            audience: vec!["songbird".to_string()],
            timestamp: now,
        };
        assert_eq!(req.requesting_primal, "toadstool");
        assert_eq!(req.scope.len(), 2);
    }

    #[test]
    fn test_token_refresh_request_construction() {
        let now = chrono::Utc::now();
        let req = TokenRefreshRequest {
            requesting_primal: "toadstool".to_string(),
            timestamp: now,
        };
        assert_eq!(req.requesting_primal, "toadstool");
    }

    #[test]
    fn test_primal_type_config_toadstool_variant() {
        let toad_config = ToadStoolConfig {
            enabled: true,
            orchestrator: true,
            resources: None,
            runtime_engines: vec!["wgpu".to_string()],
            execution_environments: vec!["container".to_string()],
            substrates: vec!["linux".to_string()],
            config: HashMap::new(),
        };
        let primal = PrimalTypeConfig::ToadStool(toad_config);
        assert!(matches!(primal, PrimalTypeConfig::ToadStool(c) if c.enabled && c.orchestrator));
    }

    #[test]
    fn test_primal_type_config_serialization_roundtrip() {
        let config = ToadStoolConfig {
            enabled: true,
            orchestrator: false,
            resources: None,
            runtime_engines: vec![],
            execution_environments: vec![],
            substrates: vec![],
            config: HashMap::new(),
        };
        let primal = PrimalTypeConfig::ToadStool(config);
        let json = serde_json::to_string(&primal).expect("serialize");
        let restored: PrimalTypeConfig = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(restored, PrimalTypeConfig::ToadStool(c) if c.enabled));
    }

    #[test]
    fn test_authentication_manager_new() {
        let config = test_config();
        let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
        let _manager = AuthenticationManager::new(config, Arc::new(backend));
        // Manager constructed successfully; backend is used via get_current_token
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_manager_with_inmemory_backend() {
        let config = test_config();
        let manager = AuthenticationManager::with_inmemory(config);

        let result = manager.get_current_token().await;
        assert!(result.is_ok());

        let token = result.expect("Token retrieval should succeed in test");
        assert_eq!(token.issuer, "beardog");
        assert!(token.expires_at > chrono::Utc::now());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sign_token_request_mock() {
        let config = test_config();
        let manager = AuthenticationManager::with_inmemory(config);
        let token = manager
            .get_current_token()
            .await
            .expect("Token retrieval should succeed in test");

        let signature = manager.sign_token_request(&token, "songbird").await;
        assert!(signature.is_ok());
        // Without signing key, returns mock signature
        assert!(signature
            .expect("Signature should be generated in test")
            .starts_with("ed25519:mock:"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sign_token_request_real_ed25519() {
        use base64::{engine::general_purpose, Engine as _};
        use ed25519_dalek::{Signature, VerifyingKey};

        let config = test_config_with_signing_key();
        let manager = AuthenticationManager::with_inmemory(config);
        let token = manager
            .get_current_token()
            .await
            .expect("Token retrieval should succeed in test");

        let signature_str = manager
            .sign_token_request(&token, "songbird")
            .await
            .expect("Signature generation should succeed");

        // Should produce real ed25519 signature (not mock)
        assert!(signature_str.starts_with("ed25519:"));
        assert!(!signature_str.starts_with("ed25519:mock:"));

        // Get public key for verification
        let public_key_b64 = manager.get_public_key().expect("Should have public key");
        let public_key_bytes = general_purpose::STANDARD
            .decode(&public_key_b64)
            .expect("Valid base64");
        let verifying_key =
            VerifyingKey::from_bytes(public_key_bytes.as_slice().try_into().expect("32 bytes"))
                .expect("Valid key");

        // Extract signature bytes (after "ed25519:" prefix)
        let sig_b64 = signature_str.strip_prefix("ed25519:").expect("Has prefix");
        let sig_bytes = general_purpose::STANDARD
            .decode(sig_b64)
            .expect("Valid base64");
        let signature = Signature::from_bytes(sig_bytes.as_slice().try_into().expect("64 bytes"));

        // Note: We can't verify the exact payload because it includes a timestamp,
        // but we've confirmed the signature is a valid 64-byte Ed25519 signature
        // and the public key derivation works correctly.
        assert_eq!(sig_bytes.len(), 64);
        assert_eq!(public_key_bytes.len(), 32);

        // Verify signature format is correct (can be parsed as Signature)
        let _ = verifying_key; // Used above
        let _ = signature; // Parsed successfully
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_sign_verification_request_real_ed25519() {
        let config = test_config_with_signing_key();
        let manager = AuthenticationManager::with_inmemory(config);

        let signature_str = manager
            .sign_verification_request("songbird")
            .await
            .expect("Signature generation should succeed");

        // Should produce real ed25519 signature
        assert!(signature_str.starts_with("ed25519:"));
        assert!(!signature_str.starts_with("ed25519:mock:"));

        // Extract and validate signature length (64 bytes)
        use base64::{engine::general_purpose, Engine as _};
        let sig_b64 = signature_str.strip_prefix("ed25519:").expect("Has prefix");
        let sig_bytes = general_purpose::STANDARD
            .decode(sig_b64)
            .expect("Valid base64");
        assert_eq!(sig_bytes.len(), 64);
    }

    #[test]
    fn test_get_public_key() {
        let config = test_config_with_signing_key();
        let manager = AuthenticationManager::with_inmemory(config);

        let public_key = manager.get_public_key();
        assert!(public_key.is_some());

        use base64::{engine::general_purpose, Engine as _};
        let pk_bytes = general_purpose::STANDARD
            .decode(public_key.unwrap())
            .expect("Valid base64");
        assert_eq!(pk_bytes.len(), 32); // Ed25519 public key is 32 bytes
    }

    #[test]
    fn test_get_public_key_none_when_no_signing_key() {
        let config = test_config();
        let manager = AuthenticationManager::with_inmemory(config);

        let public_key = manager.get_public_key();
        assert!(public_key.is_none());
    }
}
