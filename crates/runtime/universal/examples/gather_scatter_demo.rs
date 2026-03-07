// SPDX-License-Identifier: AGPL-3.0-or-later
//! Gather and Scatter Operations Demo
//!
//! Demonstrates indexing operations fundamental to:
//! - Data manipulation (reordering, sampling)
//! - Neural networks (embedding lookups, attention)
//! - Sparse operations (sparse matrix operations)
//! - Graph algorithms (neighbor access)

use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::{
    DataType, OperationType, Workload, WorkloadData, WorkloadParams,
};
use toadstool_runtime_universal::ComputeError;

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Gather & Scatter Operations Demo    ║");
    println!("║  barraCuda Phase 1 - Indexing Pattern Learning          ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!("✅ Found {} compute unit(s)", runtime.num_units());
    println!();

    // Demo 1: Gather Operation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Gather (Select by Indices)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Gather: Select elements from data using indices");
    println!();

    let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let indices = vec![0, 2, 4, 6, 8]; // Select even indices

    println!("Data:    {data:?}");
    println!("Indices: {indices:?}");
    println!();

    let gather_workload = Workload {
        operation: OperationType::Gather,
        data_type: DataType::F32,
        num_operations: indices.len(),
        required_memory: (data.len() + indices.len()) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecIndexed(data.clone(), indices.clone()),
        params: WorkloadParams::default(),
    };

    let gather_result = runtime.execute_optimal(gather_workload).await?;

    if let WorkloadData::F32Vec(output) = &gather_result.data {
        println!("Result: {output:?}");
        let expected = vec![10.0, 30.0, 50.0, 70.0, 90.0];
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
    println!("Executed on: {}", gather_result.metadata.unit_name);
    println!("Duration:    {:?}", gather_result.metadata.duration);
    println!();

    // Demo 2: Scatter Operation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Scatter (Place by Indices)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Scatter: Place values into output at specified indices");
    println!("(Using scatter-add: accumulate when indices overlap)");
    println!();

    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let scatter_indices = vec![1, 3, 5, 7, 9];

    println!("Values:  {values:?}");
    println!("Indices: {scatter_indices:?}");
    println!();

    let scatter_workload = Workload {
        operation: OperationType::Scatter,
        data_type: DataType::F32,
        num_operations: values.len(),
        required_memory: (values.len() + scatter_indices.len() + 10) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecIndexed(values.clone(), scatter_indices.clone()),
        params: WorkloadParams::default(),
    };

    let scatter_result = runtime.execute_optimal(scatter_workload).await?;

    if let WorkloadData::F32Vec(output) = &scatter_result.data {
        println!("Result: {output:?}");
        println!("Expected: [0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0]");
        let expected = vec![0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0];
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
    println!("Executed on: {}", scatter_result.metadata.unit_name);
    println!("Duration:    {:?}", scatter_result.metadata.duration);
    println!();

    // Demo 3: Scatter-Add (Overlapping Indices)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Scatter-Add (Overlapping Indices)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("When multiple values scatter to same index, they accumulate:");
    println!();

    let overlap_values = vec![10.0, 20.0, 30.0, 40.0];
    let overlap_indices = vec![1, 1, 2, 2]; // Two pairs to same indices

    println!("Values:  {overlap_values:?}");
    println!("Indices: {overlap_indices:?}");
    println!();

    let overlap_workload = Workload {
        operation: OperationType::Scatter,
        data_type: DataType::F32,
        num_operations: overlap_values.len(),
        required_memory: (overlap_values.len() + overlap_indices.len() + 3)
            * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecIndexed(overlap_values.clone(), overlap_indices.clone()),
        params: WorkloadParams::default(),
    };

    let overlap_result = runtime.execute_optimal(overlap_workload).await?;

    if let WorkloadData::F32Vec(output) = &overlap_result.data {
        println!("Result: {output:?}");
        println!("Expected: [0.0, 30.0, 70.0] (10+20=30 at [1], 30+40=70 at [2])");
        let expected = vec![0.0, 30.0, 70.0];
        let all_match = output.len() == expected.len()
            && output
                .iter()
                .zip(&expected)
                .all(|(a, b)| (a - b).abs() < 0.001);
        println!(
            "Verification: {} ✅",
            if all_match { "PASS" } else { "FAIL" }
        );
    }

    println!();
    println!("Executed on: {}", overlap_result.metadata.unit_name);
    println!("Duration:    {:?}", overlap_result.metadata.duration);
    println!();

    // Demo 4: Gather + Scatter Round-Trip
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 4: Gather + Scatter Round-Trip");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Pattern: Gather → Process → Scatter (common in sparse ops)");
    println!();

    let original = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    let select_indices = vec![1, 3]; // Select indices 1 and 3

    println!("Step 1: Gather from original data");
    println!("Original: {original:?}");
    println!("Select:   {select_indices:?}");

    let gather_step = Workload {
        operation: OperationType::Gather,
        data_type: DataType::F32,
        num_operations: select_indices.len(),
        required_memory: (original.len() + select_indices.len()) * std::mem::size_of::<f32>(),
        input: WorkloadData::F32VecIndexed(original.clone(), select_indices.clone()),
        params: WorkloadParams::default(),
    };

    let gathered = runtime.execute_optimal(gather_step).await?;

    if let WorkloadData::F32Vec(gathered_values) = gathered.data {
        println!("Gathered: {gathered_values:?}");
        println!();

        println!("Step 2: Process (multiply by 2)");
        let processed: Vec<f32> = gathered_values.iter().map(|x| x * 2.0).collect();
        println!("Processed: {processed:?}");
        println!();

        println!("Step 3: Scatter back to original positions");
        let scatter_step = Workload {
            operation: OperationType::Scatter,
            data_type: DataType::F32,
            num_operations: processed.len(),
            required_memory: (processed.len() + select_indices.len() + 4)
                * std::mem::size_of::<f32>(),
            input: WorkloadData::F32VecIndexed(processed.clone(), select_indices.clone()),
            params: WorkloadParams::default(),
        };

        let scattered = runtime.execute_optimal(scatter_step).await?;

        if let WorkloadData::F32Vec(final_result) = scattered.data {
            println!("Result: {final_result:?}");
            println!("Expected: [0.0, 400.0, 0.0, 800.0] (200*2=400, 400*2=800)");
        }
    }

    println!();

    // Demo 5: Real-World Use Cases
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 5: Real-World Use Cases");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("📊 Gather Use Cases:");
    println!("  • Embedding lookup (NLP: word → vector)");
    println!("  • Data sampling (select subset of dataset)");
    println!("  • Graph algorithms (gather neighbor values)");
    println!("  • Attention mechanism (select relevant tokens)");
    println!("  • Database indexing (retrieve by keys)");
    println!();

    println!("📊 Scatter Use Cases:");
    println!("  • Histogram building (accumulate counts)");
    println!("  • Gradient updates (backpropagation to embeddings)");
    println!("  • Sparse matrix operations");
    println!("  • Graph algorithms (update node values)");
    println!("  • Binning operations (place into bins)");
    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Gather:");
    println!("  • Parallelism: 100% embarrassingly parallel (reads)");
    println!("  • Pattern: Map with indirect addressing");
    println!("  • CPU: Excellent (parallel reads, no conflicts)");
    println!("  • GPU: Excellent (naturally parallel, coalescing matters)");
    println!("  • Memory pattern: Random access (cache-unfriendly if sparse)");
    println!("  • Bottleneck: Memory latency (indirect access)");
    println!();

    println!("Scatter:");
    println!("  • Parallelism: Depends on index overlap!");
    println!("  • No overlap: 100% parallel");
    println!("  • With overlap: Requires atomics or segmentation");
    println!("  • CPU: Sequential or atomic (current impl: sequential)");
    println!("  • GPU: Atomic operations (modern GPUs: fast)");
    println!("  • Memory pattern: Random writes");
    println!("  • Bottleneck: Write conflicts (if overlapping indices)");
    println!();

    println!("Key Insights:");
    println!("  1. Gather = Map with indirect read (fully parallel)");
    println!("  2. Scatter = Inverse of Gather (may need atomics)");
    println!("  3. Scatter-add is common pattern (histogram, gradients)");
    println!("  4. Indexing patterns critical for sparse operations");
    println!("  5. GPU needs coalesced access for performance");
    println!();

    println!("barraCuda Opportunities:");
    println!("  • Detect gather/scatter pairs → optimize locality");
    println!("  • Recognize when indices don't overlap → parallel scatter");
    println!("  • Fuse gather + map + scatter → single kernel");
    println!("  • Use shared memory for index reuse (GPU)");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("Gather and Scatter are fundamental building blocks for:");
    println!("  • Neural networks (embeddings, attention)");
    println!("  • Sparse operations (sparse matrix, graph algorithms)");
    println!("  • Data manipulation (sampling, reordering)");
    println!("  • Parallel algorithms (histogram, binning)");
    println!();
    println!("Universal Runtime makes indexing operations hardware-agnostic! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
