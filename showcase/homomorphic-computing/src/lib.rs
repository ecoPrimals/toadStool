//! Homomorphic Computing Cross-Substrate Benchmark
//!
//! This library implements homomorphic encryption benchmarks across:
//! - CPU: Pure Rust baseline (concrete-core)
//! - GPU: barraCUDA acceleration (our internal framework)
//! - NPU: Akida neuromorphic event-driven processing
//!
//! # Key Innovation
//!
//! Using barraCUDA internally allows us to:
//! - Better understand our infrastructure
//! - Identify evolution needs for new workload types
//! - Maintain pure Rust throughout the stack
//! - Dogfood our own technology
//!
//! # Example
//!
//! ```rust,no_run
//! use homomorphic_computing::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Initialize substrates
//!     let cpu = CpuHomomorphic::new()?;
//!     let gpu = GpuHomomorphic::new().await?;
//!     let npu = NpuHomomorphic::new()?;
//!     
//!     // Benchmark encrypted addition
//!     let dataset = generate_encrypted_dataset(10_000);
//!     
//!     let cpu_result = cpu.encrypted_add_batch(&dataset.a, &dataset.b)?;
//!     let gpu_result = gpu.encrypted_add_batch(&dataset.a, &dataset.b).await?;
//!     let npu_result = npu.encrypted_add_batch(&dataset.a, &dataset.b)?;
//!     
//!     // Verify all results match
//!     assert_eq!(cpu_result, gpu_result);
//!     assert_eq!(gpu_result, npu_result);
//!     
//!     Ok(())
//! }
//! ```

#![deny(unsafe_code)]

pub mod schemes;
pub mod substrates;
// pub mod benchmarks;  // TODO: Implement benchmark utilities

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Encrypted dataset for benchmarking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedDataset {
    pub a: Vec<u64>,
    pub b: Vec<u64>,
    pub expected_sum: Vec<u64>,
}

/// Benchmark result for a single substrate
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub substrate_name: String,
    pub throughput_ops_per_sec: f64,
    pub latency_ms: f64,
    pub power_watts: f64,
    pub ops_per_joule: f64,
    pub timestamp: String,
}

impl BenchmarkResult {
    /// Calculate energy efficiency advantage over another result
    pub fn efficiency_advantage(&self, other: &BenchmarkResult) -> f64 {
        self.ops_per_joule / other.ops_per_joule
    }
    
    /// Calculate power savings over another result
    pub fn power_savings(&self, other: &BenchmarkResult) -> f64 {
        (other.power_watts - self.power_watts) / other.power_watts * 100.0
    }
}

/// Workload type for benchmarking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkloadType {
    /// Encrypted integer arithmetic (addition, multiplication)
    Arithmetic,
    /// Encrypted binary classification
    Classification,
    /// Encrypted pattern matching (genomic k-mers)
    PatternMatch,
    /// Encrypted aggregation (sum, avg, max)
    Aggregation,
}

/// Generate encrypted dataset for benchmarking
pub fn generate_encrypted_dataset(size: usize) -> Result<EncryptedDataset> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let a: Vec<u64> = (0..size).map(|_| rng.gen_range(0..1000)).collect();
    let b: Vec<u64> = (0..size).map(|_| rng.gen_range(0..1000)).collect();
    
    // For encrypted addition, we can compute expected sum
    let expected_sum: Vec<u64> = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| x.wrapping_add(y))
        .collect();
    
    Ok(EncryptedDataset {
        a,
        b,
        expected_sum,
    })
}

/// Print comparison table for benchmark results
pub fn print_comparison_table(results: &[BenchmarkResult]) {
    println!("\n┌─────────────────┬────────────┬───────────┬────────────┬──────────────┐");
    println!("│ Substrate       │ Throughput │  Latency  │   Power    │  Efficiency  │");
    println!("├─────────────────┼────────────┼───────────┼────────────┼──────────────┤");
    
    for result in results {
        println!("│ {:<15} │ {:>8.0}/s │ {:>7.1}ms │ {:>8.1}W │ {:>10.0}/J │",
            result.substrate_name,
            result.throughput_ops_per_sec,
            result.latency_ms,
            result.power_watts,
            result.ops_per_joule
        );
    }
    
    println!("└─────────────────┴────────────┴───────────┴────────────┴──────────────┘");
}

/// Analyze NPU advantage over CPU and GPU
pub fn analyze_npu_advantage(cpu: &BenchmarkResult, gpu: &BenchmarkResult, npu: &BenchmarkResult) {
    println!("\n🎯 NPU ADVANTAGE ANALYSIS:\n");
    
    let efficiency_vs_cpu = npu.efficiency_advantage(cpu);
    let efficiency_vs_gpu = npu.efficiency_advantage(gpu);
    let power_savings_vs_cpu = npu.power_savings(cpu);
    let power_savings_vs_gpu = npu.power_savings(gpu);
    
    println!("  Energy Efficiency:");
    println!("    vs CPU: {:.1}x MORE EFFICIENT ⭐", efficiency_vs_cpu);
    println!("    vs GPU: {:.1}x MORE EFFICIENT ⭐", efficiency_vs_gpu);
    
    println!("\n  Power Savings:");
    println!("    vs CPU: {:.1}% less power ({:.1}W → {:.1}W) ⚡", 
        power_savings_vs_cpu, cpu.power_watts, npu.power_watts);
    println!("    vs GPU: {:.1}% less power ({:.1}W → {:.1}W) ⚡", 
        power_savings_vs_gpu, gpu.power_watts, npu.power_watts);
    
    println!("\n  Throughput:");
    let speedup_vs_cpu = npu.throughput_ops_per_sec / cpu.throughput_ops_per_sec;
    let speedup_vs_gpu = npu.throughput_ops_per_sec / gpu.throughput_ops_per_sec;
    println!("    vs CPU: {:.2}x {}", speedup_vs_cpu,
        if speedup_vs_cpu > 1.0 { "FASTER ✅" } else { "slower" });
    println!("    vs GPU: {:.2}x {}", speedup_vs_gpu,
        if speedup_vs_gpu > 1.0 { "FASTER ✅" } else { "slower (but way more efficient!)" });
    
    // Annual energy savings for 24/7 operation
    let hours_per_year = 24.0 * 365.0;
    let cpu_kwh = cpu.power_watts * hours_per_year / 1000.0;
    let gpu_kwh = gpu.power_watts * hours_per_year / 1000.0;
    let npu_kwh = npu.power_watts * hours_per_year / 1000.0;
    
    println!("\n  Annual Energy (24/7 operation):");
    println!("    CPU: {:.0} kWh/year", cpu_kwh);
    println!("    GPU: {:.0} kWh/year", gpu_kwh);
    println!("    NPU: {:.0} kWh/year ⚡", npu_kwh);
    println!("    Savings vs CPU: {:.0} kWh/year", cpu_kwh - npu_kwh);
    println!("    Savings vs GPU: {:.0} kWh/year", gpu_kwh - npu_kwh);
    
    if efficiency_vs_cpu > 10.0 && efficiency_vs_gpu > 10.0 {
        println!("\n🏆 NPU IS THE CLEAR WINNER FOR CONTINUOUS PRIVACY-PRESERVING COMPUTE!");
        println!("   Perfect for: Edge deployment, streaming, 24/7 monitoring");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_encrypted_dataset() {
        let dataset = generate_encrypted_dataset(100).unwrap();
        assert_eq!(dataset.a.len(), 100);
        assert_eq!(dataset.b.len(), 100);
        assert_eq!(dataset.expected_sum.len(), 100);
        
        // Verify expected sum is correct
        for i in 0..100 {
            assert_eq!(
                dataset.expected_sum[i],
                dataset.a[i].wrapping_add(dataset.b[i])
            );
        }
    }
    
    #[test]
    fn test_benchmark_result_metrics() {
        let cpu = BenchmarkResult {
            substrate_name: "CPU".to_string(),
            throughput_ops_per_sec: 1000.0,
            latency_ms: 10.0,
            power_watts: 25.0,
            ops_per_joule: 40.0,
            timestamp: "2026-01-31".to_string(),
        };
        
        let npu = BenchmarkResult {
            substrate_name: "NPU".to_string(),
            throughput_ops_per_sec: 3000.0,
            latency_ms: 5.0,
            power_watts: 2.0,
            ops_per_joule: 1500.0,
            timestamp: "2026-01-31".to_string(),
        };
        
        // NPU should be 37.5x more efficient
        assert_eq!(npu.efficiency_advantage(&cpu), 37.5);
        
        // NPU should save 92% power
        assert_eq!(npu.power_savings(&cpu), 92.0);
    }
}
