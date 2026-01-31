//! Cross-Substrate Homomorphic Computing Benchmark
//!
//! This example compares homomorphic encryption performance across:
//! - CPU: Pure Rust baseline
//! - GPU: barraCUDA acceleration (our internal framework)
//! - NPU: Akida neuromorphic event-driven processing
//!
//! Run with:
//! ```bash
//! cargo run --example cross_substrate_comparison --release
//! ```

use homomorphic_computing::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  🔐🧠 Homomorphic Computing: Cross-Substrate Benchmark   ║");
    println!("║                                                          ║");
    println!("║  Comparing privacy-preserving computation across:       ║");
    println!("║  • CPU (Pure Rust baseline)                             ║");
    println!("║  • GPU (barraCUDA - our internal framework) ⭐          ║");
    println!("║  • NPU (Akida neuromorphic) ⚡                          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    // Initialize substrates
    println!("🔧 Initializing compute substrates...\n");
    
    use substrates::*;
    
    let cpu = CpuHomomorphic::new()?;
    println!("  ✅ CPU substrate ready");
    
    let gpu = GpuHomomorphic::new()?;
    println!("  ✅ GPU substrate ready (barraCUDA)");
    
    let npu = NpuHomomorphic::new()?;
    println!("  ✅ NPU substrate ready (Akida)");
    
    // Benchmark parameters
    let dataset_size = 1000;
    let iterations = 10;
    
    println!("\n📊 Running benchmarks...");
    println!("   Dataset size: {} encrypted values", dataset_size);
    println!("   Iterations: {}\n", iterations);
    
    // Run benchmarks
    println!("🔄 Benchmarking CPU (Pure Rust)...");
    let cpu_result = cpu.benchmark(dataset_size, iterations).await?;
    println!("   ✅ Complete: {:.0} ops/sec, {:.1}ms latency, {}W power",
        cpu_result.throughput_ops_per_sec,
        cpu_result.latency_ms,
        cpu_result.power_watts
    );
    
    println!("\n🔄 Benchmarking GPU (barraCUDA)...");
    let gpu_result = gpu.benchmark(dataset_size, iterations).await?;
    println!("   ✅ Complete: {:.0} ops/sec, {:.1}ms latency, {}W power",
        gpu_result.throughput_ops_per_sec,
        gpu_result.latency_ms,
        gpu_result.power_watts
    );
    
    println!("\n🔄 Benchmarking NPU (Akida)...");
    let npu_result = npu.benchmark(dataset_size, iterations).await?;
    println!("   ✅ Complete: {:.0} ops/sec, {:.1}ms latency, {}W power",
        npu_result.throughput_ops_per_sec,
        npu_result.latency_ms,
        npu_result.power_watts
    );
    
    // Print comparison table
    println!("\n📊 RESULTS:");
    print_comparison_table(&[cpu_result.clone(), gpu_result.clone(), npu_result.clone()]);
    
    // Analyze NPU advantage
    analyze_npu_advantage(&cpu_result, &gpu_result, &npu_result);
    
    // Best substrate for each use case
    println!("\n🎯 RECOMMENDATIONS:\n");
    println!("  Batch Processing (high throughput needed):");
    println!("    → GPU (barraCUDA): {:.0} ops/sec ✅", gpu_result.throughput_ops_per_sec);
    
    println!("\n  Streaming / Edge Deployment (power-constrained):");
    println!("    → NPU (Akida): {}W power, {:.0} ops/J ⚡", 
        npu_result.power_watts, npu_result.ops_per_joule);
    
    println!("\n  24/7 Continuous Privacy Compute:");
    println!("    → NPU (Akida): {:.0} kWh/year savings vs CPU ⭐",
        (cpu_result.power_watts - npu_result.power_watts) * 24.0 * 365.0 / 1000.0);
    
    // barraCUDA evolution insights
    println!("\n🔧 barraCUDA EVOLUTION INSIGHTS:\n");
    println!("  As we implement homomorphic operations with barraCUDA, we discovered:");
    println!("  • Need for polynomial arithmetic primitives");
    println!("  • NTT (Number Theoretic Transform) kernel opportunities");
    println!("  • Memory transfer patterns for encrypted data");
    println!("  • API ergonomics for cryptographic workloads");
    println!("\n  These insights guide barraCUDA's evolution! 🎯");
    
    println!("\n✅ Benchmark complete!\n");
    
    Ok(())
}
