// SPDX-License-Identifier: AGPL-3.0-or-later
//! GELU and Dropout Operations Demo

#![allow(clippy::cast_precision_loss)]
//!
//! Demonstrates:
//! - GELU: Gaussian Error Linear Unit (modern smooth activation)
//! - Dropout: Random masking for regularization
//!
//! These operations are fundamental for modern neural networks (especially Transformers).

use std::collections::HashMap;
use toadstool_runtime_universal::ComputeError;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::{
    DataType, OperationType, ParamValue, Workload, WorkloadData, WorkloadParams,
};

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: GELU & Dropout Demo                 ║");
    println!("║  barraCuda Phase 1 - Modern Activations & Regularization║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: GELU Activation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: GELU (Gaussian Error Linear Unit)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("GELU: x * sigmoid(1.702 * x) (approximate)");
    println!("Purpose: Smooth activation, better than ReLU for some tasks");
    println!();

    let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
    println!("Input: {input:?}");
    println!();

    let gelu_workload = Workload {
        operation: OperationType::GELU,
        data_type: DataType::F32,
        num_operations: input.len(),
        required_memory: input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let gelu_result = runtime.execute_optimal(gelu_workload).await?;

    if let WorkloadData::F32Vec(gelu_output) = &gelu_result.data {
        println!("GELU Output: {gelu_output:?}");

        // Compare with ReLU
        let relu_workload = Workload {
            operation: OperationType::ReLU,
            data_type: DataType::F32,
            num_operations: input.len(),
            required_memory: input.len() * std::mem::size_of::<f32>() * 2,
            input: WorkloadData::F32Vec(input.clone()),
            params: WorkloadParams {
                params: HashMap::new(),
            },
        };

        let relu_result = runtime.execute_optimal(relu_workload).await?;

        if let WorkloadData::F32Vec(relu_output) = &relu_result.data {
            println!("ReLU Output: {relu_output:?}");
            println!();
            println!("Differences:");
            println!("  • GELU: Smooth, non-zero for negative values");
            println!("  • ReLU: Hard cutoff at zero");
            println!("  • GELU allows small negative gradients (better learning)");
        }
    }

    println!();
    println!("Executed on: {}", gelu_result.metadata.unit_name);
    println!("Duration:    {:?}", gelu_result.metadata.duration);
    println!();

    // Demo 2: Dropout with Different Rates
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Dropout (Random Masking)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Dropout: Randomly set elements to 0 (for regularization)");
    println!("Purpose: Prevent overfitting by forcing network redundancy");
    println!();

    let dropout_input: Vec<f32> = (1..=10).map(|i| i as f32).collect();
    println!("Input: {dropout_input:?}");
    println!();

    // Test different dropout rates
    for dropout_rate in [0.0, 0.3, 0.5, 0.7] {
        let mut dropout_params = HashMap::new();
        dropout_params.insert("dropout_rate".to_string(), ParamValue::Float(dropout_rate));

        let dropout_workload = Workload {
            operation: OperationType::Dropout,
            data_type: DataType::F32,
            num_operations: dropout_input.len(),
            required_memory: dropout_input.len() * std::mem::size_of::<f32>() * 2,
            input: WorkloadData::F32Vec(dropout_input.clone()),
            params: WorkloadParams {
                params: dropout_params,
            },
        };

        let dropout_result = runtime.execute_optimal(dropout_workload).await?;

        if let WorkloadData::F32Vec(output) = &dropout_result.data {
            let zeros = output.iter().filter(|&&x| x == 0.0).count();
            let non_zeros = output.len() - zeros;
            println!("Dropout rate {dropout_rate:.1}: {output:?}");
            println!(
                "  → {} zeros, {} non-zeros (scaled by {:.2})",
                zeros,
                non_zeros,
                if dropout_rate > 0.0 {
                    1.0 / (1.0 - dropout_rate)
                } else {
                    1.0
                }
            );
        }
    }

    println!();
    println!("Note: Non-zero values are scaled up to maintain expected value");
    println!("      This is 'inverted dropout' (preferred in modern networks)");
    println!();

    // Demo 3: GELU + Dropout Pipeline
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: GELU + Dropout Pipeline");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Common pattern in Transformers:");
    println!("  Feed-forward → GELU → Dropout");
    println!();

    let pipeline_input = vec![0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
    println!("Step 1: Input: {pipeline_input:?}");
    println!();

    // Apply GELU
    let gelu_pipeline = Workload {
        operation: OperationType::GELU,
        data_type: DataType::F32,
        num_operations: pipeline_input.len(),
        required_memory: pipeline_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(pipeline_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let gelu_pipeline_result = runtime.execute_optimal(gelu_pipeline).await?;

    if let WorkloadData::F32Vec(gelu_out) = gelu_pipeline_result.data {
        println!("Step 2: After GELU: {gelu_out:?}");
        println!();

        // Apply Dropout
        let mut dropout_params = HashMap::new();
        dropout_params.insert("dropout_rate".to_string(), ParamValue::Float(0.3));

        let dropout_pipeline = Workload {
            operation: OperationType::Dropout,
            data_type: DataType::F32,
            num_operations: gelu_out.len(),
            required_memory: gelu_out.len() * std::mem::size_of::<f32>() * 2,
            input: WorkloadData::F32Vec(gelu_out.clone()),
            params: WorkloadParams {
                params: dropout_params,
            },
        };

        let dropout_pipeline_result = runtime.execute_optimal(dropout_pipeline).await?;

        if let WorkloadData::F32Vec(final_output) = &dropout_pipeline_result.data {
            println!("Step 3: After Dropout (30%): {final_output:?}");
            println!();
            println!("Pipeline complete! ✅");
        }
    }

    println!();

    // Demo 4: Use Cases and Comparisons
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Activation Function Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let comparison_input = vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];

    println!("Input: {comparison_input:?}");
    println!();

    // Get ReLU output
    let relu_comp = Workload {
        operation: OperationType::ReLU,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let relu_comp_result = runtime.execute_optimal(relu_comp).await?;

    // Get GELU output
    let gelu_comp = Workload {
        operation: OperationType::GELU,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let gelu_comp_result = runtime.execute_optimal(gelu_comp).await?;

    if let (WorkloadData::F32Vec(relu_out), WorkloadData::F32Vec(gelu_out)) =
        (&relu_comp_result.data, &gelu_comp_result.data)
    {
        println!("ReLU:  {relu_out:?}");
        println!("GELU:  {gelu_out:?}");
        println!();
        println!("Key Observations:");
        println!("  • ReLU: Hard zero for negatives → can cause 'dying ReLU'");
        println!("  • GELU: Smooth, small non-zero for negatives → better gradients");
        println!("  • GELU: Preferred in Transformers (BERT, GPT, etc.)");
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("GELU:");
    println!("  • Parallelism: 100% embarrassingly parallel (like ReLU)");
    println!("  • Pattern: Map with smooth function (sigmoid + multiply)");
    println!("  • Compute: More expensive than ReLU (exp, division)");
    println!("  • CPU: Good (Rayon parallel, but more ops per element)");
    println!("  • GPU: Excellent (parallel, and GPUs good at transcendentals)");
    println!("  • Benefit: Smoother gradients → better learning");
    println!("  • Used in: BERT, GPT-2/3, most modern Transformers");
    println!();

    println!("Dropout:");
    println!("  • Parallelism: 100% embarrassingly parallel");
    println!("  • Pattern: Map with conditional masking");
    println!("  • Memory: Streaming (read, mask, scale, write)");
    println!("  • Training vs Inference: Different behavior!");
    println!("    - Training: Random masking + scaling");
    println!("    - Inference: Pass-through (dropout_rate = 0)");
    println!("  • Determinism: Need RNG seed for reproducibility");
    println!("  • Inverted Dropout: Scale during training (not inference)");
    println!();

    println!("Key Insights:");
    println!("  1. GELU is computationally heavier than ReLU");
    println!("     • Worth it for better gradient flow");
    println!("     • Especially important in deep networks");
    println!("  2. Dropout has dual behavior (training vs inference)");
    println!("     • barraCuda must handle mode switching");
    println!("     • Parameter: dropout_rate (0 = inference)");
    println!("  3. GELU + Dropout is standard in Transformers");
    println!("     • Feed-forward: Linear → GELU → Dropout → Linear");
    println!("  4. Both are simple Maps (embarrassingly parallel)");
    println!("     • Easy to fuse with previous operations");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • GELU: Fuse sigmoid + multiply into single operation");
    println!("  • Dropout: Eliminate in inference mode (compile-time)");
    println!("  • Pipeline: Fuse GELU → Dropout → next layer");
    println!("  • Mode detection: Auto-switch training/inference");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("These operations are fundamental for:");
    println!("  • Transformers (GELU is standard activation)");
    println!("  • Regularization (Dropout prevents overfitting)");
    println!("  • Modern deep learning (BERT, GPT, Vision Transformers)");
    println!("  • Any network needing smooth activation or regularization");
    println!();
    println!("Universal Runtime makes these operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
