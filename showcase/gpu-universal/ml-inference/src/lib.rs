// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::redundant_static_lifetimes)]
//! Real ML Inference Showcase - No Mocks
//!
//! Production-grade neural network inference with validation

// Comprehensive error handling (Week 1: Safety First)
pub mod error;

pub mod cnn;
#[cfg(feature = "opencl")]
pub mod conv2d_kernels;
#[cfg(feature = "opencl")]
pub mod gpu_kernels;

// Modern processing substrate abstraction (ZERO deep debt!)
pub mod substrate;

// Modern modular WGPU implementation (part of barraCuda)
pub mod wgpu;

// Legacy compatibility re-export (will be deprecated)
// TODO: Remove after tests migrated to new API
pub use wgpu as wgpu_executor;

pub mod experiments;
pub mod gpu_selector;
pub mod mnist;
pub mod network;

#[cfg(feature = "vulkan")]
pub mod vulkan_executor;

// Optional modules (not all showcases use these)
pub mod cpu_inference;
pub mod gpu_inference;
pub mod training;

// barraCuda GPU operations (Week 3-10) - THE COMPLETE SET!
pub mod advanced_conv;
pub mod advanced_linear;
pub mod attention;
pub mod final_operations;
pub mod normalization;
pub mod quantization;
pub mod random;
pub mod recurrent; // Operations #92-100 - MISSION COMPLETE!

/// GPU test resilience for NVK/Nouveau (skip on driver panics under concurrent load)
pub mod gpu_resilience;

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
        println!(
            "  Accuracy:   {:.2}% vs {:.2}%",
            self.accuracy * 100.0,
            baseline.accuracy * 100.0
        );
        println!(
            "  Latency:    {:.3}ms vs {:.3}ms ({:.1}x faster)",
            self.avg_latency_ms,
            baseline.avg_latency_ms,
            self.speedup_vs(baseline)
        );
        println!(
            "  Throughput: {:.0}/s vs {:.0}/s",
            self.throughput_per_sec, baseline.throughput_per_sec
        );
    }
}
