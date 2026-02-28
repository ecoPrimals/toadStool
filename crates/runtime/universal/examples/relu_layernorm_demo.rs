//! ReLU and LayerNorm Operations Demo
//!
//! Demonstrates:
//! - ReLU: Rectified Linear Unit activation (max(0, x))
//! - LeakyReLU: Variant with small negative slope
//! - LayerNorm: Layer normalization (composite pattern)
//!
//! These operations are fundamental building blocks for neural networks.

use std::collections::HashMap;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::*;
use toadstool_runtime_universal::ComputeError;

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: ReLU & LayerNorm Demo               ║");
    println!("║  barraCuda Phase 1 - Activations & Normalization        ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Standard ReLU
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: ReLU (Rectified Linear Unit)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("ReLU: f(x) = max(0, x)");
    println!("Purpose: Introduce non-linearity, zero out negative values");
    println!();

    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
    println!("Input:  {:?}", input);
    println!();

    let relu_workload = Workload {
        operation: OperationType::ReLU,
        data_type: DataType::F32,
        num_operations: input.len(),
        required_memory: input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(input.clone()),
        params: WorkloadParams {
            params: HashMap::new(), // No params = standard ReLU
        },
    };

    let relu_result = runtime.execute_optimal(relu_workload).await?;

    if let WorkloadData::F32Vec(output) = &relu_result.data {
        println!("Output: {:?}", output);
        let expected = vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0];
        println!("Expected: {:?}", expected);
        let all_match = output
            .iter()
            .zip(&expected)
            .all(|(a, b)| (a - b).abs() < 1e-6);
        println!(
            "Verification: {} ✅",
            if all_match { "PASS" } else { "FAIL" }
        );
    }

    println!();
    println!("Executed on: {}", relu_result.metadata.unit_name);
    println!("Duration:    {:?}", relu_result.metadata.duration);
    println!();

    // Demo 2: LeakyReLU
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: LeakyReLU (with negative slope)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("LeakyReLU: f(x) = max(alpha * x, x)");
    println!("Purpose: Allow small gradient for negative values");
    println!("Alpha: 0.01 (1% of input for negative values)");
    println!();

    let mut leaky_params = HashMap::new();
    leaky_params.insert("alpha".to_string(), ParamValue::Float(0.01));

    let leaky_workload = Workload {
        operation: OperationType::ReLU,
        data_type: DataType::F32,
        num_operations: input.len(),
        required_memory: input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(input.clone()),
        params: WorkloadParams {
            params: leaky_params,
        },
    };

    let leaky_result = runtime.execute_optimal(leaky_workload).await?;

    if let WorkloadData::F32Vec(output) = &leaky_result.data {
        println!("Input:  {:?}", input);
        println!("Output: {:?}", output);
        println!();
        println!("Observations:");
        println!("  • Negative values: Not zero, but small (1% of input)");
        println!("  • Positive values: Unchanged");
        println!("  • Benefit: Prevents \"dying ReLU\" problem");
    }

    println!();
    println!("Executed on: {}", leaky_result.metadata.unit_name);
    println!("Duration:    {:?}", leaky_result.metadata.duration);
    println!();

    // Demo 3: Layer Normalization
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Layer Normalization");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("LayerNorm: (x - mean) / sqrt(variance + epsilon)");
    println!("Purpose: Normalize layer activations (mean=0, variance=1)");
    println!();

    let layer_input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    println!("Input:  {:?}", layer_input);
    println!();

    let layernorm_workload = Workload {
        operation: OperationType::LayerNorm,
        data_type: DataType::F32,
        num_operations: layer_input.len() * 3, // mean + variance + normalize
        required_memory: layer_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(layer_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(), // Use default epsilon
        },
    };

    let layernorm_result = runtime.execute_optimal(layernorm_workload).await?;

    if let WorkloadData::F32Vec(output) = &layernorm_result.data {
        println!("Output: {:?}", output);
        println!();

        // Verify properties of LayerNorm
        let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;
        let variance: f32 =
            output.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / output.len() as f32;

        println!("Properties:");
        println!("  Mean: {:.6} (should be ~0.0)", mean);
        println!("  Variance: {:.6} (should be ~1.0)", variance);
        println!();

        let mean_close_to_zero = mean.abs() < 1e-5;
        let variance_close_to_one = (variance - 1.0).abs() < 0.01;

        println!(
            "Verification: {} ✅",
            if mean_close_to_zero && variance_close_to_one {
                "PASS"
            } else {
                "FAIL"
            }
        );
    }

    println!();
    println!("Executed on: {}", layernorm_result.metadata.unit_name);
    println!("Duration:    {:?}", layernorm_result.metadata.duration);
    println!();

    // Demo 4: LayerNorm Composite Structure
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: LayerNorm Decomposition");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("LayerNorm is a COMPOSITE pattern (like Softmax!):");
    println!();
    println!("4-Phase Decomposition:");
    println!("  1. Reduce (calculate mean)");
    println!("  2. Map (subtract mean: x - mean)");
    println!("  3. Reduce (calculate variance: mean of squared differences)");
    println!("  4. Map (normalize: (x - mean) / sqrt(variance + epsilon))");
    println!();
    println!("Similar to Softmax:");
    println!("  Softmax:    Reduce → Map → Reduce → Map");
    println!("  LayerNorm:  Reduce → Map → Reduce → Map");
    println!();
    println!("Pattern: Both are 4-phase normalization composites!");
    println!();

    // Demo 5: ReLU + LayerNorm Pipeline
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 5: ReLU + LayerNorm Pipeline");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Common pattern in neural networks:");
    println!("  Linear → ReLU → LayerNorm");
    println!();

    let pipeline_input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    println!("Step 1: Input: {:?}", pipeline_input);
    println!();

    // Apply ReLU
    let relu_pipeline = Workload {
        operation: OperationType::ReLU,
        data_type: DataType::F32,
        num_operations: pipeline_input.len(),
        required_memory: pipeline_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(pipeline_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let relu_pipeline_result = runtime.execute_optimal(relu_pipeline).await?;

    if let WorkloadData::F32Vec(relu_output) = relu_pipeline_result.data {
        println!("Step 2: After ReLU: {:?}", relu_output);
        println!();

        // Apply LayerNorm
        let layernorm_pipeline = Workload {
            operation: OperationType::LayerNorm,
            data_type: DataType::F32,
            num_operations: relu_output.len() * 3,
            required_memory: relu_output.len() * std::mem::size_of::<f32>() * 2,
            input: WorkloadData::F32Vec(relu_output.clone()),
            params: WorkloadParams {
                params: HashMap::new(),
            },
        };

        let layernorm_pipeline_result = runtime.execute_optimal(layernorm_pipeline).await?;

        if let WorkloadData::F32Vec(final_output) = &layernorm_pipeline_result.data {
            println!("Step 3: After LayerNorm: {:?}", final_output);
            println!();
            println!("Pipeline complete! ✅");
        }
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("ReLU:");
    println!("  • Parallelism: 100% embarrassingly parallel");
    println!("  • Pattern: Simple Map (max(0, x))");
    println!("  • CPU: Excellent (trivial operation, perfect for SIMD)");
    println!("  • GPU: Excellent (naturally parallel)");
    println!("  • Compute intensity: Very low (single comparison + select)");
    println!("  • Memory pattern: Streaming (read, compute, write)");
    println!();

    println!("LeakyReLU:");
    println!("  • Parallelism: Same as ReLU (100% parallel)");
    println!("  • Slight overhead: One multiply for negative values");
    println!("  • Benefit: Prevents dying ReLU (neurons stuck at zero)");
    println!("  • Configurable: Alpha parameter (typically 0.01)");
    println!();

    println!("LayerNorm:");
    println!("  • Parallelism: Composite (4 phases)");
    println!("  • Decomposition:");
    println!("    1. Reduce (mean) - tree-based");
    println!("    2. Map (x - mean) - embarrassingly parallel");
    println!("    3. Reduce (variance) - tree-based");
    println!("    4. Map (normalize) - embarrassingly parallel");
    println!("  • CPU: Excellent (Rayon handles all phases)");
    println!("  • GPU: Excellent (each phase optimizes independently)");
    println!("  • Numerical stability: Epsilon prevents division by zero");
    println!();

    println!("Key Insights:");
    println!("  1. ReLU is the simplest activation (just max operation)");
    println!("  2. LeakyReLU solves dying ReLU with minimal cost");
    println!("  3. LayerNorm is ANOTHER 4-phase composite!");
    println!("     • Same structure as Softmax (R→M→R→M)");
    println!("     • Different operations in each phase");
    println!("  4. Pipeline: ReLU + LayerNorm is common in transformers");
    println!("  5. barraCuda can fuse multi-phase operations");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • ReLU: Trivial to fuse with previous operation");
    println!("  • LayerNorm: Fuse 4 phases into single kernel");
    println!("  • Pipeline: Detect ReLU → LayerNorm → fuse all");
    println!("  • SIMD: ReLU perfect for vectorization");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("These operations are fundamental for:");
    println!("  • Neural networks (ReLU: most popular activation)");
    println!("  • Transformers (LayerNorm: critical for training stability)");
    println!("  • CNNs (ReLU after convolution layers)");
    println!("  • Deep learning (prevent vanishing gradients)");
    println!();
    println!("Universal Runtime makes these operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
