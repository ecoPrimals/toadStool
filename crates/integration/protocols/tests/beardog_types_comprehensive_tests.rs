// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for BearDog Integration Types
//!
//! This test suite provides extensive coverage of BearDog security integration types
//! including authentication, authorization, and policy structures.

use std::collections::HashMap;
// Removed unused import: ServiceAuthConfig
use toadstool_integration_protocols::*;

// ============================================================================
// BearDogConfig Tests
// ============================================================================

#[test]
fn test_beardog_config_default() {
    // EVOLVED: Pure Rust Unix socket (no HTTP endpoints!)
    let config = BearDogConfig::default();

    assert!(config.socket_path.contains("beardog.sock"));
}

#[test]
fn test_beardog_config_default_timeouts() {
    let config = BearDogConfig::default();

    assert_eq!(config.request_timeout_secs, 30);
    assert_eq!(config.token_refresh_interval_secs, 300);
    assert_eq!(config.zero_trust_validation_interval_secs, 60);
}

#[test]
fn test_beardog_config_default_monitoring() {
    let config = BearDogConfig::default();
    assert!(config.continuous_monitoring);
}

#[test]
fn test_beardog_config_unix_socket_based() {
    // EVOLVED: No API tokens! Unix socket auth via file permissions
    let config = BearDogConfig::default();
    assert!(std::path::Path::new(&config.socket_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sock")));
}

#[test]
fn test_beardog_config_clone() {
    let config1 = BearDogConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.socket_path, config2.socket_path);
    assert_eq!(config1.request_timeout_secs, config2.request_timeout_secs);
}

// ============================================================================
// AuthRequest Tests
// ============================================================================

#[test]
fn test_auth_request_creation() {
    let request = AuthRequest {
        service_id: "test-service".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["execute".to_string()],
        security_context: toadstool::security::SecurityContext::default(),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(request.service_id, "test-service");
    assert_eq!(request.capabilities.len(), 1);
}

#[test]
fn test_auth_request_clone() {
    let request1 = AuthRequest {
        service_id: "test".to_string(),
        service_type: "test".to_string(),
        capabilities: vec![],
        security_context: toadstool::security::SecurityContext::default(),
        timestamp: std::time::SystemTime::now(),
    };

    let request2 = request1.clone();
    assert_eq!(request1.service_id, request2.service_id);
}

// ============================================================================
// AuthResponse Tests
// ============================================================================

#[test]
fn test_auth_response_creation() {
    let response = AuthResponse {
        access_token: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string(), "write".to_string()],
        security_level: "high".to_string(),
        policies: vec![],
    };

    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.expires_in, 3600);
    assert_eq!(response.scope.len(), 2);
}

#[test]
fn test_auth_response_with_policies() {
    let policy = SecurityPolicy {
        id: "policy-1".to_string(),
        name: "Default Policy".to_string(),
        description: "Test policy".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: std::time::SystemTime::now(),
    };

    let response = AuthResponse {
        access_token: "token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec![],
        security_level: "high".to_string(),
        policies: vec![policy],
    };

    assert_eq!(response.policies.len(), 1);
}

// ============================================================================
// AuthzRequest Tests
// ============================================================================

#[test]
fn test_authz_request_creation() {
    let request = AuthzRequest {
        access_token: "token-123".to_string(),
        resource: "/api/data".to_string(),
        action: "read".to_string(),
        context: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(request.resource, "/api/data");
    assert_eq!(request.action, "read");
}

#[test]
fn test_authz_request_with_context() {
    let mut context = HashMap::new();
    context.insert("user_id".to_string(), serde_json::json!("user-123"));
    context.insert("ip_address".to_string(), serde_json::json!("192.168.1.1"));

    let request = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/api/data".to_string(),
        action: "write".to_string(),
        context,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(request.context.len(), 2);
}

// ============================================================================
// AuthzResponse Tests
// ============================================================================

#[test]
fn test_authz_response_allowed() {
    let response = AuthzResponse {
        allowed: true,
        reason: None,
        policies_applied: vec!["policy-1".to_string()],
        security_recommendations: vec![],
        audit_id: "audit-123".to_string(),
    };

    assert!(response.allowed);
    assert_eq!(response.policies_applied.len(), 1);
}

#[test]
fn test_authz_response_denied_with_reason() {
    let response = AuthzResponse {
        allowed: false,
        reason: Some("insufficient permissions".to_string()),
        policies_applied: vec![],
        security_recommendations: vec!["Request elevated access".to_string()],
        audit_id: "audit-456".to_string(),
    };

    assert!(!response.allowed);
    assert!(response.reason.is_some());
    assert_eq!(response.security_recommendations.len(), 1);
}

// ============================================================================
// SecurityPolicy Tests
// ============================================================================

#[test]
fn test_security_policy_creation() {
    let policy = SecurityPolicy {
        id: "policy-1".to_string(),
        name: "Data Access Policy".to_string(),
        description: "Controls data access".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: std::time::SystemTime::now(),
    };

    assert_eq!(policy.id, "policy-1");
    assert_eq!(policy.enforcement_level, "strict");
}

#[test]
fn test_security_policy_with_rules() {
    let rule = PolicyRule {
        condition: "resource == '/api/data'".to_string(),
        action: "allow".to_string(),
        parameters: HashMap::new(),
    };

    let policy = SecurityPolicy {
        id: "policy-2".to_string(),
        name: "API Policy".to_string(),
        description: "API access rules".to_string(),
        rules: vec![rule],
        enforcement_level: "moderate".to_string(),
        created_at: std::time::SystemTime::now(),
    };

    assert_eq!(policy.rules.len(), 1);
}

// ============================================================================
// PolicyRule Tests
// ============================================================================

#[test]
fn test_policy_rule_creation() {
    let rule = PolicyRule {
        condition: "time >= 09:00 AND time <= 17:00".to_string(),
        action: "allow".to_string(),
        parameters: HashMap::new(),
    };

    assert_eq!(rule.action, "allow");
}

#[test]
fn test_policy_rule_with_parameters() {
    let mut params = HashMap::new();
    params.insert("max_attempts".to_string(), serde_json::json!(3));
    params.insert("timeout".to_string(), serde_json::json!(300));

    let rule = PolicyRule {
        condition: "attempts < max_attempts".to_string(),
        action: "allow".to_string(),
        parameters: params,
    };

    assert_eq!(rule.parameters.len(), 2);
}

// ============================================================================
// SecurityAuditEvent Tests
// ============================================================================

#[test]
fn test_security_audit_event_creation() {
    let event = SecurityAuditEvent {
        event_id: "event-123".to_string(),
        event_type: "authorization".to_string(),
        service_id: "service-1".to_string(),
        user_id: Some("user-456".to_string()),
        resource: "/api/data".to_string(),
        action: "read".to_string(),
        result: "allowed".to_string(),
        security_context: toadstool::security::SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(event.event_type, "authorization");
    assert_eq!(event.result, "allowed");
}

#[test]
fn test_security_audit_event_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("ip_address".to_string(), serde_json::json!("192.168.1.1"));
    metadata.insert("user_agent".to_string(), serde_json::json!("ToadStool/1.0"));

    let event = SecurityAuditEvent {
        event_id: "event-789".to_string(),
        event_type: "authentication".to_string(),
        service_id: "service-2".to_string(),
        user_id: None,
        resource: "/auth".to_string(),
        action: "login".to_string(),
        result: "success".to_string(),
        security_context: toadstool::security::SecurityContext::default(),
        metadata,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(event.metadata.len(), 2);
    assert!(event.user_id.is_none());
}

#[test]
fn test_security_audit_event_clone() {
    let event1 = SecurityAuditEvent {
        event_id: "event-1".to_string(),
        event_type: "test".to_string(),
        service_id: "service-1".to_string(),
        user_id: None,
        resource: "/test".to_string(),
        action: "test".to_string(),
        result: "success".to_string(),
        security_context: toadstool::security::SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let event2 = event1.clone();
    assert_eq!(event1.event_id, event2.event_id);
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_beardog_types_coverage_summary() {
    println!("============================================");
    println!("BearDog Types Tests Summary:");
    println!("============================================");
    println!("BearDogConfig:           5 tests");
    println!("AuthRequest:             2 tests");
    println!("AuthResponse:            2 tests");
    println!("AuthzRequest:            2 tests");
    println!("AuthzResponse:           2 tests");
    println!("SecurityPolicy:          2 tests");
    println!("PolicyRule:              2 tests");
    println!("SecurityAuditEvent:      3 tests");
    println!("============================================");
    println!("Total BearDog Tests:    20 tests");
    println!("============================================");
}
