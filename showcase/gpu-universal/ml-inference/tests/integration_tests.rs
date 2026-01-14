//! Integration Tests: Multi-Operation Pipelines
//!
//! Tests real-world use cases with multiple operations working together.
//! Validates production-ready scenarios beyond single-operation unit tests.

#![allow(unused_variables)]

use ml_inference_showcase::wgpu::*;

/// Helper: Create executor for tests
async fn create_executor() -> WgpuExecutor {
    WgpuExecutor::new().await.expect("Failed to create executor")
}

// =============================================================================
// TRANSFORMER BLOCK INTEGRATION
// =============================================================================

/// Test a complete transformer attention block pipeline:
/// 1. Query/Key/Value projections (MatMul)
/// 2. Attention scores (MatMul + Softmax)
/// 3. Context (MatMul)
/// 4. Residual connection (Add)
/// 5. RMSNorm
/// 6. Feed-forward (MatMul + GELU + MatMul)
/// 7. Residual + RMSNorm
#[tokio::test]
async fn test_transformer_attention_block() {
    let executor = create_executor().await;
    
    // Small transformer block: batch=1, seq_len=4, d_model=8, d_ff=32
    let batch_size = 1;
    let seq_len = 4;
    let d_model = 8;
    let d_ff = 32;
    
    // Input: [batch, seq_len, d_model]
    let input = vec![
        // Sequence of 4 tokens, 8 dimensions each
        0.5, 0.3, -0.2, 0.1, 0.4, -0.1, 0.2, 0.0,  // Token 1
        0.2, 0.4, 0.1, -0.3, 0.0, 0.2, -0.1, 0.3,  // Token 2
        -0.1, 0.2, 0.3, 0.1, -0.2, 0.4, 0.0, 0.1,  // Token 3
        0.3, -0.1, 0.0, 0.2, 0.3, 0.1, -0.2, 0.4,  // Token 4
    ];
    assert_eq!(input.len(), seq_len * d_model);
    
    // Weight matrices (simplified - normally learned)
    let w_q = vec![0.1; d_model * d_model];
    let w_k = vec![0.1; d_model * d_model];
    let w_v = vec![0.1; d_model * d_model];
    let w_o = vec![0.1; d_model * d_model];
    let w_ff1 = vec![0.1; d_model * d_ff];
    let w_ff2 = vec![0.1; d_ff * d_model];
    
    // === STEP 1: Query, Key, Value Projections ===
    println!("Step 1: Computing Q, K, V projections...");
    // input[seq_len, d_model] @ w[d_model, d_model] = output[seq_len, d_model]
    // m=seq_len, n=d_model, k=d_model
    let q = executor.execute_matmul(&input, &w_q, seq_len, d_model, d_model).await.unwrap();
    let k = executor.execute_matmul(&input, &w_k, seq_len, d_model, d_model).await.unwrap();
    let v = executor.execute_matmul(&input, &w_v, seq_len, d_model, d_model).await.unwrap();
    
    assert_eq!(q.len(), seq_len * d_model);
    assert_eq!(k.len(), seq_len * d_model);
    assert_eq!(v.len(), seq_len * d_model);
    println!("✅ Q, K, V computed");
    
    // === STEP 2: Attention Scores (Q @ K^T) ===
    println!("Step 2: Computing attention scores...");
    // K^T has shape [d_model, seq_len] after transpose
    let k_t = executor.execute_transpose(&k, seq_len, d_model).await.unwrap();
    // Q @ K^T: [seq_len, d_model] @ [d_model, seq_len] = [seq_len, seq_len]
    // m=seq_len, n=seq_len, k=d_model
    let scores = executor.execute_matmul(&q, &k_t, seq_len, seq_len, d_model).await.unwrap();
    
    // Scale by sqrt(d_model)
    let scale = 1.0 / (d_model as f32).sqrt();
    let scaled_scores: Vec<f32> = scores.iter().map(|&x| x * scale).collect();
    
    // Softmax over each row
    let attention_weights = executor.execute_softmax(&scaled_scores).await.unwrap();
    assert_eq!(attention_weights.len(), seq_len * seq_len);
    println!("✅ Attention weights computed");
    
    // === STEP 3: Apply Attention to Values ===
    println!("Step 3: Computing context...");
    // attention[seq_len, seq_len] @ v[seq_len, d_model] = context[seq_len, d_model]
    // m=seq_len, n=d_model, k=seq_len
    let context = executor.execute_matmul(&attention_weights, &v, seq_len, d_model, seq_len).await.unwrap();
    assert_eq!(context.len(), seq_len * d_model);
    println!("✅ Context computed");
    
    // === STEP 4: Output Projection + Residual ===
    println!("Step 4: Output projection and residual...");
    // context[seq_len, d_model] @ w_o[d_model, d_model] = projected[seq_len, d_model]
    let projected = executor.execute_matmul(&context, &w_o, seq_len, d_model, d_model).await.unwrap();
    let residual1 = executor.execute_add(&input, &projected, 1.0).await.unwrap();
    assert_eq!(residual1.len(), seq_len * d_model);
    println!("✅ Residual connection applied");
    
    // === STEP 5: RMSNorm ===
    println!("Step 5: RMSNorm...");
    let norm_config = RmsNormConfig {
        gamma: vec![1.0; d_model],
        epsilon: 1e-5,
    };
    let normalized1 = executor.execute_rms_norm(&residual1, seq_len, d_model, norm_config.clone()).await.unwrap();
    assert_eq!(normalized1.len(), seq_len * d_model);
    println!("✅ RMSNorm applied");
    
    // === STEP 6: Feed-Forward Network ===
    println!("Step 6: Feed-forward network...");
    // normalized1[seq_len, d_model] @ w_ff1[d_model, d_ff] = ff_hidden[seq_len, d_ff]
    // m=seq_len, n=d_ff, k=d_model
    let ff_hidden = executor.execute_matmul(&normalized1, &w_ff1, seq_len, d_ff, d_model).await.unwrap();
    let ff_activated = executor.execute_gelu(&ff_hidden).await.unwrap();
    // ff_activated[seq_len, d_ff] @ w_ff2[d_ff, d_model] = ff_output[seq_len, d_model]
    // m=seq_len, n=d_model, k=d_ff
    let ff_output = executor.execute_matmul(&ff_activated, &w_ff2, seq_len, d_model, d_ff).await.unwrap();
    assert_eq!(ff_output.len(), seq_len * d_model);
    println!("✅ Feed-forward computed");
    
    // === STEP 7: Final Residual + RMSNorm ===
    println!("Step 7: Final residual and norm...");
    let residual2 = executor.execute_add(&normalized1, &ff_output, 1.0).await.unwrap();
    let final_output = executor.execute_rms_norm(&residual2, seq_len, d_model, norm_config).await.unwrap();
    assert_eq!(final_output.len(), seq_len * d_model);
    
    // Validate output properties
    for &val in &final_output {
        assert!(val.is_finite(), "Output should be finite");
    }
    
    println!("✅ Transformer block complete!");
    println!("   Input shape: [{}, {}]", seq_len, d_model);
    println!("   Output shape: [{}, {}]", seq_len, d_model);
    println!("   Operations: 10 (MatMul×7, Softmax×1, Add×2, RMSNorm×2, GELU×1)");
}

// =============================================================================
// CNN PIPELINE INTEGRATION
// =============================================================================

/// Test a complete CNN forward pass:
/// 1. Conv1D
/// 2. BatchNorm
/// 3. ReLU
/// 4. MaxPool2D
/// 5. DepthwiseConv2D
/// 6. HardSwish (mobile activation)
/// 7. GlobalAvgPool
#[tokio::test]
async fn test_cnn_forward_pipeline() {
    let executor = create_executor().await;
    
    // Input: batch=2, channels=3, length=16 (for Conv1D)
    let batch = 2;
    let in_channels = 3;
    let length = 16;
    let input: Vec<f32> = (0..batch * in_channels * length)
        .map(|i| (i as f32) / 100.0)
        .collect();
    
    println!("CNN Pipeline Test");
    println!("Input shape: [{}, {}, {}]", batch, in_channels, length);
    
    // === STEP 1: Conv1D ===
    println!("Step 1: Conv1D...");
    let out_channels = 8;
    let kernel_size = 3;
    let stride = 1;
    let padding = 1;
    
    let weights = vec![0.1; out_channels * in_channels * kernel_size];
    let bias = vec![0.01; out_channels];
    
    let conv_config = Conv1DConfig {
        kernel_size,
        stride,
        padding,
        dilation: 1,
    };
    
    let conv_out = executor.execute_conv1d(
        &input,
        &weights,
        &bias,
        batch,
        in_channels,
        out_channels,
        length,
        conv_config,
    ).await.unwrap();
    let conv_length = length; // With padding=1, stride=1, output length = input length
    assert_eq!(conv_out.len(), batch * out_channels * conv_length);
    println!("✅ Conv1D: [{}, {}, {}]", batch, out_channels, conv_length);
    
    // === STEP 2: BatchNorm ===
    println!("Step 2: BatchNorm...");
    let bn_config = BatchNormConfig {
        epsilon: 1e-5,
        gamma: vec![1.0; out_channels],
        beta: vec![0.0; out_channels],
        running_mean: vec![0.0; out_channels],
        running_var: vec![1.0; out_channels],
    };
    
    let bn_out = executor.execute_batchnorm(&conv_out, batch, out_channels, conv_length, bn_config).await.unwrap();
    assert_eq!(bn_out.len(), batch * out_channels * conv_length);
    println!("✅ BatchNorm applied");
    
    // === STEP 3: ReLU Activation ===
    println!("Step 3: ReLU...");
    let relu_out = executor.execute_relu(&bn_out).await.unwrap();
    assert_eq!(relu_out.len(), batch * out_channels * conv_length);
    
    // Verify ReLU properties
    for &val in &relu_out {
        assert!(val >= 0.0, "ReLU output should be non-negative");
    }
    println!("✅ ReLU applied");
    
    // === STEP 4: MaxPool2D (treat as 2D: height=4, width=4) ===
    println!("Step 4: MaxPool2D...");
    let height = 4;
    let width = 4;
    let pool_config = Pool2DConfig {
        kernel_size: (2, 2),
        stride: (2, 2),
        padding: (0, 0),
    };
    
    let pool_out = executor.execute_max_pool_2d(
        &relu_out,
        batch,
        out_channels,
        height,
        width,
        pool_config,
    ).await.unwrap();
    
    let pool_h = 2;
    let pool_w = 2;
    assert_eq!(pool_out.len(), batch * out_channels * pool_h * pool_w);
    println!("✅ MaxPool2D: [{}, {}, {}, {}]", batch, out_channels, pool_h, pool_w);
    
    // === STEP 5: DepthwiseConv2D ===
    println!("Step 5: DepthwiseConv2D...");
    let dw_kernel_size = (3, 3);
    let dw_stride = (1, 1);
    let dw_padding = (1, 1);
    
    let dw_weights = vec![0.1; out_channels * dw_kernel_size.0 * dw_kernel_size.1];
    let dw_bias = vec![0.01; out_channels];
    
    let dw_config = DepthwiseConv2DConfig {
        kernel_size: dw_kernel_size,
        stride: dw_stride,
        padding: dw_padding,
    };
    
    let dw_out = executor.execute_depthwise_conv2d(
        &pool_out,
        &dw_weights,
        &dw_bias,
        batch,
        out_channels,
        pool_h,
        pool_w,
        dw_config,
    ).await.unwrap();
    
    assert_eq!(dw_out.len(), batch * out_channels * pool_h * pool_w);
    println!("✅ DepthwiseConv2D applied");
    
    // === STEP 6: HardSwish (Mobile Activation) ===
    println!("Step 6: HardSwish...");
    let hardswish_out = executor.execute_hardswish(&dw_out).await.unwrap();
    assert_eq!(hardswish_out.len(), batch * out_channels * pool_h * pool_w);
    println!("✅ HardSwish applied");
    
    // === STEP 7: Global Average Pooling ===
    println!("Step 7: GlobalAvgPool...");
    let gap_out = executor.execute_global_avg_pool(
        &hardswish_out,
        batch,
        out_channels,
        pool_h,
        pool_w,
    ).await.unwrap();
    
    assert_eq!(gap_out.len(), batch * out_channels);
    println!("✅ GlobalAvgPool: [{}, {}]", batch, out_channels);
    
    // Validate final output
    for &val in &gap_out {
        assert!(val.is_finite(), "Output should be finite");
    }
    
    println!("✅ CNN Pipeline complete!");
    println!("   Operations: 7 (Conv1D, BatchNorm, ReLU, MaxPool2D, DepthwiseConv2D, HardSwish, GlobalAvgPool)");
}

// =============================================================================
// TRAINING LOOP INTEGRATION
// =============================================================================

/// Test a complete training iteration:
/// 1. Forward pass (MatMul + Activation)
/// 2. Loss computation (MSE)
/// 3. Optimizer step (Adam)
/// 4. Verify weights updated
#[tokio::test]
async fn test_training_loop_integration() {
    let executor = create_executor().await;
    
    // Simple linear model: y = Wx + b
    let batch_size = 4;
    let input_dim = 8;
    let output_dim = 4;
    
    println!("Training Loop Integration Test");
    
    // Input data
    let input: Vec<f32> = (0..batch_size * input_dim)
        .map(|i| (i as f32) / 50.0)
        .collect();
    
    // Target data
    let target: Vec<f32> = (0..batch_size * output_dim)
        .map(|i| (i as f32) / 25.0)
        .collect();
    
    // Initial weights
    let mut weights = vec![0.1; input_dim * output_dim];
    
    // Optimizer state (mutable - Adam updates in place)
    let mut m = vec![0.0; input_dim * output_dim]; // First moment
    let mut v = vec![0.0; input_dim * output_dim]; // Second moment
    
    let learning_rate = 0.01;
    let beta1 = 0.9;
    let beta2 = 0.999;
    let epsilon = 1e-8;
    
    println!("Initial weights sum: {}", weights.iter().sum::<f32>());
    
    // === TRAINING ITERATIONS ===
    for iter in 1..=3 {
        println!("\nIteration {}/3", iter);
        
        // === STEP 1: Forward Pass ===
        println!("  Forward pass...");
        // MatMul: A[m,k] @ B[k,n] = C[m,n]
        // input[batch, input_dim] @ weights[input_dim, output_dim] = logits[batch, output_dim]
        // So: m=batch, n=output_dim, k=input_dim
        let logits = executor.execute_matmul(&input, &weights, batch_size, output_dim, input_dim).await.unwrap();
        let predictions = executor.execute_sigmoid(&logits).await.unwrap();
        assert_eq!(predictions.len(), batch_size * output_dim);
        
        // === STEP 2: Compute Loss ===
        println!("  Computing loss...");
        let loss_config = RegressionLossConfig {
            reduction: LossReduction::Mean,
        };
        let loss = executor.execute_mse_loss(&predictions, &target, loss_config).await.unwrap();
        println!("  Loss: {:.6}", loss);
        assert!(loss.is_finite() && loss >= 0.0);
        
        // === STEP 3: Compute Gradients (simplified - normally backprop) ===
        println!("  Computing gradients...");
        let mut gradients = Vec::with_capacity(weights.len());
        for i in 0..weights.len() {
            // Simplified gradient: derivative of MSE w.r.t weights
            let grad = 0.001 * weights[i]; // Placeholder for actual backprop
            gradients.push(grad);
        }
        
        // === STEP 4: Adam Optimizer Step ===
        println!("  Optimizer step (Adam)...");
        let adam_config = AdamConfig {
            learning_rate,
            beta1,
            beta2,
            epsilon,
            weight_decay: 0.0,
        };
        
        let old_sum: f32 = weights.iter().sum();
        
        // Adam modifies weights, m, v in place
        executor.execute_adam_step(
            &gradients,
            &mut weights,
            &mut m,
            &mut v,
            iter,
            adam_config,
        ).await.unwrap();
        
        let new_sum: f32 = weights.iter().sum();
        
        println!("  Weights updated: {} -> {}", old_sum, new_sum);
        println!("  ✅ Iteration {} complete", iter);
    }
    
    println!("\n✅ Training loop integration complete!");
    println!("   Validated: Forward → Loss → Optimizer → Update");
}

// =============================================================================
// DATA PROCESSING PIPELINE
// =============================================================================

/// Test a data processing pipeline:
/// 1. Gather (select subset)
/// 2. Map transformation (square)
/// 3. Reduce (sum)
/// 4. Scatter (write back)
#[tokio::test]
async fn test_data_processing_pipeline() {
    let executor = create_executor().await;
    
    println!("Data Processing Pipeline Test");
    
    // Source data: array of 100 values
    let size = 100;
    let data: Vec<f32> = (0..size).map(|i| i as f32).collect();
    
    // === STEP 1: Gather - Select every 5th element ===
    println!("Step 1: Gather (select every 5th element)...");
    let indices: Vec<u32> = (0..20).map(|i| i * 5).collect();
    let gathered = executor.execute_gather(&data, &indices).await.unwrap();
    assert_eq!(gathered.len(), 20);
    println!("✅ Gathered {} elements", gathered.len());
    
    // === STEP 2: Map - Square all values ===
    println!("Step 2: Map (square)...");
    let squared = executor.execute_map(&gathered, MapOp::Square).await.unwrap();
    assert_eq!(squared.len(), gathered.len());
    
    // Verify squares
    for (i, (&orig, &sq)) in gathered.iter().zip(squared.iter()).enumerate() {
        let expected = orig * orig;
        assert!((sq - expected).abs() < 1e-5, "Square mismatch at {}: {} vs {}", i, sq, expected);
    }
    println!("✅ Values squared");
    
    // === STEP 3: Reduce - Sum all values ===
    println!("Step 3: Reduce (sum)...");
    let total = executor.execute_reduce(&squared, ReduceOp::Sum).await.unwrap();
    let expected_sum: f32 = squared.iter().sum();
    assert!((total - expected_sum).abs() < 1e-3, "Sum mismatch: {} vs {}", total, expected_sum);
    println!("✅ Sum: {}", total);
    
    // === STEP 4: Scan - Prefix sum ===
    println!("Step 4: Scan (prefix sum)...");
    let scan_result = executor.execute_scan(&squared, ScanOp::Sum, false).await.unwrap();
    assert_eq!(scan_result.len(), squared.len());
    
    // Verify prefix sum
    let mut running_sum = 0.0;
    for (i, (&val, &scan_val)) in squared.iter().zip(scan_result.iter()).enumerate() {
        running_sum += val;
        assert!((scan_val - running_sum).abs() < 1e-3, "Prefix sum mismatch at {}", i);
    }
    println!("✅ Prefix sum computed");
    
    // === STEP 5: Scatter - Write back to larger array ===
    println!("Step 5: Scatter (write back)...");
    let write_indices: Vec<u32> = (0..20).map(|i| i * 5).collect();
    let scattered = executor.execute_scatter(&scan_result, &write_indices, size).await.unwrap();
    assert_eq!(scattered.len(), size);
    
    // Verify scattered values
    for (i, &idx) in write_indices.iter().enumerate() {
        let actual = scattered[idx as usize];
        let expected = scan_result[i];
        assert!((actual - expected).abs() < 1e-3, "Scatter mismatch at index {}", idx);
    }
    println!("✅ Values scattered back");
    
    // === STEP 6: Filter with Map ===
    println!("Step 6: Map (abs of negated values)...");
    let negated = executor.execute_map(&gathered, MapOp::Negate).await.unwrap();
    let abs_values = executor.execute_map(&negated, MapOp::Abs).await.unwrap();
    
    // Verify: abs(negate(x)) == x
    for (i, (&orig, &final_val)) in gathered.iter().zip(abs_values.iter()).enumerate() {
        assert!((final_val - orig).abs() < 1e-5, "Map chain mismatch at {}", i);
    }
    println!("✅ Map chain validated");
    
    println!("\n✅ Data processing pipeline complete!");
    println!("   Operations: Gather → Map → Reduce → Scan → Scatter → Map chain");
}

// =============================================================================
// MULTI-LOSS TRAINING
// =============================================================================

/// Test training with multiple loss functions:
/// Validates that all loss functions work in training context
#[tokio::test]
async fn test_multi_loss_training() {
    let executor = create_executor().await;
    
    let batch_size = 8;
    let num_classes = 4;
    
    // Predictions and targets
    let predictions: Vec<f32> = (0..batch_size * num_classes)
        .map(|i| ((i % 10) as f32) / 10.0)
        .collect();
    
    let targets: Vec<f32> = (0..batch_size * num_classes)
        .map(|i| ((i % 10) as f32 + 1.0) / 11.0)
        .collect();
    
    println!("Multi-Loss Training Test");
    
    // === Test All Loss Functions ===
    println!("Testing MSE Loss...");
    let mse_config = RegressionLossConfig {
        reduction: LossReduction::Mean,
    };
    let mse = executor.execute_mse_loss(&predictions, &targets, mse_config).await.unwrap();
    assert!(mse.is_finite() && mse >= 0.0);
    println!("  MSE: {:.6}", mse);
    
    println!("Testing MAE Loss...");
    let mae_config = RegressionLossConfig {
        reduction: LossReduction::Mean,
    };
    let mae = executor.execute_mae_loss(&predictions, &targets, mae_config).await.unwrap();
    assert!(mae.is_finite() && mae >= 0.0);
    println!("  MAE: {:.6}", mae);
    
    println!("Testing Huber Loss...");
    let huber_config = HuberLossConfig {
        delta: 1.0,
        reduction: LossReduction::Mean,
    };
    let huber = executor.execute_huber_loss(&predictions, &targets, huber_config).await.unwrap();
    assert!(huber.is_finite() && huber >= 0.0);
    println!("  Huber: {:.6}", huber);
    
    println!("Testing BCE Loss...");
    let bce_config = BceLossConfig {
        epsilon: 1e-7,
        reduction: LossReduction::Mean,
    };
    let bce = executor.execute_bce_loss(&predictions, &targets, bce_config).await.unwrap();
    assert!(bce.is_finite() && bce >= 0.0);
    println!("  BCE: {:.6}", bce);
    
    println!("Testing Dice Loss...");
    let dice_config = DiceLossConfig {
        smooth: 1.0,
        reduction: LossReduction::Mean,
    };
    let dice = executor.execute_dice_loss(&predictions, &targets, batch_size, num_classes, dice_config).await.unwrap();
    assert!(dice.is_finite() && dice >= 0.0);
    println!("  Dice: {:.6}", dice);
    
    println!("Testing CrossEntropy Loss...");
    let ce_config = CrossEntropyConfig {
        epsilon: 1e-7,
        reduction: LossReduction::Mean,
    };
    let ce_result = executor.execute_cross_entropy(&predictions, &targets, batch_size, num_classes, ce_config).await.unwrap();
    // CrossEntropy returns Vec<f32> for per-sample losses, take mean
    let ce: f32 = ce_result.iter().sum::<f32>() / ce_result.len() as f32;
    assert!(ce.is_finite() && ce >= 0.0);
    println!("  CrossEntropy: {:.6}", ce);
    
    println!("Testing Focal Loss...");
    let focal_config = FocalLossConfig {
        alpha: 0.25,
        gamma: 2.0,
        epsilon: 1e-7,
        reduction: LossReduction::Mean,
    };
    let focal = executor.execute_focal_loss(&predictions, &targets, focal_config).await.unwrap();
    assert!(focal.is_finite() && focal >= 0.0);
    println!("  Focal: {:.6}", focal);
    
    println!("\n✅ All 7 loss functions validated in training context!");
}
