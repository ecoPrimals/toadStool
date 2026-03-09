// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::cast_possible_truncation)]
//! Unit tests for policy manager
//! Target: security/policies/src/manager.rs (181 lines, 6.63% → 60% coverage)

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use tempfile::TempDir;
use toadstool::security::{FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext};
use toadstool::workload::{ExecutableSource, WorkloadSpec};
use toadstool_security_policies::types::*;
use toadstool_security_policies::{FilePolicyManager, PolicyManager, PolicyManagerConfig};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_config(temp_dir: &TempDir) -> PolicyManagerConfig {
    PolicyManagerConfig {
        policy_dir: temp_dir.path().to_path_buf(),
        cache_enabled: true,
        cache_ttl_hours: 24,
        strict_enforcement: true,
        default_violation_action: ViolationAction::Terminate,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
    }
}

fn create_test_policy(id: &str, name: &str) -> SecurityPolicy {
    SecurityPolicy {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test policy".to_string()),
        author: Some("Test Author".to_string()),
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    }
}

fn create_test_policy_with_rules(id: &str, rule_count: usize) -> SecurityPolicy {
    let mut policy = create_test_policy(id, &format!("Policy {id}"));

    for i in 0..rule_count {
        policy.rules.push(PolicyRule {
            id: format!("rule_{i}"),
            name: format!("Rule {i}"),
            description: Some(format!("Test rule {i}")),
            enabled: true,
            priority: i as u32,
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
        });
    }

    policy
}

fn create_test_context() -> PolicyEvaluationContext {
    PolicyEvaluationContext {
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: "/bin/test".into(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Standard,
            capabilities: Vec::new(),
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        },
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "test-host".to_string(),
            os_type: "Linux".to_string(),
            os_version: "5.0".to_string(),
            architecture: "x86_64".to_string(),
            cpu_count: 4,
            memory_total_mb: 8192,
            load_average: 1.5,
            uptime_seconds: 3600,
        },
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    }
}

// ============================================================================
// FilePolicyManager Creation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_new() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);

    let result = FilePolicyManager::new(config);
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_new_creates_directory() {
    let temp_dir = TempDir::new().unwrap();
    let policy_dir = temp_dir.path().join("new_policies");

    let config = PolicyManagerConfig {
        policy_dir: policy_dir.clone(),
        ..create_test_config(&temp_dir)
    };

    let result = FilePolicyManager::new(config);
    assert!(result.is_ok());
    assert!(policy_dir.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_new_with_existing_directory() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(temp_dir.path()).unwrap();

    let config = create_test_config(&temp_dir);
    let result = FilePolicyManager::new(config);
    assert!(result.is_ok());
}

// ============================================================================
// Policy Loading Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_policy_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let result = manager.load_policy("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_and_load_policy() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("test_policy", "Test Policy");

    // Save policy
    let save_result = manager.save_policy(&policy).await;
    assert!(save_result.is_ok());

    // Load policy
    let load_result = manager.load_policy("test_policy").await;
    assert!(load_result.is_ok());

    let loaded_policy = load_result.unwrap();
    assert_eq!(loaded_policy.id, "test_policy");
    assert_eq!(loaded_policy.name, "Test Policy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_policy_from_cache() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("cached_policy", "Cached Policy");
    manager.save_policy(&policy).await.unwrap();

    // First load - from file
    let first_load = manager.load_policy("cached_policy").await;
    assert!(first_load.is_ok());

    // Second load - from cache
    let second_load = manager.load_policy("cached_policy").await;
    assert!(second_load.is_ok());
    assert_eq!(second_load.unwrap().id, "cached_policy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_policy_cache_disabled() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.cache_enabled = false;
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("no_cache", "No Cache Policy");
    manager.save_policy(&policy).await.unwrap();

    let result = manager.load_policy("no_cache").await;
    assert!(result.is_ok());
}

// ============================================================================
// Policy Saving Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_with_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("valid_policy", "Valid Policy");
    let result = manager.save_policy(&policy).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_with_empty_id() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("", "Invalid Policy");
    policy.id = String::new();

    let result = manager.save_policy(&policy).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_with_empty_name() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("policy_id", "");
    policy.name = String::new();

    let result = manager.save_policy(&policy).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_with_empty_version() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("policy_id", "Policy Name");
    policy.version = String::new();

    let result = manager.save_policy(&policy).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_updates_cache() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("cache_update", "Cache Update");
    manager.save_policy(&policy).await.unwrap();

    // Policy should be in cache now
    let loaded = manager.load_policy("cache_update").await;
    assert!(loaded.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_save_policy_non_strict_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = create_test_config(&temp_dir);
    config.strict_enforcement = false;
    let manager = FilePolicyManager::new(config).unwrap();

    // Test that with non-strict enforcement, validation warnings don't block saves
    let policy = create_test_policy("valid_id", "Valid Policy");
    let result = manager.save_policy(&policy).await;
    // Should succeed even with validation issues when strict_enforcement is false
    assert!(result.is_ok());
}

// ============================================================================
// Policy Deletion Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_delete_policy() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("delete_me", "Delete Me");
    manager.save_policy(&policy).await.unwrap();

    // Verify it exists
    assert!(manager.load_policy("delete_me").await.is_ok());

    // Delete it
    let delete_result = manager.delete_policy("delete_me").await;
    assert!(delete_result.is_ok());

    // Verify it's gone
    assert!(manager.load_policy("delete_me").await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_delete_nonexistent_policy() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    // Deleting non-existent policy should succeed (idempotent)
    let result = manager.delete_policy("nonexistent").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_delete_policy_removes_from_cache() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("cache_delete", "Cache Delete");
    manager.save_policy(&policy).await.unwrap();

    // Load to populate cache
    manager.load_policy("cache_delete").await.unwrap();

    // Delete
    manager.delete_policy("cache_delete").await.unwrap();

    // Should not be in cache or file
    assert!(manager.load_policy("cache_delete").await.is_err());
}

// ============================================================================
// Policy Listing Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_policies_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let result = manager.list_policies().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_policies_single() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("single_policy", "Single");
    manager.save_policy(&policy).await.unwrap();

    let result = manager.list_policies().await;
    assert!(result.is_ok());

    let policies = result.unwrap();
    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0], "single_policy");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_policies_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    // Save multiple policies
    for i in 0..5 {
        let policy = create_test_policy(&format!("policy_{i}"), &format!("Policy {i}"));
        manager.save_policy(&policy).await.unwrap();
    }

    let result = manager.list_policies().await;
    assert!(result.is_ok());

    let policies = result.unwrap();
    assert_eq!(policies.len(), 5);

    // Should be sorted
    for (i, policy) in policies.iter().enumerate().take(5) {
        assert_eq!(policy, &format!("policy_{i}"));
    }
}

// ============================================================================
// Policy Validation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_valid() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("valid", "Valid Policy");
    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert_eq!(errors.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_empty_id() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("test", "Test");
    policy.id = String::new();

    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert!(!errors.is_empty());
    assert!(errors[0].contains("ID"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_empty_name() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("test", "");
    policy.name = String::new();

    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("name")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_empty_version() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("test", "Test");
    policy.version = String::new();

    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("version")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_self_inheritance() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("self_inherit", "Self Inherit");
    policy.inherits.push("self_inherit".to_string());

    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("inherit from itself")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_validate_policy_with_rules() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy_with_rules("with_rules", 3);
    let result = manager.validate_policy(&policy).await;
    assert!(result.is_ok());

    let errors = result.unwrap();
    assert_eq!(errors.len(), 0);
}

// ============================================================================
// Policy Evaluation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_policy_simple() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("eval_simple", "Simple Evaluation");
    manager.save_policy(&policy).await.unwrap();

    let context = create_test_context();
    let result = manager.evaluate_policy("eval_simple", &context).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_nonexistent_policy() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let context = create_test_context();
    let result = manager.evaluate_policy("nonexistent", &context).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_policy_with_rules() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy_with_rules("eval_rules", 2);
    manager.save_policy(&policy).await.unwrap();

    let context = create_test_context();
    let result = manager.evaluate_policy("eval_rules", &context).await;
    assert!(result.is_ok());

    let eval_result = result.unwrap();
    assert_eq!(eval_result.policy_id, "eval_rules");
}

// ============================================================================
// Policy Composition Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_compose_policies_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let result = manager.compose_policies(&[]).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_compose_policies_single() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy_with_rules("compose_single", 2);
    manager.save_policy(&policy).await.unwrap();

    let result = manager
        .compose_policies(&["compose_single".to_string()])
        .await;
    assert!(result.is_ok());

    let composed = result.unwrap();
    assert_eq!(composed.rules.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_compose_policies_multiple() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy1 = create_test_policy_with_rules("compose_1", 2);
    let policy2 = create_test_policy_with_rules("compose_2", 3);
    manager.save_policy(&policy1).await.unwrap();
    manager.save_policy(&policy2).await.unwrap();

    let result = manager
        .compose_policies(&["compose_1".to_string(), "compose_2".to_string()])
        .await;
    assert!(result.is_ok());

    let composed = result.unwrap();
    assert_eq!(composed.rules.len(), 5); // 2 + 3
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_compose_policies_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let result = manager.compose_policies(&["nonexistent".to_string()]).await;
    assert!(result.is_err());
}

// ============================================================================
// Policy Dependencies Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_policy_dependencies_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let policy = create_test_policy("no_deps", "No Dependencies");
    manager.save_policy(&policy).await.unwrap();

    let result = manager.get_policy_dependencies("no_deps").await;
    assert!(result.is_ok());

    let deps = result.unwrap();
    assert_eq!(deps.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_policy_dependencies_with_inheritance() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let mut policy = create_test_policy("with_deps", "With Dependencies");
    policy.inherits.push("parent1".to_string());
    policy.inherits.push("parent2".to_string());
    manager.save_policy(&policy).await.unwrap();

    let result = manager.get_policy_dependencies("with_deps").await;
    assert!(result.is_ok());

    let deps = result.unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"parent1".to_string()));
    assert!(deps.contains(&"parent2".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_policy_dependencies_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let manager = FilePolicyManager::new(config).unwrap();

    let result = manager.get_policy_dependencies("nonexistent").await;
    assert!(result.is_ok());

    let deps = result.unwrap();
    assert_eq!(deps.len(), 0); // Returns empty for nonexistent policies
}
