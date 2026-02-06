//! BarraCUDA vs CUDA Benchmarking Framework
//!
//! **Purpose**: Compare BarraCUDA performance against CUDA across hardware
//!
//! This module provides comprehensive benchmarking tools to:
//! - Compare BarraCUDA (WGSL/WebGPU) vs CUDA performance
//! - Test across different hardware (NVIDIA, AMD, Intel, Apple)
//! - Measure operation throughput, latency, and efficiency
//! - Generate performance reports and visualizations
//!
//! **Deep Debt Principles**:
//! - ✅ Fair comparison (same algorithms, same precision)
//! - ✅ Hardware diversity (test on all available GPUs)
//! - ✅ Reproducible (fixed random seeds, warm-up runs)
//! - ✅ Comprehensive (cover all operation types)

use crate::error::Result;
use std::time::Duration;

pub mod operations;
pub mod harness;
pub mod report;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of warm-up iterations (to prime caches)
    pub warmup_iterations: usize,
    
    /// Number of measurement iterations
    pub measurement_iterations: usize,
    
    /// Minimum benchmark duration (ensures statistically significant results)
    pub min_duration: Duration,
    
    /// Enable CUDA comparison (requires CUDA installation)
    pub compare_cuda: bool,
    
    /// Precision (FP32, FP16, etc.)
    pub precision: Precision,
    
    /// Random seed (for reproducibility)
    pub random_seed: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
            min_duration: Duration::from_secs(5),
            compare_cuda: true,
            precision: Precision::FP32,
            random_seed: 42,
        }
    }
}

/// Precision for benchmarks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    FP16,
    FP32,
    FP64,
    INT8,
}

/// Benchmark result for a single operation
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Operation name
    pub operation: String,
    
    /// Hardware name (e.g., "NVIDIA RTX 4090", "AMD RX 7900 XTX")
    pub hardware: String,
    
    /// Framework (BarraCUDA or CUDA)
    pub framework: Framework,
    
    /// Median execution time
    pub median_time: Duration,
    
    /// Mean execution time
    pub mean_time: Duration,
    
    /// Standard deviation
    pub std_dev: Duration,
    
    /// Minimum time
    pub min_time: Duration,
    
    /// Maximum time
    pub max_time: Duration,
    
    /// Throughput (operations/second)
    pub throughput: f64,
    
    /// Memory bandwidth utilized (GB/s)
    pub bandwidth_gbps: f64,
    
    /// TFLOPS achieved
    pub tflops: f64,
}

/// Framework identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    BarraCUDA,
    CUDA,
    PyTorchCUDA,
    TensorFlowCUDA,
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Framework::BarraCUDA => write!(f, "BarraCUDA"),
            Framework::CUDA => write!(f, "CUDA"),
            Framework::PyTorchCUDA => write!(f, "PyTorch+CUDA"),
            Framework::TensorFlowCUDA => write!(f, "TensorFlow+CUDA"),
        }
    }
}

/// Benchmark comparison result
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Operation name
    pub operation: String,
    
    /// Hardware name
    pub hardware: String,
    
    /// BarraCUDA result
    pub barracuda: BenchmarkResult,
    
    /// CUDA result
    pub cuda: Option<BenchmarkResult>,
    
    /// Speedup (positive = BarraCUDA faster, negative = CUDA faster)
    /// 2.0 = BarraCUDA is 2x faster
    /// -2.0 = CUDA is 2x faster
    pub speedup: f64,
    
    /// Parity percentage (100% = same speed, >100% = BarraCUDA faster)
    pub parity_percent: f64,
}

impl ComparisonResult {
    /// Create comparison from two results
    pub fn new(barracuda: BenchmarkResult, cuda: Option<BenchmarkResult>) -> Self {
        let operation = barracuda.operation.clone();
        let hardware = barracuda.hardware.clone();
        
        let (speedup, parity_percent) = if let Some(ref cuda_result) = cuda {
            let barracuda_secs = barracuda.median_time.as_secs_f64();
            let cuda_secs = cuda_result.median_time.as_secs_f64();
            
            let speedup = cuda_secs / barracuda_secs;
            let parity = (cuda_secs / barracuda_secs) * 100.0;
            
            (speedup, parity)
        } else {
            (0.0, 100.0)
        };
        
        Self {
            operation,
            hardware,
            barracuda,
            cuda,
            speedup,
            parity_percent,
        }
    }
    
    /// Check if BarraCUDA achieves target parity (e.g., 90%)
    pub fn achieves_parity(&self, target_percent: f64) -> bool {
        self.parity_percent >= target_percent
    }
}

/// Benchmark suite for all operations
pub struct BenchmarkSuite {
    #[allow(dead_code)] // Used in run_all() implementation
    config: BenchmarkConfig,
    results: Vec<ComparisonResult>,
}

impl BenchmarkSuite {
    /// Create new benchmark suite
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
        }
    }
    
    /// Run all benchmarks
    pub async fn run_all(&mut self) -> Result<()> {
        println!("🚀 Starting BarraCUDA vs CUDA Benchmark Suite");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        
        // Discover hardware
        let hardware = self.discover_hardware().await?;
        println!("📊 Discovered {} compute device(s)", hardware.len());
        for hw in &hardware {
            println!("   • {}", hw);
        }
        println!();
        
        // Run operation benchmarks
        self.benchmark_matrix_operations().await?;
        self.benchmark_activations().await?;
        self.benchmark_reductions().await?;
        self.benchmark_convolutions().await?;
        
        Ok(())
    }
    
    async fn discover_hardware(&self) -> Result<Vec<String>> {
        // TODO: Implement hardware discovery
        Ok(vec!["CPU".to_string()])
    }
    
    async fn benchmark_matrix_operations(&mut self) -> Result<()> {
        println!("📐 Matrix Operations");
        // TODO: Implement matrix benchmarks
        Ok(())
    }
    
    async fn benchmark_activations(&mut self) -> Result<()> {
        println!("⚡ Activation Functions");
        // TODO: Implement activation benchmarks
        Ok(())
    }
    
    async fn benchmark_reductions(&mut self) -> Result<()> {
        println!("📉 Reduction Operations");
        // TODO: Implement reduction benchmarks
        Ok(())
    }
    
    async fn benchmark_convolutions(&mut self) -> Result<()> {
        println!("🔲 Convolution Operations");
        // TODO: Implement convolution benchmarks
        Ok(())
    }
    
    /// Get all results
    pub fn results(&self) -> &[ComparisonResult] {
        &self.results
    }
    
    /// Generate summary report
    pub fn summary(&self) -> BenchmarkSummary {
        let total_ops = self.results.len();
        let ops_with_parity_90 = self.results.iter()
            .filter(|r| r.achieves_parity(90.0))
            .count();
        let ops_with_parity_95 = self.results.iter()
            .filter(|r| r.achieves_parity(95.0))
            .count();
        let ops_with_parity_98 = self.results.iter()
            .filter(|r| r.achieves_parity(98.0))
            .count();
        
        let mean_parity = if total_ops > 0 {
            self.results.iter()
                .map(|r| r.parity_percent)
                .sum::<f64>() / total_ops as f64
        } else {
            0.0
        };
        
        BenchmarkSummary {
            total_operations: total_ops,
            ops_at_90_parity: ops_with_parity_90,
            ops_at_95_parity: ops_with_parity_95,
            ops_at_98_parity: ops_with_parity_98,
            mean_parity_percent: mean_parity,
        }
    }
}

/// Benchmark summary statistics
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub total_operations: usize,
    pub ops_at_90_parity: usize,
    pub ops_at_95_parity: usize,
    pub ops_at_98_parity: usize,
    pub mean_parity_percent: f64,
}

impl std::fmt::Display for BenchmarkSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "📊 Benchmark Summary")?;
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(f, "Total Operations: {}", self.total_operations)?;
        writeln!(f, "≥90% Parity: {} ({:.1}%)", 
            self.ops_at_90_parity,
            (self.ops_at_90_parity as f64 / self.total_operations as f64) * 100.0
        )?;
        writeln!(f, "≥95% Parity: {} ({:.1}%)", 
            self.ops_at_95_parity,
            (self.ops_at_95_parity as f64 / self.total_operations as f64) * 100.0
        )?;
        writeln!(f, "≥98% Parity: {} ({:.1}%)", 
            self.ops_at_98_parity,
            (self.ops_at_98_parity as f64 / self.total_operations as f64) * 100.0
        )?;
        writeln!(f, "Mean Parity: {:.2}%", self.mean_parity_percent)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.warmup_iterations, 10);
        assert_eq!(config.measurement_iterations, 100);
        assert_eq!(config.precision, Precision::FP32);
    }

    #[test]
    fn test_framework_display() {
        assert_eq!(Framework::BarraCUDA.to_string(), "BarraCUDA");
        assert_eq!(Framework::CUDA.to_string(), "CUDA");
    }
}
