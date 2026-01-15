//! Real ML Inference Showcase - No Mocks
//!
//! Production-grade neural network inference with validation

#[cfg(feature = "opencl")]
pub mod gpu_kernels;
#[cfg(feature = "opencl")]
pub mod conv2d_kernels;
pub mod cnn;

// Modern modular WGPU implementation (part of barraCUDA)
pub mod wgpu;

// Legacy compatibility re-export (will be deprecated)
// TODO: Remove after tests migrated to new API
pub use wgpu as wgpu_executor;

pub mod gpu_selector;
pub mod experiments;
pub mod mnist;
pub mod network;

#[cfg(feature = "vulkan")]
pub mod vulkan_executor;

// Optional modules (not all showcases use these)
pub mod cpu_inference;
pub mod gpu_inference;
pub mod training;

// barraCUDA GPU operations (Week 3-8)
pub mod attention;
pub mod recurrent;
pub mod advanced_conv;
pub mod quantization;
pub mod random;
pub mod advanced_linear;

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Inference result with validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub predicted_class: usize,
    pub confidence: f32,
    pub all_probabilities: Vec<f32>,
    pub latency: Duration,
    pub backend: String,
}

impl InferenceResult {
    /// Validate that two results match
    pub fn matches(&self, other: &Self, tolerance: f32) -> bool {
        // Class must match
        if self.predicted_class != other.predicted_class {
            return false;
        }
        
        // Confidence must be within tolerance
        if (self.confidence - other.confidence).abs() > tolerance {
            return false;
        }
        
        // All probabilities must be within tolerance
        for (a, b) in self.all_probabilities.iter().zip(&other.all_probabilities) {
            if (a - b).abs() > tolerance {
                return false;
            }
        }
        
        true
    }
}

/// Benchmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    pub backend: String,
    pub samples: usize,
    pub correct: usize,
    pub accuracy: f32,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub throughput_per_sec: f64,
    pub total_time_ms: f64,
}

impl BenchmarkStats {
    pub fn speedup_vs(&self, baseline: &Self) -> f64 {
        baseline.avg_latency_ms / self.avg_latency_ms
    }
    
    pub fn display_comparison(&self, baseline: &Self) {
        println!("\n═══ {} vs {} ═══", self.backend, baseline.backend);
        println!("  Accuracy:   {:.2}% vs {:.2}%", 
            self.accuracy * 100.0, baseline.accuracy * 100.0);
        println!("  Latency:    {:.3}ms vs {:.3}ms ({:.1}x faster)", 
            self.avg_latency_ms, baseline.avg_latency_ms, self.speedup_vs(baseline));
        println!("  Throughput: {:.0}/s vs {:.0}/s", 
            self.throughput_per_sec, baseline.throughput_per_sec);
    }
}

