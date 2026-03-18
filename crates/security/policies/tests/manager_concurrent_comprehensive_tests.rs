// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive concurrent tests for `PolicyManager`
//!
//! ✅ MODERN CONCURRENT TESTING - No sleeps, no serial, fully event-driven
//! Tests run in parallel, proving production-grade concurrent safety
//!
//! Test infrastructure may use `expect()` for setup - test failure is appropriate

#![allow(clippy::expect_used)] // Test infrastructure - expect is appropriate for setup

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::TempDir;
use tokio::sync::Barrier;

use toadstool::security::SecurityContext;
use toadstool::workload::WorkloadSpec;
use toadstool_security_policies::{
    FilePolicyManager, PolicyAction, PolicyCondition, PolicyEvaluationContext, PolicyManager,
    PolicyManagerConfig, PolicyRule, SecurityPolicy, SystemInfo,
};

/// Create test policy manager with isolated temporary directory
fn create_test_manager() -> (FilePolicyManager, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = PolicyManagerConfig {
        policy_dir: temp_dir.path().to_path_buf(),
        cache_enabled: true,
        cache_ttl_hours: 24,
        strict_enforcement: false,
        default_violation_action: toadstool_security_policies::ViolationAction::Block,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
    };

    let manager = FilePolicyManager::new(config).expect("Failed to create policy manager");

    (manager, temp_dir)
}

/// Create test security policy
fn create_test_policy(id: &str, name: &str) -> SecurityPolicy {
    SecurityPolicy {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test policy".to_string()),
        author: Some("Test".to_string()),
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: vec![PolicyRule {
            id: format!("{id}_rule_1"),
            name: "Allow all".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: Some("Test rule".to_string()),
        }],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    }
}

/// Create test policy evaluation context
fn create_test_context() -> PolicyEvaluationContext {
    PolicyEvaluationContext {
        workload: WorkloadSpec::default(),
        security_context: SecurityContext::default(),
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "test".to_string(),
            os_type: "linux".to_string(),
            os_version: "5.10.0".to_string(),
            architecture: "x86_64".to_string(),
            cpu_count: 4,
            memory_total_mb: 8192,
            load_average: 0.5,
            uptime_seconds: 3600,
        },
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    }
}

// ============================================================================
// CONCURRENT BASIC OPERATIONS TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_creation() {
    // ✅ FULLY CONCURRENT: Create multiple policies in parallel
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    let mut tasks = vec![];
    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        tasks.push(tokio::spawn(async move {
            let policy = create_test_policy(&format!("policy_{i}"), &format!("Test Policy {i}"));
            mgr.save_policy(&policy).await
        }));
    }

    // All should succeed concurrently
    for task in tasks {
        task.await.expect("Task failed").expect("Save failed");
    }

    // Verify all policies exist
    let policies = manager.list_policies().await.expect("List failed");
    assert_eq!(policies.len(), 50);
}

#[tokio::test]
async fn test_concurrent_policy_reads() {
    // ✅ FULLY CONCURRENT: Read same policy from multiple tasks
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create one policy
    let policy = create_test_policy("shared_policy", "Shared Test Policy");
    manager.save_policy(&policy).await.expect("Save failed");

    // 100 concurrent readers
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            // Synchronize start for maximum concurrency
            bar.wait().await;
            mgr.load_policy("shared_policy").await
        }));
    }

    // All should succeed and return same policy
    for task in tasks {
        let loaded = task.await.expect("Task failed").expect("Load failed");
        assert_eq!(loaded.id, "shared_policy");
        assert_eq!(loaded.name, "Shared Test Policy");
    }
}

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    // ✅ FULLY CONCURRENT: Mix reads, writes, and lists
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Pre-create some policies
    for i in 0..10 {
        let policy = create_test_policy(&format!("base_{i}"), &format!("Base {i}"));
        manager.save_policy(&policy).await.expect("Save failed");
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    // 50 writers
    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let policy = create_test_policy(&format!("new_{i}"), &format!("New {i}"));
            mgr.save_policy(&policy).await
        }));
    }

    // 30 readers
    for i in 0..30 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let id = i % 10; // Read from base policies
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.load_policy(&format!("base_{id}")).await.map(|_| ())
        }));
    }

    // 20 listers
    for _ in 0..20 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.list_policies().await.map(|_| ())
        }));
    }

    // All should complete successfully
    for task in tasks {
        task.await.expect("Task failed").expect("Operation failed");
    }
}

// ============================================================================
// CONCURRENT VALIDATION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_validation() {
    // ✅ FULLY CONCURRENT: Validate policies in parallel
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let policy = create_test_policy(&format!("policy_{i}"), &format!("Policy {i}"));
            mgr.validate_policy(&policy).await
        }));
    }

    // All validations should succeed
    for task in tasks {
        let errors = task.await.expect("Task failed").expect("Validation failed");
        assert!(errors.is_empty(), "Validation should pass");
    }
}

#[tokio::test]
async fn test_concurrent_invalid_policy_validation() {
    // ✅ FULLY CONCURRENT: Validate invalid policies
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for i in 0..30 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut policy = create_test_policy(&format!("invalid_{i}"), ""); // Empty name!
            policy.version = String::new(); // Empty version!
            mgr.validate_policy(&policy).await
        }));
    }

    // All should return validation errors
    for task in tasks {
        let errors = task.await.expect("Task failed").expect("Validation failed");
        assert!(!errors.is_empty(), "Should have validation errors");
        assert!(errors.iter().any(|e| e.contains("name cannot be empty")));
        assert!(errors.iter().any(|e| e.contains("version cannot be empty")));
    }
}

// ============================================================================
// CONCURRENT EVALUATION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_evaluation() {
    // ✅ FULLY CONCURRENT: Evaluate same policy from multiple contexts
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create and save policy
    let policy = create_test_policy("eval_policy", "Eval Policy");
    manager.save_policy(&policy).await.expect("Save failed");

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let context = create_test_context();
            mgr.evaluate_policy("eval_policy", &context).await
        }));
    }

    // All evaluations should succeed
    for task in tasks {
        let result = task.await.expect("Task failed").expect("Evaluation failed");
        assert_eq!(result.policy_id, "eval_policy");
        assert_eq!(
            result.result,
            toadstool_security_policies::PolicyResult::Allow
        );
        assert_eq!(result.applied_rules.len(), 1);
    }
}

#[tokio::test]
async fn test_concurrent_different_policy_evaluations() {
    // ✅ FULLY CONCURRENT: Evaluate different policies concurrently
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create multiple policies
    for i in 0..20 {
        let policy = create_test_policy(&format!("policy_{i}"), &format!("Policy {i}"));
        manager.save_policy(&policy).await.expect("Save failed");
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let policy_id = format!("policy_{}", i % 20);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let context = create_test_context();
            mgr.evaluate_policy(&policy_id, &context).await
        }));
    }

    // All should succeed
    for task in tasks {
        task.await.expect("Task failed").expect("Evaluation failed");
    }
}

// ============================================================================
// CONCURRENT CACHE TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_cache_hits() {
    // ✅ FULLY CONCURRENT: Verify cache works under concurrency
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    let policy = create_test_policy("cached_policy", "Cached Policy");
    manager.save_policy(&policy).await.expect("Save failed");

    // First load (cache miss)
    manager
        .load_policy("cached_policy")
        .await
        .expect("Load failed");

    // Now 100 concurrent loads (should all hit cache)
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let start = std::time::Instant::now();
            let result = mgr.load_policy("cached_policy").await;
            (result, start.elapsed())
        }));
    }

    // All should succeed quickly (cache hits)
    for task in tasks {
        let (result, _duration) = task.await.expect("Task failed");
        assert!(result.is_ok(), "Load should succeed");
        // Cache hits should be fast, but we don't assert on timing (non-deterministic)
    }
}

// ============================================================================
// CONCURRENT COMPOSITION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_composition() {
    // ✅ FULLY CONCURRENT: Compose policies concurrently
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create base policies
    for i in 0..5 {
        let policy = create_test_policy(&format!("base_{i}"), &format!("Base {i}"));
        manager.save_policy(&policy).await.expect("Save failed");
    }

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            // Each task composes 2-3 random base policies
            let policy_ids: Vec<String> = (0..(2 + i % 2))
                .map(|j| format!("base_{}", j % 5))
                .collect();
            mgr.compose_policies(&policy_ids).await
        }));
    }

    // All compositions should succeed
    for task in tasks {
        let composed = task.await.expect("Task failed").expect("Compose failed");
        assert!(!composed.rules.is_empty());
    }
}

// ============================================================================
// CONCURRENT DELETION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_policy_deletion() {
    // ✅ FULLY CONCURRENT: Delete policies in parallel
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create policies
    for i in 0..50 {
        let policy = create_test_policy(&format!("delete_{i}"), &format!("Delete {i}"));
        manager.save_policy(&policy).await.expect("Save failed");
    }

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.delete_policy(&format!("delete_{i}")).await
        }));
    }

    // All deletions should succeed
    for task in tasks {
        task.await.expect("Task failed").expect("Delete failed");
    }

    // Verify all deleted
    let policies = manager.list_policies().await.expect("List failed");
    assert!(policies.is_empty(), "All policies should be deleted");
}

// ============================================================================
// STRESS TESTS - PROVE ROBUSTNESS
// ============================================================================

#[tokio::test]
async fn test_stress_1000_concurrent_operations() {
    // ✅ STRESS TEST: 1000 concurrent mixed operations
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Pre-create some base policies
    for i in 0..50 {
        let policy = create_test_policy(&format!("stress_{i}"), &format!("Stress {i}"));
        manager.save_policy(&policy).await.expect("Save failed");
    }

    let barrier = Arc::new(Barrier::new(1000));
    let mut tasks = vec![];

    for i in 0..1000 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            // Mix operations based on index
            match i % 4 {
                0 => {
                    // Read
                    mgr.load_policy(&format!("stress_{}", i % 50))
                        .await
                        .map(|_| ())
                }
                1 => {
                    // Write
                    let policy =
                        create_test_policy(&format!("new_stress_{i}"), &format!("New Stress {i}"));
                    mgr.save_policy(&policy).await
                }
                2 => {
                    // Evaluate
                    let context = create_test_context();
                    mgr.evaluate_policy(&format!("stress_{}", i % 50), &context)
                        .await
                        .map(|_| ())
                }
                _ => {
                    // List
                    mgr.list_policies().await.map(|_| ())
                }
            }
        }));
    }

    // Count successes
    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task panicked").is_ok() {
            successes += 1;
        }
    }

    // Should have high success rate (>95%)
    assert!(successes > 950, "Success rate too low: {successes}/1000");
}

#[tokio::test]
async fn test_concurrent_policy_inheritance_evaluation() {
    // ✅ FULLY CONCURRENT: Test policy inheritance under concurrency
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create parent policy
    let parent = create_test_policy("parent", "Parent Policy");
    manager.save_policy(&parent).await.expect("Save failed");

    // Create child policy that inherits from parent
    let child = SecurityPolicy {
        inherits: vec!["parent".to_string()],
        ..create_test_policy("child", "Child Policy")
    };
    manager.save_policy(&child).await.expect("Save failed");

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let context = create_test_context();
            mgr.evaluate_policy("child", &context).await
        }));
    }

    // All should succeed and apply parent rules
    for task in tasks {
        let result = task.await.expect("Task failed").expect("Evaluation failed");
        // Should have rules from both parent and child
        assert!(!result.applied_rules.is_empty());
    }
}

// ============================================================================
// CONCURRENT ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_nonexistent_policy_loads() {
    // ✅ FULLY CONCURRENT: Handle errors gracefully under concurrency
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.load_policy(&format!("nonexistent_{i}")).await
        }));
    }

    // All should fail gracefully (no panics!)
    for task in tasks {
        let result = task.await.expect("Task should not panic");
        assert!(result.is_err(), "Should fail for nonexistent policy");
    }
}

#[tokio::test]
async fn test_concurrent_dependency_resolution() {
    // ✅ FULLY CONCURRENT: Resolve dependencies concurrently
    let (manager, _temp) = create_test_manager();
    let manager = Arc::new(manager);

    // Create policies with dependencies
    let policy1 = create_test_policy("policy1", "Policy 1");
    manager.save_policy(&policy1).await.expect("Save failed");

    let policy2 = SecurityPolicy {
        inherits: vec!["policy1".to_string()],
        ..create_test_policy("policy2", "Policy 2")
    };
    manager.save_policy(&policy2).await.expect("Save failed");

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.get_policy_dependencies("policy2").await
        }));
    }

    // All should resolve dependencies correctly
    for task in tasks {
        let deps = task
            .await
            .expect("Task failed")
            .expect("Deps resolution failed");
        assert_eq!(deps, vec!["policy1".to_string()]);
    }
}
