//! NeuroBench benchmark definitions
//!
//! Implements the standard NeuroBench benchmark suite.

#[allow(unused_imports)]
use crate::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Available benchmarks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Benchmark {
    /// DVS Gesture recognition
    DvsGesture,
    /// Few-shot keyword spotting
    KeywordFscil,
    /// Chaotic function prediction
    ChaoticFunction,
    /// Neural prosthetics motor prediction
    NhpMotor,
    /// Event camera object detection
    EventCamera,
}

impl Benchmark {
    /// Get benchmark description
    pub fn description(&self) -> &'static str {
        match self {
            Self::DvsGesture => "Dynamic Vision Sensor gesture recognition (11 classes)",
            Self::KeywordFscil => "Few-shot keyword spotting with incremental classes",
            Self::ChaoticFunction => "Chaotic time series prediction (Lorenz attractor)",
            Self::NhpMotor => "Non-human primate motor cortex decoding",
            Self::EventCamera => "Event camera object detection",
        }
    }
    
    /// Get number of classes
    pub fn num_classes(&self) -> usize {
        match self {
            Self::DvsGesture => 11,
            Self::KeywordFscil => 35, // base + novel
            Self::ChaoticFunction => 1, // regression
            Self::NhpMotor => 2, // x/y velocity
            Self::EventCamera => 80, // COCO classes
        }
    }
    
    /// Get expected input shape [batch, channels, height, width] or [batch, timesteps, features]
    pub fn input_shape(&self) -> Vec<usize> {
        match self {
            Self::DvsGesture => vec![1, 2, 128, 128], // ON/OFF events
            Self::KeywordFscil => vec![1, 49, 10], // MFCC features
            Self::ChaoticFunction => vec![1, 1000, 3], // Lorenz x,y,z
            Self::NhpMotor => vec![1, 200, 96], // Neural channels
            Self::EventCamera => vec![1, 2, 240, 320], // ON/OFF events
        }
    }
}

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of inference iterations
    pub num_iterations: usize,
    /// Warmup iterations (not counted in metrics)
    pub warmup_iterations: usize,
    /// Use quantized model
    pub quantized: bool,
    /// Enable power measurement
    pub measure_power: bool,
    /// Random seed for reproducibility
    pub seed: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_iterations: 1000,
            warmup_iterations: 100,
            quantized: true,
            measure_power: true,
            seed: 42,
        }
    }
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub benchmark: Benchmark,
    /// Classification accuracy (0.0-1.0) or MSE for regression
    pub accuracy: f64,
    /// Mean latency per inference
    pub mean_latency: Duration,
    /// 95th percentile latency
    pub p95_latency: Duration,
    /// 99th percentile latency
    pub p99_latency: Duration,
    /// Throughput (inferences per second)
    pub throughput: f64,
    /// Mean power consumption (mW)
    pub mean_power_mw: Option<f64>,
    /// Energy per inference (uJ)
    pub energy_per_inference_uj: Option<f64>,
    /// Total samples processed
    pub num_samples: usize,
    /// Correct classifications
    pub num_correct: usize,
}

impl BenchmarkResult {
    /// Create a new result
    pub fn new(benchmark: Benchmark) -> Self {
        Self {
            benchmark,
            accuracy: 0.0,
            mean_latency: Duration::ZERO,
            p95_latency: Duration::ZERO,
            p99_latency: Duration::ZERO,
            throughput: 0.0,
            mean_power_mw: None,
            energy_per_inference_uj: None,
            num_samples: 0,
            num_correct: 0,
        }
    }
    
    /// Calculate derived metrics
    pub fn finalize(&mut self, latencies: &[Duration]) {
        if latencies.is_empty() {
            return;
        }
        
        // Sort for percentiles
        let mut sorted: Vec<_> = latencies.iter().copied().collect();
        sorted.sort();
        
        // Mean latency
        let total: Duration = sorted.iter().sum();
        self.mean_latency = total / sorted.len() as u32;
        
        // Percentiles
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        self.p95_latency = sorted.get(p95_idx).copied().unwrap_or(Duration::ZERO);
        self.p99_latency = sorted.get(p99_idx).copied().unwrap_or(Duration::ZERO);
        
        // Throughput
        if self.mean_latency.as_nanos() > 0 {
            self.throughput = 1_000_000_000.0 / self.mean_latency.as_nanos() as f64;
        }
        
        // Energy per inference
        if let Some(power) = self.mean_power_mw {
            let latency_us = self.mean_latency.as_micros() as f64;
            self.energy_per_inference_uj = Some(power * latency_us / 1000.0);
        }
        
        // Accuracy
        if self.num_samples > 0 {
            self.accuracy = self.num_correct as f64 / self.num_samples as f64;
        }
    }
    
    /// Print summary to stdout
    pub fn print_summary(&self) {
        println!("\n=== {} ===", self.benchmark.description());
        println!("Accuracy:     {:.2}%", self.accuracy * 100.0);
        println!("Throughput:   {:.1} inf/s", self.throughput);
        println!("Mean latency: {:.3} ms", self.mean_latency.as_secs_f64() * 1000.0);
        println!("P95 latency:  {:.3} ms", self.p95_latency.as_secs_f64() * 1000.0);
        println!("P99 latency:  {:.3} ms", self.p99_latency.as_secs_f64() * 1000.0);
        if let Some(power) = self.mean_power_mw {
            println!("Mean power:   {:.1} mW", power);
        }
        if let Some(energy) = self.energy_per_inference_uj {
            println!("Energy/inf:   {:.1} uJ", energy);
        }
        println!("Samples:      {}", self.num_samples);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_descriptions() {
        for benchmark in [
            Benchmark::DvsGesture,
            Benchmark::KeywordFscil,
            Benchmark::ChaoticFunction,
            Benchmark::NhpMotor,
            Benchmark::EventCamera,
        ] {
            println!("{:?}: {}", benchmark, benchmark.description());
            println!("  Classes: {}", benchmark.num_classes());
            println!("  Input: {:?}", benchmark.input_shape());
        }
    }
    
    #[test]
    fn test_result_finalize() {
        let latencies: Vec<Duration> = (0..100)
            .map(|i| Duration::from_micros(100 + i * 10))
            .collect();
        
        let mut result = BenchmarkResult::new(Benchmark::DvsGesture);
        result.num_samples = 100;
        result.num_correct = 92;
        result.mean_power_mw = Some(50.0);
        result.finalize(&latencies);
        
        assert!(result.accuracy > 0.9);
        assert!(result.throughput > 0.0);
        result.print_summary();
    }
}
