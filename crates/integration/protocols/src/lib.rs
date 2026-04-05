// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::must_use_candidate,
    clippy::unused_async,
    clippy::needless_pass_by_value,
    clippy::assigning_clones,
    clippy::unreadable_literal,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::missing_errors_doc
)]

//! # `ToadStool` Protocol Integration Layer
//!
//! Capability-based integration with ecosystem security services.
//! ToadStool discovers PKI/auth capabilities at runtime — no baked-in primal names.
//!
//! Pure Rust: Unix sockets for inter-service communication (no reqwest!)

// Sub-modules
pub mod bear_dog;
pub mod client;
pub mod config;
pub mod tarpc_service;
pub mod transport;
pub mod types;

// Re-export Security / PKI security types for backward compatibility
pub use bear_dog::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use toadstool::security::SecurityContext;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.socket_path.contains("security") || config.socket_path.contains("sock"));
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.token_refresh_interval_secs, 300);
        assert_eq!(config.zero_trust_validation_interval_secs, 60);
        assert!(config.continuous_monitoring);
    }

    #[test]
    fn test_auth_response_standalone() {
        let resp = AuthResponse::standalone();
        assert!(resp.is_standalone());
        assert_eq!(resp.access_token, "standalone");
        assert_eq!(resp.token_type, "bearer");
        assert_eq!(resp.expires_in, 3600);
        assert_eq!(resp.security_level, "standard");
    }

    #[test]
    fn test_auth_response_is_standalone_false() {
        let resp = AuthResponse {
            access_token: "real-token".to_string(),
            token_type: "bearer".to_string(),
            expires_in: 3600,
            scope: vec!["read".to_string()],
            security_level: "enhanced".to_string(),
            policies: vec![],
        };
        assert!(!resp.is_standalone());
    }

    #[test]
    fn test_security_new() {
        let config = SecurityConfig::default();
        let result = SecurityServiceIntegration::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_rule_serialization() {
        let rule = PolicyRule {
            condition: "resource == 'x'".to_string(),
            action: "allow".to_string(),
            parameters: HashMap::new(),
        };
        let json = serde_json::to_value(&rule).unwrap();
        assert_eq!(json["condition"], "resource == 'x'");
        assert_eq!(json["action"], "allow");
    }

    #[test]
    fn test_security_policy_serialization() {
        let policy = SecurityPolicy {
            id: "pol-1".to_string(),
            name: "Test Policy".to_string(),
            description: "Policy desc".to_string(),
            rules: vec![],
            enforcement_level: "strict".to_string(),
            created_at: std::time::SystemTime::now(),
        };
        let json = serde_json::to_value(&policy).unwrap();
        assert_eq!(json["id"], "pol-1");
        assert_eq!(json["name"], "Test Policy");
    }

    #[test]
    fn test_auth_request_serialization() {
        let req = AuthRequest {
            service_id: "svc-1".to_string(),
            service_type: "compute".to_string(),
            capabilities: vec!["encrypt".to_string()],
            security_context: SecurityContext::default(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["service_id"], "svc-1");
        assert_eq!(json["service_type"], "compute");
    }

    #[test]
    fn test_authz_response_serialization() {
        let resp = AuthzResponse {
            allowed: true,
            reason: Some("ok".to_string()),
            policies_applied: vec!["p1".to_string()],
            security_recommendations: vec![],
            audit_id: "audit-1".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["allowed"], true);
        assert_eq!(json["audit_id"], "audit-1");
    }

    #[test]
    fn test_security_audit_event_serialization() {
        let event = SecurityAuditEvent {
            event_id: "evt-1".to_string(),
            event_type: "auth".to_string(),
            service_id: "toadstool".to_string(),
            user_id: None,
            resource: "/api".to_string(),
            action: "read".to_string(),
            result: "allowed".to_string(),
            security_context: SecurityContext::default(),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event_id"], "evt-1");
        assert_eq!(json["result"], "allowed");
    }

    #[test]
    fn test_security_service_config_alias() {
        let _config: SecurityServiceConfig = SecurityConfig::default();
    }

    #[test]
    fn test_authz_request_roundtrip() {
        let req = AuthzRequest {
            access_token: "tok".to_string(),
            resource: "/api".to_string(),
            action: "read".to_string(),
            context: HashMap::new(),
            timestamp: std::time::SystemTime::UNIX_EPOCH,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["access_token"], "tok");
        assert_eq!(json["resource"], "/api");
        assert_eq!(json["action"], "read");
    }

    #[test]
    fn test_authz_response_roundtrip() {
        let resp = AuthzResponse {
            allowed: false,
            reason: Some("denied".to_string()),
            policies_applied: vec![],
            security_recommendations: vec![],
            audit_id: "a1".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["allowed"], false);
        assert_eq!(json["audit_id"], "a1");
    }

    #[test]
    fn test_security_config_env_socket_path() {
        temp_env::with_var("BEARDOG_SOCKET", Some("/custom/security.sock"), || {
            let config = SecurityConfig::default();
            assert_eq!(config.socket_path, "/custom/security.sock");
        });
    }

    #[test]
    fn test_auth_request_roundtrip() {
        let req = AuthRequest {
            service_id: "svc".to_string(),
            service_type: "compute".to_string(),
            capabilities: vec!["encrypt".to_string()],
            security_context: SecurityContext::default(),
            timestamp: std::time::SystemTime::UNIX_EPOCH,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["service_id"], "svc");
        assert_eq!(json["service_type"], "compute");
    }

    #[tokio::test]
    async fn test_security_authenticate_standalone_when_unavailable() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            ..SecurityConfig::default()
        };
        let integration = SecurityServiceIntegration::new(config).expect("new");
        let result = integration
            .authenticate(
                "test-svc",
                "compute",
                vec!["encrypt".to_string()],
                SecurityContext::default(),
            )
            .await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.is_standalone());
    }

    #[tokio::test]
    async fn test_security_authorize_fails_without_token() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            ..SecurityConfig::default()
        };
        let integration = SecurityServiceIntegration::new(config).expect("new");
        let result = integration
            .authorize("/api/resource", "read", HashMap::new())
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("token"));
    }

    #[tokio::test]
    async fn test_security_zero_trust_validation_standalone_when_unavailable() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            ..SecurityConfig::default()
        };
        let integration = SecurityServiceIntegration::new(config).expect("new");
        let result = integration
            .zero_trust_validation(&SecurityContext::default())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_security_config_custom_values() {
        let config = SecurityConfig {
            socket_path: "/custom/sock".to_string(),
            request_timeout_secs: 60,
            token_refresh_interval_secs: 600,
            zero_trust_validation_interval_secs: 120,
            continuous_monitoring: false,
        };
        assert_eq!(config.socket_path, "/custom/sock");
        assert_eq!(config.request_timeout_secs, 60);
        assert!(!config.continuous_monitoring);
    }

    #[test]
    fn test_authz_response_denied() {
        let resp = AuthzResponse {
            allowed: false,
            reason: Some("Access denied".to_string()),
            policies_applied: vec!["strict".to_string()],
            security_recommendations: vec![],
            audit_id: "audit-123".to_string(),
        };
        assert!(!resp.allowed);
        assert_eq!(resp.reason.as_deref(), Some("Access denied"));
    }

    #[test]
    fn test_security_policy_enforcement_level() {
        let policy = SecurityPolicy {
            id: "p1".to_string(),
            name: "Strict".to_string(),
            description: "Strict policy".to_string(),
            rules: vec![],
            enforcement_level: "strict".to_string(),
            created_at: std::time::SystemTime::now(),
        };
        assert_eq!(policy.enforcement_level, "strict");
    }

    #[tokio::test]
    async fn test_security_start_background_tasks() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            continuous_monitoring: false,
            ..SecurityConfig::default()
        };
        let integration = Arc::new(SecurityServiceIntegration::new(config).expect("new"));
        let result = integration.clone().start_background_tasks().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_security_trait_authenticate() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            ..SecurityConfig::default()
        };
        let integration: Box<dyn SecurityServiceIntegrationTrait> =
            Box::new(SecurityServiceIntegration::new(config).expect("new"));
        let result = integration
            .authenticate(
                "svc",
                "compute",
                vec!["encrypt".to_string()],
                SecurityContext::default(),
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_standalone());
    }

    #[tokio::test]
    async fn test_security_trait_zero_trust_validation() {
        let config = SecurityConfig {
            socket_path: "/nonexistent/security.sock".to_string(),
            ..SecurityConfig::default()
        };
        let integration: Box<dyn SecurityServiceIntegrationTrait> =
            Box::new(SecurityServiceIntegration::new(config).expect("new"));
        let result = integration
            .zero_trust_validation(&SecurityContext::default())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_authz_request_serialization() {
        let req = AuthzRequest {
            access_token: "tok".to_string(),
            resource: "/api".to_string(),
            action: "read".to_string(),
            context: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["access_token"], "tok");
        assert_eq!(json["resource"], "/api");
    }
}
