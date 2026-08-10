// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Security Hardening Module
//!
//! This module provides production-ready security hardening features for `ToadStool`:
//! - Input validation and sanitization
//! - Rate limiting and `DDoS` protection
//! - Audit logging and intrusion detection
//! - Security context validation
//!
//! # Architecture (Feb 14, 2026 Refactor)
//!
//! Split into focused submodules for maintainability:
//! - `config` - Configuration types
//! - `rate_limiter` - Rate limiting engine
//! - `input_validator` - Input validation and sanitization
//! - `audit` - Security event logging
//! - `intrusion` - Intrusion detection system

pub mod audit;
pub mod config;
pub mod input_validator;
#[cfg(feature = "runtime")]
pub mod intrusion;
pub mod rate_limiter;

// Re-export all public types for backward compatibility
pub use audit::{SecurityAuditEvent, SecurityAuditLogger, SecurityEventType, SecuritySeverity};
pub use config::{
    AuditConfig, IntrusionDetectionConfig, RateLimitingConfig, SecurityHardeningConfig,
    ValidationRules,
};
pub use input_validator::InputValidator;
#[cfg(feature = "runtime")]
pub use intrusion::{ActivityType, IntrusionDetectionSystem};
pub use rate_limiter::RateLimiter;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use crate::security::SecurityContext;
use crate::{ToadStoolError, ToadStoolResult};

/// Security hardening manager
///
/// Orchestrates all security hardening components.
pub struct SecurityHardeningManager {
    /// Configuration
    config: SecurityHardeningConfig,
    /// Rate limiter
    rate_limiter: Arc<RateLimiter>,
    /// Input validator
    input_validator: Arc<InputValidator>,
    /// Audit logger
    audit_logger: Arc<SecurityAuditLogger>,
    /// Intrusion detection
    #[cfg(feature = "runtime")]
    intrusion_detection: Arc<IntrusionDetectionSystem>,
}

impl SecurityHardeningManager {
    /// Create new security hardening manager
    #[must_use]
    pub fn new(config: SecurityHardeningConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limiting.clone()));
        let input_validator = Arc::new(InputValidator::new(config.validation_rules.clone()));
        let audit_logger = Arc::new(SecurityAuditLogger::new(config.audit_config.clone()));
        #[cfg(feature = "runtime")]
        let intrusion_detection = Arc::new(IntrusionDetectionSystem::new(
            config.intrusion_detection.clone(),
        ));

        Self {
            config,
            rate_limiter,
            input_validator,
            audit_logger,
            #[cfg(feature = "runtime")]
            intrusion_detection,
        }
    }

    /// Check security context
    ///
    /// # Errors
    ///
    /// Returns error if the client is banned, rate limited, or the context fails validation.
    pub async fn check_security_context(
        &self,
        client_id: &str,
        context: &SecurityContext,
    ) -> ToadStoolResult<()> {
        // Check if client is banned
        #[cfg(feature = "runtime")]
        if self.config.enable_intrusion_detection
            && self.intrusion_detection.is_banned(client_id).await
        {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: crate::generate_uuid(),
                    event_type: SecurityEventType::IntrusionAttempt,
                    timestamp: SystemTime::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::High,
                })
                .await;

            return Err(ToadStoolError::security("Client is banned".to_string()));
        }

        // Check rate limit
        if self.config.enable_rate_limiting
            && !self.rate_limiter.check_rate_limit(client_id).await?
        {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: crate::generate_uuid(),
                    event_type: SecurityEventType::RateLimitExceeded,
                    timestamp: SystemTime::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::Medium,
                })
                .await;

            return Err(ToadStoolError::security("Rate limit exceeded".to_string()));
        }

        // Validate security context
        context.validate()?;

        // Record successful activity
        #[cfg(feature = "runtime")]
        if self.config.enable_intrusion_detection {
            self.intrusion_detection
                .record_activity(client_id, ActivityType::Request)
                .await;
        }

        Ok(())
    }

    /// Validate input
    ///
    /// # Errors
    ///
    /// Returns error if validation is enabled and the input violates rules.
    pub fn validate_input(&self, input: &str) -> ToadStoolResult<()> {
        if self.config.enable_input_validation {
            self.input_validator.validate_input(input)
        } else {
            Ok(())
        }
    }

    /// Sanitize input
    #[must_use]
    pub fn sanitize_input(&self, input: &str) -> String {
        if self.config.enable_input_validation {
            self.input_validator.sanitize_input(input)
        } else {
            input.to_string()
        }
    }

    /// Log security event
    pub async fn log_security_event(&self, event: SecurityAuditEvent) {
        if self.config.enable_audit_logging {
            self.audit_logger.log_event(event).await;
        }
    }

    /// Record security failure
    pub async fn record_security_failure(&self, client_id: &str, failure_type: SecurityEventType) {
        #[cfg(feature = "runtime")]
        if self.config.enable_intrusion_detection {
            self.intrusion_detection
                .record_activity(client_id, ActivityType::FailedAttempt)
                .await;
        }

        if self.config.enable_audit_logging {
            self.audit_logger
                .log_event(SecurityAuditEvent {
                    id: crate::generate_uuid(),
                    event_type: failure_type,
                    timestamp: SystemTime::now(),
                    client_id: Some(client_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    details: HashMap::new(),
                    severity: SecuritySeverity::High,
                })
                .await;
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
