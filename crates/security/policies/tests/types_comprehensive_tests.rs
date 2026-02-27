//! Comprehensive tests for Security Policy types
//!
//! Coverage targets:
//! - PolicyManagerConfig
//! - SecurityPolicy
//! - PolicyRule
//! - PolicyCondition variants
//! - PolicyAction variants
//! - ViolationAction
//! - Evaluation results

use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use toadstool_security_policies::{
    PolicyAction, PolicyCondition, PolicyManagerConfig, PolicyRule, SecurityPolicy, ViolationAction,
};

// ============================================================================
// PolicyManagerConfig Tests (10 tests)
// ============================================================================

#[test]
fn test_policy_manager_config_default() {
    let config = PolicyManagerConfig::default();

    assert!(config.cache_enabled);
    assert_eq!(config.cache_ttl_hours, 24);
    assert!(config.strict_enforcement);
    assert!(matches!(
        config.default_violation_action,
        ViolationAction::Terminate
    ));
    assert_eq!(config.max_composition_depth, 10);
    assert_eq!(config.validation_timeout_ms, 5000);
}

#[test]
fn test_policy_manager_config_custom() {
    let config = PolicyManagerConfig {
        policy_dir: PathBuf::from("/custom/policies"),
        cache_enabled: false,
        cache_ttl_hours: 48,
        strict_enforcement: false,
        default_violation_action: ViolationAction::LogAndContinue,
        max_composition_depth: 20,
        validation_timeout_ms: 10000,
    };

    assert!(!config.cache_enabled);
    assert_eq!(config.cache_ttl_hours, 48);
    assert!(!config.strict_enforcement);
    assert_eq!(config.max_composition_depth, 20);
}

#[test]
fn test_policy_manager_config_clone() {
    let config = PolicyManagerConfig::default();
    let cloned = config.clone();

    assert_eq!(config.cache_ttl_hours, cloned.cache_ttl_hours);
    assert_eq!(config.strict_enforcement, cloned.strict_enforcement);
}

#[test]
fn test_policy_manager_config_debug() {
    let config = PolicyManagerConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("PolicyManagerConfig"));
    assert!(debug_str.contains("cache_enabled"));
}

#[test]
fn test_policy_manager_config_serialization() {
    let config = PolicyManagerConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    assert!(json.contains("policy_dir"));
    assert!(json.contains("cache_enabled"));
}

#[test]
fn test_policy_manager_config_deserialization() {
    let json = r#"{
        "policy_dir": "/test/policies",
        "cache_enabled": true,
        "cache_ttl_hours": 12,
        "strict_enforcement": true,
        "default_violation_action": "Terminate",
        "max_composition_depth": 5,
        "validation_timeout_ms": 3000
    }"#;

    let config: PolicyManagerConfig = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(config.cache_ttl_hours, 12);
    assert_eq!(config.max_composition_depth, 5);
    assert_eq!(config.validation_timeout_ms, 3000);
}

#[test]
fn test_policy_manager_config_strict_enforcement() {
    let config = PolicyManagerConfig {
        strict_enforcement: true,
        default_violation_action: ViolationAction::Terminate,
        ..PolicyManagerConfig::default()
    };

    assert!(config.strict_enforcement);
    assert!(matches!(
        config.default_violation_action,
        ViolationAction::Terminate
    ));
}

#[test]
fn test_policy_manager_config_permissive() {
    let config = PolicyManagerConfig {
        strict_enforcement: false,
        default_violation_action: ViolationAction::LogAndContinue,
        ..PolicyManagerConfig::default()
    };

    assert!(!config.strict_enforcement);
    assert!(matches!(
        config.default_violation_action,
        ViolationAction::LogAndContinue
    ));
}

#[test]
fn test_policy_manager_config_cache_disabled() {
    let config = PolicyManagerConfig {
        cache_enabled: false,
        cache_ttl_hours: 0,
        ..PolicyManagerConfig::default()
    };

    assert!(!config.cache_enabled);
    assert_eq!(config.cache_ttl_hours, 0);
}

#[test]
fn test_policy_manager_config_validation_timeout() {
    let config = PolicyManagerConfig {
        validation_timeout_ms: 15000,
        ..PolicyManagerConfig::default()
    };

    assert!(config.validation_timeout_ms > 0);
    assert_eq!(config.validation_timeout_ms, 15000);
}

// ============================================================================
// SecurityPolicy Tests (12 tests)
// ============================================================================

#[test]
fn test_security_policy_creation() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-001".to_string(),
        name: "Test Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test description".to_string()),
        author: Some("Test Author".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.id, "policy-001");
    assert_eq!(policy.name, "Test Policy");
    assert_eq!(policy.version, "1.0.0");
}

#[test]
fn test_security_policy_with_rules() {
    let now = SystemTime::now();
    let rule = PolicyRule {
        id: "rule-001".to_string(),
        name: "Test Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("Test rule".to_string()),
    };

    let policy = SecurityPolicy {
        id: "policy-002".to_string(),
        name: "Policy with Rules".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![rule.clone()],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.rules.len(), 1);
    assert_eq!(policy.rules[0].id, "rule-001");
}

#[test]
fn test_security_policy_with_inheritance() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-003".to_string(),
        name: "Child Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec!["base-policy".to_string(), "common-policy".to_string()],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.inherits.len(), 2);
    assert!(policy.inherits.contains(&"base-policy".to_string()));
}

#[test]
fn test_security_policy_with_metadata() {
    let now = SystemTime::now();
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), json!("production"));
    metadata.insert("version".to_string(), json!("2.0"));

    let policy = SecurityPolicy {
        id: "policy-004".to_string(),
        name: "Policy with Metadata".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata,
        signature: None,
    };

    assert_eq!(policy.metadata.len(), 2);
    assert!(policy.metadata.contains_key("environment"));
}

#[test]
fn test_security_policy_clone() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-005".to_string(),
        name: "Cloneable Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    let cloned = policy.clone();
    assert_eq!(policy.id, cloned.id);
    assert_eq!(policy.name, cloned.name);
}

#[test]
fn test_security_policy_serialization() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-006".to_string(),
        name: "Serializable Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    let json = serde_json::to_string(&policy).expect("Failed to serialize");
    assert!(json.contains("policy-006"));
    assert!(json.contains("Serializable Policy"));
}

#[test]
fn test_security_policy_with_signature() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-007".to_string(),
        name: "Signed Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: Some("Trusted Authority".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: Some("sha256:abcd1234".to_string()),
    };

    assert!(policy.signature.is_some());
    assert_eq!(policy.signature.unwrap(), "sha256:abcd1234");
}

#[test]
fn test_security_policy_multiple_rules() {
    let now = SystemTime::now();
    let rules = vec![
        PolicyRule {
            id: "rule-1".to_string(),
            name: "Rule 1".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "rule-2".to_string(),
            name: "Rule 2".to_string(),
            condition: PolicyCondition::Never,
            action: PolicyAction::Deny,
            priority: 50,
            enabled: true,
            description: None,
        },
    ];

    let policy = SecurityPolicy {
        id: "policy-008".to_string(),
        name: "Multi-Rule Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules,
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert_eq!(policy.rules.len(), 2);
    assert_eq!(policy.rules[0].priority, 100);
    assert_eq!(policy.rules[1].priority, 50);
}

#[test]
fn test_security_policy_version_format() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-009".to_string(),
        name: "Versioned Policy".to_string(),
        version: "2.1.3".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.version.contains('.'));
    assert_eq!(policy.version, "2.1.3");
}

#[test]
fn test_security_policy_timestamp_ordering() {
    let created = SystemTime::now();
    let modified = SystemTime::now();

    let policy = SecurityPolicy {
        id: "policy-010".to_string(),
        name: "Time Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: created,
        modified_at: modified,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.modified_at >= policy.created_at);
}

#[test]
fn test_security_policy_empty_description() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-011".to_string(),
        name: "No Description".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.description.is_none());
}

#[test]
fn test_security_policy_with_author() {
    let now = SystemTime::now();
    let policy = SecurityPolicy {
        id: "policy-012".to_string(),
        name: "Authored Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: Some("Security Team".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    assert!(policy.author.is_some());
    assert_eq!(policy.author.unwrap(), "Security Team");
}

// ============================================================================
// PolicyRule Tests (8 tests)
// ============================================================================

#[test]
fn test_policy_rule_creation() {
    let rule = PolicyRule {
        id: "rule-test-001".to_string(),
        name: "Test Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("A test rule".to_string()),
    };

    assert_eq!(rule.id, "rule-test-001");
    assert_eq!(rule.priority, 100);
    assert!(rule.enabled);
}

#[test]
fn test_policy_rule_disabled() {
    let rule = PolicyRule {
        id: "rule-test-002".to_string(),
        name: "Disabled Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Deny,
        priority: 50,
        enabled: false,
        description: None,
    };

    assert!(!rule.enabled);
}

#[test]
fn test_policy_rule_high_priority() {
    let rule = PolicyRule {
        id: "rule-test-003".to_string(),
        name: "High Priority Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1000,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1000);
}

#[test]
fn test_policy_rule_low_priority() {
    let rule = PolicyRule {
        id: "rule-test-004".to_string(),
        name: "Low Priority Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1);
}

#[test]
fn test_policy_rule_clone() {
    let rule = PolicyRule {
        id: "rule-test-005".to_string(),
        name: "Cloneable Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: None,
    };

    let cloned = rule.clone();
    assert_eq!(rule.id, cloned.id);
    assert_eq!(rule.priority, cloned.priority);
}

#[test]
fn test_policy_rule_serialization() {
    let rule = PolicyRule {
        id: "rule-test-006".to_string(),
        name: "Serializable Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: None,
    };

    let json = serde_json::to_string(&rule).expect("Failed to serialize");
    assert!(json.contains("rule-test-006"));
    assert!(json.contains("Serializable Rule"));
}

#[test]
fn test_policy_rule_with_description() {
    let rule = PolicyRule {
        id: "rule-test-007".to_string(),
        name: "Described Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("This rule allows all access".to_string()),
    };

    assert!(rule.description.is_some());
    assert!(rule.description.unwrap().contains("allows"));
}

#[test]
fn test_policy_rule_priority_ordering() {
    let rule1 = PolicyRule {
        id: "rule-high".to_string(),
        name: "High".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: None,
    };

    let rule2 = PolicyRule {
        id: "rule-low".to_string(),
        name: "Low".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 10,
        enabled: true,
        description: None,
    };

    assert!(rule1.priority > rule2.priority);
}

// ============================================================================
// PolicyCondition Tests (5 tests)
// ============================================================================

#[test]
fn test_policy_condition_always() {
    let condition = PolicyCondition::Always;
    let debug_str = format!("{:?}", condition);
    assert!(debug_str.contains("Always"));
}

#[test]
fn test_policy_condition_never() {
    let condition = PolicyCondition::Never;
    let debug_str = format!("{:?}", condition);
    assert!(debug_str.contains("Never"));
}

#[test]
fn test_policy_condition_clone() {
    let condition = PolicyCondition::Always;
    let cloned = condition.clone();

    assert!(format!("{:?}", condition) == format!("{:?}", cloned));
}

#[test]
fn test_policy_condition_serialization() {
    let condition = PolicyCondition::Always;
    let json = serde_json::to_string(&condition).expect("Failed to serialize");

    assert!(!json.is_empty());
}

#[test]
fn test_policy_condition_debug() {
    let condition = PolicyCondition::Always;
    let debug_str = format!("{:?}", condition);

    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("Always"));
}

// ============================================================================
// PolicyAction Tests (5 tests)
// ============================================================================

#[test]
fn test_policy_action_allow() {
    let action = PolicyAction::Allow;
    let debug_str = format!("{:?}", action);
    assert!(debug_str.contains("Allow"));
}

#[test]
fn test_policy_action_deny() {
    let action = PolicyAction::Deny;
    let debug_str = format!("{:?}", action);
    assert!(debug_str.contains("Deny"));
}

#[test]
fn test_policy_action_clone() {
    let action = PolicyAction::Allow;
    let cloned = action.clone();

    assert!(format!("{:?}", action) == format!("{:?}", cloned));
}

#[test]
fn test_policy_action_serialization() {
    let action = PolicyAction::Allow;
    let json = serde_json::to_string(&action).expect("Failed to serialize");

    assert!(!json.is_empty());
}

#[test]
fn test_policy_action_debug() {
    let action = PolicyAction::Deny;
    let debug_str = format!("{:?}", action);

    assert!(!debug_str.is_empty());
    assert!(debug_str.contains("Deny"));
}

// ============================================================================
// ViolationAction Tests (4 tests)
// ============================================================================

#[test]
fn test_violation_action_log_and_continue() {
    let action = ViolationAction::LogAndContinue;
    let debug_str = format!("{:?}", action);
    assert!(debug_str.contains("LogAndContinue"));
}

#[test]
fn test_violation_action_terminate() {
    let action = ViolationAction::Terminate;
    let debug_str = format!("{:?}", action);
    assert!(debug_str.contains("Terminate"));
}

#[test]
fn test_violation_action_serialization() {
    let action = ViolationAction::Alert;
    let json = serde_json::to_string(&action).expect("Failed to serialize");

    assert!(!json.is_empty());
}

#[test]
fn test_violation_action_deserialization() {
    let json = r#""Terminate""#;
    let action: ViolationAction = serde_json::from_str(json).expect("Failed to deserialize");

    assert!(matches!(action, ViolationAction::Terminate));
}
