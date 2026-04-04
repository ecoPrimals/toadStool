// SPDX-License-Identifier: AGPL-3.0-only
//! Dot Product and Elementwise Binary Operations Demo

#![expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)]
//!
//! Demonstrates fundamental vector operations:
//! - Dot Product: Inner product of two vectors
//! - Elementwise Binary: Element-by-element operations (add, multiply, etc.)
//!
//! These operations are building blocks for linear algebra and ML.

use toadstool_runtime_universal::ComputeError;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::{
    DataType, OperationType, Workload, WorkloadData, WorkloadParams,
};

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Dot Product & Elementwise Ops Demo  ║");
    println!("║  barraCuda Phase 1 - Building Block Pattern Learning    ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover all available compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;

    println!("✅ Found {} compute unit(s)", runtime.num_units());
    for unit in runtime.units() {
        println!(
            "   • {} - {} units, {:.2} GB memory",
            unit.name(),
            unit.capabilities().parallelism.num_units,
            unit.capabilities().memory_capacity as f64 / 1e9
        );
    }
    println!();

    // Demo 1: Dot Product
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Dot Product (Vector Inner Product)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let vec_a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let vec_b = vec![2.0, 3.0, 4.0, 5.0, 6.0];

    println!("Vector A: {vec_a:?}");
    println!("Vector B: {vec_b:?}");
    println!();

    let dot_workload = Workload {
        operation: OperationType::DotProduct,
        data_type: DataType::F32,
        num_operations: vec_a.len(), // n multiply-adds
        required_memory: (vec_a.len() + vec_b.len()) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecPair(vec_a.clone(), vec_b.clone()),
        params: WorkloadParams::default(),
    };

    let dot_result = runtime.execute_optimal(dot_workload).await?;

    if let WorkloadData::F32Vec(output) = dot_result.data {
        let result = output[0];
        // Manual verification: 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 2 + 6 + 12 + 20 + 30 = 70
        let expected = 70.0;
        println!("Result: {result:.1}");
        println!("Expected: {expected:.1}");
        println!(
            "Verification: {} ✅",
            if (result - expected).abs() < 0.001 {
                "PASS"
            } else {
                "FAIL"
            }
        );
    }

    println!();
    println!("Executed on: {}", dot_result.metadata.unit_name);
    println!("Duration:    {:?}", dot_result.metadata.duration);
    println!();

    // Demo 2: Elementwise Binary Operation (Addition)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Elementwise Binary Operation (Addition)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let vec_c = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let vec_d = vec![1.0, 2.0, 3.0, 4.0, 5.0];

    println!("Vector C: {vec_c:?}");
    println!("Vector D: {vec_d:?}");
    println!();

    let elementwise_workload = Workload {
        operation: OperationType::ElementwiseBinary,
        data_type: DataType::F32,
        num_operations: vec_c.len(), // n additions
        required_memory: (vec_c.len() + vec_d.len() + vec_c.len()) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecPair(vec_c.clone(), vec_d.clone()),
        params: WorkloadParams::default(),
    };

    let elementwise_result = runtime.execute_optimal(elementwise_workload).await?;

    if let WorkloadData::F32Vec(output) = elementwise_result.data {
        println!("Result: {output:?}");
        let expected: Vec<f32> = vec_c.iter().zip(&vec_d).map(|(a, b)| a + b).collect();
        println!("Expected: {expected:?}");
        let all_match = output
            .iter()
            .zip(&expected)
            .all(|(a, b)| (a - b).abs() < 0.001);
        println!(
            "Verification: {} ✅",
            if all_match { "PASS" } else { "FAIL" }
        );
    }

    println!();
    println!("Executed on: {}", elementwise_result.metadata.unit_name);
    println!("Duration:    {:?}", elementwise_result.metadata.duration);
    println!();

    // Demo 3: Use Cases and Patterns
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Real-World Use Cases");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("📊 Dot Product Use Cases:");
    println!("  • Cosine similarity (document comparison)");
    println!("  • Neural network forward pass (layer outputs)");
    println!("  • Physics simulations (work = force · displacement)");
    println!("  • Projection (vector onto another vector)");
    println!();

    println!("📊 Elementwise Binary Use Cases:");
    println!("  • Vector addition (combine features)");
    println!("  • Hadamard product (element-wise multiply)");
    println!("  • Residual connections (ResNet: x + f(x))");
    println!("  • Masking (multiply by 0/1 mask)");
    println!();

    // Demo 4: Larger Scale Performance
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Scaling Behavior");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for size in [100, 1_000, 10_000, 100_000] {
        let large_a: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let large_b: Vec<f32> = (0..size).map(|i| (i + 1) as f32).collect();

        let workload = Workload {
            operation: OperationType::DotProduct,
            data_type: DataType::F32,
            num_operations: size,
            required_memory: (size + size) * std::mem::size_of::<f32>(),
            input: WorkloadData::F32VecPair(large_a, large_b),
            params: WorkloadParams::default(),
        };

        let result = runtime.execute_optimal(workload).await?;
        println!(
            "  Size: {:>7} | Duration: {:>10.3?} | Unit: {}",
            size, result.metadata.duration, result.metadata.unit_name
        );
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Dot Product:");
    println!("  • Parallelism: Map (element-wise multiply) + Reduce (sum)");
    println!("  • Pattern: Embarrassingly parallel map, then tree reduction");
    println!("  • CPU: Excellent (Rayon zip + sum)");
    println!("  • GPU: Excellent (massive parallelism for map, then reduce)");
    println!("  • Bottleneck: Memory bandwidth (simple operations)");
    println!("  • Insight: Composition of two patterns we already know!");
    println!();

    println!("Elementwise Binary:");
    println!("  • Parallelism: 100% embarrassingly parallel");
    println!("  • Pattern: Same as Map, but with two inputs");
    println!("  • CPU: Excellent (Rayon zip)");
    println!("  • GPU: Excellent (naturally parallel)");
    println!("  • Memory pattern: Streaming (read A, read B, write C)");
    println!("  • Insight: Even simpler than Map (no complex function)");
    println!();

    println!("Key Learning:");
    println!("  • Complex operations = composition of simple patterns");
    println!("  • Dot Product = Map + Reduce");
    println!("  • Elementwise = Map with 2 inputs");
    println!("  • barraCuda opportunity: Recognize and optimize compositions");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("These building blocks enable:");
    println!("  • Linear algebra (dot products everywhere!)");
    println!("  • Neural networks (forward/backward pass)");
    println!("  • Signal processing (convolution via dot products)");
    println!("  • Physics simulations (vector math)");
    println!();
    println!("Universal Runtime makes these operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
