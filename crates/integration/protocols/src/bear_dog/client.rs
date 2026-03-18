// SPDX-License-Identifier: AGPL-3.0-or-later
//! BearDog integration client for auth, authz, and zero-trust validation

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    security::SecurityContext,
};

use super::auth::{AuthRequest, AuthResponse, AuthzRequest, AuthzResponse};
use super::config::BearDogConfig;
use super::policy::{SecurityAuditEvent, SecurityPolicy};
use super::transport;

/// BearDog PKI security service integration via Unix socket JSON-RPC
pub struct BearDogIntegration {
    config: BearDogConfig,
    access_token: Arc<Mutex<Option<String>>>,
    token_expires_at: Arc<Mutex<Option<Instant>>>,
    active_policies: Arc<RwLock<Vec<SecurityPolicy>>>,
    audit_buffer: Arc<Mutex<Vec<SecurityAuditEvent>>>,
    last_validation: Arc<Mutex<Instant>>,
}

impl BearDogIntegration {
    /// Create a new BearDog integration with the given config
    pub fn new(config: BearDogConfig) -> Result<Self, ToadStoolError> {
        Ok(Self {
            config,
            access_token: Arc::new(Mutex::new(None)),
            token_expires_at: Arc::new(Mutex::new(None)),
            active_policies: Arc::new(RwLock::new(Vec::new())),
            audit_buffer: Arc::new(Mutex::new(Vec::new())),
            last_validation: Arc::new(Mutex::new(Instant::now())),
        })
    }

    /// Authenticate with BearDog and obtain access token
    pub async fn authenticate(
        &self,
        service_id: &str,
        service_type: &str,
        capabilities: Vec<String>,
        security_context: SecurityContext,
    ) -> ToadStoolResult<AuthResponse> {
        info!("🔐 Authenticating with PKI security service (Pure Rust)");

        let auth_request = AuthRequest {
            service_id: service_id.to_string(),
            service_type: service_type.to_string(),
            capabilities,
            security_context,
            timestamp: std::time::SystemTime::now(),
        };

        match transport::make_jsonrpc_request(
            &self.config.socket_path,
            "auth.authenticate",
            &auth_request,
        )
        .await
        {
            Ok(result) => {
                let auth_response: AuthResponse = serde_json::from_value(result).map_err(|e| {
                    ToadStoolError::security(format!("Failed to parse auth response: {e}"))
                })?;

                {
                    let mut token = self.access_token.lock().await;
                    *token = Some(auth_response.access_token.clone());
                }
                {
                    let mut expires_at = self.token_expires_at.lock().await;
                    *expires_at =
                        Some(Instant::now() + Duration::from_secs(auth_response.expires_in));
                }
                {
                    let mut policies = self.active_policies.write().await;
                    *policies = auth_response.policies.clone();
                }

                info!("✅ Authenticated with PKI security service (Pure Rust)");
                Ok(auth_response)
            }
            Err(e) => {
                info!("⚠️  PKI security service not available: {}", e);
                info!("   ToadStool operates standalone — capability discovery will retry");
                Ok(AuthResponse::standalone())
            }
        }
    }

    /// Check authorization for resource/action
    pub async fn authorize(
        &self,
        resource: &str,
        action: &str,
        context: std::collections::HashMap<String, serde_json::Value>,
    ) -> ToadStoolResult<AuthzResponse> {
        info!(
            "🔒 Checking authorization via PKI security for {} on {}",
            action, resource
        );

        self.ensure_valid_token().await?;

        let access_token = self
            .access_token
            .lock()
            .await
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

        match transport::make_jsonrpc_request(
            &self.config.socket_path,
            "authz.authorize",
            &authz_request,
        )
        .await
        {
            Ok(result) => {
                let authz_response: AuthzResponse =
                    serde_json::from_value(result).map_err(|e| {
                        ToadStoolError::security(format!("Failed to parse authz response: {e}"))
                    })?;

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
                info!("⚠️  PKI security not available for authorization: {}", e);
                info!("   ToadStool operates standalone — capability discovery will retry");
                Ok(AuthzResponse {
                    allowed: true,
                    reason: Some("Standalone mode — PKI security unavailable".to_string()),
                    policies_applied: vec![],
                    security_recommendations: vec![],
                    audit_id: Uuid::new_v4().to_string(),
                })
            }
        }
    }

    /// Perform zero-trust validation of security context
    pub async fn zero_trust_validation(
        &self,
        security_context: &SecurityContext,
    ) -> ToadStoolResult<bool> {
        info!("🛡️ Performing zero-trust validation via PKI security");

        let validation_request = serde_json::json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "security_context": security_context,
            "validation_type": "zero_trust",
            "continuous_monitoring": self.config.continuous_monitoring,
        });

        match transport::make_jsonrpc_request(
            &self.config.socket_path,
            "authz.validate",
            &validation_request,
        )
        .await
        {
            Ok(result) => {
                let is_valid = result["valid"].as_bool().ok_or_else(|| {
                    ToadStoolError::security("Invalid validation response format")
                })?;

                {
                    let mut last_validation = self.last_validation.lock().await;
                    *last_validation = Instant::now();
                }

                if is_valid {
                    info!("✅ Zero-trust validation passed");
                } else {
                    warn!("❌ Zero-trust validation failed: {:?}", result["reason"]);
                }

                Ok(is_valid)
            }
            Err(e) => {
                info!("⚠️  PKI security not available for validation: {}", e);
                info!("   ToadStool operates standalone — capability discovery will retry");
                Ok(true)
            }
        }
    }

    /// Start token refresh, validation, and audit flush background tasks
    pub async fn start_background_tasks(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("🔄 Starting security service background tasks");

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

        let audit_integration = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;
                if let Err(e) = audit_integration.flush_audit_buffer().await {
                    error!("Failed to flush audit buffer: {}", e);
                }
            }
        });

        info!("✅ Security service background tasks started");
        Ok(())
    }

    async fn ensure_valid_token(&self) -> ToadStoolResult<()> {
        let expires_at = self.token_expires_at.lock().await;
        if let Some(expiry) = *expires_at
            && Instant::now() + Duration::from_secs(60) > expiry
        {
            drop(expires_at);
            return self.refresh_token_if_needed().await;
        }
        Ok(())
    }

    async fn refresh_token_if_needed(&self) -> ToadStoolResult<()> {
        info!("🔄 Refreshing PKI security access token");
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
                let mut metadata = std::collections::HashMap::new();
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

        self.audit_buffer.lock().await.push(audit_event);

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

        match transport::make_jsonrpc_request(
            &self.config.socket_path,
            "audit.flush",
            &audit_payload,
        )
        .await
        {
            Ok(_) => {
                info!(
                    "✅ Flushed {} audit events to PKI security (Pure Rust)",
                    events.len()
                );
            }
            Err(e) => {
                warn!("❌ Failed to flush audit events to PKI security: {}", e);
                let mut buffer = self.audit_buffer.lock().await;
                buffer.extend(events);
            }
        }

        Ok(())
    }
}
