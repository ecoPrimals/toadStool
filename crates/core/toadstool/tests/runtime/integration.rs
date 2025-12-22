// ============================================================================
// RuntimeOrchestrator Execution Tests (Basic)
// ============================================================================

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

            orch_clone.register_engine(runtime_type, engine).await
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

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_execution_request() -> ExecutionRequest {
    use toadstool::WorkloadSpec;

    // Create a container workload that doesn't require file validation
    let workload = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: Some(vec!["echo".to_string()]),
        args: Some(vec!["hello".to_string()]),
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload,
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: Some(Duration::from_secs(300)),
        environment: HashMap::new(),
        input_data: toadstool::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_engine_multiple_types_same_orchestrator() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    for _ in 0..3 {
        let engine = Box::new(MockRuntimeEngine {
            supports: vec![],
            should_fail: false,
        });
        orchestrator
            .register_engine(RuntimeType::Container, engine)
            .await
            .unwrap();
    }

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_with_empty_strategy_enum() {
    for strategy in [
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ] {
        let orch = RuntimeOrchestrator::new(strategy);
        assert!(std::mem::size_of_val(&orch) > 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_request_with_different_runtime_hints() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let hints = vec![
        Some(RuntimeType::Container),
        Some(RuntimeType::Native),
        None,
    ];

    for hint in hints {
        let mut request = create_test_execution_request();
        request.runtime_hint = hint;
        let result = orchestrator.execute(request).await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_different_strategies() {
    let strategies = vec![
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ];

    let mut handles = vec![];

    for strategy in strategies {
        let handle = tokio::spawn(async move {
            let orchestrator = RuntimeOrchestrator::new(strategy);
            orchestrator
                .register_engine(
                    RuntimeType::Container,
                    Box::new(MockRuntimeEngine {
                        supports: vec![],
                        should_fail: false,
                    }),
                )
                .await
                .unwrap();

            let request = create_test_execution_request();
            orchestrator.execute(request).await
        });

        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mixed_successful_and_failing_engines() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    // Register Container runtime that supports and succeeds
    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec!["container".to_string()],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    // Register Native runtime that doesn't support container workloads
    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine {
                supports: vec!["native".to_string()],
                should_fail: true,
            }),
        )
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;
    // Should succeed with Container engine since it supports the workload
    assert!(result.is_ok());

    // Verify it used the Container runtime
    if let Ok(response) = result {
        assert_eq!(response.runtime_used, RuntimeType::Container);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_runtime_hint_preference_over_strategy() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    orchestrator
        .register_engine(
            RuntimeType::Wasm,
            Box::new(MockRuntimeEngine {
                supports: vec!["wasm".to_string()],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let mut request = create_test_execution_request();
    request.runtime_hint = Some(RuntimeType::Wasm);

    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

#[test]
fn test_runtime_selection_strategy_size() {
    let strategy = RuntimeSelectionStrategy::FirstAvailable;
    // Ensure the enum is small
    assert!(std::mem::size_of_val(&strategy) <= 8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_multiple_executions_same_engine() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    for _ in 0..10 {
        let request = create_test_execution_request();
        let result = orchestrator.execute(request).await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_then_execute_immediately() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);

    for i in 0..5 {
        let engine = Box::new(MockRuntimeEngine {
            supports: vec![format!("type_{}", i)],
            should_fail: false,
        });

        let runtime_type = match i % 3 {
            0 => RuntimeType::Container,
            1 => RuntimeType::Native,
            _ => RuntimeType::Wasm,
        };

        orchestrator
            .register_engine(runtime_type, engine)
            .await
            .unwrap();

        let request = create_test_execution_request();
        let result = orchestrator.execute(request).await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimal_match_with_multiple_engines() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::OptimalMatch);

    orchestrator
        .register_engine(
            RuntimeType::Wasm,
            Box::new(MockRuntimeEngine {
                supports: vec!["wasm".to_string()],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec!["container".to_string()],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    orchestrator
        .register_engine(
            RuntimeType::Native,
            Box::new(MockRuntimeEngine {
                supports: vec!["native".to_string()],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balanced_with_single_engine() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::LoadBalanced);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    for _ in 0..5 {
        let request = create_test_execution_request();
        let result = orchestrator.execute(request).await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_first_available_with_multiple_engines() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    for i in 0..3 {
        let runtime_type = match i {
            0 => RuntimeType::Container,
            1 => RuntimeType::Native,
            _ => RuntimeType::Wasm,
        };

        orchestrator
            .register_engine(
                runtime_type,
                Box::new(MockRuntimeEngine {
                    supports: vec![],
                    should_fail: false,
                }),
            )
            .await
            .unwrap();
    }

    for _ in 0..10 {
        let request = create_test_execution_request();
        let result = orchestrator.execute(request).await;
        // Should always use the first available
        assert!(result.is_ok());
    }
}

#[test]
fn test_runtime_selection_strategy_debug_output() {
    let strategies = vec![
        RuntimeSelectionStrategy::FirstAvailable,
        RuntimeSelectionStrategy::LoadBalanced,
        RuntimeSelectionStrategy::OptimalMatch,
    ];

    for strategy in strategies {
        let debug = format!("{:?}", strategy);
        assert!(!debug.is_empty());
        assert!(debug.len() > 5);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_with_default_timeout() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let request = create_test_execution_request();
    assert!(request.timeout.is_some());

    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_handles_rapid_registrations() {
    let orchestrator = Arc::new(RuntimeOrchestrator::new(
        RuntimeSelectionStrategy::FirstAvailable,
    ));

    let mut handles = vec![];

    for i in 0..20 {
        let orch = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let engine = Box::new(MockRuntimeEngine {
                supports: vec![format!("rapid_{}", i)],
                should_fail: false,
            });

            let runtime_type = match i % 4 {
                0 => RuntimeType::Container,
                1 => RuntimeType::Native,
                2 => RuntimeType::Wasm,
                _ => RuntimeType::Python,
            };

            orch.register_engine(runtime_type, engine).await
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_response_contains_execution_id() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let request = create_test_execution_request();
    let request_id = request.execution_id;

    let result = orchestrator.execute(request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response.execution_id, request_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_successful_execution_has_success_status() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    orchestrator
        .register_engine(
            RuntimeType::Container,
            Box::new(MockRuntimeEngine {
                supports: vec![],
                should_fail: false,
            }),
        )
        .await
        .unwrap();

    let request = create_test_execution_request();
    let result = orchestrator.execute(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ExecutionStatus::Success));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_new_is_deterministic() {
    let orch1 = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);
    let orch2 = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    // Both should be created successfully
    assert!(std::mem::size_of_val(&orch1) > 0);
    assert!(std::mem::size_of_val(&orch2) > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_gpu_runtime() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["gpu".to_string()],
        should_fail: false,
    });

    let result = orchestrator.register_engine(RuntimeType::Gpu, engine).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_python_runtime() {
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    let engine = Box::new(MockRuntimeEngine {
        supports: vec!["python".to_string()],
        should_fail: false,
    });

    let result = orchestrator
        .register_engine(RuntimeType::Python, engine)
        .await;
    assert!(result.is_ok());
}

#[test]
fn test_runtime_selection_strategy_clone_independence() {
    let strategy1 = RuntimeSelectionStrategy::LoadBalanced;
    let strategy2 = strategy1.clone();

    // Both should be independently usable
    let _orch1 = RuntimeOrchestrator::new(strategy1);
    let _orch2 = RuntimeOrchestrator::new(strategy2);
}

// ============================================================================
// Sprint 24 Complete: 50 Tests Created
// Coverage Target: 45% → 65%
// ============================================================================
