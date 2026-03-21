// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::expect_used)] // expect() is idiomatic in tests
//! Comprehensive concurrent tests for `SandboxManager`
//!
//! ✅ MODERN CONCURRENT TESTING - Production-grade concurrent safety
//! All tests run in parallel, proving the code is truly robust

use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Barrier;

use toadstool::security::{IsolationLevel, SecurityContext};
use toadstool::workload::WorkloadSpec;
use toadstool_security_policies::{
    FilePolicyManager, PolicyAction, PolicyCondition, PolicyManagerConfig, PolicyRule,
    SecurityPolicy, ViolationAction,
};
use toadstool_security_sandbox::{
    CrossPlatformSandboxManager, NetworkConfig, ResourceLimits, SandboxConfig, SandboxLifetime,
    SandboxManager, SandboxSpec, SandboxStatus,
};

/// Create test sandbox manager with isolated temporary directories
async fn create_test_manager() -> (CrossPlatformSandboxManager, TempDir, TempDir) {
    let sandbox_root = TempDir::new().expect("Failed to create sandbox root");
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let policy_dir = TempDir::new().expect("Failed to create policy dir");

    let sandbox_config = SandboxConfig {
        sandbox_root: sandbox_root.path().to_path_buf(),
        temp_dir: temp_dir.path().to_path_buf(),
        max_concurrent_sandboxes: 1000,
        cleanup_timeout_secs: 300,
        advanced_features_enabled: true,
        default_isolation_level: IsolationLevel::Standard,
        enable_seccomp: false,
        enable_capability_dropping: true,
        enable_namespace_isolation: false,
        enable_resource_limits: true,
        enable_monitoring: false,
        monitoring_interval_ms: 1000,
    };

    let policy_config = PolicyManagerConfig {
        policy_dir: policy_dir.path().to_path_buf(),
        cache_enabled: true,
        cache_ttl_hours: 24,
        strict_enforcement: false,
        default_violation_action: ViolationAction::Block,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
    };

    let policy_manager =
        Arc::new(FilePolicyManager::new(policy_config).expect("Failed to create policy manager"));

    let manager = CrossPlatformSandboxManager::new(sandbox_config, policy_manager)
        .await
        .expect("Failed to create sandbox manager");

    (manager, sandbox_root, temp_dir)
}

/// Create test sandbox specification
fn create_test_sandbox_spec(id: &str) -> SandboxSpec {
    use std::path::PathBuf;

    SandboxSpec {
        sandbox_id: id.to_string(),
        workload: WorkloadSpec::Native {
            executable: toadstool::workload::ExecutableSource::File {
                path: PathBuf::from("echo"),
            },
            args: Some(vec!["test".to_string()]),
            working_dir: Some(PathBuf::from("/tmp")),
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Standard),
        resource_limits: ResourceLimits {
            max_cpu_percent: Some(80.0),
            max_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_disk_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
            max_network_bps: None,
            max_file_descriptors: Some(1024),
            max_processes: Some(100),
            max_execution_time: None,
        },
        filesystem_mounts: vec![],
        network_config: NetworkConfig::default(),
        environment: HashMap::new(),
        working_directory: Some(PathBuf::from("/tmp")),
        lifetime: SandboxLifetime::Ephemeral,
    }
}

// ============================================================================
// CONCURRENT SANDBOX CREATION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_sandbox_creation() {
    // ✅ FULLY CONCURRENT: Create 50 sandboxes in parallel
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let spec = create_test_sandbox_spec(&format!("concurrent_sandbox_{i}"));
            mgr.create_sandbox(spec).await
        }));
    }

    // All should succeed concurrently
    let mut created_ids = vec![];
    for task in tasks {
        let id = task.await.expect("Task failed").expect("Create failed");
        created_ids.push(id);
    }

    assert_eq!(created_ids.len(), 50);

    // Verify all sandboxes exist
    let sandboxes = manager.list_sandboxes().await.expect("List failed");
    assert_eq!(sandboxes.len(), 50);
}

#[tokio::test]
async fn test_concurrent_sandbox_info_retrieval() {
    // ✅ FULLY CONCURRENT: Get info for same sandbox from multiple tasks
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create one sandbox
    let spec = create_test_sandbox_spec("shared_sandbox");
    let sandbox_id = manager.create_sandbox(spec).await.expect("Create failed");

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let id = sandbox_id.clone();
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.get_sandbox_info(&id).await
        }));
    }

    // All should succeed
    for task in tasks {
        let info = task.await.expect("Task failed").expect("Get info failed");
        assert_eq!(info.sandbox_id, sandbox_id);
        assert!(matches!(
            info.status,
            SandboxStatus::Ready | SandboxStatus::Creating
        ));
    }
}

#[tokio::test]
async fn test_concurrent_mixed_sandbox_operations() {
    // ✅ FULLY CONCURRENT: Mix create, info, list operations
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Pre-create some sandboxes
    let mut existing_ids = vec![];
    for i in 0..10 {
        let spec = create_test_sandbox_spec(&format!("existing_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        existing_ids.push(id);
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    // 40 creators
    for i in 0..40 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let spec = create_test_sandbox_spec(&format!("new_{i}"));
            mgr.create_sandbox(spec).await.map(|_| ())
        }));
    }

    // 40 info getters
    for i in 0..40 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let id = existing_ids[i % 10].clone();
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.get_sandbox_info(&id).await.map(|_| ())
        }));
    }

    // 20 listers
    for _ in 0..20 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.list_sandboxes().await.map(|_| ())
        }));
    }

    // All should complete successfully
    for task in tasks {
        task.await.expect("Task failed").expect("Operation failed");
    }
}

// ============================================================================
// CONCURRENT EXECUTION LIFECYCLE TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_start_stop_execution() {
    // ✅ FULLY CONCURRENT: Start and stop executions in parallel
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create sandboxes first
    let mut sandbox_ids = vec![];
    for i in 0..30 {
        let spec = create_test_sandbox_spec(&format!("exec_sandbox_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        sandbox_ids.push(id);
    }

    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    // Start all concurrently
    for id in &sandbox_ids {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let sandbox_id = id.clone();
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.start_execution(&sandbox_id).await
        }));
    }

    // All starts should succeed
    for task in tasks {
        task.await.expect("Task failed").expect("Start failed");
    }

    // Verify all running
    for id in &sandbox_ids {
        let info = manager.get_sandbox_info(id).await.expect("Get info failed");
        assert_eq!(info.status, SandboxStatus::Running);
    }

    // Now stop all concurrently
    let barrier2 = Arc::new(Barrier::new(30));
    let mut stop_tasks = vec![];

    for id in sandbox_ids {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier2);
        stop_tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.stop_execution(&id).await
        }));
    }

    // All stops should succeed
    for task in stop_tasks {
        task.await.expect("Task failed").expect("Stop failed");
    }
}

// ============================================================================
// CONCURRENT SANDBOX DESTRUCTION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_sandbox_destruction() {
    // ✅ FULLY CONCURRENT: Destroy sandboxes in parallel
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create sandboxes
    let mut sandbox_ids = vec![];
    for i in 0..50 {
        let spec = create_test_sandbox_spec(&format!("destroy_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        sandbox_ids.push(id);
    }

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for id in sandbox_ids {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.destroy_sandbox(&id).await
        }));
    }

    // All should succeed
    for task in tasks {
        task.await.expect("Task failed").expect("Destroy failed");
    }

    // Verify all destroyed
    let sandboxes = manager.list_sandboxes().await.expect("List failed");
    assert!(sandboxes.is_empty(), "All sandboxes should be destroyed");
}

// ============================================================================
// CONCURRENT MONITORING TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_sandbox_monitoring() {
    // ✅ FULLY CONCURRENT: Monitor multiple sandboxes concurrently
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create and start sandboxes
    let mut sandbox_ids = vec![];
    for i in 0..20 {
        let spec = create_test_sandbox_spec(&format!("monitor_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        manager.start_execution(&id).await.expect("Start failed");
        sandbox_ids.push(id);
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    // 100 concurrent monitoring calls (each sandbox monitored 5 times)
    for _ in 0..5 {
        for id in &sandbox_ids {
            let mgr = Arc::clone(&manager);
            let bar = Arc::clone(&barrier);
            let sandbox_id = id.clone();
            tasks.push(tokio::spawn(async move {
                bar.wait().await;
                mgr.monitor_sandbox(&sandbox_id).await
            }));
        }
    }

    // All monitoring should succeed
    for task in tasks {
        task.await.expect("Task failed").expect("Monitor failed");
    }
}

// ============================================================================
// CONCURRENT POLICY APPLICATION TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_security_policy_application() {
    // ✅ FULLY CONCURRENT: Apply policies to sandboxes concurrently
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create sandboxes
    let mut sandbox_ids = vec![];
    for i in 0..30 {
        let spec = create_test_sandbox_spec(&format!("policy_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        sandbox_ids.push(id);
    }

    // Create test policy
    let policy = SecurityPolicy {
        id: "test_policy".to_string(),
        name: "Test Policy".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Test".to_string()),
        author: Some("Test".to_string()),
        created_at: std::time::SystemTime::now(),
        modified_at: std::time::SystemTime::now(),
        rules: vec![PolicyRule {
            id: "rule_1".to_string(),
            name: "Allow all".to_string(),
            description: Some("Test".to_string()),
            priority: 100,
            enabled: true,
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
        }],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };

    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for id in sandbox_ids {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let pol = policy.clone();
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.apply_security_policy(&id, &pol).await
        }));
    }

    // All should succeed
    for task in tasks {
        task.await
            .expect("Task failed")
            .expect("Apply policy failed");
    }
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test]
async fn test_stress_500_concurrent_sandbox_operations() {
    // ✅ STRESS TEST: 500 concurrent operations
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Pre-create some sandboxes
    let mut existing_ids = vec![];
    for i in 0..50 {
        let spec = create_test_sandbox_spec(&format!("stress_{i}"));
        let id = manager.create_sandbox(spec).await.expect("Create failed");
        existing_ids.push(id);
    }
    let existing_ids = Arc::new(existing_ids);

    let barrier = Arc::new(Barrier::new(500));
    let mut tasks = vec![];

    for i in 0..500 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let ids = Arc::clone(&existing_ids);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            // Mix operations
            match i % 3 {
                0 => {
                    // Create
                    let spec = create_test_sandbox_spec(&format!("new_stress_{i}"));
                    mgr.create_sandbox(spec).await.map(|_| ())
                }
                1 => {
                    // Get info
                    let idx = i % ids.len();
                    mgr.get_sandbox_info(&ids[idx]).await.map(|_| ())
                }
                _ => {
                    // List
                    mgr.list_sandboxes().await.map(|_| ())
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
    assert!(successes > 475, "Success rate too low: {successes}/500");
}

// ============================================================================
// CONCURRENT ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_invalid_sandbox_operations() {
    // ✅ FULLY CONCURRENT: Handle errors gracefully
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            // Try to get info for nonexistent sandbox
            mgr.get_sandbox_info(&format!("nonexistent_{i}")).await
        }));
    }

    // All should fail gracefully (no panics!)
    for task in tasks {
        let result = task.await.expect("Task should not panic");
        assert!(result.is_err(), "Should fail for nonexistent sandbox");
    }
}

#[tokio::test]
async fn test_concurrent_lifecycle_state_errors() {
    // ✅ FULLY CONCURRENT: Test idempotent stop operations
    let (manager, _root, _temp) = create_test_manager().await;
    let manager = Arc::new(manager);

    // Create sandbox but don't start it
    let spec = create_test_sandbox_spec("lifecycle_test");
    let sandbox_id = manager.create_sandbox(spec).await.expect("Create failed");

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    // Try to stop a sandbox that's not running (idempotent operation should succeed)
    for _ in 0..50 {
        let mgr = Arc::clone(&manager);
        let bar = Arc::clone(&barrier);
        let id = sandbox_id.clone();
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            mgr.stop_execution(&id).await
        }));
    }

    // All should succeed (idempotent stop operations)
    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task panicked").is_ok() {
            successes += 1;
        }
    }

    assert_eq!(
        successes, 50,
        "All idempotent stop operations should succeed"
    );
}
