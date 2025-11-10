//! Comprehensive tests for BearDog integration
//!
//! These tests cover the BearDog security integration layer,
//! including authentication, authorization, and audit functionality.

use chrono::Utc;
use std::collections::HashMap;
use toadstool::security::SecurityContext;
use toadstool_integration_protocols::*;
use uuid::Uuid;

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_beardog_config_default() {
    let config = BearDogConfig::default();

    assert!(config.auth_endpoint.contains("/auth"));
    assert!(config.authz_endpoint.contains("/authz"));
    assert!(config.policy_endpoint.contains("/policy"));
    assert!(config.audit_endpoint.contains("/audit"));
    assert_eq!(config.api_token, None);
    assert_eq!(config.request_timeout_secs, 30);
    assert_eq!(config.token_refresh_interval_secs, 300);
    assert_eq!(config.zero_trust_validation_interval_secs, 60);
    assert!(config.continuous_monitoring);
}

#[test]
fn test_beardog_config_custom() {
    let config = BearDogConfig {
        auth_endpoint: "https://beardog.example.com/auth".to_string(),
        authz_endpoint: "https://beardog.example.com/authz".to_string(),
        policy_endpoint: "https://beardog.example.com/policy".to_string(),
        audit_endpoint: "https://beardog.example.com/audit".to_string(),
        api_token: Some("test-token-123".to_string()),
        request_timeout_secs: 60,
        token_refresh_interval_secs: 600,
        zero_trust_validation_interval_secs: 120,
        continuous_monitoring: false,
    };

    assert_eq!(config.auth_endpoint, "https://beardog.example.com/auth");
    assert_eq!(config.api_token, Some("test-token-123".to_string()));
    assert_eq!(config.request_timeout_secs, 60);
    assert!(!config.continuous_monitoring);
}

#[test]
fn test_beardog_config_serialization() {
    let config = BearDogConfig::default();
    let serialized = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: BearDogConfig =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(config.auth_endpoint, deserialized.auth_endpoint);
    assert_eq!(
        config.request_timeout_secs,
        deserialized.request_timeout_secs
    );
}

// ============================================================================
// Authentication Request/Response Tests
// ============================================================================

#[test]
fn test_auth_request_creation() {
    let auth_request = AuthRequest {
        service_id: "toadstool-1".to_string(),
        service_type: "compute".to_string(),
        capabilities: vec!["execute".to_string(), "schedule".to_string()],
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    assert_eq!(auth_request.service_id, "toadstool-1");
    assert_eq!(auth_request.service_type, "compute");
    assert_eq!(auth_request.capabilities.len(), 2);
}

#[test]
fn test_auth_request_serialization() {
    let auth_request = AuthRequest {
        service_id: "test-service".to_string(),
        service_type: "test".to_string(),
        capabilities: vec!["test-cap".to_string()],
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    let serialized = serde_json::to_string(&auth_request).expect("Failed to serialize");
    let deserialized: AuthRequest =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(auth_request.service_id, deserialized.service_id);
    assert_eq!(auth_request.capabilities, deserialized.capabilities);
}

#[test]
fn test_auth_response_creation() {
    let auth_response = AuthResponse {
        access_token: "token-abc-123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string(), "write".to_string()],
        security_level: "high".to_string(),
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
        description: "A test security policy".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: Utc::now(),
    };

    let auth_response = AuthResponse {
        access_token: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 7200,
        scope: vec!["execute".to_string()],
        security_level: "high".to_string(),
        policies: vec![policy.clone()],
    };

    assert_eq!(auth_response.policies.len(), 1);
    assert_eq!(auth_response.policies[0].id, "policy-1");
}

// ============================================================================
// Authorization Request/Response Tests
// ============================================================================

#[test]
fn test_authz_request_creation() {
    let mut context = HashMap::new();
    context.insert("ip_address".to_string(), serde_json::json!("192.168.1.1"));
    context.insert("user_agent".to_string(), serde_json::json!("ToadStool/1.0"));

    let authz_request = AuthzRequest {
        access_token: "token-xyz".to_string(),
        resource: "/api/compute/execute".to_string(),
        action: "POST".to_string(),
        context,
        timestamp: Utc::now(),
    };

    assert_eq!(authz_request.resource, "/api/compute/execute");
    assert_eq!(authz_request.action, "POST");
    assert_eq!(authz_request.context.len(), 2);
}

#[test]
fn test_authz_request_serialization() {
    let authz_request = AuthzRequest {
        access_token: "test-token".to_string(),
        resource: "/test/resource".to_string(),
        action: "READ".to_string(),
        context: HashMap::new(),
        timestamp: Utc::now(),
    };

    let serialized = serde_json::to_string(&authz_request).expect("Failed to serialize");
    let deserialized: AuthzRequest =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(authz_request.resource, deserialized.resource);
    assert_eq!(authz_request.action, deserialized.action);
}

#[test]
fn test_authz_response_allowed() {
    let authz_response = AuthzResponse {
        allowed: true,
        reason: None,
        policies_applied: vec!["policy-1".to_string(), "policy-2".to_string()],
        security_recommendations: vec![],
        audit_id: Uuid::new_v4().to_string(),
    };

    assert!(authz_response.allowed);
    assert_eq!(authz_response.reason, None);
    assert_eq!(authz_response.policies_applied.len(), 2);
}

#[test]
fn test_authz_response_denied() {
    let authz_response = AuthzResponse {
        allowed: false,
        reason: Some("Insufficient permissions".to_string()),
        policies_applied: vec!["policy-deny".to_string()],
        security_recommendations: vec!["Request elevated access".to_string()],
        audit_id: Uuid::new_v4().to_string(),
    };

    assert!(!authz_response.allowed);
    assert_eq!(
        authz_response.reason,
        Some("Insufficient permissions".to_string())
    );
    assert_eq!(authz_response.security_recommendations.len(), 1);
}

// ============================================================================
// Security Policy Tests
// ============================================================================

#[test]
fn test_security_policy_creation() {
    let policy = SecurityPolicy {
        id: "pol-123".to_string(),
        name: "Data Access Policy".to_string(),
        description: "Controls access to sensitive data".to_string(),
        rules: vec![],
        enforcement_level: "strict".to_string(),
        created_at: Utc::now(),
    };

    assert_eq!(policy.id, "pol-123");
    assert_eq!(policy.name, "Data Access Policy");
    assert_eq!(policy.enforcement_level, "strict");
}

#[test]
fn test_policy_rule_creation() {
    let mut params = HashMap::new();
    params.insert("max_requests".to_string(), serde_json::json!(100));
    params.insert("time_window".to_string(), serde_json::json!("1h"));

    let rule = PolicyRule {
        condition: "rate_limit_exceeded".to_string(),
        action: "deny".to_string(),
        parameters: params,
    };

    assert_eq!(rule.condition, "rate_limit_exceeded");
    assert_eq!(rule.action, "deny");
    assert_eq!(rule.parameters.len(), 2);
}

#[test]
fn test_security_policy_with_rules() {
    let rule1 = PolicyRule {
        condition: "time_of_day".to_string(),
        action: "allow".to_string(),
        parameters: HashMap::new(),
    };

    let rule2 = PolicyRule {
        condition: "ip_whitelist".to_string(),
        action: "allow".to_string(),
        parameters: HashMap::new(),
    };

    let policy = SecurityPolicy {
        id: "pol-multi".to_string(),
        name: "Multi-Rule Policy".to_string(),
        description: "Policy with multiple rules".to_string(),
        rules: vec![rule1, rule2],
        enforcement_level: "moderate".to_string(),
        created_at: Utc::now(),
    };

    assert_eq!(policy.rules.len(), 2);
    assert_eq!(policy.rules[0].condition, "time_of_day");
    assert_eq!(policy.rules[1].condition, "ip_whitelist");
}

// ============================================================================
// Security Audit Event Tests
// ============================================================================

#[test]
fn test_audit_event_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("client_ip".to_string(), serde_json::json!("10.0.0.1"));

    let audit_event = SecurityAuditEvent {
        event_id: Uuid::new_v4().to_string(),
        event_type: "authorization_check".to_string(),
        service_id: "toadstool-prod".to_string(),
        user_id: Some("user-123".to_string()),
        resource: "/api/data".to_string(),
        action: "READ".to_string(),
        result: "allowed".to_string(),
        security_context: SecurityContext::default(),
        metadata,
        timestamp: Utc::now(),
    };

    assert_eq!(audit_event.event_type, "authorization_check");
    assert_eq!(audit_event.result, "allowed");
    assert_eq!(audit_event.metadata.len(), 1);
}

#[test]
fn test_audit_event_serialization() {
    let audit_event = SecurityAuditEvent {
        event_id: "evt-456".to_string(),
        event_type: "access_denied".to_string(),
        service_id: "service-1".to_string(),
        user_id: None,
        resource: "/protected/resource".to_string(),
        action: "DELETE".to_string(),
        result: "denied".to_string(),
        security_context: SecurityContext::default(),
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    let serialized = serde_json::to_string(&audit_event).expect("Failed to serialize");
    let deserialized: SecurityAuditEvent =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(audit_event.event_id, deserialized.event_id);
    assert_eq!(audit_event.result, deserialized.result);
}

// ============================================================================
// BearDogIntegration Client Tests
// ============================================================================

#[test]
fn test_beardog_integration_creation() {
    let config = BearDogConfig::default();
    let integration = BearDogIntegration::new(config);

    assert!(integration.is_ok());
}

#[test]
fn test_beardog_integration_with_custom_config() {
    let config = BearDogConfig {
        auth_endpoint: "https://custom.beardog.com/auth".to_string(),
        authz_endpoint: "https://custom.beardog.com/authz".to_string(),
        policy_endpoint: "https://custom.beardog.com/policy".to_string(),
        audit_endpoint: "https://custom.beardog.com/audit".to_string(),
        api_token: Some("custom-token".to_string()),
        request_timeout_secs: 45,
        token_refresh_interval_secs: 450,
        zero_trust_validation_interval_secs: 90,
        continuous_monitoring: true,
    };

    let integration = BearDogIntegration::new(config);
    assert!(integration.is_ok());
}

// ============================================================================
// Integration Edge Cases
// ============================================================================

#[test]
fn test_empty_capabilities_list() {
    let auth_request = AuthRequest {
        service_id: "test".to_string(),
        service_type: "test".to_string(),
        capabilities: vec![], // Empty capabilities
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    assert!(auth_request.capabilities.is_empty());
}

#[test]
fn test_large_capabilities_list() {
    let capabilities: Vec<String> = (0..100).map(|i| format!("capability-{}", i)).collect();

    let auth_request = AuthRequest {
        service_id: "test".to_string(),
        service_type: "test".to_string(),
        capabilities: capabilities.clone(),
        security_context: SecurityContext::default(),
        timestamp: Utc::now(),
    };

    assert_eq!(auth_request.capabilities.len(), 100);
    assert_eq!(auth_request.capabilities[0], "capability-0");
    assert_eq!(auth_request.capabilities[99], "capability-99");
}

#[test]
fn test_empty_authz_context() {
    let authz_request = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/resource".to_string(),
        action: "READ".to_string(),
        context: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert!(authz_request.context.is_empty());
}

#[test]
fn test_complex_authz_context() {
    let mut context = HashMap::new();
    context.insert(
        "user".to_string(),
        serde_json::json!({"id": 123, "role": "admin"}),
    );
    context.insert(
        "device".to_string(),
        serde_json::json!({"type": "mobile", "os": "iOS"}),
    );
    context.insert(
        "location".to_string(),
        serde_json::json!({"country": "US", "region": "CA"}),
    );

    let authz_request = AuthzRequest {
        access_token: "token".to_string(),
        resource: "/api/admin".to_string(),
        action: "POST".to_string(),
        context,
        timestamp: Utc::now(),
    };

    assert_eq!(authz_request.context.len(), 3);
    assert!(authz_request.context.contains_key("user"));
    assert!(authz_request.context.contains_key("device"));
    assert!(authz_request.context.contains_key("location"));
}

// ============================================================================
// Security Level Validation
// ============================================================================

#[test]
fn test_security_levels() {
    let levels = vec!["low", "moderate", "high", "critical"];

    for level in levels {
        let auth_response = AuthResponse {
            access_token: "token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            scope: vec![],
            security_level: level.to_string(),
            policies: vec![],
        };

        assert_eq!(auth_response.security_level, level);
    }
}

#[test]
fn test_enforcement_levels() {
    let levels = vec!["audit", "moderate", "strict"];

    for level in levels {
        let policy = SecurityPolicy {
            id: format!("pol-{}", level),
            name: format!("{} Policy", level),
            description: String::new(),
            rules: vec![],
            enforcement_level: level.to_string(),
            created_at: Utc::now(),
        };

        assert_eq!(policy.enforcement_level, level);
    }
}

// ============================================================================
// Token Expiration Tests
// ============================================================================

#[test]
fn test_short_token_expiration() {
    let auth_response = AuthResponse {
        access_token: "short-lived-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 60, // 1 minute
        scope: vec![],
        security_level: "high".to_string(),
        policies: vec![],
    };

    assert_eq!(auth_response.expires_in, 60);
}

#[test]
fn test_long_token_expiration() {
    let auth_response = AuthResponse {
        access_token: "long-lived-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 86400, // 24 hours
        scope: vec![],
        security_level: "moderate".to_string(),
        policies: vec![],
    };

    assert_eq!(auth_response.expires_in, 86400);
}

// ============================================================================
// Policy Combinations
// ============================================================================

#[test]
fn test_multiple_policies_in_response() {
    let policies = vec![
        SecurityPolicy {
            id: "pol-1".to_string(),
            name: "Policy 1".to_string(),
            description: String::new(),
            rules: vec![],
            enforcement_level: "strict".to_string(),
            created_at: Utc::now(),
        },
        SecurityPolicy {
            id: "pol-2".to_string(),
            name: "Policy 2".to_string(),
            description: String::new(),
            rules: vec![],
            enforcement_level: "moderate".to_string(),
            created_at: Utc::now(),
        },
        SecurityPolicy {
            id: "pol-3".to_string(),
            name: "Policy 3".to_string(),
            description: String::new(),
            rules: vec![],
            enforcement_level: "audit".to_string(),
            created_at: Utc::now(),
        },
    ];

    let auth_response = AuthResponse {
        access_token: "multi-policy-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        scope: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        security_level: "high".to_string(),
        policies,
    };

    assert_eq!(auth_response.policies.len(), 3);
    assert_eq!(auth_response.policies[0].enforcement_level, "strict");
    assert_eq!(auth_response.policies[1].enforcement_level, "moderate");
    assert_eq!(auth_response.policies[2].enforcement_level, "audit");
}
