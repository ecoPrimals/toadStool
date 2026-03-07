// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for BearDog protocol (`bear_dog.rs`)
//! Target: `crates/integration/protocols/src/bear_dog.rs`
//! No real Unix socket - tests structs, `AuthResponse::standalone`, authenticate/authorize fallbacks.

use std::collections::HashMap;

use toadstool::security::SecurityContext;
use toadstool_integration_protocols::{
    AuthRequest, AuthResponse, AuthzRequest, AuthzResponse, BearDogConfig, BearDogIntegration,
    PolicyRule, SecurityAuditEvent, SecurityPolicy,
};

#[test]
fn test_beardog_config_default() {
    let config = BearDogConfig::default();
    assert!(!config.socket_path.is_empty());
    assert!(config.socket_path.contains("beardog") || config.socket_path.contains("sock"));
    assert_eq!(config.request_timeout_secs, 30);
    assert_eq!(config.token_refresh_interval_secs, 300);
}

#[test]
fn test_beardog_config_env_override() {
    let old = std::env::var("BEARDOG_SOCKET").ok();
    std::env::set_var("BEARDOG_SOCKET", "/custom/beardog.sock");

    let config = BearDogConfig::default();
    assert_eq!(config.socket_path, "/custom/beardog.sock");

    if let Some(v) = old {
        std::env::set_var("BEARDOG_SOCKET", v);
    } else {
        std::env::remove_var("BEARDOG_SOCKET");
    }
}

#[test]
fn test_auth_response_standalone() {
    let resp = AuthResponse::standalone();
    assert!(resp.is_standalone());
    assert_eq!(resp.access_token, "standalone");
    assert_eq!(resp.token_type, "bearer");
    assert_eq!(resp.security_level, "standard");
}

#[test]
fn test_auth_response_not_standalone() {
    let resp = AuthResponse {
        access_token: "real-token".to_string(),
        token_type: "bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string()],
        security_level: "elevated".to_string(),
        policies: vec![],
    };
    assert!(!resp.is_standalone());
}

#[test]
fn test_auth_request_serialization() {
    let req = AuthRequest {
        service_id: "toadstool".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["compute".to_string()],
        security_context: SecurityContext::default(),
        timestamp: std::time::SystemTime::now(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("toadstool"));
}

#[test]
fn test_authz_request_serialization() {
    let req = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/workloads".to_string(),
        action: "create".to_string(),
        context: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("create"));
}

#[test]
fn test_authz_response_serialization() {
    let resp = AuthzResponse {
        allowed: true,
        reason: None,
        policies_applied: vec!["policy1".to_string()],
        security_recommendations: vec![],
        audit_id: "audit-1".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("allowed"));
}

#[test]
fn test_policy_rule_creation() {
    let rule = PolicyRule {
        condition: "resource == /workloads".to_string(),
        action: "allow".to_string(),
        parameters: HashMap::new(),
    };
    assert_eq!(rule.action, "allow");
}

#[test]
fn test_security_policy_creation() {
    let policy = SecurityPolicy {
        id: "pol-1".to_string(),
        name: "Test Policy".to_string(),
        description: "Test".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: std::time::SystemTime::now(),
    };
    assert_eq!(policy.enforcement_level, "strict");
}

#[test]
fn test_security_audit_event_creation() {
    let event = SecurityAuditEvent {
        event_id: "evt-1".to_string(),
        event_type: "auth".to_string(),
        service_id: "toadstool".to_string(),
        user_id: None,
        resource: "/".to_string(),
        action: "read".to_string(),
        result: "allowed".to_string(),
        security_context: SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };
    assert_eq!(event.result, "allowed");
}

#[test]
fn test_beardog_integration_new() {
    let config = BearDogConfig::default();
    let result = BearDogIntegration::new(config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_authenticate_returns_standalone_when_unreachable() {
    let config = BearDogConfig {
        socket_path: "/nonexistent/beardog.sock".to_string(),
        ..BearDogConfig::default()
    };
    let integration = BearDogIntegration::new(config).unwrap();

    let result = integration
        .authenticate(
            "toadstool",
            "compute",
            vec!["compute".to_string()],
            SecurityContext::default(),
        )
        .await;

    assert!(result.is_ok());
    let resp = result.unwrap();
    assert!(resp.is_standalone());
}

#[tokio::test]
async fn test_authorize_without_token_returns_err() {
    let config = BearDogConfig {
        socket_path: "/nonexistent/beardog.sock".to_string(),
        ..BearDogConfig::default()
    };
    let integration = BearDogIntegration::new(config).unwrap();

    // Authorize without authenticating first - no token
    let result = integration
        .authorize("/workloads", "create", HashMap::new())
        .await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("token") || err_str.contains("Token"));
}

#[tokio::test]
async fn test_zero_trust_validation_returns_true_when_unreachable() {
    let config = BearDogConfig {
        socket_path: "/nonexistent/beardog.sock".to_string(),
        ..BearDogConfig::default()
    };
    let integration = BearDogIntegration::new(config).unwrap();

    let result = integration
        .zero_trust_validation(&SecurityContext::default())
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}
