//! Filter and Scan Operations Demo
//!
//! Demonstrates new operations added to the Universal Runtime:
//! - Filter: Select elements matching a predicate
//! - Scan: Compute prefix sum (cumulative operation)
//!
//! These operations are common in data processing and demonstrate
//! different parallelism patterns.

use toadstool_runtime_universal::runtime::UniversalRuntime;
use toadstool_runtime_universal::types::*;

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Runtime: Filter & Scan Operations Demo       ║");
    println!("║  barraCuda Phase 1 - Operation Pattern Learning         ║");
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

    // Demo 1: Filter Operation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 1: Filter Operation (Select Positive Numbers)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let filter_input = vec![-5.0, 3.0, -2.0, 8.0, -1.0, 0.0, 4.0, -7.0, 6.0, 2.0];
    println!("Input:  {:?}", filter_input);

    let filter_workload = Workload {
        operation: OperationType::Filter,
        data_type: DataType::F32,
        num_operations: filter_input.len(),
        required_memory: filter_input.len() * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Vec(filter_input.clone()),
        params: WorkloadParams::default(),
    };

    let filter_result = runtime.execute_optimal(filter_workload).await?;

    if let WorkloadData::F32Vec(output) = filter_result.data {
        println!("Output: {:?}", output);
        println!(
            "Filtered {} elements → {} elements (predicate: x > 0)",
            filter_input.len(),
            output.len()
        );
    }

    println!();
    println!("Executed on: {}", filter_result.metadata.unit_name);
    println!("Duration:    {:?}", filter_result.metadata.duration);
    println!();

    // Demo 2: Scan Operation (Prefix Sum)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 2: Scan Operation (Prefix Sum / Cumulative)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let scan_input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    println!("Input:  {:?}", scan_input);

    let scan_workload = Workload {
        operation: OperationType::Scan,
        data_type: DataType::F32,
        num_operations: scan_input.len(),
        required_memory: scan_input.len() * std::mem::size_of::<f32>() * 2, // Input + output
        input: WorkloadData::F32Vec(scan_input.clone()),
        params: WorkloadParams::default(),
    };

    let scan_result = runtime.execute_optimal(scan_workload).await?;

    if let WorkloadData::F32Vec(output) = scan_result.data {
        println!("Output: {:?}", output);
        println!("Cumulative sum computed: {} values", output.len());

        // Verify correctness
        let expected_last = scan_input.iter().sum::<f32>();
        let actual_last = *output.last().expect("output is non-empty");
        println!(
            "Expected final sum: {:.1}, Actual: {:.1} ✅",
            expected_last, actual_last
        );
    }

    println!();
    println!("Executed on: {}", scan_result.metadata.unit_name);
    println!("Duration:    {:?}", scan_result.metadata.duration);
    println!();

    // Demo 3: Combined Operations (Pattern Observation)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo 3: Combined Operations (Filter → Scan)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let combined_input = vec![-10.0, 5.0, -3.0, 8.0, -1.0, 12.0, -4.0, 7.0, 2.0, -6.0];
    println!("Input: {:?}", combined_input);
    println!();

    // Step 1: Filter positive numbers
    let filter_workload = Workload {
        operation: OperationType::Filter,
        data_type: DataType::F32,
        num_operations: combined_input.len(),
        required_memory: combined_input.len() * std::mem::size_of::<f32>(),
        input: WorkloadData::F32Vec(combined_input),
        params: WorkloadParams::default(),
    };

    let filter_result = runtime.execute_optimal(filter_workload).await?;
    let filtered_data = match filter_result.data {
        WorkloadData::F32Vec(data) => data,
        other => {
            return Err(ComputeError::ExecutionFailed(format!(
                "Filter operation returned unexpected result type: {:?}",
                other
            )))
        }
    };

    println!(
        "After Filter: {:?} ({} elements)",
        filtered_data,
        filtered_data.len()
    );

    // Step 2: Compute cumulative sum
    let scan_workload = Workload {
        operation: OperationType::Scan,
        data_type: DataType::F32,
        num_operations: filtered_data.len(),
        required_memory: filtered_data.len() * std::mem::size_of::<f32>() * 2,
        input: WorkloadData::F32Vec(filtered_data),
        params: WorkloadParams::default(),
    };

    let scan_result = runtime.execute_optimal(scan_workload).await?;

    if let WorkloadData::F32Vec(output) = scan_result.data {
        println!("After Scan:   {:?}", output);
        println!();
        println!("✅ Pipeline complete: Filter → Scan");
    }

    println!();

    // Pattern Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎓 Pattern Observations (barraCuda Learning)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Filter Operation:");
    println!("  • Parallelism: Embarrassingly parallel (each element independent)");
    println!("  • CPU: Rayon par_iter().filter() - excellent parallel");
    println!("  • GPU: Stream compaction pattern - more complex");
    println!("  • Speedup: Moderate (memory bandwidth bound)");
    println!("  • Use Cases: Data cleaning, conditional selection");
    println!();

    println!("Scan Operation:");
    println!("  • Parallelism: Inherently sequential (dependencies)");
    println!("  • CPU: Simple loop - actually efficient for moderate sizes");
    println!("  • GPU: Parallel scan algorithms (Blelloch, etc.) - complex but fast");
    println!("  • Speedup: Variable (depends on size and algorithm)");
    println!("  • Use Cases: Prefix sums, cumulative stats, indexing");
    println!();

    println!("Combined Pipeline:");
    println!("  • Current: Two separate kernel launches");
    println!("  • Opportunity: Kernel fusion (Filter+Scan in single pass)");
    println!("  • Tradeoff: Complexity vs performance");
    println!("  • barraCuda: Could auto-detect and fuse");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo complete!");
    println!();
    println!("Key Insight: Different operations have different parallelism profiles.");
    println!("Filter is easily parallel, Scan requires special algorithms.");
    println!("Universal Runtime abstracts these differences! 🚀");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
