// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discover all compute units example
//!
//! This demonstrates the core principle: CPU, GPU, neuromorphic - all
//! are just different orders of the same parallel compute architecture.

use toadstool_runtime_universal::{
    ComputeError, OperationType, ParamValue, UniversalRuntime, WorkloadBuilder,
};

#[tokio::main]
async fn main() -> Result<(), ComputeError> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Universal Compute Runtime - Discovery Demo             ║");
    println!("║  CPU, GPU, Neuromorphic: Different orders of same arch  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Discover all available compute units
    println!("🔍 Discovering compute units...");
    let runtime = UniversalRuntime::discover().await?;
    println!();

    // Display discovered units
    println!("═══════════════════════════════════════════════════════════");
    println!("DISCOVERED COMPUTE UNITS");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    for (idx, unit) in runtime.units().iter().enumerate() {
        let caps = unit.capabilities();
        println!("Unit {}:", idx);
        println!("  Name: {}", unit.name());
        println!("  Type: {}", caps.unit_type);
        println!(
            "  Parallelism: {} units ({:?})",
            caps.parallelism.num_units, caps.parallelism.model
        );
        println!("  Power: {:?}", caps.power_profile);
        println!(
            "  Latency: {} ms (deterministic: {})",
            caps.latency.typical_ms, caps.latency.deterministic
        );
        println!("  Memory: {:.2} GB", caps.memory_capacity as f64 / 1e9);
        println!("  Throughput: {:.2} GFLOPS", caps.compute_throughput / 1e9);
        println!("  Optimal batch: {} elements", caps.optimal_batch_size);
        println!();
    }

    // Display aggregate statistics
    println!("═══════════════════════════════════════════════════════════");
    println!("{}", runtime.stats());
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Demonstrate capability-based execution
    println!("═══════════════════════════════════════════════════════════");
    println!("CAPABILITY-BASED EXECUTION");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Create a simple workload
    let input_data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
    println!("Input: {:?}", input_data);
    println!();

    let workload = WorkloadBuilder::new()
        .operation(OperationType::Map)
        .data_f32(input_data.clone())
        .param("function", ParamValue::String("x * 2.0 + 1.0".to_string()))
        .build()?;

    println!("Workload:");
    println!("  Operation: {:?}", workload.operation);
    println!("  Data type: {:?}", workload.data_type);
    println!("  Size: {} elements", workload.num_operations);
    println!("  Memory: {} bytes", workload.required_memory);
    println!();

    // Runtime selects optimal unit
    println!("🎯 Runtime selecting optimal compute unit...");
    match runtime.execute_optimal(workload).await {
        Ok(output) => {
            println!();
            println!("✅ Execution successful!");
            println!("  Unit used: {}", output.metadata.unit_name);
            println!("  Type: {}", output.metadata.unit_type);
            println!("  Duration: {:?}", output.metadata.duration);
            if let Some(power) = output.metadata.power_consumed_mw {
                println!("  Power: {:.2} mW", power);
            }
            println!();

            // Display result
            match output.data {
                toadstool_runtime_universal::WorkloadData::F32Vec(v) => {
                    println!("Output: {:?}", v);
                }
                _ => println!("Output: (non-f32 data)"),
            }
        }
        Err(e) => {
            println!();
            println!("❌ Execution failed: {}", e);
            println!("  (Note: Full GPU execution not yet implemented in universal runtime)");
            println!("  (CPU execution should work!)");
        }
    }
    println!();

    // Key insights
    println!("═══════════════════════════════════════════════════════════");
    println!("KEY INSIGHTS");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("1. UNIFIED INTERFACE");
    println!("   • CPU, GPU, Neuromorphic all implement ComputeUnit");
    println!("   • Same workload can run on any unit");
    println!("   • Application doesn't need to know which");
    println!();
    println!("2. CAPABILITY-BASED SELECTION");
    println!("   • Runtime discovers what each unit can do");
    println!("   • No hardcoding, no assumptions");
    println!("   • Selects optimal unit for workload");
    println!();
    println!("3. DIFFERENT ORDERS, SAME ARCHITECTURE");
    println!("   • CPU: Low parallelism, low latency");
    println!("   • GPU: High parallelism, high throughput");
    println!("   • Neuromorphic: Event-driven, ultra-low power");
    println!("   • All are parallel compute units!");
    println!();
    println!("4. PURE RUST EVOLUTION PATH");
    println!("   • wgpu: Pure Rust, no FFI");
    println!("   • Type-safe, memory-safe");
    println!("   • Future: barraCuda builds on this");
    println!();

    println!("═══════════════════════════════════════════════════════════");
    println!("🎉 Universal Compute - Run Anywhere!");
    println!("═══════════════════════════════════════════════════════════");

    Ok(())
}
