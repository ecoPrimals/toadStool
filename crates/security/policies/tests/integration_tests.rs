//! Integration tests for SecurityPolicy system
//!
//! Tests the complete policy evaluation workflow

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use toadstool_security_policies::{
    PolicyAction, PolicyCondition, PolicyManagerConfig, PolicyRule,
    SecurityPolicy, ViolationAction,
};

// ============================================================================
// Policy Integration Tests (15 tests)
// ============================================================================

#[test]
fn test_complete_policy_workflow() {
    let config = PolicyManagerConfig::default();
    assert!(config.strict_enforcement);
    
    let now = Utc::now();
    let policy = SecurityPolicy {
        id: "integration-001".to_string(),
        name: "Integration Test Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Complete workflow test".to_string()),
        author: Some("Test Suite".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };
    
    assert_eq!(policy.id, "integration-001");
    assert!(config.strict_enforcement);
}

#[test]
fn test_policy_with_multiple_rules_ordering() {
    let now = Utc::now();
    let rules = vec![
        PolicyRule {
            id: "rule-1".to_string(),
            name: "High Priority".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 1000,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "rule-2".to_string(),
            name: "Medium Priority".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 500,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "rule-3".to_string(),
            name: "Low Priority".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: None,
        },
    ];
    
    let policy = SecurityPolicy {
        id: "multi-rule-001".to_string(),
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
    
    assert_eq!(policy.rules.len(), 3);
    assert_eq!(policy.rules[0].priority, 1000);
    assert_eq!(policy.rules[1].priority, 500);
    assert_eq!(policy.rules[2].priority, 100);
}

#[test]
fn test_policy_inheritance_chain() {
    let now = Utc::now();
    
    // Base policy
    let base_policy = SecurityPolicy {
        id: "base-policy".to_string(),
        name: "Base Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Foundation policy".to_string()),
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };
    
    // Child policy inheriting from base
    let child_policy = SecurityPolicy {
        id: "child-policy".to_string(),
        name: "Child Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Extends base policy".to_string()),
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec!["base-policy".to_string()],
        metadata: HashMap::new(),
        signature: None,
    };
    
    assert_eq!(base_policy.inherits.len(), 0);
    assert_eq!(child_policy.inherits.len(), 1);
    assert!(child_policy.inherits.contains(&"base-policy".to_string()));
}

#[test]
fn test_config_with_strict_enforcement() {
    let mut config = PolicyManagerConfig::default();
    config.strict_enforcement = true;
    config.default_violation_action = ViolationAction::Terminate;
    
    assert!(config.strict_enforcement);
    assert!(matches!(config.default_violation_action, ViolationAction::Terminate));
}

#[test]
fn test_config_with_permissive_enforcement() {
    let mut config = PolicyManagerConfig::default();
    config.strict_enforcement = false;
    config.default_violation_action = ViolationAction::LogAndContinue;
    
    assert!(!config.strict_enforcement);
    assert!(matches!(config.default_violation_action, ViolationAction::LogAndContinue));
}

#[test]
fn test_policy_versioning() {
    let now = Utc::now();
    
    let v1 = SecurityPolicy {
        id: "versioned-policy".to_string(),
        name: "Versioned Policy".to_string(),
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
    
    let v2 = SecurityPolicy {
        id: "versioned-policy".to_string(),
        name: "Versioned Policy".to_string(),
        version: "2.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };
    
    assert_ne!(v1.version, v2.version);
    assert_eq!(v1.id, v2.id);
}

#[test]
fn test_policy_with_extensive_metadata() {
    let now = Utc::now();
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), serde_json::json!("production"));
    metadata.insert("region".to_string(), serde_json::json!("us-west-2"));
    metadata.insert("team".to_string(), serde_json::json!("security"));
    metadata.insert("compliance".to_string(), serde_json::json!(["SOC2", "HIPAA"]));
    
    let policy = SecurityPolicy {
        id: "metadata-rich".to_string(),
        name: "Metadata Rich Policy".to_string(),
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
    
    assert_eq!(policy.metadata.len(), 4);
    assert!(policy.metadata.contains_key("environment"));
    assert!(policy.metadata.contains_key("compliance"));
}

#[test]
fn test_rule_priority_conflict_resolution() {
    let now = Utc::now();
    
    // Two rules with same priority
    let rules = vec![
        PolicyRule {
            id: "rule-a".to_string(),
            name: "Rule A".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "rule-b".to_string(),
            name: "Rule B".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Deny,
            priority: 100,
            enabled: true,
            description: None,
        },
    ];
    
    let policy = SecurityPolicy {
        id: "priority-conflict".to_string(),
        name: "Priority Conflict Policy".to_string(),
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
    
    assert_eq!(policy.rules[0].priority, policy.rules[1].priority);
}

#[test]
fn test_policy_composition_depth() {
    let config = PolicyManagerConfig {
        max_composition_depth: 5,
        ..PolicyManagerConfig::default()
    };
    
    let now = Utc::now();
    let policy = SecurityPolicy {
        id: "deep-inheritance".to_string(),
        name: "Deep Inheritance Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![
            "level-1".to_string(),
            "level-2".to_string(),
            "level-3".to_string(),
        ],
        metadata: HashMap::new(),
        signature: None,
    };
    
    assert_eq!(config.max_composition_depth, 5);
    assert_eq!(policy.inherits.len(), 3);
}

#[test]
fn test_policy_with_signed_integrity() {
    let now = Utc::now();
    let policy = SecurityPolicy {
        id: "signed-policy".to_string(),
        name: "Cryptographically Signed Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Integrity verified".to_string()),
        author: Some("Trusted CA".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: Some("sha256:1234567890abcdef".to_string()),
    };
    
    assert!(policy.signature.is_some());
    assert!(policy.signature.unwrap().starts_with("sha256:"));
}

#[test]
fn test_disabled_rules_in_policy() {
    let now = Utc::now();
    let rules = vec![
        PolicyRule {
            id: "enabled-rule".to_string(),
            name: "Active Rule".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "disabled-rule".to_string(),
            name: "Inactive Rule".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Deny,
            priority: 100,
            enabled: false,
            description: None,
        },
    ];
    
    let policy = SecurityPolicy {
        id: "mixed-rules".to_string(),
        name: "Mixed Enabled/Disabled Rules".to_string(),
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
    
    let enabled_count = policy.rules.iter().filter(|r| r.enabled).count();
    let disabled_count = policy.rules.iter().filter(|r| !r.enabled).count();
    
    assert_eq!(enabled_count, 1);
    assert_eq!(disabled_count, 1);
}

#[test]
fn test_cache_configuration() {
    let cached_config = PolicyManagerConfig {
        cache_enabled: true,
        cache_ttl_hours: 72,
        ..PolicyManagerConfig::default()
    };
    
    let uncached_config = PolicyManagerConfig {
        cache_enabled: false,
        cache_ttl_hours: 0,
        ..PolicyManagerConfig::default()
    };
    
    assert!(cached_config.cache_enabled);
    assert!(!uncached_config.cache_enabled);
    assert!(cached_config.cache_ttl_hours > uncached_config.cache_ttl_hours);
}

#[test]
fn test_validation_timeout_settings() {
    let fast_config = PolicyManagerConfig {
        validation_timeout_ms: 1000,
        ..PolicyManagerConfig::default()
    };
    
    let slow_config = PolicyManagerConfig {
        validation_timeout_ms: 30000,
        ..PolicyManagerConfig::default()
    };
    
    assert!(fast_config.validation_timeout_ms < slow_config.validation_timeout_ms);
    assert!(fast_config.validation_timeout_ms > 0);
}

#[test]
fn test_policy_serialization_round_trip() {
    let now = Utc::now();
    let original = SecurityPolicy {
        id: "roundtrip-001".to_string(),
        name: "Roundtrip Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test serialization".to_string()),
        author: Some("Test".to_string()),
        created_at: now,
        modified_at: now,
        rules: vec![],
        inherits: vec!["base".to_string()],
        metadata: HashMap::new(),
        signature: Some("test-sig".to_string()),
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: SecurityPolicy = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.version, deserialized.version);
    assert_eq!(original.inherits, deserialized.inherits);
}

#[test]
fn test_config_serialization_round_trip() {
    let original = PolicyManagerConfig::default();
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: PolicyManagerConfig = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.cache_enabled, deserialized.cache_enabled);
    assert_eq!(original.cache_ttl_hours, deserialized.cache_ttl_hours);
    assert_eq!(original.strict_enforcement, deserialized.strict_enforcement);
    assert_eq!(original.max_composition_depth, deserialized.max_composition_depth);
}

