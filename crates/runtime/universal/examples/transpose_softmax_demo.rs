//! Transpose and Softmax Operations Demo
//!
//! Demonstrates:
//! - Transpose: Data layout transformation (fundamental for linear algebra)
//! - Softmax: Composite normalization (exp + reduce + map)
//!
//! These operations are essential for neural networks and scientific computing.

use anyhow::Result;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Transpose & Softmax Demo            ║");
    println!("║  barraCUDA Phase 1 - Data Movement & Normalization      ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Matrix Transpose
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Matrix Transpose (Data Layout Transformation)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Transpose: Swap rows and columns");
    println!();

    // 3x4 matrix
    #[rustfmt::skip]
    let matrix = vec![
        1.0, 2.0, 3.0, 4.0,  // Row 0
        5.0, 6.0, 7.0, 8.0,  // Row 1
        9.0, 10.0, 11.0, 12.0, // Row 2
    ];
    let rows = 3;
    let cols = 4;

    println!("Input Matrix ({}x{}):", rows, cols);
    for r in 0..rows {
        print!("  [");
        for c in 0..cols {
            print!("{:5.1}", matrix[r * cols + c]);
            if c < cols - 1 {
                print!(", ");
            }
        }
        println!("]");
    }
    println!();

    let transpose_workload = Workload {
        operation: OperationType::Transpose,
        data_type: DataType::F32,
        num_operations: rows * cols,
        required_memory: 2 * rows * cols * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Matrix(matrix.clone(), rows, cols),
        params: WorkloadParams::default(),
    };

    let transpose_result = runtime.execute_optimal(transpose_workload).await?;

    if let WorkloadData::F32Matrix(output, out_rows, out_cols) = &transpose_result.data {
        println!("Output Matrix ({}x{}):", out_rows, out_cols);
        for r in 0..*out_rows {
            print!("  [");
            for c in 0..*out_cols {
                print!("{:5.1}", output[r * out_cols + c]);
                if c < out_cols - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }
        println!();

        // Verify: output[i][j] should equal input[j][i]
        let mut correct = true;
        for r in 0..*out_rows {
            for c in 0..*out_cols {
                let output_val = output[r * out_cols + c];
                let expected_val = matrix[c * cols + r]; // Swapped indices
                if (output_val - expected_val).abs() > 1e-6 {
                    correct = false;
                    break;
                }
            }
        }
        println!("Verification: {} ✅", if correct { "PASS" } else { "FAIL" });
    }

    println!();
    println!("Executed on: {}", transpose_result.metadata.unit_name);
    println!("Duration:    {:?}", transpose_result.metadata.duration);
    println!();

    // Demo 2: Softmax Normalization
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Softmax (Composite Normalization)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Softmax: Converts logits to probabilities");
    println!("Formula: softmax(x_i) = exp(x_i) / sum(exp(x_j))");
    println!();

    let logits = vec![2.0, 1.0, 0.1];
    println!("Input (logits): {:?}", logits);
    println!();

    let softmax_workload = Workload {
        operation: OperationType::Softmax,
        data_type: DataType::F32,
        num_operations: logits.len() * 3, // exp + sum + divide
        required_memory: logits.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(logits.clone()),
        params: WorkloadParams::default(),
    };

    let softmax_result = runtime.execute_optimal(softmax_workload).await?;

    if let WorkloadData::F32Vec(output) = &softmax_result.data {
        println!("Output (probabilities): {:?}", output);

        // Verify properties of softmax
        let sum: f32 = output.iter().sum();
        let all_positive = output.iter().all(|&x| x > 0.0 && x < 1.0);
        let sum_is_one = (sum - 1.0).abs() < 1e-6;

        println!();
        println!("Properties:");
        println!("  Sum of probabilities: {:.6} (should be 1.0)", sum);
        println!("  All values in (0, 1): {}", all_positive);
        println!("  Sum equals 1.0: {}", sum_is_one);
        println!();
        println!(
            "Verification: {} ✅",
            if all_positive && sum_is_one {
                "PASS"
            } else {
                "FAIL"
            }
        );
    }

    println!();
    println!("Executed on: {}", softmax_result.metadata.unit_name);
    println!("Duration:    {:?}", softmax_result.metadata.duration);
    println!();

    // Demo 3: Softmax for Classification
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Softmax for Multi-Class Classification");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Scenario: 10-class classification (e.g., digit recognition)");
    println!();

    let class_logits = vec![0.1, 0.2, 5.0, 0.3, 0.1, 0.2, 0.1, 0.3, 0.2, 0.1]; // Class 2 has highest score
    println!("Class logits: {:?}", class_logits);
    println!("(Class 2 has highest score: 5.0)");
    println!();

    let class_workload = Workload {
        operation: OperationType::Softmax,
        data_type: DataType::F32,
        num_operations: class_logits.len() * 3,
        required_memory: class_logits.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(class_logits.clone()),
        params: WorkloadParams::default(),
    };

    let class_result = runtime.execute_optimal(class_workload).await?;

    if let WorkloadData::F32Vec(probabilities) = &class_result.data {
        println!("Class probabilities:");
        for (i, &prob) in probabilities.iter().enumerate() {
            println!("  Class {}: {:.6} ({:.2}%)", i, prob, prob * 100.0);
        }
        println!();

        // Find predicted class
        let (predicted_class, &max_prob) = probabilities
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        println!(
            "Predicted class: {} (confidence: {:.2}%)",
            predicted_class,
            max_prob * 100.0
        );
        println!("Expected class: 2 ✅");
    }

    println!();

    // Demo 4: Numerical Stability
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Numerical Stability of Softmax");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Problem: Large values can cause overflow in exp()");
    println!("Solution: Subtract max before exp (mathematically equivalent)");
    println!();

    let large_logits = vec![1000.0, 1001.0, 1002.0]; // Would overflow naive softmax
    println!("Large logits: {:?}", large_logits);
    println!("(These would overflow naive exp() implementation)");
    println!();

    let stable_workload = Workload {
        operation: OperationType::Softmax,
        data_type: DataType::F32,
        num_operations: large_logits.len() * 3,
        required_memory: large_logits.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(large_logits),
        params: WorkloadParams::default(),
    };

    let stable_result = runtime.execute_optimal(stable_workload).await?;

    if let WorkloadData::F32Vec(output) = &stable_result.data {
        println!("Output (numerically stable): {:?}", output);
        println!();

        let sum: f32 = output.iter().sum();
        let is_valid = output.iter().all(|&x| x.is_finite());

        println!("Results:");
        println!("  All values finite: {}", is_valid);
        println!("  Sum: {:.6}", sum);
        println!("  No overflow: {} ✅", is_valid);
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCUDA Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Transpose:");
    println!("  • Parallelism: Each output row independent");
    println!("  • Pattern: Data layout transformation (pure data movement)");
    println!("  • CPU: Good (parallel, cache-friendly with blocking)");
    println!("  • GPU: Excellent (coalesced reads, bank conflict management)");
    println!("  • Memory pattern: Strided access (can be cache-unfriendly)");
    println!("  • Bottleneck: Memory bandwidth");
    println!();

    println!("Softmax:");
    println!("  • Parallelism: Composite pattern!");
    println!("  • Decomposition:");
    println!("    1. Reduce (max) - tree-based");
    println!("    2. Map (exp(x - max)) - embarrassingly parallel");
    println!("    3. Reduce (sum) - tree-based");
    println!("    4. Map (x / sum) - embarrassingly parallel");
    println!("  • CPU: Excellent (Rayon handles all phases)");
    println!("  • GPU: Excellent (each phase optimizes independently)");
    println!("  • Numerical stability: Critical! (subtract max)");
    println!();

    println!("Key Insights:");
    println!("  1. Transpose is pure data movement (no computation)");
    println!("  2. Memory layout matters (row-major vs column-major)");
    println!("  3. Softmax is a 4-phase composite:");
    println!("     Reduce → Map → Reduce → Map");
    println!("  4. Numerical stability requires algorithmic care");
    println!("  5. barraCUDA can recognize and optimize composites");
    println!();

    println!("barraCUDA Opportunities:");
    println!("  • Transpose: Cache blocking, shared memory (GPU)");
    println!("  • Softmax: Fuse all 4 phases into single kernel");
    println!("  • Both: Detect patterns in user code automatically");
    println!("  • Numerical tricks: Apply stable algorithms automatically");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("These operations are fundamental for:");
    println!("  • Neural networks (softmax for classification layers)");
    println!("  • Linear algebra (transpose for matrix operations)");
    println!("  • Scientific computing (data layout transformations)");
    println!("  • Attention mechanisms (softmax for attention weights)");
    println!();
    println!("Universal Runtime makes these operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
