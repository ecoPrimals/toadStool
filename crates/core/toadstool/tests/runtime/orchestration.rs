// SPDX-License-Identifier: AGPL-3.0-or-later
// ============================================================================
// RuntimeOrchestrator Engine Registration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_engine_success() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    let result = orchestrator
        .register_engine(RuntimeType::Container, engine)
        .await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_engines() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine1 = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    let engine2 = Box::new(MockRuntimeEngine {
        supports: vec!["wasm".to_string()],
        should_fail: false,
    });

    let result1 = orchestrator
        .register_engine(RuntimeType::Container, engine1)
        .await;
    let result2 = orchestrator
        .register_engine(RuntimeType::Wasm, engine2)
        .await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_engine_overwrite() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine1 = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string()],
        should_fail: false,
    });

    let engine2 = Box::new(MockRuntimeEngine {
        supports: vec!["container".to_string(), "docker".to_string()],
        should_fail: false,
    });

    // Register first engine
    let result1 = orchestrator
        .register_engine(RuntimeType::Container, engine1)
        .await;
    assert!(result1.is_ok());

    // Register second engine with same type (should overwrite)
    let result2 = orchestrator
        .register_engine(RuntimeType::Container, engine2)
        .await;
    assert!(result2.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_all_runtime_types() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let runtime_types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
        RuntimeType::Python,
    ];

    for runtime_type in runtime_types {
        let engine = Box::new(MockRuntimeEngine {
            supports: vec!["test".to_string()],
            should_fail: false,
        });

        let result = orchestrator.register_engine(runtime_type, engine);
        assert!(result.is_ok());
    }
}

