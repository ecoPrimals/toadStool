// SPDX-License-Identifier: AGPL-3.0-only
// ============================================================================
// Extended Policy Action Tests
// ============================================================================

use std::collections::HashMap;
use std::time::SystemTime;
use toadstool_security_policies::*;
use toadstool::security::Capability;
use toadstool::security::IsolationLevel;

#[test]
fn test_allow_with_warning_action() {
    let action = PolicyAction::AllowWithWarning {
        message: "Proceeding with caution".to_string(),
    };

    if let PolicyAction::AllowWithWarning { message } = action {
        assert!(message.contains("caution"));
    } else {
        panic!("Expected AllowWithWarning action");
    }
}

#[test]
fn test_deny_with_message_action() {
    let action = PolicyAction::DenyWithMessage {
        message: "Access denied: insufficient permissions".to_string(),
    };

    if let PolicyAction::DenyWithMessage { message } = action {
        assert!(message.contains("denied"));
    } else {
        panic!("Expected DenyWithMessage action");
    }
}

#[test]
fn test_modify_security_context_add_capabilities() {
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: None,
        add_capabilities: vec![Capability::NetworkClient, Capability::Read],
        remove_capabilities: vec![],
    };

    if let PolicyAction::ModifySecurityContext {
        add_capabilities, ..
    } = action
    {
        assert_eq!(add_capabilities.len(), 2);
    } else {
        panic!("Expected ModifySecurityContext action");
    }
}

#[test]
fn test_modify_security_context_remove_capabilities() {
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: None,
        add_capabilities: vec![],
        remove_capabilities: vec![Capability::NetworkClient],
    };

    if let PolicyAction::ModifySecurityContext {
        remove_capabilities,
        ..
    } = action
    {
        assert_eq!(remove_capabilities.len(), 1);
    } else {
        panic!("Expected ModifySecurityContext action");
    }
}

#[test]
fn test_modify_security_context_isolation() {
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: Some(IsolationLevel::Maximum),
        add_capabilities: vec![],
        remove_capabilities: vec![],
    };

    if let PolicyAction::ModifySecurityContext {
        isolation_level, ..
    } = action
    {
        assert!(isolation_level.is_some());
    } else {
        panic!("Expected ModifySecurityContext action");
    }
}

#[test]
fn test_apply_resource_limits_cpu() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(25.0),
        memory_mb: None,
        network_mbps: None,
    };

    if let PolicyAction::ApplyResourceLimits { cpu_percent, .. } = action {
        assert_eq!(cpu_percent, Some(25.0));
    } else {
        panic!("Expected ApplyResourceLimits action");
    }
}

#[test]
fn test_apply_resource_limits_memory() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: None,
        memory_mb: Some(512),
        network_mbps: None,
    };

    if let PolicyAction::ApplyResourceLimits { memory_mb, .. } = action {
        assert_eq!(memory_mb, Some(512));
    } else {
        panic!("Expected ApplyResourceLimits action");
    }
}

#[test]
fn test_apply_resource_limits_network() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: None,
        memory_mb: None,
        network_mbps: Some(100.0),
    };

    if let PolicyAction::ApplyResourceLimits { network_mbps, .. } = action {
        assert_eq!(network_mbps, Some(100.0));
    } else {
        panic!("Expected ApplyResourceLimits action");
    }
}

#[test]
fn test_apply_resource_limits_all() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
        network_mbps: Some(50.0),
    };

    if let PolicyAction::ApplyResourceLimits {
        cpu_percent,
        memory_mb,
        network_mbps,
    } = action
    {
        assert!(cpu_percent.is_some());
        assert!(memory_mb.is_some());
        assert!(network_mbps.is_some());
    } else {
        panic!("Expected ApplyResourceLimits action");
    }
}

#[test]
fn test_require_authentication_action() {
    let action = PolicyAction::RequireAuthentication {
        method: "mfa".to_string(),
    };

    if let PolicyAction::RequireAuthentication { method } = action {
        assert_eq!(method, "mfa");
    } else {
        panic!("Expected RequireAuthentication action");
    }
}

#[test]
fn test_log_and_continue_action() {
    let action = PolicyAction::LogAndContinue {
        level: "warn".to_string(),
        message: "Suspicious activity detected".to_string(),
    };

    if let PolicyAction::LogAndContinue { level, message } = action {
        assert_eq!(level, "warn");
        assert!(message.contains("Suspicious"));
    } else {
        panic!("Expected LogAndContinue action");
    }
}

#[test]
fn test_custom_action() {
    let mut parameters = HashMap::new();
    parameters.insert("timeout".to_string(), serde_json::json!(30));

    let action = PolicyAction::Custom {
        action: "custom_check".to_string(),
        parameters,
    };

    if let PolicyAction::Custom { action, parameters } = action {
        assert_eq!(action, "custom_check");
        assert_eq!(parameters.len(), 1);
    } else {
        panic!("Expected Custom action");
    }
}

// ============================================================================
// Policy Rule Tests - Complex Scenarios
// ============================================================================

#[test]
fn test_policy_rule_high_priority() {
    let rule = PolicyRule {
        id: "high-priority".to_string(),
        name: "High Priority Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1000,
        enabled: true,
        description: Some("Critical rule".to_string()),
    };

    assert_eq!(rule.priority, 1000);
    assert!(rule.enabled);
}

#[test]
fn test_policy_rule_disabled() {
    let rule = PolicyRule {
        id: "disabled-rule".to_string(),
        name: "Disabled Rule".to_string(),
        condition: PolicyCondition::Never,
        action: PolicyAction::Deny,
        priority: 50,
        enabled: false,
        description: Some("Temporarily disabled".to_string()),
    };

    assert!(!rule.enabled);
}

#[test]
fn test_policy_rule_with_composite_condition() {
    let rule = PolicyRule {
        id: "composite-rule".to_string(),
        name: "Composite Condition Rule".to_string(),
        condition: PolicyCondition::Composite {
            operator: LogicalOperator::And,
            conditions: vec![
                PolicyCondition::Always,
                PolicyCondition::ResourceUsage {
                    cpu_percent: Some(90.0),
                    memory_mb: None,
                },
            ],
        },
        action: PolicyAction::Deny,
        priority: 100,
        enabled: true,
        description: None,
    };

    if let PolicyCondition::Composite { conditions, .. } = rule.condition {
        assert_eq!(conditions.len(), 2);
    } else {
        panic!("Expected Composite condition");
    }
}

// ============================================================================
// Policy Inheritance and Composition Tests
// ============================================================================

#[test]
fn test_policy_single_inheritance() {
    let policy = SecurityPolicy {
        id: "child".to_string(),
        name: "Child Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec!["parent".to_string()],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.inherits.len(), 1);
    assert_eq!(policy.inherits[0], "parent");
}

#[test]
fn test_policy_multiple_inheritance() {
    let policy = SecurityPolicy {
        id: "multi-inherit".to_string(),
        name: "Multi Inheritance Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![
            "base".to_string(),
            "network".to_string(),
            "filesystem".to_string(),
        ],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.inherits.len(), 3);
}

#[test]
fn test_policy_no_inheritance() {
    let policy = SecurityPolicy {
        id: "standalone".to_string(),
        name: "Standalone Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.inherits.is_empty());
}

// ============================================================================
// Policy Metadata Tests
// ============================================================================

#[test]
fn test_policy_metadata_environment() {
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), serde_json::json!("production"));
    metadata.insert("region".to_string(), serde_json::json!("us-west-2"));

    let policy = SecurityPolicy {
        id: "env-policy".to_string(),
        name: "Environment Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata,
        signature: None,
    };

    assert_eq!(policy.metadata.len(), 2);
    assert_eq!(
        policy.metadata.get("environment"),
        Some(&serde_json::json!("production"))
    );
}

#[test]
fn test_policy_metadata_tags() {
    let mut metadata = HashMap::new();
    metadata.insert(
        "tags".to_string(),
        serde_json::json!(["security", "compliance", "audit"]),
    );

    let policy = SecurityPolicy {
        id: "tagged-policy".to_string(),
        name: "Tagged Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata,
        signature: None,
    };

    assert!(policy.metadata.contains_key("tags"));
}

// ============================================================================
// Policy Versioning Tests
// ============================================================================

#[test]
fn test_policy_version_semantic() {
    let policy = SecurityPolicy {
        id: "versioned".to_string(),
        name: "Versioned Policy".to_string(),
        version: "2.1.3".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.version.starts_with("2.1"));
}

#[test]
fn test_policy_version_with_prerelease() {
    let policy = SecurityPolicy {
        id: "beta".to_string(),
        name: "Beta Policy".to_string(),
        version: "1.0.0-beta.1".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.version.contains("beta"));
}

#[test]
fn test_policy_result_modified() {
    let result = PolicyResult::Modified;
    assert!(matches!(result, PolicyResult::Modified));
}

#[test]
fn test_policy_result_clone() {
    let result1 = PolicyResult::Allow;
    let result2 = result1.clone();
    assert!(matches!(result2, PolicyResult::Allow));
}

// ============================================================================
// PolicyEvaluationResult Tests (NEW)
// ============================================================================

#[test]
fn test_policy_evaluation_result_creation() {
    let result = PolicyEvaluationResult {
        evaluation_id: uuid::Uuid::new_v4(),
        policy_id: "test-policy".to_string(),
        result: PolicyResult::Allow,
        applied_rules: vec![],
        security_modifications: vec![],
        resource_modifications: vec![],
        warnings: vec![],
        evaluation_duration: std::time::Duration::from_millis(50),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(result.policy_id, "test-policy");
    assert!(matches!(result.result, PolicyResult::Allow));
    assert_eq!(result.evaluation_duration.as_millis(), 50);
}

#[test]
fn test_policy_evaluation_result_with_rules() {
    let applied_rule = AppliedRule {
        rule_id: "rule-1".to_string(),
        rule_name: "Test Rule".to_string(),
        priority: 100,
        action: PolicyAction::Allow,
        condition_matched: true,
    };

    let result = PolicyEvaluationResult {
        evaluation_id: uuid::Uuid::new_v4(),
        policy_id: "test".to_string(),
        result: PolicyResult::Allow,
        applied_rules: vec![applied_rule],
        security_modifications: vec![],
        resource_modifications: vec![],
        warnings: vec![],
        evaluation_duration: std::time::Duration::from_millis(1),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(result.policy_id, "test");
    assert!(matches!(result.result, PolicyResult::Allow));
    assert_eq!(result.applied_rules.len(), 1);
}
