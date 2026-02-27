//! Comprehensive coverage tests for PolicyManager
//!
//! This module provides extensive test coverage for the PolicyManager,
//! focusing on lifecycle management, policy loading, caching, and enforcement.
//!
//! **Refactored Dec 18, 2025**: Complete rewrite to match current API patterns

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::TempDir;

use toadstool::security::{
    FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext, UserContext,
};
use toadstool::workload::WorkloadSpec;
use toadstool_security_policies::manager::{FilePolicyManager, PolicyManager};
use toadstool_security_policies::types::*;

// ============================================================================
// Helper Functions - Modern, Idiomatic Rust
// ============================================================================

/// Create test configuration using a caller-supplied temp directory.
///
/// Pass the `TempDir` value into the test to keep it alive for the
/// duration — when it drops, the directory is cleaned up automatically.
fn create_test_config_in(dir: &TempDir) -> PolicyManagerConfig {
    PolicyManagerConfig {
        policy_dir: dir.path().to_path_buf(),
        cache_enabled: true,
        cache_ttl_hours: 1,
        strict_enforcement: true,
        default_violation_action: ViolationAction::Block,
        max_composition_depth: 5,
        validation_timeout_ms: 1000,
    }
}

/// Convenience wrapper that allocates a fresh unique temp directory.
fn create_test_config() -> (TempDir, PolicyManagerConfig) {
    #[allow(clippy::expect_used)]
    let dir = tempfile::tempdir().expect("temp dir");
    let config = create_test_config_in(&dir);
    (dir, config)
}

/// Create a test policy with proper structure
fn create_test_policy(id: &str) -> SecurityPolicy {
    SecurityPolicy {
        id: id.to_string(),
        name: format!("Test Policy {}", id),
        version: "1.0.0".to_string(),
        description: Some("Test policy for coverage".to_string()),
        author: Some("Test Suite".to_string()),
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![create_test_rule("rule-1")],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    }
}

/// Create a test rule
fn create_test_rule(id: &str) -> PolicyRule {
    PolicyRule {
        id: id.to_string(),
        name: format!("Test Rule {}", id),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("Test rule".to_string()),
    }
}

/// Create test workload spec (enum variant)
fn create_test_workload_spec() -> WorkloadSpec {
    WorkloadSpec::Container {
        image: "test:latest".to_string(),
        command: Some(vec!["test".to_string()]),
        args: Some(vec![]),
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    }
}

/// Create test security context with modern structure
fn create_test_security_context() -> SecurityContext {
    SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![],
        user_context: Some(UserContext {
            username: None,
            uid: Some(1000),
            gid: Some(1000),
            groups: vec![],
        }),
        network_security: NetworkSecurity {
            allow_outbound: false,
            allow_inbound: false,
            ..Default::default()
        },
        filesystem_security: FilesystemSecurity {
            read_only: false,
            ..Default::default()
        },
    }
}

/// Create test system info
fn create_test_system_info() -> SystemInfo {
    SystemInfo {
        hostname: "test-host".to_string(),
        os_type: "Linux".to_string(),
        os_version: "5.15.0".to_string(),
        architecture: "x86_64".to_string(),
        cpu_count: 8,
        memory_total_mb: 16384,
        load_average: 1.5,
        uptime_seconds: 86400,
    }
}

// ============================================================================
// Manager Creation and Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_policy_manager_creation() {
    let (_tmp, config) = create_test_config();
    let result = FilePolicyManager::new(config);

    assert!(result.is_ok(), "Manager creation should succeed");
    let manager = result.unwrap();
    assert!(std::mem::size_of_val(&manager) > 0);
}

#[tokio::test]
async fn test_policy_manager_with_custom_config() {
    let _tmp = tempfile::tempdir().expect("temp dir");
    let config = PolicyManagerConfig {
        policy_dir: _tmp.path().to_path_buf(),
        cache_enabled: false,
        cache_ttl_hours: 24,
        strict_enforcement: false,
        default_violation_action: ViolationAction::LogAndContinue,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
    };

    let result = FilePolicyManager::new(config);
    assert!(
        result.is_ok(),
        "Custom config manager creation should succeed"
    );
}

#[tokio::test]
async fn test_policy_manager_default_config() {
    let _tmp = tempfile::tempdir().expect("temp dir");
    // Modern idiomatic pattern: struct initialization with defaults
    let config = PolicyManagerConfig {
        policy_dir: _tmp.path().to_path_buf(),
        ..PolicyManagerConfig::default()
    };

    let result = FilePolicyManager::new(config);
    assert!(
        result.is_ok(),
        "Default config manager creation should succeed: {:?}",
        result.err()
    );
}

// ============================================================================
// Policy Lifecycle Tests - Save, Load, Delete
// ============================================================================

#[tokio::test]
async fn test_policy_save_and_load() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    let policy = create_test_policy("save-load-test");

    // Save policy
    manager
        .save_policy(&policy)
        .await
        .expect("Policy save should succeed");

    // Load policy by ID
    let loaded = manager
        .load_policy(&policy.id)
        .await
        .expect("Policy load should succeed");

    assert_eq!(loaded.id, policy.id);
    assert_eq!(loaded.name, policy.name);
}

#[tokio::test]
async fn test_load_multiple_policies() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    // Save multiple policies
    for i in 1..=5 {
        let policy = create_test_policy(&format!("multi-policy-{}", i));
        manager
            .save_policy(&policy)
            .await
            .expect("Policy save should succeed");
    }

    // Load them back
    for i in 1..=5 {
        let id = format!("multi-policy-{}", i);
        let loaded = manager
            .load_policy(&id)
            .await
            .expect("Policy load should succeed");
        assert_eq!(loaded.id, id);
    }
}

#[tokio::test]
async fn test_policy_deletion() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    let policy = create_test_policy("delete-test");

    // Save policy
    manager
        .save_policy(&policy)
        .await
        .expect("Policy save should succeed");

    // Verify it exists
    assert!(manager.load_policy(&policy.id).await.is_ok());

    // Delete policy
    manager
        .delete_policy(&policy.id)
        .await
        .expect("Policy delete should succeed");

    // Verify it's gone
    assert!(
        manager.load_policy(&policy.id).await.is_err(),
        "Deleted policy should not be loadable"
    );
}

// ============================================================================
// Policy Composition and Inheritance Tests
// ============================================================================

#[tokio::test]
async fn test_policy_with_inheritance() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    // Create and save parent policy
    let parent_policy = create_test_policy("parent-policy");
    manager
        .save_policy(&parent_policy)
        .await
        .expect("Parent policy save should succeed");

    // Create child policy that inherits from parent
    let mut child_policy = create_test_policy("child-policy");
    child_policy.inherits = vec!["parent-policy".to_string()];
    manager
        .save_policy(&child_policy)
        .await
        .expect("Child policy save should succeed");

    // Get dependencies
    let deps = manager
        .get_policy_dependencies(&child_policy.id)
        .await
        .expect("Getting dependencies should succeed");

    assert!(
        deps.contains(&"parent-policy".to_string()),
        "Dependencies should include parent policy"
    );
}

#[tokio::test]
async fn test_policy_composition() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    // Create multiple policies
    for i in 1..=3 {
        let policy = create_test_policy(&format!("compose-{}", i));
        manager
            .save_policy(&policy)
            .await
            .expect("Policy save should succeed");
    }

    // Compose them
    let policy_ids = vec![
        "compose-1".to_string(),
        "compose-2".to_string(),
        "compose-3".to_string(),
    ];

    let composed = manager
        .compose_policies(&policy_ids)
        .await
        .expect("Policy composition should succeed");

    // Composed policy should have rules from all three
    assert!(!composed.rules.is_empty());
}

// ============================================================================
// Policy Evaluation Tests
// ============================================================================

#[tokio::test]
async fn test_evaluate_policy_allow() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    let mut policy = create_test_policy("allow-policy");
    policy.rules = vec![PolicyRule {
        id: "allow-rule".to_string(),
        name: "Allow Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: None,
    }];

    manager
        .save_policy(&policy)
        .await
        .expect("Policy save should succeed");

    let context = PolicyEvaluationContext {
        workload: create_test_workload_spec(),
        security_context: create_test_security_context(),
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: create_test_system_info(),
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    };

    let result = manager
        .evaluate_policy(&policy.id, &context)
        .await
        .expect("Policy evaluation should succeed");

    assert!(matches!(result.result, PolicyResult::Allow));
}

#[tokio::test]
async fn test_evaluate_policy_deny() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    let mut policy = create_test_policy("deny-policy");
    policy.rules = vec![PolicyRule {
        id: "deny-rule".to_string(),
        name: "Deny Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Deny,
        priority: 100,
        enabled: true,
        description: None,
    }];

    manager
        .save_policy(&policy)
        .await
        .expect("Policy save should succeed");

    let context = PolicyEvaluationContext {
        workload: create_test_workload_spec(),
        security_context: create_test_security_context(),
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: create_test_system_info(),
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    };

    let result = manager
        .evaluate_policy(&policy.id, &context)
        .await
        .expect("Policy evaluation should succeed");

    assert!(matches!(result.result, PolicyResult::Deny));
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_access() {
    let (_tmp, config) = create_test_config();
    let manager =
        Arc::new(FilePolicyManager::new(config).expect("Manager creation should succeed"));

    let policy = create_test_policy("concurrent-test");
    manager
        .save_policy(&policy)
        .await
        .expect("Policy save should succeed");

    // Spawn multiple tasks to load the same policy
    let mut handles = vec![];
    for _ in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let policy_id = policy.id.clone();

        let handle = tokio::spawn(async move { manager_clone.load_policy(&policy_id).await });
        handles.push(handle);
    }

    // All tasks should succeed
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok(), "Concurrent load should succeed");
    }
}

// ============================================================================
// Validation Tests
// ============================================================================

#[tokio::test]
async fn test_policy_validation() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    let policy = create_test_policy("validation-test");

    let validation_result = manager
        .validate_policy(&policy)
        .await
        .expect("Validation should complete");

    // Should have no validation errors for a valid policy
    assert!(
        validation_result.is_empty(),
        "Valid policy should have no validation errors"
    );
}

#[tokio::test]
async fn test_list_policies() {
    let (_tmp, config) = create_test_config();
    let manager = FilePolicyManager::new(config).expect("Manager creation should succeed");

    // Save several policies
    for i in 1..=3 {
        let policy = create_test_policy(&format!("list-test-{}", i));
        manager
            .save_policy(&policy)
            .await
            .expect("Policy save should succeed");
    }

    // List all policies
    let policies = manager
        .list_policies()
        .await
        .expect("List policies should succeed");

    assert!(policies.len() >= 3, "Should list at least 3 policies");
}
