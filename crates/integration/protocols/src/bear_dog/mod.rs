// SPDX-License-Identifier: AGPL-3.0-or-later
//! BearDog / PKI security service integration.
//!
//! Capability-based integration with ecosystem security services.
//! Pure Rust: Unix sockets for inter-primal communication (no reqwest).

mod auth;
mod client;
mod config;
mod policy;
mod trait_;
mod transport;

pub use auth::{AuthRequest, AuthResponse, AuthzRequest, AuthzResponse};
pub use client::BearDogIntegration;
pub use config::BearDogConfig;
pub use policy::{PolicyRule, SecurityAuditEvent, SecurityPolicy};
pub use trait_::BearDogIntegrationTrait;

pub type SecurityServiceConfig = BearDogConfig;
pub type SecurityServiceIntegration = BearDogIntegration;
pub type SecurityServiceTrait = dyn BearDogIntegrationTrait;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use toadstool::security::SecurityContext;

    #[test]
    fn bear_dog_config_default_values() {
        let config = BearDogConfig::default();
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.token_refresh_interval_secs, 300);
        assert_eq!(config.zero_trust_validation_interval_secs, 60);
        assert!(config.continuous_monitoring);
    }

    #[test]
    fn bear_dog_config_default_socket_path() {
        let config = BearDogConfig::default();
        assert!(!config.socket_path.is_empty());
        assert!(
            std::path::Path::new(&config.socket_path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
        );
    }

    #[test]
    fn auth_response_standalone_scope() {
        let resp = AuthResponse::standalone();
        assert_eq!(resp.scope, vec!["standalone"]);
        assert_eq!(resp.policies.len(), 0);
    }

    #[test]
    fn authz_response_audit_id_format() {
        let resp = AuthzResponse {
            allowed: true,
            reason: Some("test".to_string()),
            policies_applied: vec![],
            security_recommendations: vec![],
            audit_id: uuid::Uuid::new_v4().to_string(),
        };
        assert!(!resp.audit_id.is_empty());
    }

    #[test]
    fn security_policy_rules_empty() {
        let policy = SecurityPolicy {
            id: "p1".to_string(),
            name: "Test".to_string(),
            description: "Desc".to_string(),
            rules: vec![],
            enforcement_level: "strict".to_string(),
            created_at: std::time::SystemTime::now(),
        };
        assert!(policy.rules.is_empty());
    }

    #[test]
    fn policy_rule_parameters() {
        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::json!("value"));
        let rule = PolicyRule {
            condition: "cond".to_string(),
            action: "allow".to_string(),
            parameters: params,
        };
        assert_eq!(rule.parameters.len(), 1);
    }

    #[tokio::test]
    async fn authenticate_standalone_stores_token() {
        let config = BearDogConfig {
            socket_path: "/nonexistent/beardog-test.sock".to_string(),
            ..BearDogConfig::default()
        };
        let integration = BearDogIntegration::new(config).unwrap();
        let result = integration
            .authenticate(
                "test",
                "compute",
                vec!["compute".to_string()],
                SecurityContext::default(),
            )
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_standalone());
    }

    #[tokio::test]
    async fn authorize_without_token_returns_error() {
        let config = BearDogConfig {
            socket_path: "/nonexistent/beardog-authz.sock".to_string(),
            ..BearDogConfig::default()
        };
        let integration = BearDogIntegration::new(config).unwrap();
        integration
            .authenticate(
                "test",
                "compute",
                vec!["compute".to_string()],
                SecurityContext::default(),
            )
            .await
            .unwrap();
        let result = integration
            .authorize("/resource", "read", HashMap::new())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn zero_trust_validation_standalone_returns_true() {
        let config = BearDogConfig {
            socket_path: "/nonexistent/zt-validation.sock".to_string(),
            ..BearDogConfig::default()
        };
        let integration = BearDogIntegration::new(config).unwrap();
        let result = integration
            .zero_trust_validation(&SecurityContext::default())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn type_aliases_compile() {
        let _: SecurityServiceConfig = BearDogConfig::default();
    }
}
