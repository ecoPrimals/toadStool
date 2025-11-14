//! Comprehensive tests for Integration Protocol types
//!
//! Week 17 Sprint 5: Integration Protocol types and structures tests
//! Target: ~20 tests

use chrono::Utc;
use std::collections::HashMap;
use toadstool::security::SecurityContext;
use toadstool_integration_protocols::*;

// ============================================================================
// BearDogConfig Tests (4 tests)
// ============================================================================

#[test]
fn test_beardog_config_default() {
    let config = BearDogConfig::default();

    assert!(config.auth_endpoint.contains("/auth"));
    assert!(config.authz_endpoint.contains("/authz"));
    assert!(config.policy_endpoint.contains("/policy"));
    assert!(config.audit_endpoint.contains("/audit"));
    assert_eq!(config.request_timeout_secs, 30);
    assert_eq!(config.token_refresh_interval_secs, 300);
    assert_eq!(config.zero_trust_validation_interval_secs, 60);
    assert!(config.continuous_monitoring);
}

#[test]
fn test_beardog_config_clone() {
    let config1 = BearDogConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.auth_endpoint, config2.auth_endpoint);
    assert_eq!(config1.request_timeout_secs, config2.request_timeout_secs);
    assert_eq!(config1.continuous_monitoring, config2.continuous_monitoring);
}

#[test]
fn test_beardog_config_debug() {
    let config = BearDogConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("BearDogConfig"));
    assert!(debug_str.contains("auth_endpoint"));
}

#[test]
fn test_beardog_config_with_api_token() {
    let config = BearDogConfig {
        api_token: Some("test-token-123".to_string()),
        ..Default::default()
    };

    assert_eq!(config.api_token, Some("test-token-123".to_string()));
}

// ============================================================================
// AuthRequest Tests (4 tests)
// ============================================================================

#[test]
fn test_auth_request_creation() {
    let auth_request = AuthRequest {
        service_id: "toadstool-123".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["wasm".to_string(), "native".to_string()],
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    assert_eq!(auth_request.service_id, "toadstool-123");
    assert_eq!(auth_request.service_type, "compute");
    assert_eq!(auth_request.capabilities.len(), 2);
}

#[test]
fn test_auth_request_clone() {
    let auth_request1 = AuthRequest {
        service_id: "test".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["wasm".to_string()],
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    let auth_request2 = auth_request1.clone();
    assert_eq!(auth_request1.service_id, auth_request2.service_id);
    assert_eq!(auth_request1.capabilities, auth_request2.capabilities);
}

#[test]
fn test_auth_request_debug() {
    let auth_request = AuthRequest {
        service_id: "test".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec![],
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    let debug_str = format!("{:?}", auth_request);
    assert!(debug_str.contains("AuthRequest"));
}

#[test]
fn test_auth_request_with_multiple_capabilities() {
    let capabilities = vec![
        "wasm".to_string(),
        "native".to_string(),
        "container".to_string(),
        "gpu".to_string(),
        "python".to_string(),
    ];

    let auth_request = AuthRequest {
        service_id: "toadstool-multi".to_string(),
        service_type: "universal-compute".to_string(),
        capabilities: capabilities.clone(),
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    assert_eq!(auth_request.capabilities.len(), 5);
    assert!(auth_request.capabilities.contains(&"gpu".to_string()));
}

// ============================================================================
// AuthResponse Tests (3 tests)
// ============================================================================

#[test]
fn test_auth_response_creation() {
    let auth_response = AuthResponse {
        access_token: "token-abc-123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string(), "write".to_string()],
        security_level: "strict".to_string(),
        policies: vec![],
    };

    assert_eq!(auth_response.access_token, "token-abc-123");
    assert_eq!(auth_response.token_type, "Bearer");
    assert_eq!(auth_response.expires_in, 3600);
    assert_eq!(auth_response.scope.len(), 2);
}

#[test]
fn test_auth_response_with_policies() {
    let policy = SecurityPolicy {
        id: "policy-1".to_string(),
        name: "Test Policy".to_string(),
        description: "Test description".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: Utc::now(),
    };

    let auth_response = AuthResponse {
        access_token: "token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec!["admin".to_string()],
        security_level: "maximum".to_string(),
        policies: vec![policy],
    };

    assert_eq!(auth_response.policies.len(), 1);
    assert_eq!(auth_response.security_level, "maximum");
}

#[test]
fn test_auth_response_clone_debug() {
    let auth_response1 = AuthResponse {
        access_token: "token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 1800,
        scope: vec!["read".to_string()],
        security_level: "basic".to_string(),
        policies: vec![],
    };

    let auth_response2 = auth_response1.clone();
    assert_eq!(auth_response1.access_token, auth_response2.access_token);

    let debug_str = format!("{:?}", auth_response1);
    assert!(debug_str.contains("AuthResponse"));
}

// ============================================================================
// AuthzRequest Tests (3 tests)
// ============================================================================

#[test]
fn test_authz_request_creation() {
    let authz_request = AuthzRequest {
        access_token: "token-123".to_string(),
        resource: "/api/workloads".to_string(),
        action: "create".to_string(),
        context: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert_eq!(authz_request.resource, "/api/workloads");
    assert_eq!(authz_request.action, "create");
}

#[test]
fn test_authz_request_with_context() {
    let mut context = HashMap::new();
    context.insert("user_id".to_string(), serde_json::json!("user-123"));
    context.insert("ip_address".to_string(), serde_json::json!("192.168.1.1"));
    context.insert("severity".to_string(), serde_json::json!(5));

    let authz_request = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/api/admin".to_string(),
        action: "delete".to_string(),
        context: context.clone(),
        timestamp: Utc::now(),
    };

    assert_eq!(authz_request.context.len(), 3);
    assert_eq!(
        authz_request.context.get("user_id").unwrap(),
        &serde_json::json!("user-123")
    );
}

#[test]
fn test_authz_request_clone_debug() {
    let authz_request1 = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/resource".to_string(),
        action: "read".to_string(),
        context: HashMap::new(),
        timestamp: Utc::now(),
    };

    let authz_request2 = authz_request1.clone();
    assert_eq!(authz_request1.resource, authz_request2.resource);

    let debug_str = format!("{:?}", authz_request1);
    assert!(debug_str.contains("AuthzRequest"));
}

// ============================================================================
// AuthzResponse Tests (3 tests)
// ============================================================================

#[test]
fn test_authz_response_allowed() {
    let authz_response = AuthzResponse {
        allowed: true,
        reason: None,
        policies_applied: vec!["policy-1".to_string(), "policy-2".to_string()],
        security_recommendations: vec![],
        audit_id: "audit-123".to_string(),
    };

    assert!(authz_response.allowed);
    assert!(authz_response.reason.is_none());
    assert_eq!(authz_response.policies_applied.len(), 2);
}

#[test]
fn test_authz_response_denied() {
    let authz_response = AuthzResponse {
        allowed: false,
        reason: Some("Insufficient permissions".to_string()),
        policies_applied: vec!["deny-policy".to_string()],
        security_recommendations: vec!["Contact admin".to_string()],
        audit_id: "audit-456".to_string(),
    };

    assert!(!authz_response.allowed);
    assert_eq!(
        authz_response.reason,
        Some("Insufficient permissions".to_string())
    );
    assert_eq!(authz_response.security_recommendations.len(), 1);
}

#[test]
fn test_authz_response_clone_debug() {
    let authz_response1 = AuthzResponse {
        allowed: true,
        reason: None,
        policies_applied: vec![],
        security_recommendations: vec![],
        audit_id: "audit-789".to_string(),
    };

    let authz_response2 = authz_response1.clone();
    assert_eq!(authz_response1.allowed, authz_response2.allowed);

    let debug_str = format!("{:?}", authz_response1);
    assert!(debug_str.contains("AuthzResponse"));
}

// ============================================================================
// SecurityPolicy & PolicyRule Tests (4 tests)
// ============================================================================

#[test]
fn test_security_policy_creation() {
    let policy = SecurityPolicy {
        id: "policy-secure-123".to_string(),
        name: "Strict Security Policy".to_string(),
        description: "Enforces strict security controls".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: Utc::now(),
    };

    assert_eq!(policy.id, "policy-secure-123");
    assert_eq!(policy.enforcement_level, "strict");
}

#[test]
fn test_policy_rule_creation() {
    let mut params = HashMap::new();
    params.insert("max_attempts".to_string(), serde_json::json!(3));
    params.insert("lockout_duration".to_string(), serde_json::json!(300));

    let rule = PolicyRule {
        condition: "failed_login_attempts > max_attempts".to_string(),
        action: "lock_account".to_string(),
        parameters: params.clone(),
    };

    assert_eq!(rule.action, "lock_account");
    assert_eq!(rule.parameters.len(), 2);
}

#[test]
fn test_security_policy_with_rules() {
    let rule1 = PolicyRule {
        condition: "time_of_day == night".to_string(),
        action: "require_mfa".to_string(),
        parameters: HashMap::new(),
    };

    let rule2 = PolicyRule {
        condition: "ip_address not in allowlist".to_string(),
        action: "deny_access".to_string(),
        parameters: HashMap::new(),
    };

    let policy = SecurityPolicy {
        id: "multi-rule-policy".to_string(),
        name: "Multi-Rule Policy".to_string(),
        description: "Policy with multiple rules".to_string(),
        rules: vec![rule1, rule2],
        enforcement_level: "maximum".to_string(),
        created_at: Utc::now(),
    };

    assert_eq!(policy.rules.len(), 2);
    assert_eq!(policy.rules[0].action, "require_mfa");
    assert_eq!(policy.rules[1].action, "deny_access");
}

#[test]
fn test_security_policy_clone_debug() {
    let policy1 = SecurityPolicy {
        id: "test-policy".to_string(),
        name: "Test".to_string(),
        description: "Test policy".to_string(),
        rules: vec![],
        enforcement_level: "basic".to_string(),
        created_at: Utc::now(),
    };

    let policy2 = policy1.clone();
    assert_eq!(policy1.id, policy2.id);

    let debug_str = format!("{:?}", policy1);
    assert!(debug_str.contains("SecurityPolicy"));
}

// ============================================================================
// SecurityAuditEvent Tests (3 tests)
// ============================================================================

#[test]
fn test_security_audit_event_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("ip_address".to_string(), serde_json::json!("10.0.0.1"));
    metadata.insert("user_agent".to_string(), serde_json::json!("ToadStool/1.0"));

    let audit_event = SecurityAuditEvent {
        event_id: "event-123".to_string(),
        event_type: "authorization_decision".to_string(),
        service_id: "toadstool-compute".to_string(),
        user_id: Some("user-456".to_string()),
        resource: "/api/workloads".to_string(),
        action: "create".to_string(),
        result: "allowed".to_string(),
        security_context: SecurityContext::default(),
        metadata: metadata.clone(),
        timestamp: Utc::now(),
    };

    assert_eq!(audit_event.event_type, "authorization_decision");
    assert_eq!(audit_event.result, "allowed");
    assert_eq!(audit_event.metadata.len(), 2);
    assert!(audit_event.user_id.is_some());
}

#[test]
fn test_security_audit_event_denied() {
    let audit_event = SecurityAuditEvent {
        event_id: "event-deny-789".to_string(),
        event_type: "authentication_failure".to_string(),
        service_id: "toadstool".to_string(),
        user_id: None,
        resource: "/api/admin".to_string(),
        action: "access".to_string(),
        result: "denied".to_string(),
        security_context: SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert_eq!(audit_event.result, "denied");
    assert!(audit_event.user_id.is_none());
}

#[test]
fn test_security_audit_event_clone_debug() {
    let audit_event1 = SecurityAuditEvent {
        event_id: "event-1".to_string(),
        event_type: "test".to_string(),
        service_id: "service".to_string(),
        user_id: Some("user".to_string()),
        resource: "/resource".to_string(),
        action: "action".to_string(),
        result: "success".to_string(),
        security_context: SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    let audit_event2 = audit_event1.clone();
    assert_eq!(audit_event1.event_id, audit_event2.event_id);

    let debug_str = format!("{:?}", audit_event1);
    assert!(debug_str.contains("SecurityAuditEvent"));
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_protocol_types_coverage_summary() {
    println!("=== Integration Protocol Types Test Coverage ===");
    println!("BearDogConfig Tests:              4 tests");
    println!("AuthRequest Tests:                4 tests");
    println!("AuthResponse Tests:               3 tests");
    println!("AuthzRequest Tests:               3 tests");
    println!("AuthzResponse Tests:              3 tests");
    println!("SecurityPolicy & Rules Tests:     4 tests");
    println!("SecurityAuditEvent Tests:         3 tests");
    println!("──────────────────────────────────────────");
    println!("Total:                           24 tests");
    println!("Module Coverage:                  Expanded");
    println!("================================================");
}
