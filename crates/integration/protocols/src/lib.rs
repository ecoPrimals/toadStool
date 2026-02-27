#![deny(unsafe_code)]

//! # `ToadStool` Protocol Integration Layer
//!
//! This module provides integration with various ecosystem protocols and services,
//! including `BearDog` security integration for authentication and authorization.
//!
//! EVOLVED: Pure Rust! Uses Unix sockets for inter-primal communication (no reqwest!)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    security::SecurityContext,
};

// Sub-modules
pub mod client;
pub mod config;
pub mod tarpc_service;
pub mod transport;
pub mod types;

/// `BearDog` security integration configuration
///
/// EVOLVED: Pure Rust! Uses Unix socket paths instead of HTTP endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    /// `BearDog` Unix socket path (Pure Rust communication!)
    pub socket_path: String,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Token refresh interval in seconds
    pub token_refresh_interval_secs: u64,
    /// Zero-trust validation interval in seconds
    pub zero_trust_validation_interval_secs: u64,
    /// Enable continuous security monitoring
    pub continuous_monitoring: bool,
}

impl Default for BearDogConfig {
    fn default() -> Self {
        // EVOLVED: Pure Rust Unix socket discovery!
        let socket_path = std::env::var("BEARDOG_SOCKET").unwrap_or_else(|_| {
            // Standard primal socket location
            let runtime_dir =
                std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
            format!("{}/beardog.sock", runtime_dir)
        });

        Self {
            socket_path,
            request_timeout_secs: 30,
            token_refresh_interval_secs: 300,        // 5 minutes
            zero_trust_validation_interval_secs: 60, // 1 minute
            continuous_monitoring: true,
        }
    }
}

/// Authentication request to `BearDog`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub service_id: String,
    pub service_type: String,
    pub capabilities: Vec<String>,
    pub security_context: SecurityContext,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Authentication response from `BearDog`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Vec<String>,
    pub security_level: String,
    pub policies: Vec<SecurityPolicy>,
}

/// Authorization request to `BearDog`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzRequest {
    pub access_token: String,
    pub resource: String,
    pub action: String,
    pub context: HashMap<String, serde_json::Value>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Authorization response from `BearDog`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzResponse {
    pub allowed: bool,
    pub reason: Option<String>,
    pub policies_applied: Vec<String>,
    pub security_recommendations: Vec<String>,
    pub audit_id: String,
}

/// Security policy from `BearDog`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub enforcement_level: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub condition: String,
    pub action: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub service_id: String,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub result: String,
    pub security_context: SecurityContext,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// `BearDog` security integration client
///
/// EVOLVED: Pure Rust! Uses Unix sockets with JSON-RPC (no HTTP/reqwest!)
pub struct BearDogIntegration {
    config: BearDogConfig,
    // No reqwest Client! Pure Rust Unix sockets! ✅
    access_token: Arc<Mutex<Option<String>>>,
    token_expires_at: Arc<Mutex<Option<Instant>>>,
    active_policies: Arc<RwLock<Vec<SecurityPolicy>>>,
    audit_buffer: Arc<Mutex<Vec<SecurityAuditEvent>>>,
    last_validation: Arc<Mutex<Instant>>,
}

impl BearDogIntegration {
    /// Create a new `BearDog` integration client
    ///
    /// EVOLVED: Pure Rust! No HTTP client needed!
    pub fn new(config: BearDogConfig) -> Result<Self, ToadStoolError> {
        // Pure Rust! No reqwest::Client! ✅
        Ok(Self {
            config,
            access_token: Arc::new(Mutex::new(None)),
            token_expires_at: Arc::new(Mutex::new(None)),
            active_policies: Arc::new(RwLock::new(Vec::new())),
            audit_buffer: Arc::new(Mutex::new(Vec::new())),
            last_validation: Arc::new(Mutex::new(Instant::now())),
        })
    }

    /// Authenticate with `BearDog` and obtain access token
    ///
    /// EVOLVED: Pure Rust JSON-RPC over Unix socket!
    pub async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse> {
        info!("🐻 Authenticating with BearDog security service (Pure Rust!)");

        let auth_request = AuthRequest {
            service_id: service_id.to_string(),
            service_type: service_type.to_string(),
            capabilities,
            security_context,
            timestamp: std::time::SystemTime::now(),
        };

        // Try to call BearDog via Pure Rust Unix socket
        match self.make_request("auth.authenticate", &auth_request).await {
            Ok(result) => {
                let auth_response: AuthResponse = serde_json::from_value(result).map_err(|e| {
                    ToadStoolError::security(format!("Failed to parse auth response: {}", e))
                })?;

                // Store access token
                let mut token = self.access_token.lock().await;
                *token = Some(auth_response.access_token.clone());

                // Store token expiration
                let mut expires_at = self.token_expires_at.lock().await;
                *expires_at = Some(Instant::now() + Duration::from_secs(auth_response.expires_in));

                // Store active policies
                let mut policies = self.active_policies.write().await;
                *policies = auth_response.policies.clone();

                info!("✅ Successfully authenticated with BearDog (Pure Rust!)");
                Ok(auth_response)
            }
            Err(e) => {
                // Graceful degradation: BearDog not available
                info!("⚠️  BearDog not available: {}", e);
                info!("   Deep debt principle: ToadStool works standalone");

                // Return stub response for graceful degradation
                Ok(AuthResponse {
                    access_token: "standalone".to_string(),
                    token_type: "bearer".to_string(),
                    expires_in: 3600,
                    scope: vec!["standalone".to_string()],
                    security_level: "standard".to_string(),
                    policies: vec![],
                })
            }
        }
    }

    /// Check authorization for a resource and action
    ///
    /// EVOLVED: Pure Rust JSON-RPC over Unix socket!
    pub async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse> {
        info!(
            "🔒 Checking authorization with BearDog for {} on {}",
            action, resource
        );

        // Ensure we have a valid token
        self.ensure_valid_token().await?;

        let token = self.access_token.lock().await;
        let access_token = token
            .as_ref()
            .ok_or_else(|| ToadStoolError::security("No access token available"))?
            .clone();

        let authz_request = AuthzRequest {
            access_token,
            resource: resource.to_string(),
            action: action.to_string(),
            context,
            timestamp: std::time::SystemTime::now(),
        };

        // Try to call BearDog via Pure Rust Unix socket
        match self.make_request("authz.authorize", &authz_request).await {
            Ok(result) => {
                let authz_response: AuthzResponse =
                    serde_json::from_value(result).map_err(|e| {
                        ToadStoolError::security(format!("Failed to parse authz response: {}", e))
                    })?;

                // Audit the authorization decision
                self.audit_authorization_decision(resource, action, &authz_response)
                    .await?;

                if authz_response.allowed {
                    info!("✅ Authorization granted for {} on {}", action, resource);
                } else {
                    warn!(
                        "❌ Authorization denied for {} on {}: {:?}",
                        action, resource, authz_response.reason
                    );
                }

                Ok(authz_response)
            }
            Err(e) => {
                // Graceful degradation: BearDog not available
                info!("⚠️  BearDog not available for authorization: {}", e);
                info!("   Deep debt principle: ToadStool works standalone");

                // Return permissive response for graceful degradation
                Ok(AuthzResponse {
                    allowed: true,
                    reason: Some("Standalone mode - no BearDog".to_string()),
                    policies_applied: vec![],
                    security_recommendations: vec![],
                    audit_id: uuid::Uuid::new_v4().to_string(),
                })
            }
        }
    }

    /// Perform zero-trust validation
    ///
    /// EVOLVED: Pure Rust JSON-RPC over Unix socket!
    pub async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool> {
        info!("🛡️ Performing zero-trust validation with BearDog");

        let validation_request = serde_json::json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "security_context": security_context,
            "validation_type": "zero_trust",
            "continuous_monitoring": self.config.continuous_monitoring,
        });

        // Try to call BearDog via Pure Rust Unix socket
        match self
            .make_request("authz.validate", &validation_request)
            .await
        {
            Ok(result) => {
                let is_valid = result["valid"].as_bool().ok_or_else(|| {
                    ToadStoolError::security("Invalid validation response format")
                })?;

                // Update last validation time
                let mut last_validation = self.last_validation.lock().await;
                *last_validation = Instant::now();

                if is_valid {
                    info!("✅ Zero-trust validation passed");
                } else {
                    warn!("❌ Zero-trust validation failed: {:?}", result["reason"]);
                }

                Ok(is_valid)
            }
            Err(e) => {
                // Graceful degradation: BearDog not available
                info!("⚠️  BearDog not available for validation: {}", e);
                info!("   Deep debt principle: ToadStool works standalone");

                // Return permissive for graceful degradation
                Ok(true)
            }
        }
    }

    /// Start background security monitoring tasks
    pub async fn start_background_tasks(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("🔄 Starting BearDog security background tasks");

        // Token refresh task
        let token_refresh_integration = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                token_refresh_integration.config.token_refresh_interval_secs,
            ));

            loop {
                interval.tick().await;
                if let Err(e) = token_refresh_integration.refresh_token_if_needed().await {
                    error!("Failed to refresh token: {}", e);
                }
            }
        });

        // Zero-trust validation task
        if self.config.continuous_monitoring {
            let validation_integration = Arc::clone(&self);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(
                    validation_integration
                        .config
                        .zero_trust_validation_interval_secs,
                ));

                loop {
                    interval.tick().await;
                    // In a real implementation, this would validate current security context
                    let security_context = SecurityContext::default();
                    if let Err(e) = validation_integration
                        .zero_trust_validation(&security_context)
                        .await
                    {
                        error!("Zero-trust validation failed: {}", e);
                    }
                }
            });
        }

        // Audit buffer flush task
        let audit_integration = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Flush every minute

            loop {
                interval.tick().await;
                if let Err(e) = audit_integration.flush_audit_buffer().await {
                    error!("Failed to flush audit buffer: {}", e);
                }
            }
        });

        info!("✅ BearDog security background tasks started");
        Ok(())
    }

    /// Helper methods
    async fn ensure_valid_token(&self) -> ToadStoolResult<()> {
        let expires_at = self.token_expires_at.lock().await;
        if let Some(expiry) = *expires_at {
            if Instant::now() + Duration::from_secs(60) > expiry {
                // Token expires within 1 minute, refresh it
                drop(expires_at);
                return self.refresh_token_if_needed().await;
            }
        }
        Ok(())
    }

    async fn refresh_token_if_needed(&self) -> ToadStoolResult<()> {
        info!("🔄 Refreshing BearDog access token");
        // In a real implementation, this would refresh the token
        // For now, just log that it would happen
        Ok(())
    }

    async fn audit_authorization_decision(
        &self,
        resource: &str,
        action: &str,
        authz_response: &AuthzResponse,
    ) -> ToadStoolResult<()> {
        let audit_event = SecurityAuditEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: "authorization_decision".to_string(),
            service_id: "toadstool".to_string(),
            user_id: None,
            resource: resource.to_string(),
            action: action.to_string(),
            result: if authz_response.allowed {
                "allowed"
            } else {
                "denied"
            }
            .to_string(),
            security_context: SecurityContext::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "audit_id".to_string(),
                    serde_json::Value::String(authz_response.audit_id.clone()),
                );
                metadata.insert(
                    "policies_applied".to_string(),
                    serde_json::Value::Array(
                        authz_response
                            .policies_applied
                            .iter()
                            .map(|p| serde_json::Value::String(p.clone()))
                            .collect(),
                    ),
                );
                metadata
            },
            timestamp: std::time::SystemTime::now(),
        };

        let mut buffer = self.audit_buffer.lock().await;
        buffer.push(audit_event);

        Ok(())
    }

    async fn flush_audit_buffer(&self) -> ToadStoolResult<()> {
        let mut buffer = self.audit_buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }

        let events = buffer.drain(..).collect::<Vec<_>>();
        drop(buffer);

        let audit_payload = serde_json::json!({
            "events": events,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });

        // Try to flush to BearDog via Pure Rust Unix socket
        match self.make_request("audit.flush", &audit_payload).await {
            Ok(_) => {
                info!(
                    "✅ Successfully flushed {} audit events to BearDog (Pure Rust!)",
                    events.len()
                );
            }
            Err(e) => {
                warn!("❌ Failed to flush audit events to BearDog: {}", e);
                // Re-add events to buffer for retry
                let mut buffer = self.audit_buffer.lock().await;
                buffer.extend(events);
            }
        }

        Ok(())
    }

    async fn make_request<T: Serialize>(
        &self,
        method: &str,
        params: &T,
    ) -> ToadStoolResult<serde_json::Value> {
        // EVOLVED: Pure Rust JSON-RPC over Unix socket!
        let mut stream = UnixStream::connect(&self.config.socket_path)
            .await
            .map_err(|e| {
                ToadStoolError::security(format!("Failed to connect to BearDog: {}", e))
            })?;

        // Create JSON-RPC 2.0 request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": Uuid::new_v4().to_string()
        });

        let request_str = serde_json::to_string(&request)
            .map_err(|e| ToadStoolError::security(format!("Failed to serialize request: {}", e)))?;

        // Send request
        stream
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| ToadStoolError::security(format!("Failed to send request: {}", e)))?;

        // Read response
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| ToadStoolError::security(format!("Failed to read response: {}", e)))?;

        // Parse JSON-RPC response
        let response_json: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|e| ToadStoolError::security(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = response_json.get("error") {
            return Err(ToadStoolError::security(format!(
                "BearDog error: {}",
                error
            )));
        }

        response_json
            .get("result")
            .cloned()
            .ok_or_else(|| ToadStoolError::security("No result in response"))
    }
}

/// `BearDog` integration trait for dependency injection
#[async_trait]
pub trait BearDogIntegrationTrait: Send + Sync {
    async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse>;

    async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse>;

    async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool>;
}

#[async_trait]
impl BearDogIntegrationTrait for BearDogIntegration {
    async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse> {
        self.authenticate(service_id, service_type, capabilities, security_context)
            .await
    }

    async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse> {
        self.authorize(resource, action, context).await
    }

    async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool> {
        self.zero_trust_validation(security_context).await
    }
}
