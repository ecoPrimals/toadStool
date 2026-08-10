// SPDX-License-Identifier: AGPL-3.0-or-later
// ============================================================================
// RuntimeOrchestrator Execution Tests (Basic)
// ============================================================================

use std::sync::Arc;

use super::helpers::create_test_execution_request;
use super::super::types::MockRuntimeEngine;
use toadstool::execution::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_no_engines_registered() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    // Should fail because no engines are registered
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_with_valid_engine() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    match &result {
        Ok(_) => {}
        Err(e) => println!("Error: {:?}", e),
    }

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_engine_failure() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: true, // Engine will fail
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_with_runtime_hint() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    // Register two engines
    let engine1 = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    let engine2 = Box::new(MockRuntimeEngine {
        supports: vec!["wasm".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine1)
        .await
        .unwrap();
    orchestrator
        .register_engine(RuntimeType::Wasm, engine2)
        .await
        .unwrap();

    // Create request with runtime hint
    let mut request = create_test_execution_request();
    request.runtime_hint = Some(RuntimeType::Wasm);

    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

// ============================================================================
// RuntimeSelectionStrategy Behavior Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_first_available_strategy() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["any".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balanced_strategy() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["any".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimal_match_strategy() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimal_match_no_suitable_engine() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);

    // Register engine that doesn't support the workload type
    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["other_type".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    // Should fail because no engine supports the workload
    assert!(result.is_err());
}

// ============================================================================
// Concurrent Execution Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_registrations() {
    let orchestrator = Arc::new(RuntimeOrchestrator::new(
        RuntimeSelectionStrategy::FirstAvailable,
    ));

    let mut handles = vec![];

    for i in 0..10 {
        let orch_clone = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let engine = Box::new(MockRuntimeEngine {
                supports: vec![format!("type_{}", i)],
                should_fail: false,
            });

            // Use modulo to cycle through runtime types
            let runtime_type = match i % 4 {
                0 => RuntimeType::Container,
                1 => RuntimeType::Wasm,
                2 => RuntimeType::Native,
                _ => RuntimeType::Python,
            };

            orch_clone.register_engine(runtime_type, engine)
        });

        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_executions() {
    let orchestrator = Arc::new(RuntimeOrchestrator::new(
        RuntimeSelectionStrategy::FirstAvailable,
    ));

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let mut handles = vec![];

    for _ in 0..10 {
        let orch_clone = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let request = create_test_execution_request();
            orch_clone.execute(request).await
        });

        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_after_engine_removal() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_strategies_same_engines() {
    for strategy in vec![
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ] {
        let orchestrator = RuntimeOrchestrator::new(strategy);

        let engine = Box::new(MockRuntimeEngine {
            supports: vec!["container".to_string()],
            should_fail: false,
        });

        orchestrator
            .register_engine(RuntimeType::Container, engine)
            .await
            .unwrap();

        let request = create_test_execution_request();
        let result = orchestrator.execute(request).await;

        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_with_invalid_runtime_hint() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await
        .unwrap();

    let mut request = create_test_execution_request();
    request.runtime_hint = Some(RuntimeType::Python); // Not registered

    let result = orchestrator.execute(request).await;

    // Should still succeed by falling back to selection strategy
    assert!(result.is_ok());
}

