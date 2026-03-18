// SPDX-License-Identifier: AGPL-3.0-or-later
//! Matrix Multiplication (`MatMul`) Operation Demo

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
//!
//! Demonstrates:
//! - `MatMul`: C = A * B (fundamental deep learning operation)
//! - Tiled/blocked approach for cache efficiency
//! - Parallel execution with Rayon
//! - Various matrix sizes
//!
//! `MatMul` is THE most important operation in deep learning - used everywhere!

use std::collections::HashMap;
use toadstool_runtime_universal::ComputeError;
use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::{
    DataType, OperationType, Workload, WorkloadData, WorkloadParams,
};

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Matrix Multiplication Demo          ║");
    println!("║  barraCuda Phase 1 - THE Fundamental Operation          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Small Matrix Multiplication (Easy to Verify)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Small MatMul (2x3) * (3x2) = (2x2)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("MatMul: C = A * B");
    println!("Used in: ALL fully-connected layers, attention, embeddings");
    println!();

    // A: 2x3 matrix
    #[rustfmt::skip]
    let a = vec![
        1.0, 2.0, 3.0,  // row 0
        4.0, 5.0, 6.0,  // row 1
    ];
    let a_rows = 2;
    let a_cols = 3;

    // B: 3x2 matrix
    #[rustfmt::skip]
    let b = vec![
        7.0,  8.0,  // row 0
        9.0, 10.0,  // row 1
       11.0, 12.0,  // row 2
    ];
    let b_rows = 3;
    let b_cols = 2;

    println!("Matrix A ({a_rows}x{a_cols}):");
    println!("  [{}, {}, {}]", a[0], a[1], a[2]);
    println!("  [{}, {}, {}]", a[3], a[4], a[5]);
    println!();

    println!("Matrix B ({b_rows}x{b_cols}):");
    println!("  [{}, {}]", b[0], b[1]);
    println!("  [{}, {}]", b[2], b[3]);
    println!("  [{}, {}]", b[4], b[5]);
    println!();

    let matmul_workload = Workload {
        operation: OperationType::MatMul,
        data_type: DataType::F32,
        num_operations: a_rows * a_cols * b_cols, // M * K * N operations
        required_memory: (a.len() + b.len() + a_rows * b_cols) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32MatrixPair(a.clone(), a_rows, a_cols, b.clone(), b_rows, b_cols),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let result = runtime.execute_optimal(matmul_workload).await?;

    if let WorkloadData::F32Matrix(c, c_rows, c_cols) = &result.data {
        println!("Result C ({c_rows}x{c_cols}):");
        for i in 0..*c_rows {
            print!("  [");
            for j in 0..*c_cols {
                print!("{:>6.1}", c[i * c_cols + j]);
                if j < c_cols - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }
        println!();

        // Manual verification for first element: C[0,0] = 1*7 + 2*9 + 3*11 = 7 + 18 + 33 = 58
        let expected_00 = 1.0 * 7.0 + 2.0 * 9.0 + 3.0 * 11.0;
        println!("Verification:");
        println!("  C[0,0] = A[0,:]·B[:,0] = 1*7 + 2*9 + 3*11 = {expected_00:.1}");
        println!("  Actual: {:.1}", c[0]);
        println!(
            "  Match: {} ✅",
            if (c[0] - expected_00).abs() < 1e-6 {
                "PASS"
            } else {
                "FAIL"
            }
        );
    }

    println!();
    println!("Executed on: {}", result.metadata.unit_name);
    println!("Duration:    {:?}", result.metadata.duration);
    println!();

    // Demo 2: Square Matrix (Common in Neural Networks)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Square MatMul (4x4) * (4x4) = (4x4)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Square matrices are common in:");
    println!("  • Attention mechanisms (Q·K^T)");
    println!("  • State transitions in RNNs");
    println!("  • Weight matrices in MLPs");
    println!();

    // Identity matrix * Random matrix = Random matrix
    let size = 4;
    #[rustfmt::skip]
    let identity = vec![
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    #[rustfmt::skip]
    let matrix = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
        9.0, 10.0, 11.0, 12.0,
        13.0, 14.0, 15.0, 16.0,
    ];

    println!("Identity(4x4) * Matrix(4x4):");

    let square_workload = Workload {
        operation: OperationType::MatMul,
        data_type: DataType::F32,
        num_operations: size * size * size,
        required_memory: (identity.len() + matrix.len() + size * size) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32MatrixPair(
            identity.clone(),
            size,
            size,
            matrix.clone(),
            size,
            size,
        ),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let square_result = runtime.execute_optimal(square_workload).await?;

    if let WorkloadData::F32Matrix(c, _, _) = &square_result.data {
        println!("  Result should equal original matrix:");
        for i in 0..size {
            print!("  [");
            for j in 0..size {
                print!("{:>5.1}", c[i * size + j]);
                if j < size - 1 {
                    print!(", ");
                }
            }
            println!("]");
        }

        // Verify identity property
        let all_match = matrix
            .iter()
            .zip(c.iter())
            .all(|(a, b)| (a - b).abs() < 1e-6);
        println!();
        println!(
            "  Identity property verified: {} ✅",
            if all_match { "PASS" } else { "FAIL" }
        );
    }

    println!();
    println!("Executed on: {}", square_result.metadata.unit_name);
    println!("Duration:    {:?}", square_result.metadata.duration);
    println!();

    // Demo 3: Larger Matrix (Shows Performance Scaling)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Large MatMul (128x256) * (256x128)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Typical neural network layer sizes:");
    println!("  • Input: 128 samples × 256 features");
    println!("  • Weights: 256 features × 128 hidden units");
    println!("  • Output: 128 samples × 128 hidden units");
    println!();

    let m = 128;
    let k = 256;
    let n = 128;

    // Create random-like matrices (deterministic for reproducibility)
    let large_a: Vec<f32> = (0..m * k).map(|i| ((i % 100) as f32) / 100.0).collect();
    let large_b: Vec<f32> = (0..k * n).map(|i| ((i % 50) as f32) / 50.0).collect();

    println!("Computing ({m}x{k}) * ({k}x{n}) = ({m}x{n})...");
    println!("  Total FLOPs: {} (2*M*K*N)", 2 * m * k * n);
    println!();

    let large_workload = Workload {
        operation: OperationType::MatMul,
        data_type: DataType::F32,
        num_operations: m * k * n,
        required_memory: (large_a.len() + large_b.len() + m * n) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32MatrixPair(large_a.clone(), m, k, large_b.clone(), k, n),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let large_result = runtime.execute_optimal(large_workload).await?;

    if let WorkloadData::F32Matrix(c, c_rows, c_cols) = &large_result.data {
        println!("✅ Computation complete!");
        println!("  Output shape: ({c_rows} x {c_cols})");
        println!(
            "  First 4 values: [{:.4}, {:.4}, {:.4}, {:.4}]",
            c[0], c[1], c[2], c[3]
        );
        println!(
            "  Last 4 values:  [{:.4}, {:.4}, {:.4}, {:.4}]",
            c[c.len() - 4],
            c[c.len() - 3],
            c[c.len() - 2],
            c[c.len() - 1]
        );
    }

    println!();
    println!("Executed on: {}", large_result.metadata.unit_name);
    println!("Duration:    {:?}", large_result.metadata.duration);
    println!(
        "  Throughput: ~{:.2} GFLOPS",
        (2 * m * k * n) as f64 / large_result.metadata.duration.as_secs_f64() / 1e9
    );
    println!();

    // Demo 4: Non-Square (Attention Q·K^T pattern)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Attention Pattern (seq_len x d_k) * (d_k x seq_len)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Transformer attention: Q·K^T");
    println!("  Q: (seq_len x d_k)  - Query matrix");
    println!("  K^T: (d_k x seq_len) - Key matrix transposed");
    println!("  Result: (seq_len x seq_len) - Attention scores");
    println!();

    let seq_len = 16; // Sequence length
    let d_k = 64; // Key/Query dimension

    let q: Vec<f32> = (0..seq_len * d_k).map(|i| (i as f32) / 100.0).collect();
    let k_t: Vec<f32> = (0..d_k * seq_len).map(|i| (i as f32) / 100.0).collect();

    println!("Computing attention scores ({seq_len}x{d_k}) * ({d_k}x{seq_len})...");

    let attention_workload = Workload {
        operation: OperationType::MatMul,
        data_type: DataType::F32,
        num_operations: seq_len * d_k * seq_len,
        required_memory: (q.len() + k_t.len() + seq_len * seq_len) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32MatrixPair(q, seq_len, d_k, k_t, d_k, seq_len),
        params: WorkloadParams {
            params: HashMap::new(),
        },
    };

    let attention_result = runtime.execute_optimal(attention_workload).await?;

    if let WorkloadData::F32Matrix(scores, rows, cols) = &attention_result.data {
        println!("✅ Attention scores computed!");
        println!("  Shape: ({rows} x {cols}) - attention matrix");
        println!("  First row (attention from token 0):");
        print!("    [");
        for (i, &score) in scores.iter().enumerate().take(8.min(*cols)) {
            print!("{score:.3}");
            if i < 7.min(*cols - 1) {
                print!(", ");
            }
        }
        if *cols > 8 {
            print!(", ...");
        }
        println!("]");
        println!();
        println!("  Next step: Apply Softmax to each row for attention weights!");
    }

    println!();
    println!("Executed on: {}", attention_result.metadata.unit_name);
    println!("Duration:    {:?}", attention_result.metadata.duration);
    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("MatMul:");
    println!("  • Parallelism: Tiled + Row-parallel (excellent scalability)");
    println!("  • Pattern: Triple nested loop (i, j, k) with tiling");
    println!("  • Compute: O(M*K*N) - cubic complexity");
    println!("  • Memory: Sequential + blocked (cache-friendly)");
    println!("  • CPU: Excellent with tiling (64x64 tiles for L1 cache)");
    println!("  • GPU: Excellent (naturally parallel, shared memory)");
    println!();

    println!("Tiling Benefits:");
    println!("  • Tile size: 64x64 (optimized for L1 cache ~32KB)");
    println!("  • Cache hits: High (reuse A and B within tile)");
    println!("  • Memory bandwidth: Reduced (fewer DRAM accesses)");
    println!("  • Speedup: 2-10x over naive implementation");
    println!();

    println!("Use Cases (MatMul is EVERYWHERE):");
    println!("  1. Fully-connected layers: X·W + b");
    println!("  2. Attention: Q·K^T, then scores·V");
    println!("  3. Embeddings: indices → E[indices,:]");
    println!("  4. RNN state updates: h_t = tanh(W_h·h_{{t-1}} + W_x·x_t)");
    println!("  5. CNN reshaping: flatten → FC layer");
    println!();

    println!("Transformer Attention (Complete Pattern):");
    println!("  1. Q·K^T → scores (MatMul) ✅");
    println!("  2. scores / sqrt(d_k) → scaled (Map)");
    println!("  3. Softmax(scaled) → attention_weights (Softmax) ✅");
    println!("  4. attention_weights·V → output (MatMul) ✅");
    println!();
    println!("  All operations now implemented in barraCuda! 🎯");
    println!();

    println!("Key Insights:");
    println!("  1. MatMul is THE bottleneck in deep learning");
    println!("     • 90%+ of compute time in Transformers");
    println!("     • Optimization is critical");
    println!();
    println!("  2. Tiling is essential for performance");
    println!("     • Without: Memory-bound (slow)");
    println!("     • With: Compute-bound (fast)");
    println!();
    println!("  3. Different matrix shapes have different characteristics:");
    println!("     • Square (NxN): Balanced");
    println!("     • Tall (M>>N): Row-parallel dominates");
    println!("     • Wide (N>>M): Column-parallel better");
    println!();
    println!("  4. Composition with other operations:");
    println!("     • MatMul + ReLU (fully-connected layer)");
    println!("     • MatMul + Softmax (attention)");
    println!("     • MatMul + LayerNorm (feed-forward)");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • Fusion: MatMul + activation in single kernel");
    println!("  • Auto-tuning: Select tile size based on matrix shape");
    println!("  • Strassen: For very large square matrices (>1024)");
    println!("  • GPU offload: Automatic for large matrices (>256x256)");
    println!("  • Mixed precision: FP16 for compute, FP32 for accumulation");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("MatMul is now implemented! This is THE fundamental operation.");
    println!();
    println!("With MatMul complete, barraCuda can now:");
    println!("  ✅ Analyze complete Transformer architectures");
    println!("  ✅ Optimize attention mechanisms");
    println!("  ✅ Handle all fully-connected layers");
    println!("  ✅ Process RNN/LSTM state updates");
    println!();
    println!("Universal Runtime makes MatMul hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
