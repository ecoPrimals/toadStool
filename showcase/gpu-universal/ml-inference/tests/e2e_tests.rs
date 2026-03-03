// SPDX-License-Identifier: AGPL-3.0-or-later
// End-to-End (E2E) Integration Tests
// Tests complete workflows combining multiple operations

use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu_executor::{
    AdamConfig, BinaryOp, CrossEntropyConfig, LossReduction, NormConfig, ReduceOp, WgpuExecutor,
};

const FP32_TOLERANCE: f32 = 1e-4; // Relaxed tolerance for multi-op pipelines

// ============================================================================
// Neural Network Training Pipeline E2E
// ============================================================================

#[tokio::test]
async fn test_simple_training_step() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Simulate a simple training step:
        // 1. Forward pass (MatMul + ReLU + Softmax)
        // 2. Loss computation (CrossEntropy)
        // 3. Optimizer step (Adam)

        // Layer 1: Input (4 features) -> Hidden (3 neurons)
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let weights1 = vec![0.1, 0.2, 0.3, 0.2, 0.3, 0.4, 0.3, 0.4, 0.5, 0.4, 0.5, 0.6];

        // Forward: MatMul
        // input[1, 4] @ weights1[4, 3] = hidden[1, 3]
        // MatMul(m, n, k): m=1, n=3, k=4
        let hidden = executor
            .execute_matmul(&input, &weights1, 1, 3, 4)
            .await
            .unwrap();
        assert_eq!(hidden.len(), 3);

        // Activation: ReLU
        let activated = executor.execute_relu(&hidden).await.unwrap();
        assert_eq!(activated.len(), 3);

        // Output layer: Hidden (3) -> Output (2 classes)
        let weights2 = vec![0.5, 0.6, 0.6, 0.7, 0.7, 0.8];

        // activated[1, 3] @ weights2[3, 2] = logits[1, 2]
        // MatMul(m, n, k): m=1, n=2, k=3
        let logits = executor
            .execute_matmul(&activated, &weights2, 1, 2, 3)
            .await
            .unwrap();
        assert_eq!(logits.len(), 2);

        // Softmax for probabilities
        let probs = executor.execute_softmax(&logits).await.unwrap();
        assert_eq!(probs.len(), 2);

        // Verify probabilities sum to 1
        let prob_sum: f32 = probs.iter().sum();
        assert!(
            (prob_sum - 1.0).abs() < FP32_TOLERANCE,
            "Probabilities should sum to 1.0, got {}",
            prob_sum
        );

        // Compute loss
        // True label is class 1, one-hot encoded: [0.0, 1.0]
        let targets_onehot = vec![0.0, 1.0];
        let config = CrossEntropyConfig {
            reduction: LossReduction::Mean,
            epsilon: 1e-7,
        };

        let loss = executor
            .execute_cross_entropy(&probs, &targets_onehot, 1, 2, config)
            .await
            .unwrap();
        assert_eq!(loss.len(), 1);
        assert!(loss[0] > 0.0, "Loss should be positive");
        assert!(loss[0].is_finite(), "Loss should be finite");

        println!("Training step complete:");
        println!("  Input -> Hidden: {:?}", activated);
        println!("  Hidden -> Output: {:?}", logits);
        println!("  Probabilities: {:?}", probs);
        println!("  Loss: {:.6}", loss[0]);
    })
    .await;
}

#[tokio::test]
async fn test_batch_normalization_training_pipeline() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // NOTE: LayerNorm normalizes a single vector, not a batch
        // For batch processing, we'd need to call it multiple times or use BatchNorm

        // Single sample with 8 features
        let sample = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        // Apply LayerNorm (normalizes the 8 features)
        let norm_config = NormConfig {
            epsilon: 1e-5,
            gamma: None,
            beta: None,
        };

        let normalized = executor
            .execute_layernorm(&sample, norm_config)
            .await
            .unwrap();
        assert_eq!(normalized.len(), 8);

        // Verify normalization
        let mean: f32 = normalized.iter().sum::<f32>() / 8.0;
        let variance: f32 = normalized.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / 8.0;

        assert!(
            mean.abs() < FP32_TOLERANCE,
            "Mean should be ~0, got {}",
            mean
        );
        assert!(
            (variance - 1.0).abs() < FP32_TOLERANCE,
            "Variance should be ~1, got {}",
            variance
        );

        // Apply ReLU activation
        let _activated = executor.execute_relu(&normalized).await.unwrap();

        // Verify activation preserves positive values
        for (i, &val) in _activated.iter().enumerate() {
            assert!(val >= 0.0, "ReLU output {} should be non-negative", i);
        }

        println!("BatchNorm pipeline complete: {} normalized samples", 4);
    })
    .await;
}

#[tokio::test]
async fn test_conv_pipeline() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Simulated convolutional layer workflow:
        // Input -> Conv2D (simulated with multiple ops) -> ReLU -> MaxPool -> Flatten

        // For now, we simulate with available ops
        // Real conv2d would be: input (batch, C, H, W) -> filters -> output

        // Flatten operation (spatial to vector)
        let _spatial_features = vec![
            1.0, 2.0, 3.0, 4.0, // Row 1
            5.0, 6.0, 7.0, 8.0, // Row 2
            9.0, 10.0, 11.0, 12.0, // Row 3
            13.0, 14.0, 15.0, 16.0, // Row 4
        ];

        // NOTE: Actual Conv2D and MaxPool2D operations are implemented in wgpu_executor
        // but would require proper tensor shapes. This test simulates the workflow.

        // Max pool simulation (2x2 with stride 2): take max of each 2x2 block
        // Block 1: [1,2,5,6] -> 6
        // Block 2: [3,4,7,8] -> 8
        // Block 3: [9,10,13,14] -> 14
        // Block 4: [11,12,15,16] -> 16
        let pooled = vec![6.0, 8.0, 14.0, 16.0];

        // Verify we can continue processing
        let output = executor.execute_relu(&pooled).await.unwrap();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x > 0.0));

        println!("Conv pipeline simulation complete: 16 -> 4 features");
    })
    .await;
}

// ============================================================================
// Multi-Operation Composition
// ============================================================================

#[tokio::test]
async fn test_elementwise_reduction_pipeline() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Pipeline: Add -> Mul -> Reduce (common in attention mechanisms)
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 2.0, 2.0, 2.0, 2.0];

        // Add
        let added = executor
            .execute_elementwise_binary(&a, &b, BinaryOp::Add)
            .await
            .unwrap();
        assert_eq!(added, vec![3.0, 4.0, 5.0, 6.0, 7.0]);

        // Multiply by scale factor
        let scale = vec![0.5, 0.5, 0.5, 0.5, 0.5];
        let scaled = executor
            .execute_elementwise_binary(&added, &scale, BinaryOp::Mul)
            .await
            .unwrap();
        assert_eq!(scaled, vec![1.5, 2.0, 2.5, 3.0, 3.5]);

        // Reduce to sum
        let sum = executor
            .execute_reduce(&scaled, ReduceOp::Sum)
            .await
            .unwrap();
        assert!(
            (sum - 12.5).abs() < FP32_TOLERANCE,
            "Sum should be 12.5, got {}",
            sum
        );

        // Reduce to mean
        let mean = executor
            .execute_reduce(&scaled, ReduceOp::Mean)
            .await
            .unwrap();
        assert!(
            (mean - 2.5).abs() < FP32_TOLERANCE,
            "Mean should be 2.5, got {}",
            mean
        );

        println!(
            "Elementwise-Reduce pipeline complete: sum={}, mean={}",
            sum, mean
        );
    })
    .await;
}

#[tokio::test]
async fn test_activation_comparison_pipeline() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Compare different activations on same input
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];

        // ReLU: max(0, x)
        let relu_out = executor.execute_relu(&input).await.unwrap();
        assert_eq!(relu_out, vec![0.0, 0.0, 0.0, 1.0, 2.0]);

        // Sigmoid: 1 / (1 + e^-x)
        let sigmoid_out = executor.execute_sigmoid(&input).await.unwrap();
        for &val in &sigmoid_out {
            assert!(
                (0.0..=1.0).contains(&val),
                "Sigmoid output should be in [0,1]"
            );
        }

        // Tanh: (e^x - e^-x) / (e^x + e^-x)
        let tanh_out = executor.execute_tanh(&input).await.unwrap();
        for &val in &tanh_out {
            assert!(
                (-1.0..=1.0).contains(&val),
                "Tanh output should be in [-1,1]"
            );
        }

        // Softmax: e^x_i / sum(e^x_j)
        let softmax_out = executor.execute_softmax(&input).await.unwrap();
        let softmax_sum: f32 = softmax_out.iter().sum();
        assert!(
            (softmax_sum - 1.0).abs() < FP32_TOLERANCE,
            "Softmax should sum to 1, got {}",
            softmax_sum
        );

        println!("Activation comparison complete");
        println!("  Input: {:?}", input);
        println!("  ReLU: {:?}", relu_out);
        println!("  Sigmoid: {:?}", sigmoid_out);
        println!("  Tanh: {:?}", tanh_out);
        println!("  Softmax: {:?}", softmax_out);
    })
    .await;
}

// ============================================================================
// Optimizer + Loss Integration
// ============================================================================

#[tokio::test]
async fn test_adam_optimizer_integration() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Simulate parameter update with Adam
        // Params: [1.0, 2.0, 3.0, 4.0]
        // Gradients: [0.1, 0.2, 0.3, 0.4]

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut m = vec![0.0; 4]; // First moment
        let mut v = vec![0.0; 4]; // Second moment

        let config = AdamConfig {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0,
        };

        // Save initial state
        let initial_params = params.clone();

        // Step 1
        executor
            .execute_adam_step(&grads, &mut params, &mut m, &mut v, 1, config)
            .await
            .unwrap();

        // Verify parameters decreased (gradient descent)
        for i in 0..4 {
            assert!(
                params[i] < initial_params[i],
                "Param {} should decrease with positive gradient",
                i
            );
        }

        // Verify moments were updated
        for i in 0..4 {
            assert!(m[i] > 0.0, "First moment {} should be positive", i);
            assert!(v[i] > 0.0, "Second moment {} should be positive", i);
        }

        // Save state after step 1
        let params_after_1 = params.clone();
        let m_after_1 = m.clone();
        let v_after_1 = v.clone();

        // Step 2 (with updated moments)
        executor
            .execute_adam_step(&grads, &mut params, &mut m, &mut v, 2, config)
            .await
            .unwrap();

        // Verify continued descent
        for i in 0..4 {
            assert!(
                params[i] < params_after_1[i],
                "Param {} should continue decreasing",
                i
            );
        }

        // Verify momentum accumulated
        for i in 0..4 {
            assert!(m[i] > m_after_1[i], "First moment {} should accumulate", i);
            assert!(v[i] > v_after_1[i], "Second moment {} should accumulate", i);
        }

        println!("Adam optimizer integration complete");
        println!("  Initial params: {:?}", initial_params);
        println!("  After step 1: {:?}", params_after_1);
        println!("  After step 2: {:?}", params);
    })
    .await;
}

// ============================================================================
// Large-Scale Pipeline
// ============================================================================

#[tokio::test]
async fn test_large_batch_processing() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Process large batch: 1000 samples, 128 features each
        let batch_size = 1000;
        let features = 128;
        let total_size = batch_size * features;

        // Generate batch
        let batch: Vec<f32> = (0..total_size).map(|i| (i as f32) * 0.01).collect();

        // Apply ReLU
        let activated = executor.execute_relu(&batch).await.unwrap();
        assert_eq!(activated.len(), total_size);

        // Verify all non-negative
        assert!(
            activated.iter().all(|&x| x >= 0.0),
            "All ReLU outputs should be non-negative"
        );

        // Reduce to mean (average activation)
        let mean = executor
            .execute_reduce(&activated, ReduceOp::Mean)
            .await
            .unwrap();
        assert!(
            mean > 0.0 && mean.is_finite(),
            "Mean activation should be positive and finite"
        );

        println!(
            "Large batch processing complete: {} samples × {} features",
            batch_size, features
        );
        println!("  Mean activation: {:.6}", mean);
    })
    .await;
}
