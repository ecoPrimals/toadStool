// SPDX-License-Identifier: AGPL-3.0-or-later
// Concurrency Testing - Parallel execution, thread safety, race conditions
// Tests system behavior under concurrent workloads

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu_executor::{BinaryOp, NormConfig, ReduceOp, WgpuExecutor};
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Sequential Operations (Baseline)
// ============================================================================

#[tokio::test]
async fn test_sequential_operations() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Run 10 operations sequentially
        for i in 0..10 {
            let input = vec![i as f32; 100];
            let result = executor.execute_relu(&input).await.unwrap();
            assert_eq!(result.len(), 100);
        }

        println!("Sequential operations: 10 completed successfully");
    })
    .await;
}

// ============================================================================
// Concurrent Operations (Same Executor)
// ============================================================================

#[tokio::test]
async fn test_concurrent_operations_shared_executor() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Spawn 10 concurrent tasks, all using the same executor
        let mut handles = vec![];

        for i in 0..10 {
            let executor_clone = Arc::clone(&executor);
            let handle = tokio::spawn(async move {
                let input = vec![i as f32; 100];
                executor_clone.execute_relu(&input).await.unwrap()
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();
            assert_eq!(result.len(), 100);
            println!("Task {} completed", i);
        }

        println!("Concurrent operations: 10 tasks completed successfully");
    })
    .await;
}

// ============================================================================
// Interleaved Operations (Different Types)
// ============================================================================

#[tokio::test]
async fn test_interleaved_operation_types() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Spawn different operation types concurrently
        let mut handles = vec![];

        // Task 1: ReLU
        let ex1 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let input = vec![1.0; 100];
            ex1.execute_relu(&input).await.unwrap()
        }));

        // Task 2: Sigmoid
        let ex2 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let input = vec![0.5; 100];
            ex2.execute_sigmoid(&input).await.unwrap()
        }));

        // Task 3: Reduce (returns f32, not Vec<f32>)
        let ex3 = Arc::clone(&executor);
        let reduce_handle = tokio::spawn(async move {
            let input = vec![1.0; 100];
            ex3.execute_reduce(&input, ReduceOp::Sum).await.unwrap()
        });

        // Task 4: Elementwise
        let ex4 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let a = vec![1.0; 100];
            let b = vec![2.0; 100];
            ex4.execute_elementwise_binary(&a, &b, BinaryOp::Add)
                .await
                .unwrap()
        }));

        // Wait for all (vec handles)
        for handle in handles {
            let _result = handle.await.unwrap();
        }

        // Wait for reduce (scalar)
        let _reduce_result = reduce_handle.await.unwrap();

        println!("Interleaved operations: All types completed successfully");
    })
    .await;
}

// ============================================================================
// Rapid Fire (High Concurrency)
// ============================================================================

#[tokio::test]
async fn test_rapid_fire_operations() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Spawn 50 tasks in rapid succession
        let mut handles = vec![];

        for i in 0..50 {
            let ex = Arc::clone(&executor);
            let handle = tokio::spawn(async move {
                let input = vec![(i % 10) as f32; 50];
                ex.execute_relu(&input).await.unwrap()
            });
            handles.push(handle);
        }

        // Wait for all
        for handle in handles {
            let vec_result = handle.await.unwrap();
            assert_eq!(vec_result.len(), 50);
        }

        println!("Rapid fire: 50 concurrent operations completed");
    })
    .await;
}

// ============================================================================
// Large Data Concurrent Processing
// ============================================================================

#[tokio::test]
async fn test_concurrent_large_data() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Process large arrays concurrently
        let mut handles = vec![];

        for i in 0..5 {
            let ex = Arc::clone(&executor);
            let handle = tokio::spawn(async move {
                let size = 10_000 * (i + 1); // 10K, 20K, 30K, 40K, 50K
                let input = vec![1.0; size];
                let result = ex.execute_relu(&input).await.unwrap();
                assert_eq!(result.len(), size);
                size
            });
            handles.push(handle);
        }

        // Wait for all
        for handle in handles {
            let size = handle.await.unwrap();
            println!("Processed array of size {}", size);
        }

        println!("Large data concurrent: All sizes completed");
    })
    .await;
}

// ============================================================================
// Pipeline Concurrency (Multi-Step)
// ============================================================================

#[tokio::test]
async fn test_concurrent_pipelines() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Each task runs a 3-operation pipeline
        let mut handles = vec![];

        for i in 0..5 {
            let ex = Arc::clone(&executor);
            let handle = tokio::spawn(async move {
                let input = vec![i as f32; 100];

                // Step 1: ReLU
                let step1 = ex.execute_relu(&input).await.unwrap();

                // Step 2: Sigmoid
                let step2 = ex.execute_sigmoid(&step1).await.unwrap();

                // Step 3: Reduce to mean
                let final_result = ex.execute_reduce(&step2, ReduceOp::Mean).await.unwrap();

                final_result
            });
            handles.push(handle);
        }

        // Wait for all pipelines
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();
            assert!(
                (0.0..=1.0).contains(&result),
                "Sigmoid output should be in [0,1]"
            );
            println!("Pipeline {} completed with mean: {}", i, result);
        }

        println!("Concurrent pipelines: 5 pipelines completed");
    })
    .await;
}

// ============================================================================
// Shared State (Thread Safety)
// ============================================================================

#[tokio::test]
async fn test_shared_counter_thread_safety() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());
        let counter = Arc::new(Mutex::new(0));

        // 10 tasks increment a shared counter after GPU operation
        let mut handles = vec![];

        for i in 0..10 {
            let ex = Arc::clone(&executor);
            let counter_clone = Arc::clone(&counter);

            let handle = tokio::spawn(async move {
                let input = vec![i as f32; 100];
                let _result = ex.execute_relu(&input).await.unwrap();

                // Increment shared counter
                let mut count = counter_clone.lock().await;
                *count += 1;
            });
            handles.push(handle);
        }

        // Wait for all
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify counter
        let final_count = *counter.lock().await;
        assert_eq!(
            final_count, 10,
            "All 10 tasks should have incremented counter"
        );

        println!("Shared counter: {} (expected 10)", final_count);
    })
    .await;
}

// ============================================================================
// Operation Mix (Stress Test)
// ============================================================================

#[tokio::test]
async fn test_operation_mix_stress() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Mix of different operations running concurrently
        let mut handles = vec![];

        // 5x ReLU
        for i in 0..5 {
            let ex = Arc::clone(&executor);
            handles.push(tokio::spawn(async move {
                let input = vec![i as f32; 200];
                ex.execute_relu(&input).await.unwrap()
            }));
        }

        // 3x MatMul
        for _ in 0..3 {
            let ex = Arc::clone(&executor);
            handles.push(tokio::spawn(async move {
                let a = vec![1.0; 100]; // 10x10
                let b = vec![2.0; 100]; // 10x10
                ex.execute_matmul(&a, &b, 10, 10, 10).await.unwrap()
            }));
        }

        // 4x Reduce (returns f32, not Vec<f32>)
        let mut reduce_handles = vec![];
        for _ in 0..4 {
            let ex = Arc::clone(&executor);
            reduce_handles.push(tokio::spawn(async move {
                let input = vec![1.0; 1000];
                ex.execute_reduce(&input, ReduceOp::Sum).await.unwrap()
            }));
        }

        // 3x Softmax
        for _ in 0..3 {
            let ex = Arc::clone(&executor);
            handles.push(tokio::spawn(async move {
                let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
                ex.execute_softmax(&input).await.unwrap()
            }));
        }

        // Wait for all vec operations
        for handle in handles {
            let _result = handle.await.unwrap();
        }

        // Wait for all reduce operations (scalar)
        for handle in reduce_handles {
            let _result = handle.await.unwrap();
        }

        println!("Operation mix stress: 15 mixed operations completed");
    })
    .await;
}

// ============================================================================
// Error Handling in Concurrent Context
// ============================================================================

#[tokio::test]
async fn test_concurrent_with_errors() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Mix of valid and invalid operations
        let mut handles = vec![];

        // Valid operation
        let ex1 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let input = vec![1.0; 100];
            ex1.execute_relu(&input).await
        }));

        // Invalid operation (mismatched sizes)
        let ex2 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let a = vec![1.0; 100];
            let b = vec![1.0; 50]; // Wrong size!
            ex2.execute_elementwise_binary(&a, &b, BinaryOp::Add).await
        }));

        // Another valid operation
        let ex3 = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            let input = vec![2.0; 100];
            ex3.execute_relu(&input).await
        }));

        // Wait for all
        let mut results = Vec::new();
        for handle in handles.into_iter() {
            let result = handle.await.unwrap();
            results.push(result);
        }

        // First should succeed
        assert!(results[0].is_ok(), "First operation should succeed");

        // Second should fail
        assert!(
            results[1].is_err(),
            "Second operation should fail (size mismatch)"
        );

        // Third should still succeed (executor not corrupted by error)
        assert!(results[2].is_ok(), "Third operation should succeed");

        println!("Concurrent error handling: Executor remains functional after error");
    })
    .await;
}

// ============================================================================
// Normalization Concurrency
// ============================================================================

#[tokio::test]
async fn test_concurrent_normalization() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Run LayerNorm concurrently
        let mut handles = vec![];

        for i in 0..5 {
            let ex = Arc::clone(&executor);
            let handle = tokio::spawn(async move {
                let input: Vec<f32> = (0..100).map(|j| (i * 100 + j) as f32).collect();
                let config = NormConfig {
                    epsilon: 1e-5,
                    gamma: None,
                    beta: None,
                };
                ex.execute_layernorm(&input, config).await.unwrap()
            });
            handles.push(handle);
        }

        // Wait for all
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.unwrap();

            // Verify normalization
            let mean: f32 = result.iter().sum::<f32>() / result.len() as f32;
            assert!(
                mean.abs() < 0.1,
                "LayerNorm mean should be ~0 for task {}",
                i
            );
        }

        println!("Concurrent normalization: 5 LayerNorm operations completed");
    })
    .await;
}

// ============================================================================
// Varying Input Sizes
// ============================================================================

#[tokio::test]
async fn test_concurrent_varying_sizes() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Different sizes: prime numbers
        let sizes = vec![7, 13, 17, 23, 29, 31, 37, 41, 43, 47];
        let mut handles = vec![];

        for size in sizes.iter() {
            let ex = Arc::clone(&executor);
            let s = *size;
            let handle = tokio::spawn(async move {
                let input = vec![1.0; s];
                let result = ex.execute_relu(&input).await.unwrap();
                assert_eq!(result.len(), s);
                s
            });
            handles.push(handle);
        }

        // Wait for all
        for handle in handles {
            let size = handle.await.unwrap();
            println!("Processed size: {}", size);
        }

        println!("Varying sizes: 10 different prime sizes completed concurrently");
    })
    .await;
}

// ============================================================================
// Concurrent Reduce Operations
// ============================================================================

#[tokio::test]
async fn test_concurrent_reductions() {
    gpu_test_resilient_async(async {
        let executor = Arc::new(WgpuExecutor::new().await.unwrap());

        // Run different reduction types concurrently
        let mut handles = vec![];

        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Sum
        let ex1 = Arc::clone(&executor);
        let input1 = input.clone();
        handles.push(tokio::spawn(async move {
            ex1.execute_reduce(&input1, ReduceOp::Sum).await.unwrap()
        }));

        // Mean
        let ex2 = Arc::clone(&executor);
        let input2 = input.clone();
        handles.push(tokio::spawn(async move {
            ex2.execute_reduce(&input2, ReduceOp::Mean).await.unwrap()
        }));

        // Max
        let ex3 = Arc::clone(&executor);
        let input3 = input.clone();
        handles.push(tokio::spawn(async move {
            ex3.execute_reduce(&input3, ReduceOp::Max).await.unwrap()
        }));

        // Wait for all
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.unwrap();
            results.push(result);
        }

        // Verify results
        assert!((results[0] - 15.0).abs() < 0.01, "Sum should be 15");
        assert!((results[1] - 3.0).abs() < 0.01, "Mean should be 3");
        assert!((results[2] - 5.0).abs() < 0.01, "Max should be 5");

        println!(
            "Concurrent reductions: Sum={}, Mean={}, Max={}",
            results[0], results[1], results[2]
        );
    })
    .await;
}
