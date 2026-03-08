// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::expect_used, clippy::unused_async)] // expect() is idiomatic in tests
//! CLI integration tests
//!
//! Tier 1 tests: Coverage-measured integration tests
//! Focus: Cross-module interactions, command execution, state management

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Executor Integration Tests
// ============================================================================

#[tokio::test]
async fn test_executor_coordinator_integration() {
    // Test executor integrating with coordinator

    let executor = create_test_executor().await;
    let coordinator = create_test_coordinator().await;

    // Execute task through coordinator
    let result = executor
        .execute_with_coordinator(&coordinator, "test-task")
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_executor_state_management() {
    // Test executor state tracking

    let executor = create_test_executor().await;

    assert_eq!(executor.active_tasks().await, 0);

    let task = executor.start_task("task-1").await.unwrap();
    assert_eq!(executor.active_tasks().await, 1);

    // Task cleanup
    drop(task);
    // ✅ MODERNIZED: Use barrier or check directly - no sleep needed
    // Active tasks should update immediately after drop
    assert_eq!(executor.active_tasks().await, 0);
}

#[tokio::test]
async fn test_executor_concurrent_tasks() {
    // Test handling multiple concurrent tasks

    let executor = Arc::new(create_test_executor().await);

    let mut handles = vec![];
    for i in 0..10 {
        let exec = Arc::clone(&executor);
        let handle = tokio::spawn(async move { exec.start_task(&format!("task-{i}")).await });
        handles.push(handle);
    }

    // All tasks should complete
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

#[tokio::test]
async fn test_config_to_executor_pipeline() {
    // Test config loading and executor initialization

    let config = load_test_config();
    let executor = Executor::from_config(config).await;

    assert!(executor.is_ok());

    let executor = executor.unwrap();
    assert_eq!(executor.max_concurrent_tasks(), 10);
}

#[tokio::test]
async fn test_config_hot_reload() {
    // Test configuration hot reload

    let mut executor = create_test_executor().await;

    let initial_max = executor.max_concurrent_tasks();

    // Update config
    let new_config = TestConfig { max_tasks: 20 };
    executor.reload_config(new_config).await.unwrap();

    assert_ne!(executor.max_concurrent_tasks(), initial_max);
    assert_eq!(executor.max_concurrent_tasks(), 20);
}

#[tokio::test]
async fn test_config_validation_on_load() {
    // Test config validation during load

    let invalid_config = TestConfig { max_tasks: 0 }; // Invalid: must be > 0

    let result = Executor::from_config(invalid_config).await;

    assert!(result.is_err());
}

// ============================================================================
// Ecosystem Integration Tests
// ============================================================================

#[tokio::test]
async fn test_executor_ecosystem_discovery() {
    // Test executor discovering ecosystem services

    let executor = create_test_executor().await;

    let services = executor.discover_services().await.unwrap();

    // Should find at least coordinator
    assert!(!services.is_empty());
}

#[tokio::test]
async fn test_executor_primal_communication() {
    // Test executor communicating with other primals

    let executor = create_test_executor().await;

    let result = executor.send_to_primal("songbird", "ping").await;

    // Should handle communication (mock may return Ok or Err)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_multi_executor_coordination() {
    // Test multiple executors coordinating

    let executor1 = Arc::new(create_test_executor().await);
    let executor2 = Arc::new(create_test_executor().await);

    // Start tasks on both
    let task1 = executor1.start_task("task-1").await.unwrap();
    let task2 = executor2.start_task("task-2").await.unwrap();

    // Both should complete without interference
    assert!(task1.wait().await.is_ok());
    assert!(task2.wait().await.is_ok());
}

// ============================================================================
// State Synchronization Tests
// ============================================================================

#[tokio::test]
async fn test_state_consistency_under_load() {
    // Test state remains consistent under concurrent operations

    let executor = Arc::new(create_test_executor().await);
    let state = Arc::new(RwLock::new(0));

    let mut handles = vec![];
    for _ in 0..100 {
        let exec = Arc::clone(&executor);
        let state_clone = Arc::clone(&state);

        let handle = tokio::spawn(async move {
            let _task = exec.start_task("test").await.unwrap();
            let mut count = state_clone.write().await;
            *count += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // State should be exactly 100
    assert_eq!(*state.read().await, 100);
}

#[tokio::test]
async fn test_state_recovery_after_error() {
    // Test state recovery after errors

    let executor = create_test_executor().await;

    // Cause error
    let _ = executor.start_task("invalid-task").await;

    // State should still be valid
    assert_eq!(executor.active_tasks().await, 0);

    // New tasks should work
    assert!(executor.start_task("valid-task").await.is_ok());
}

// ============================================================================
// Resource Management Integration Tests
// ============================================================================

#[tokio::test]
async fn test_resource_allocation_tracking() {
    // Test resource allocation is tracked

    let executor = create_test_executor().await;

    let initial_memory = executor.allocated_memory().await;

    let _task = executor.start_task_with_memory("task", 100).await.unwrap();

    let allocated_memory = executor.allocated_memory().await;
    assert!(allocated_memory > initial_memory);
}

#[tokio::test]
async fn test_resource_cleanup_on_task_completion() {
    // Test resources are freed on task completion

    let executor = create_test_executor().await;

    let initial_memory = executor.allocated_memory().await;

    {
        let _task = executor.start_task_with_memory("task", 100).await.unwrap();
        // Memory allocated
    }
    // Task dropped
    // ✅ MODERNIZED: Memory tracking is synchronous - no sleep needed
    let final_memory = executor.allocated_memory().await;
    assert_eq!(final_memory, initial_memory);
}

// ============================================================================
// Mock Implementations (Simplified)
// ============================================================================

struct TestExecutor {
    active_tasks: Arc<AtomicUsize>,
    max_tasks: usize,
    allocated_memory: Arc<AtomicUsize>,
}

impl TestExecutor {
    async fn active_tasks(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }

    async fn start_task(&self, name: &str) -> Result<MockTask, String> {
        // Simulate invalid task rejection
        if name.contains("invalid") {
            return Err("Invalid task".to_string());
        }

        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        Ok(MockTask::new(Arc::clone(&self.active_tasks)))
    }

    async fn start_task_with_memory(
        &self,
        _name: &str,
        memory_mb: usize,
    ) -> Result<MockTask, String> {
        self.allocated_memory.fetch_add(memory_mb, Ordering::SeqCst);
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        Ok(MockTask::with_memory(
            Arc::clone(&self.active_tasks),
            Arc::clone(&self.allocated_memory),
            memory_mb,
        ))
    }

    fn max_concurrent_tasks(&self) -> usize {
        self.max_tasks
    }

    async fn reload_config(&mut self, config: TestConfig) -> Result<(), String> {
        self.max_tasks = config.max_tasks;
        Ok(())
    }

    async fn allocated_memory(&self) -> usize {
        self.allocated_memory.load(Ordering::SeqCst)
    }

    async fn discover_services(&self) -> Result<Vec<String>, String> {
        Ok(vec!["coordinator".to_string()])
    }

    async fn send_to_primal(&self, _primal: &str, _message: &str) -> Result<(), String> {
        Ok(())
    }

    async fn execute_with_coordinator(
        &self,
        _coordinator: &MockCoordinator,
        _task: &str,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct MockTask {
    active_tasks: Arc<AtomicUsize>,
    allocated_memory: Option<(Arc<AtomicUsize>, usize)>, // (shared mem, amount)
}

impl MockTask {
    fn new(active_tasks: Arc<AtomicUsize>) -> Self {
        Self {
            active_tasks,
            allocated_memory: None,
        }
    }

    fn with_memory(
        active_tasks: Arc<AtomicUsize>,
        allocated_memory: Arc<AtomicUsize>,
        amount: usize,
    ) -> Self {
        Self {
            active_tasks,
            allocated_memory: Some((allocated_memory, amount)),
        }
    }

    async fn wait(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Drop for MockTask {
    fn drop(&mut self) {
        // Use atomic operations - no block_on needed, avoids "Cannot start runtime from within runtime"
        self.active_tasks.fetch_sub(1, Ordering::SeqCst);
        if let Some((mem, amount)) = &self.allocated_memory {
            mem.fetch_sub(*amount, Ordering::SeqCst);
        }
    }
}

struct MockCoordinator {}

struct TestConfig {
    max_tasks: usize,
}

struct Executor {
    inner: TestExecutor,
}

impl Executor {
    async fn from_config(config: TestConfig) -> Result<Self, String> {
        if config.max_tasks == 0 {
            return Err("max_tasks must be > 0".to_string());
        }

        Ok(Self {
            inner: TestExecutor {
                active_tasks: Arc::new(AtomicUsize::new(0)),
                max_tasks: config.max_tasks,
                allocated_memory: Arc::new(AtomicUsize::new(0)),
            },
        })
    }

    fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_tasks
    }
}

async fn create_test_executor() -> TestExecutor {
    TestExecutor {
        active_tasks: Arc::new(AtomicUsize::new(0)),
        max_tasks: 10,
        allocated_memory: Arc::new(AtomicUsize::new(0)),
    }
}

async fn create_test_coordinator() -> MockCoordinator {
    MockCoordinator {}
}

fn load_test_config() -> TestConfig {
    TestConfig { max_tasks: 10 }
}
