//! Tanh and Sigmoid Operations Demo
//!
//! Demonstrates:
//! - Tanh: Hyperbolic tangent (symmetric, range [-1, 1])
//! - Sigmoid: Logistic function (range [0, 1])
//!
//! These are classic activation functions used in traditional neural networks and LSTMs.

use anyhow::Result;
use std::collections::HashMap;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Tanh & Sigmoid Demo                 ║");
    println!("║  barraCuda Phase 1 - Classic Activations                ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Tanh Activation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Tanh (Hyperbolic Tangent)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Tanh: (exp(x) - exp(-x)) / (exp(x) + exp(-x))");
    println!("Range: (-1, 1)");
    println!("Properties: Symmetric around origin, zero-centered");
    println!();

    let input = vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
    println!("Input: {:?}", input);
    println!();

    let tanh_workload = Workload {
        operation: OperationType::Tanh,
        data_type: DataType::F32,
        num_operations: input.len(),
        required_memory: input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let tanh_result = runtime.execute_optimal(tanh_workload).await?;

    if let WorkloadData::F32Vec(output) = &tanh_result.data {
        println!("Output: {:?}", output);
        println!();
        println!("Observations:");
        println!("  • Symmetric: tanh(-x) = -tanh(x)");
        println!("  • Saturates at ±1 for large |x|");
        println!("  • Zero-centered (unlike sigmoid)");
        println!("  • Smooth, differentiable everywhere");
    }

    println!();
    println!("Executed on: {}", tanh_result.metadata.unit_name);
    println!("Duration:    {:?}", tanh_result.metadata.duration);
    println!();

    // Demo 2: Sigmoid Activation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Sigmoid (Logistic Function)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Sigmoid: 1 / (1 + exp(-x))");
    println!("Range: (0, 1)");
    println!("Properties: S-shaped, used for probabilities");
    println!();

    println!("Input: {:?}", input);
    println!();

    let sigmoid_workload = Workload {
        operation: OperationType::Sigmoid,
        data_type: DataType::F32,
        num_operations: input.len(),
        required_memory: input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let sigmoid_result = runtime.execute_optimal(sigmoid_workload).await?;

    if let WorkloadData::F32Vec(output) = &sigmoid_result.data {
        println!("Output: {:?}", output);
        println!();
        println!("Observations:");
        println!("  • Range: (0, 1) - perfect for probabilities");
        println!("  • Sigmoid(0) = 0.5");
        println!("  • Saturates at 0 and 1 for large |x|");
        println!("  • NOT zero-centered (can cause learning issues)");
    }

    println!();
    println!("Executed on: {}", sigmoid_result.metadata.unit_name);
    println!("Duration:    {:?}", sigmoid_result.metadata.duration);
    println!();

    // Demo 3: Activation Function Comparison
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Full Activation Function Comparison");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let comparison_input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
    println!("Input: {:?}", comparison_input);
    println!();

    // Get all activations
    let mut results = HashMap::new();

    // ReLU
    let relu_wl = Workload {
        operation: OperationType::ReLU,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let relu_out = runtime.execute_optimal(relu_wl).await?;
    if let WorkloadData::F32Vec(out) = relu_out.data {
        results.insert("ReLU", out);
    }

    // GELU
    let gelu_wl = Workload {
        operation: OperationType::GELU,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let gelu_out = runtime.execute_optimal(gelu_wl).await?;
    if let WorkloadData::F32Vec(out) = gelu_out.data {
        results.insert("GELU", out);
    }

    // Tanh
    let tanh_wl = Workload {
        operation: OperationType::Tanh,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let tanh_out = runtime.execute_optimal(tanh_wl).await?;
    if let WorkloadData::F32Vec(out) = tanh_out.data {
        results.insert("Tanh", out);
    }

    // Sigmoid
    let sigmoid_wl = Workload {
        operation: OperationType::Sigmoid,
        data_type: DataType::F32,
        num_operations: comparison_input.len(),
        required_memory: comparison_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(comparison_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let sigmoid_out = runtime.execute_optimal(sigmoid_wl).await?;
    if let WorkloadData::F32Vec(out) = sigmoid_out.data {
        results.insert("Sigmoid", out);
    }

    // Display comparison table
    println!("Activation  | -2.0      | -1.0      | 0.0       | 1.0       | 2.0");
    println!("------------|-----------|-----------|-----------|-----------|----------");
    for (name, values) in &results {
        print!("{:<11} |", name);
        for val in values {
            print!(" {:>9.5} |", val);
        }
        println!();
    }
    println!();

    // Demo 4: Use Cases and Properties
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Use Cases and Properties");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ Activation Function Properties                              │");
    println!("├─────────────┬────────────┬────────────┬─────────────────────┤");
    println!("│ Function    │ Range      │ Centered   │ Primary Use Case    │");
    println!("├─────────────┼────────────┼────────────┼─────────────────────┤");
    println!("│ ReLU        │ [0, ∞)     │ No         │ CNNs, MLPs (modern) │");
    println!("│ GELU        │ (-∞, ∞)    │ Yes        │ Transformers        │");
    println!("│ Tanh        │ (-1, 1)    │ Yes        │ RNNs, older nets    │");
    println!("│ Sigmoid     │ (0, 1)     │ No         │ Binary class, gates │");
    println!("└─────────────┴────────────┴────────────┴─────────────────────┘");
    println!();

    println!("Key Differences:");
    println!();
    println!("1. Zero-Centered:");
    println!("   • Tanh, GELU: YES → Better gradient flow");
    println!("   • ReLU, Sigmoid: NO → Can cause zig-zag learning");
    println!();
    println!("2. Saturation:");
    println!("   • Tanh, Sigmoid: Saturate on both ends → Vanishing gradients");
    println!("   • ReLU: Saturates only at 0 → Dying ReLU problem");
    println!("   • GELU: Smooth, no hard saturation → Best gradient flow");
    println!();
    println!("3. Computational Cost:");
    println!("   • ReLU: 1x (just max operation)");
    println!("   • Tanh: 3-4x (two exp operations)");
    println!("   • Sigmoid: 2-3x (one exp + division)");
    println!("   • GELU: 5x (sigmoid + multiply)");
    println!();

    // Demo 5: LSTM Gate Demonstration
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 5: LSTM Gate Pattern");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("LSTM uses both Sigmoid and Tanh:");
    println!("  • Sigmoid: For gates (forget, input, output) [0-1 range]");
    println!("  • Tanh: For cell state and hidden state [-1 to 1 range]");
    println!();

    let gate_input = vec![0.5, 1.0, 1.5, 2.0];
    println!("Input (pre-gate values): {:?}", gate_input);
    println!();

    // Simulate LSTM gates
    let sigmoid_gate = Workload {
        operation: OperationType::Sigmoid,
        data_type: DataType::F32,
        num_operations: gate_input.len(),
        required_memory: gate_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(gate_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let gate_result = runtime.execute_optimal(sigmoid_gate).await?;

    let tanh_state = Workload {
        operation: OperationType::Tanh,
        data_type: DataType::F32,
        num_operations: gate_input.len(),
        required_memory: gate_input.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(gate_input.clone()),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };
    let state_result = runtime.execute_optimal(tanh_state).await?;

    if let (WorkloadData::F32Vec(gate_vals), WorkloadData::F32Vec(state_vals)) =
        (&gate_result.data, &state_result.data)
    {
        println!("Sigmoid (gate):       {:?}", gate_vals);
        println!("Tanh (cell state):    {:?}", state_vals);
        println!();
        println!("Why this combination?");
        println!("  • Sigmoid [0-1]: Controls how much to let through (gate)");
        println!("  • Tanh [-1, 1]: Represents the actual value (state)");
        println!("  • Combined: gate * state → gated output");
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Tanh:");
    println!("  • Parallelism: 100% embarrassingly parallel (Map)");
    println!("  • Pattern: Simple transcendental (two exp operations)");
    println!("  • Compute: ~3-4x more expensive than ReLU");
    println!("  • CPU: Good (Rayon parallel, more ops per element)");
    println!("  • GPU: Excellent (GPUs good at transcendentals)");
    println!("  • Benefit: Zero-centered, symmetric");
    println!("  • Used in: RNNs, LSTMs, older feedforward networks");
    println!();

    println!("Sigmoid:");
    println!("  • Parallelism: 100% embarrassingly parallel (Map)");
    println!("  • Pattern: Transcendental (exp + division)");
    println!("  • Compute: ~2-3x more expensive than ReLU");
    println!("  • CPU: Good (Rayon parallel)");
    println!("  • GPU: Excellent (naturally parallel)");
    println!("  • Benefit: Output is probability [0-1]");
    println!("  • Used in: Binary classification, LSTM gates, attention");
    println!();

    println!("Key Insights:");
    println!("  1. Historical evolution: Sigmoid → Tanh → ReLU → GELU");
    println!("     • Each fixes issues with the previous");
    println!("     • Sigmoid: Not zero-centered");
    println!("     • Tanh: Zero-centered, but still saturates");
    println!("     • ReLU: No saturation (positive), but can die");
    println!("     • GELU: Smooth, best gradient flow");
    println!();
    println!("  2. Tanh is scaled/shifted sigmoid:");
    println!("     • tanh(x) = 2 * sigmoid(2x) - 1");
    println!("     • Relationship allows optimization opportunities");
    println!();
    println!("  3. Both used together in LSTMs:");
    println!("     • Sigmoid: Gates (how much?)");
    println!("     • Tanh: States (what value?)");
    println!("     • This is a fundamental pattern in recurrent networks");
    println!();
    println!("  4. Modern alternatives (GELU) are better for deep networks:");
    println!("     • But Tanh/Sigmoid still essential for RNNs/LSTMs");
    println!("     • And for specific use cases (probabilities, gates)");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • Fusion: Can fuse Tanh/Sigmoid with previous operation");
    println!("  • LSTM detection: Recognize Sigmoid+Tanh pattern → LSTM gate");
    println!("  • Relationship: Use tanh(x) = 2*sigmoid(2x) - 1 for optimization");
    println!("  • Approximation: Fast exp approximations for both");
    println!("  • SIMD: Vectorize exp operations");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("These operations are fundamental for:");
    println!("  • LSTMs & RNNs (Tanh & Sigmoid in gates)");
    println!("  • Binary classification (Sigmoid output)");
    println!("  • Traditional neural networks (Tanh hidden layers)");
    println!("  • Any network requiring bounded activations");
    println!();
    println!("Activation Function Library NOW COMPLETE! 🎯");
    println!("  ✅ ReLU, LeakyReLU, GELU, Tanh, Sigmoid, Softmax");
    println!();
    println!("Universal Runtime makes these operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
