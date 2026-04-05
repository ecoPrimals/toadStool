// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Comprehensive tests for Security protocol (`bear_dog.rs`)
//! Target: `crates/integration/protocols/src/bear_dog.rs`
//! No real Unix socket - tests structs, `AuthResponse::standalone`, authenticate/authorize fallbacks.

use std::collections::HashMap;

use std::sync::Arc;

use temp_env::with_var;
use toadstool::security::SecurityContext;
use toadstool_integration_protocols::{
    AuthRequest, AuthResponse, AuthzRequest, AuthzResponse, BearDogIntegration,
    BearDogIntegrationTrait, PolicyRule, SecurityAuditEvent, SecurityConfig, SecurityPolicy,
    SecurityServiceConfig,
};

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();
    assert!(!config.socket_path.is_empty());
    assert!(config.socket_path.contains("security.sock"));
    assert_eq!(config.request_timeout_secs, 30);
    assert_eq!(config.token_refresh_interval_secs, 300);
}

#[test]
fn test_security_config_env_override() {
    with_var("BEARDOG_SOCKET", Some("/custom/security.sock"), || {
        let config = SecurityConfig::default();
        assert_eq!(config.socket_path, "/custom/security.sock");
    });
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
fn test_security_new() {
    let config = SecurityConfig::default();
    let result = BearDogIntegration::new(config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_authenticate_returns_standalone_when_unreachable() {
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
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
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
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
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
    };
    let integration = BearDogIntegration::new(config).unwrap();

    let result = integration
        .zero_trust_validation(&SecurityContext::default())
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}

// ─── SecurityConfig constructor with custom values ─────────────────────────────

#[test]
fn test_security_config_custom_values() {
    let config = SecurityConfig {
        socket_path: "/custom/security.sock".to_string(),
        request_timeout_secs: 60,
        token_refresh_interval_secs: 600,
        zero_trust_validation_interval_secs: 120,
        continuous_monitoring: false,
    };
    assert_eq!(config.socket_path, "/custom/security.sock");
    assert_eq!(config.request_timeout_secs, 60);
    assert_eq!(config.token_refresh_interval_secs, 600);
    assert_eq!(config.zero_trust_validation_interval_secs, 120);
    assert!(!config.continuous_monitoring);
}

// ─── Serialization/deserialization roundtrips ───────────────────────────────

#[test]
fn test_auth_request_roundtrip() {
    let req = AuthRequest {
        service_id: "svc-1".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["encrypt".to_string()],
        security_context: SecurityContext::default(),
        timestamp: std::time::SystemTime::UNIX_EPOCH,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["service_id"], "svc-1");
    assert_eq!(json["service_type"], "compute");
    let parsed: AuthRequest = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.service_id, req.service_id);
}

#[test]
fn test_auth_response_roundtrip() {
    let resp = AuthResponse {
        access_token: "tok".to_string(),
        token_type: "bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string()],
        security_level: "standard".to_string(),
        policies: vec![],
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["access_token"], "tok");
    let parsed: AuthResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.access_token, resp.access_token);
}

#[test]
fn test_authz_request_roundtrip() {
    let mut ctx = HashMap::new();
    ctx.insert("key".to_string(), serde_json::json!("value"));
    let req = AuthzRequest {
        access_token: "tok".to_string(),
        resource: "/api".to_string(),
        action: "read".to_string(),
        context: ctx,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["resource"], "/api");
    let parsed: AuthzRequest = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.resource, req.resource);
}

#[test]
fn test_authz_response_roundtrip() {
    let resp = AuthzResponse {
        allowed: false,
        reason: Some("denied".to_string()),
        policies_applied: vec!["p1".to_string()],
        security_recommendations: vec![],
        audit_id: "a1".to_string(),
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["allowed"], false);
    let parsed: AuthzResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.allowed, resp.allowed);
}

#[test]
fn test_policy_rule_with_parameters_serialization() {
    let mut params = HashMap::new();
    params.insert("max_retries".to_string(), serde_json::json!(3));
    let rule = PolicyRule {
        condition: "resource == 'x'".to_string(),
        action: "allow".to_string(),
        parameters: params,
    };
    let json = serde_json::to_value(&rule).unwrap();
    assert_eq!(json["condition"], "resource == 'x'");
    assert_eq!(json["action"], "allow");
}

#[test]
fn test_security_policy_with_rules_serialization() {
    let policy = SecurityPolicy {
        id: "pol-1".to_string(),
        name: "Test".to_string(),
        description: "Desc".to_string(),
        rules: vec![PolicyRule {
            condition: "true".to_string(),
            action: "allow".to_string(),
            parameters: HashMap::new(),
        }],
        enforcement_level: "strict".to_string(),
        created_at: std::time::SystemTime::now(),
    };
    let json = serde_json::to_value(&policy).unwrap();
    assert_eq!(json["rules"].as_array().unwrap().len(), 1);
}

// ─── Error handling paths ────────────────────────────────────────────────────

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

// ─── Type aliases ───────────────────────────────────────────────────────────

#[test]
fn test_security_service_config_alias() {
    let _config: SecurityServiceConfig = SecurityConfig::default();
}

// ─── BearDogIntegrationTrait (trait object) ───────────────────────────────────

#[tokio::test]
async fn test_trait_authenticate() {
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
    };
    let integration: Box<dyn BearDogIntegrationTrait> =
        Box::new(BearDogIntegration::new(config).unwrap());
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
async fn test_trait_authorize_without_token() {
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
    };
    let integration: Box<dyn BearDogIntegrationTrait> =
        Box::new(BearDogIntegration::new(config).unwrap());
    let result = integration
        .authorize("/resource", "read", HashMap::new())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_trait_zero_trust_validation() {
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        ..SecurityConfig::default()
    };
    let integration: Box<dyn BearDogIntegrationTrait> =
        Box::new(BearDogIntegration::new(config).unwrap());
    let result = integration
        .zero_trust_validation(&SecurityContext::default())
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

// ─── start_background_tasks ──────────────────────────────────────────────────

#[tokio::test]
async fn test_start_background_tasks() {
    let config = SecurityConfig {
        socket_path: "/nonexistent/security.sock".to_string(),
        continuous_monitoring: false,
        ..SecurityConfig::default()
    };
    let integration = Arc::new(BearDogIntegration::new(config).unwrap());
    let result = integration.clone().start_background_tasks().await;
    assert!(result.is_ok());
}

// ─── SecurityAuditEvent serialization ────────────────────────────────────────

#[test]
fn test_security_audit_event_serialization() {
    let event = SecurityAuditEvent {
        event_id: "evt-1".to_string(),
        event_type: "auth".to_string(),
        service_id: "toadstool".to_string(),
        user_id: Some("user-1".to_string()),
        resource: "/api".to_string(),
        action: "read".to_string(),
        result: "allowed".to_string(),
        security_context: SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["event_id"], "evt-1");
    assert_eq!(json["user_id"], "user-1");
}
