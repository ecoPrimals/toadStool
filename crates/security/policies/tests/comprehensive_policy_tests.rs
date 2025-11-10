//! Comprehensive security policy tests
//!
//! This test suite provides extensive coverage for security policy management,
//! including policy creation, validation, composition, evaluation, and enforcement.

use toadstool_security_policies::*;

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use toadstool::security::{Capability, IsolationLevel};

// ============================================================================
// Policy Creation and Structure Tests
// ============================================================================

#[test]
fn test_policy_creation_minimal() {
    let policy = SecurityPolicy {
        id: "test-1".to_string(),
        name: "Test Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test description".to_string()),
        author: Some("Test Author".to_string()),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.id, "test-1");
    assert_eq!(policy.version, "1.0.0");
    assert!(policy.rules.is_empty());
}

#[test]
fn test_policy_with_rules() {
    let rule = PolicyRule {
        id: "rule-1".to_string(),
        name: "Allow All".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("Basic allow".to_string()),
    };

    let policy = SecurityPolicy {
        id: "test-2".to_string(),
        name: "Policy with Rules".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        rules: vec![rule.clone()],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.rules.len(), 1);
    assert_eq!(policy.rules[0].id, "rule-1");
}

#[test]
fn test_policy_with_inheritance() {
    let policy = SecurityPolicy {
        id: "child-policy".to_string(),
        name: "Child Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        rules: vec![],
        inherits: vec!["parent-policy".to_string(), "base-policy".to_string()],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.inherits.len(), 2);
    assert!(policy.inherits.contains(&"parent-policy".to_string()));
}

#[test]
fn test_policy_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("env".to_string(), serde_json::json!("production"));
    metadata.insert("team".to_string(), serde_json::json!("security"));

    let policy = SecurityPolicy {
        id: "meta-policy".to_string(),
        name: "Policy with Metadata".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        rules: vec![],
        inherits: vec![],
        metadata,
        signature: None,
    };

    assert_eq!(policy.metadata.len(), 2);
}

#[test]
fn test_policy_with_signature() {
    let policy = SecurityPolicy {
        id: "signed-policy".to_string(),
        name: "Signed Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: Some("signature-hash-here".to_string()),
    };

    assert!(policy.signature.is_some());
    assert_eq!(policy.signature.unwrap(), "signature-hash-here");
}

// ============================================================================
// Policy Rule Tests
// ============================================================================

#[test]
fn test_rule_creation() {
    let rule = PolicyRule {
        id: "test-rule".to_string(),
        name: "Test Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 50,
        enabled: true,
        description: Some("Test description".to_string()),
    };

    assert_eq!(rule.id, "test-rule");
    assert_eq!(rule.priority, 50);
    assert!(rule.enabled);
}

#[test]
fn test_rule_disabled() {
    let rule = PolicyRule {
        id: "disabled-rule".to_string(),
        name: "Disabled Rule".to_string(),
        condition: PolicyCondition::Never,
        action: PolicyAction::Deny,
        priority: 100,
        enabled: false,
        description: None,
    };

    assert!(!rule.enabled);
}

#[test]
fn test_rule_priority_high() {
    let rule = PolicyRule {
        id: "high-priority".to_string(),
        name: "High Priority".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1000,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1000);
}

#[test]
fn test_rule_priority_low() {
    let rule = PolicyRule {
        id: "low-priority".to_string(),
        name: "Low Priority".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1);
}

// ============================================================================
// Policy Condition Tests
// ============================================================================

#[test]
fn test_condition_always() {
    let condition = PolicyCondition::Always;
    assert!(matches!(condition, PolicyCondition::Always));
}

#[test]
fn test_condition_never() {
    let condition = PolicyCondition::Never;
    assert!(matches!(condition, PolicyCondition::Never));
}

#[test]
fn test_condition_workload_type() {
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["native".to_string(), "wasm".to_string()],
    };

    match condition {
        PolicyCondition::WorkloadType { workload_types } => {
            assert_eq!(workload_types.len(), 2);
            assert!(workload_types.contains(&"native".to_string()));
        }
        _ => panic!("Expected WorkloadType condition"),
    }
}

#[test]
fn test_condition_capability() {
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![
            toadstool::security::Capability::Read,
            toadstool::security::Capability::NetworkClient,
        ],
    };

    match condition {
        PolicyCondition::RequiresCapability { capabilities } => {
            assert_eq!(capabilities.len(), 2);
        }
        _ => panic!("Expected RequiresCapability condition"),
    }
}

#[test]
fn test_condition_resource_usage() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: Some(2048),
    };

    match condition {
        PolicyCondition::ResourceUsage {
            cpu_percent,
            memory_mb,
        } => {
            assert_eq!(cpu_percent, Some(80.0));
            assert_eq!(memory_mb, Some(2048));
        }
        _ => panic!("Expected ResourceUsage condition"),
    }
}

#[test]
fn test_condition_time_window() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5],
    };

    match condition {
        PolicyCondition::TimeWindow {
            start_hour,
            end_hour,
            days,
        } => {
            assert_eq!(start_hour, 9);
            assert_eq!(end_hour, 17);
            assert_eq!(days.len(), 5);
        }
        _ => panic!("Expected TimeWindow condition"),
    }
}

#[test]
fn test_condition_user_context() {
    let condition = PolicyCondition::UserContext {
        users: vec!["admin".to_string()],
        groups: vec!["admins".to_string()],
    };

    match condition {
        PolicyCondition::UserContext { users, groups } => {
            assert_eq!(users.len(), 1);
            assert_eq!(groups.len(), 1);
        }
        _ => panic!("Expected UserContext condition"),
    }
}

#[test]
fn test_condition_network_access() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["api.example.com".to_string()],
        ports: vec![443, 8080],
    };

    match condition {
        PolicyCondition::NetworkAccess { hosts, ports } => {
            assert_eq!(hosts.len(), 1);
            assert_eq!(ports.len(), 2);
        }
        _ => panic!("Expected NetworkAccess condition"),
    }
}

#[test]
fn test_condition_filesystem_access() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/tmp")],
        operations: vec!["read".to_string(), "write".to_string()],
    };

    match condition {
        PolicyCondition::FileSystemAccess { paths, operations } => {
            assert_eq!(paths.len(), 1);
            assert_eq!(operations.len(), 2);
        }
        _ => panic!("Expected FileSystemAccess condition"),
    }
}

#[test]
fn test_condition_custom() {
    let mut vars = HashMap::new();
    vars.insert("threshold".to_string(), serde_json::json!(100));

    let condition = PolicyCondition::Custom {
        expression: "value > threshold".to_string(),
        variables: vars,
    };

    match condition {
        PolicyCondition::Custom {
            expression,
            variables,
        } => {
            assert!(!expression.is_empty());
            assert_eq!(variables.len(), 1);
        }
        _ => panic!("Expected Custom condition"),
    }
}

#[test]
fn test_condition_composite_and() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::And));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected Composite condition"),
    }
}

#[test]
fn test_condition_composite_or() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Never],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Or));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected Composite OR condition"),
    }
}

#[test]
fn test_condition_composite_not() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Not));
            assert_eq!(conditions.len(), 1);
        }
        _ => panic!("Expected Composite NOT condition"),
    }
}

#[test]
fn test_condition_nested_composite() {
    let inner = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    let outer = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![inner, PolicyCondition::Never],
    };

    match outer {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Or));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected outer Composite condition"),
    }
}

// ============================================================================
// Policy Action Tests
// ============================================================================

#[test]
fn test_action_allow() {
    let action = PolicyAction::Allow;
    assert!(matches!(action, PolicyAction::Allow));
}

#[test]
fn test_action_deny() {
    let action = PolicyAction::Deny;
    assert!(matches!(action, PolicyAction::Deny));
}

#[test]
fn test_action_allow_with_warning() {
    let action = PolicyAction::AllowWithWarning {
        message: "Warning message".to_string(),
    };

    match action {
        PolicyAction::AllowWithWarning { message } => {
            assert!(!message.is_empty());
        }
        _ => panic!("Expected AllowWithWarning action"),
    }
}

#[test]
fn test_action_deny_with_message() {
    let action = PolicyAction::DenyWithMessage {
        message: "Access denied".to_string(),
    };

    match action {
        PolicyAction::DenyWithMessage { message } => {
            assert_eq!(message, "Access denied");
        }
        _ => panic!("Expected DenyWithMessage action"),
    }
}

#[test]
fn test_action_modify_security_context() {
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: Some(toadstool::security::IsolationLevel::Maximum),
        add_capabilities: vec![toadstool::security::Capability::Read],
        remove_capabilities: vec![],
    };

    match action {
        PolicyAction::ModifySecurityContext {
            isolation_level,
            add_capabilities,
            remove_capabilities,
        } => {
            assert!(isolation_level.is_some());
            assert_eq!(add_capabilities.len(), 1);
            assert!(remove_capabilities.is_empty());
        }
        _ => panic!("Expected ModifySecurityContext action"),
    }
}

#[test]
fn test_action_apply_resource_limits() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
        network_mbps: Some(100.0),
    };

    match action {
        PolicyAction::ApplyResourceLimits {
            cpu_percent,
            memory_mb,
            network_mbps,
        } => {
            assert_eq!(cpu_percent, Some(50.0));
            assert_eq!(memory_mb, Some(1024));
            assert_eq!(network_mbps, Some(100.0));
        }
        _ => panic!("Expected ApplyResourceLimits action"),
    }
}

#[test]
fn test_action_require_authentication() {
    let action = PolicyAction::RequireAuthentication {
        method: "2FA".to_string(),
    };

    match action {
        PolicyAction::RequireAuthentication { method } => {
            assert_eq!(method, "2FA");
        }
        _ => panic!("Expected RequireAuthentication action"),
    }
}

#[test]
fn test_action_log_and_continue() {
    let action = PolicyAction::LogAndContinue {
        level: "warn".to_string(),
        message: "Suspicious activity".to_string(),
    };

    match action {
        PolicyAction::LogAndContinue { level, message } => {
            assert_eq!(level, "warn");
            assert!(!message.is_empty());
        }
        _ => panic!("Expected LogAndContinue action"),
    }
}

#[test]
fn test_action_custom() {
    let mut params = HashMap::new();
    params.insert("retry_count".to_string(), serde_json::json!(3));

    let action = PolicyAction::Custom {
        action: "custom_action".to_string(),
        parameters: params,
    };

    match action {
        PolicyAction::Custom { action, parameters } => {
            assert_eq!(action, "custom_action");
            assert_eq!(parameters.len(), 1);
        }
        _ => panic!("Expected Custom action"),
    }
}

// ============================================================================
// Violation Action Tests
// ============================================================================

#[test]
fn test_violation_action_terminate() {
    let action = ViolationAction::Terminate;
    assert!(matches!(action, ViolationAction::Terminate));
}

#[test]
fn test_violation_action_block() {
    let action = ViolationAction::Block;
    assert!(matches!(action, ViolationAction::Block));
}

#[test]
fn test_violation_action_log_and_continue() {
    let action = ViolationAction::LogAndContinue;
    assert!(matches!(action, ViolationAction::LogAndContinue));
}

#[test]
fn test_violation_action_quarantine() {
    let action = ViolationAction::Quarantine;
    assert!(matches!(action, ViolationAction::Quarantine));
}

#[test]
fn test_violation_action_alert() {
    let action = ViolationAction::Alert;
    assert!(matches!(action, ViolationAction::Alert));
}

// ============================================================================
// LogicalOperator Tests (NEW)
// ============================================================================

#[test]
fn test_logical_operator_and() {
    let op = LogicalOperator::And;
    assert!(matches!(op, LogicalOperator::And));
}

#[test]
fn test_logical_operator_or() {
    let op = LogicalOperator::Or;
    assert!(matches!(op, LogicalOperator::Or));
}

#[test]
fn test_logical_operator_not() {
    let op = LogicalOperator::Not;
    assert!(matches!(op, LogicalOperator::Not));
}

#[test]
fn test_logical_operator_clone() {
    let op1 = LogicalOperator::And;
    let op2 = op1.clone();
    assert!(matches!(op2, LogicalOperator::And));
}

// ============================================================================
// PolicyResult Tests (NEW)
// ============================================================================

#[test]
fn test_policy_result_allow() {
    let result = PolicyResult::Allow;
    assert!(matches!(result, PolicyResult::Allow));
}

#[test]
fn test_policy_result_deny() {
    let result = PolicyResult::Deny;
    assert!(matches!(result, PolicyResult::Deny));
}

// ============================================================================
// Extended Policy Condition Tests - Complex Scenarios
// ============================================================================

#[test]
fn test_composite_condition_and() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    if let PolicyCondition::Composite {
        operator,
        conditions,
    } = condition
    {
        assert!(matches!(operator, LogicalOperator::And));
        assert_eq!(conditions.len(), 2);
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_composite_condition_or() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
    };

    if let PolicyCondition::Composite { operator, .. } = condition {
        assert!(matches!(operator, LogicalOperator::Or));
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_composite_condition_not() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };

    if let PolicyCondition::Composite { operator, .. } = condition {
        assert!(matches!(operator, LogicalOperator::Not));
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_time_window_condition() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5], // Monday-Friday
    };

    if let PolicyCondition::TimeWindow {
        start_hour,
        end_hour,
        days,
    } = condition
    {
        assert_eq!(start_hour, 9);
        assert_eq!(end_hour, 17);
        assert_eq!(days.len(), 5);
    } else {
        panic!("Expected TimeWindow condition");
    }
}

#[test]
fn test_time_window_24_hours() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 0,
        end_hour: 23,
        days: vec![0, 1, 2, 3, 4, 5, 6], // All days
    };

    if let PolicyCondition::TimeWindow { days, .. } = condition {
        assert_eq!(days.len(), 7);
    } else {
        panic!("Expected TimeWindow condition");
    }
}

#[test]
fn test_user_context_condition() {
    let condition = PolicyCondition::UserContext {
        users: vec!["alice".to_string(), "bob".to_string()],
        groups: vec!["admin".to_string(), "developers".to_string()],
    };

    if let PolicyCondition::UserContext { users, groups } = condition {
        assert_eq!(users.len(), 2);
        assert_eq!(groups.len(), 2);
        assert!(users.contains(&"alice".to_string()));
    } else {
        panic!("Expected UserContext condition");
    }
}

#[test]
fn test_user_context_empty_groups() {
    let condition = PolicyCondition::UserContext {
        users: vec!["root".to_string()],
        groups: vec![],
    };

    if let PolicyCondition::UserContext { users, groups } = condition {
        assert_eq!(users.len(), 1);
        assert_eq!(groups.len(), 0);
    } else {
        panic!("Expected UserContext condition");
    }
}

#[test]
fn test_network_access_condition() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["api.example.com".to_string(), "192.168.1.1".to_string()],
        ports: vec![80, 443, 8080],
    };

    if let PolicyCondition::NetworkAccess { hosts, ports } = condition {
        assert_eq!(hosts.len(), 2);
        assert_eq!(ports.len(), 3);
        assert!(ports.contains(&443));
    } else {
        panic!("Expected NetworkAccess condition");
    }
}

#[test]
fn test_network_access_all_ports() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["localhost".to_string()],
        ports: vec![],
    };

    if let PolicyCondition::NetworkAccess { ports, .. } = condition {
        assert_eq!(ports.len(), 0);
    } else {
        panic!("Expected NetworkAccess condition");
    }
}

#[test]
fn test_filesystem_access_condition() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/tmp"), PathBuf::from("/var/log")],
        operations: vec!["read".to_string(), "write".to_string()],
    };

    if let PolicyCondition::FileSystemAccess { paths, operations } = condition {
        assert_eq!(paths.len(), 2);
        assert_eq!(operations.len(), 2);
    } else {
        panic!("Expected FileSystemAccess condition");
    }
}

#[test]
fn test_filesystem_access_read_only() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/etc")],
        operations: vec!["read".to_string()],
    };

    if let PolicyCondition::FileSystemAccess { operations, .. } = condition {
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0], "read");
    } else {
        panic!("Expected FileSystemAccess condition");
    }
}

#[test]
fn test_custom_condition() {
    let mut variables = HashMap::new();
    variables.insert("threshold".to_string(), serde_json::json!(100));

    let condition = PolicyCondition::Custom {
        expression: "value > threshold".to_string(),
        variables,
    };

    if let PolicyCondition::Custom {
        expression,
        variables,
    } = condition
    {
        assert!(expression.contains("threshold"));
        assert_eq!(variables.len(), 1);
    } else {
        panic!("Expected Custom condition");
    }
}

#[test]
fn test_resource_usage_cpu_only() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: None,
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert_eq!(cpu_percent, Some(80.0));
        assert_eq!(memory_mb, None);
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

#[test]
fn test_resource_usage_memory_only() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: None,
        memory_mb: Some(2048),
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert_eq!(cpu_percent, None);
        assert_eq!(memory_mb, Some(2048));
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

#[test]
fn test_resource_usage_both() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert!(cpu_percent.is_some());
        assert!(memory_mb.is_some());
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

// ============================================================================
// Extended Policy Action Tests
// ============================================================================

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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
        created_at: Utc::now(),
        modified_at: Utc::now(),
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
