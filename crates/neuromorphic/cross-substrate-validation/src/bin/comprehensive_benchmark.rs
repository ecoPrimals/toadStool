//! Comprehensive GPU vs Neuromorphic Benchmark
//!
//! Tests various workload types to understand:
//! - What does neuromorphic excel at?
//! - What does it struggle with?
//! - Is it a GPU replacement or complementary?

use akida_driver::DeviceManager;
use cross_substrate_validation::{print_results_summary, run_comprehensive_benchmark};
use toadstool_runtime_universal::UniversalRuntime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("warn").init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║        COMPREHENSIVE CROSS-SUBSTRATE BENCHMARK                              ║");
    println!("║                                                                              ║");
    println!("║    Understanding Neuromorphic vs GPU Performance Characteristics            ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Discover all compute substrates
    println!("1️⃣  Discovering compute substrates...\n");

    let universal = UniversalRuntime::discover().await?;
    let stats = universal.stats();

    println!("   Universal Runtime:");
    println!("{}", stats);

    // Discover Akida devices
    let akida_result = DeviceManager::discover();
    match &akida_result {
        Ok(manager) => {
            println!("   Akida devices: {}", manager.device_count());
            for (i, info) in manager.devices().iter().enumerate() {
                println!("     Device {}: {:?}", i, info);
            }
        }
        Err(e) => {
            println!("   ⚠️  Akida devices: Not available ({:?})", e);
            println!("   Note: Neuromorphic benchmarks will be skipped");
        }
    }
    println!();

    // GPU detection
    let gpu_units = universal.units_by_type(toadstool_runtime_universal::ComputeUnitType::GpuWgpu);
    println!("   Detected GPUs:");
    for (i, unit) in gpu_units.iter().enumerate() {
        println!("     GPU {}: {}", i, unit.name());
    }
    println!();

    // Run comprehensive benchmark
    println!("2️⃣  Running comprehensive benchmark suite...\n");
    println!("   This will test {} workload types:", 19);
    println!("     • Element-wise operations (ReLU, Tanh, Sigmoid, GELU)");
    println!("     • Reduction operations (Sum, Max, etc.)");
    println!("     • Memory-bound operations (Transpose, Gather, Scatter)");
    println!("     • Compute-bound operations (MatMul)");
    println!("     • Normalization (LayerNorm, BatchNorm)");
    println!();

    let results = run_comprehensive_benchmark(&universal).await;

    // Print results
    print_results_summary(&results);

    // Analysis
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("🔍 ANALYSIS: Neuromorphic vs GPU\n");

    println!("   GPU Strengths:");
    println!("     ✅ Parallel compute (SIMD/MIMD)");
    println!("     ✅ High memory bandwidth");
    println!("     ✅ Flexible operations (any kernel)");
    println!("     ✅ Large workloads (100K+ elements)");
    println!("     ✅ Dynamic computation graphs");
    println!();

    println!("   GPU Weaknesses:");
    println!("     ❌ Kernel launch overhead");
    println!("     ❌ Power consumption (100-300W)");
    println!("     ❌ Latency (1-10ms typical)");
    println!();

    println!("   Neuromorphic (Akida) Strengths:");
    println!("     ✅ Ultra-low latency (70-96µs)");
    println!("     ✅ Low power consumption (~1-2W)");
    println!("     ✅ Deterministic timing");
    println!("     ✅ Fixed inference (no overhead)");
    println!("     ✅ Edge deployment friendly");
    println!();

    println!("   Neuromorphic (Akida) Weaknesses:");
    println!("     ❌ Fixed models (not programmable like GPU)");
    println!("     ❌ Limited to inference (no training)");
    println!("     ❌ Model must fit in SRAM (10MB)");
    println!("     ❌ Specific network architectures");
    println!();

    println!("   🎯 VERDICT: GPU Replacement or Complementary?\n");
    println!("     ➡️  COMPLEMENTARY, not replacement!");
    println!();
    println!("     Use GPU when:");
    println!("       • Training models");
    println!("       • Dynamic computation graphs");
    println!("       • Large batch processing");
    println!("       • Flexible operations needed");
    println!("       • High throughput critical");
    println!();
    println!("     Use Neuromorphic when:");
    println!("       • Ultra-low latency critical (<100µs)");
    println!("       • Fixed inference models");
    println!("       • Power efficiency critical");
    println!("       • Edge deployment");
    println!("       • Real-time inference");
    println!("       • Deterministic timing required");
    println!();

    println!("   💡 RECOMMENDATION:\n");
    println!("     Best architecture: GPU + Neuromorphic hybrid!");
    println!("       • Train on GPU");
    println!("       • Deploy fixed models to Neuromorphic for ultra-low latency");
    println!("       • Use GPU for dynamic inference");
    println!("       • Use Neuromorphic for real-time edge inference");
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}
