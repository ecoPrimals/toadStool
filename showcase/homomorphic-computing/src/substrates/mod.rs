//! Compute substrates for homomorphic operations
//!
//! This module implements homomorphic encryption across different substrates:
//! - CPU: Pure Rust baseline
//! - GPU: barraCUDA acceleration (our internal framework) ⭐
//! - NPU: Akida neuromorphic event-driven processing

pub mod cpu;
pub mod gpu;
pub mod npu;

pub use cpu::CpuHomomorphic;
pub use gpu::GpuHomomorphic;
pub use npu::NpuHomomorphic;

use anyhow::Result;
use crate::BenchmarkResult;

/// Trait for homomorphic compute substrates
#[async_trait::async_trait]
pub trait HomomorphicSubstrate {
    /// Get substrate name
    fn name(&self) -> &str;
    
    /// Encrypt and add two arrays of integers
    async fn encrypted_add_batch(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>>;
    
    /// Encrypt and multiply two arrays of integers
    async fn encrypted_multiply_batch(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>>;
    
    /// Benchmark the substrate performance
    async fn benchmark(&self, dataset_size: usize, iterations: usize) -> Result<BenchmarkResult>;
    
    /// Measure power consumption (if available)
    fn measure_power(&self) -> Option<f64> {
        None  // Default: no power measurement
    }
}
