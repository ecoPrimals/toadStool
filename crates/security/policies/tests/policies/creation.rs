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

