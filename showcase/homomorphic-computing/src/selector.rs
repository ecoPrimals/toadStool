//! Capability-based substrate selection
//!
//! **Deep Debt Principle**: Primal Self-Knowledge
//!
//! This module implements runtime substrate discovery and selection.
//! No hardcoded choices - substrates discover their own availability.
//!
//! ## Design Philosophy
//!
//! 1. **Runtime Discovery**: Detect what's actually available (CPU, GPU, NPU)
//! 2. **Capability-Based**: Select based on workload characteristics
//! 3. **Self-Knowledge**: Each substrate knows its own capabilities
//! 4. **No External Dependencies**: Discover other primals at runtime
//!
//! ## Example
//!
//! ```rust,ignore
//! use homomorphic_computing::*;
//!
//! // Auto-detect available substrates
//! let selector = SubstrateSelector::detect().await?;
//!
//! println!("Available substrates: {}", selector.available_count());
//! // "Available substrates: 3" (CPU, GPU, NPU)
//!
//! // Select based on workload hints
//! let hints = WorkloadHints {
//!     power_budget_watts: Some(5.0),  // Power-constrained
//!     throughput_priority: false,
//!     latency_ms_target: Some(10.0),
//! };
//!
//! let substrate = selector.select(&hints)?;
//! println!("Selected: {}", substrate.name());
//! // "Selected: NPU (Akida)" - best for power-constrained workload
//! ```

use crate::substrates::{CpuHomomorphic, GpuHomomorphic, HomomorphicSubstrate, NpuHomomorphic};
use anyhow::{anyhow, Result};

/// Workload hints for substrate selection
#[derive(Clone, Debug, Default)]
pub struct WorkloadHints {
    /// Power budget in watts (None = unlimited)
    pub power_budget_watts: Option<f64>,

    /// Prioritize throughput over latency
    pub throughput_priority: bool,

    /// Target latency in milliseconds (None = best effort)
    pub latency_ms_target: Option<f64>,

    /// Batch size (larger batches favor GPU)
    pub batch_size: Option<usize>,

    /// Continuous operation (24/7) - favors energy efficiency
    pub continuous_operation: bool,
}

impl WorkloadHints {
    /// Power-constrained edge deployment
    pub fn edge_deployment() -> Self {
        Self {
            power_budget_watts: Some(5.0),
            throughput_priority: false,
            latency_ms_target: Some(50.0),
            batch_size: Some(1),
            continuous_operation: true,
        }
    }

    /// High-throughput batch processing
    pub fn batch_processing() -> Self {
        Self {
            power_budget_watts: None,
            throughput_priority: true,
            latency_ms_target: None,
            batch_size: Some(1000),
            continuous_operation: false,
        }
    }

    /// Real-time streaming
    pub fn streaming() -> Self {
        Self {
            power_budget_watts: Some(10.0),
            throughput_priority: false,
            latency_ms_target: Some(10.0),
            batch_size: Some(1),
            continuous_operation: true,
        }
    }
}

/// Substrate capability information
#[derive(Clone, Debug)]
pub struct SubstrateCapability {
    pub name: String,
    pub power_watts: f64,
    pub typical_throughput_ops_per_sec: f64,
    pub typical_latency_ms: f64,
    pub best_for_batch: bool,
    pub best_for_streaming: bool,
    pub best_for_edge: bool,
}

/// Substrate selector - discovers and selects compute substrates
///
/// **Capability-Based Design**: No hardcoding, runtime discovery
pub struct SubstrateSelector {
    /// Available substrates (discovered at runtime)
    available: Vec<(Box<dyn HomomorphicSubstrate>, SubstrateCapability)>,
}

impl SubstrateSelector {
    /// Detect available substrates at runtime
    ///
    /// **Deep Debt**: Self-knowledge only, no external assumptions
    pub async fn detect() -> Result<Self> {
        let mut available = Vec::new();

        // Try CPU (always available - pure Rust)
        match CpuHomomorphic::new() {
            Ok(cpu) => {
                // ✅ Query real CPU power via substrate's measure_power method
                let power_watts = cpu.measure_power().unwrap_or(25.0);

                let capability = SubstrateCapability {
                    name: cpu.name().to_string(),
                    power_watts,
                    typical_throughput_ops_per_sec: 1_000_000.0,
                    typical_latency_ms: 5.0,
                    best_for_batch: false,
                    best_for_streaming: false,
                    best_for_edge: false,
                };
                available.push((Box::new(cpu) as Box<dyn HomomorphicSubstrate>, capability));
                // println!("✅ CPU substrate available");
            }
            Err(_e) => {
                // println!("❌ CPU substrate unavailable: {}", e);
            }
        }

        // Try GPU (requires wgpu/graphics drivers)
        match GpuHomomorphic::new().await {
            Ok(gpu) => {
                // ✅ Query real GPU power via substrate's measure_power method
                let power_watts = gpu.measure_power().unwrap_or(150.0);

                let capability = SubstrateCapability {
                    name: gpu.name().to_string(),
                    power_watts,
                    typical_throughput_ops_per_sec: 15_000_000.0,
                    typical_latency_ms: 2.0,
                    best_for_batch: true,
                    best_for_streaming: false,
                    best_for_edge: false,
                };
                available.push((Box::new(gpu) as Box<dyn HomomorphicSubstrate>, capability));
                // println!("✅ GPU substrate available (barraCuda)");
            }
            Err(_e) => {
                // println!("❌ GPU substrate unavailable: {}", e);
            }
        }

        // Try NPU (requires Akida hardware)
        match NpuHomomorphic::new() {
            Ok(npu) => {
                // ✅ Query real NPU power via substrate's measure_power method
                let power_watts = npu.measure_power().unwrap_or(2.0);

                let capability = SubstrateCapability {
                    name: npu.name().to_string(),
                    power_watts,
                    typical_throughput_ops_per_sec: 5_000_000.0,
                    typical_latency_ms: 3.0,
                    best_for_batch: false,
                    best_for_streaming: true,
                    best_for_edge: true,
                };
                available.push((Box::new(npu) as Box<dyn HomomorphicSubstrate>, capability));
                // println!("✅ NPU substrate available (Akida)");
            }
            Err(_e) => {
                // println!("❌ NPU substrate unavailable: {}", e);
            }
        }

        if available.is_empty() {
            return Err(anyhow!(
                "No substrates available - this should never happen (CPU is pure Rust)"
            ));
        }

        // println!("🔍 Substrate discovery complete: {} available", available.len());

        Ok(Self { available })
    }

    /// Get count of available substrates
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// List available substrate names
    pub fn available_names(&self) -> Vec<String> {
        self.available
            .iter()
            .map(|(_, cap)| cap.name.clone())
            .collect()
    }

    /// Select substrate based on workload hints
    ///
    /// **Capability-Based Selection**: Match workload to substrate strengths
    pub fn select(&self, hints: &WorkloadHints) -> Result<&dyn HomomorphicSubstrate> {
        if self.available.is_empty() {
            return Err(anyhow!("No substrates available"));
        }

        // Score each substrate based on workload hints
        let mut scored: Vec<(usize, f64)> = self
            .available
            .iter()
            .enumerate()
            .map(|(i, (_, cap))| (i, self.score_substrate(cap, hints)))
            .collect();

        // Sort by score (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_idx = scored[0].0;
        // let best_score = scored[0].1;

        // println!("🎯 Selected {} (score: {:.2})", self.available[best_idx].1.name, best_score);

        Ok(self.available[best_idx].0.as_ref())
    }

    /// Score substrate for given workload
    fn score_substrate(&self, cap: &SubstrateCapability, hints: &WorkloadHints) -> f64 {
        let mut score = 0.0;

        // Power budget constraint (hard constraint)
        if let Some(budget) = hints.power_budget_watts {
            if cap.power_watts > budget {
                return 0.0; // Disqualified
            }
            // Reward efficient use of available power budget
            score += (budget - cap.power_watts) / budget * 50.0;
        }

        // Throughput priority
        if hints.throughput_priority {
            score += cap.typical_throughput_ops_per_sec / 1_000_000.0;
            if cap.best_for_batch {
                score += 20.0;
            }
        }

        // Latency target
        if let Some(target_ms) = hints.latency_ms_target {
            if cap.typical_latency_ms <= target_ms {
                score += 30.0;
            } else {
                score -= (cap.typical_latency_ms - target_ms) * 2.0;
            }
        }

        // Batch size
        if let Some(batch) = hints.batch_size {
            if (batch >= 100 && cap.best_for_batch) || (batch <= 10 && cap.best_for_streaming) {
                score += 25.0;
            }
        }

        // Continuous operation (energy efficiency matters)
        if hints.continuous_operation {
            // Reward low power for 24/7 operation
            score += (200.0 - cap.power_watts) / 10.0;
            if cap.best_for_streaming || cap.best_for_edge {
                score += 30.0;
            }
        }

        // Edge deployment
        if cap.best_for_edge && hints.power_budget_watts.is_some() {
            score += 40.0;
        }

        score
    }

    /// Get all available substrates (for benchmarking)
    pub fn all_substrates(&self) -> Vec<&dyn HomomorphicSubstrate> {
        self.available.iter().map(|(s, _)| s.as_ref()).collect()
    }

    /// Get substrate by name (for explicit selection)
    pub fn by_name(&self, name: &str) -> Option<&dyn HomomorphicSubstrate> {
        self.available
            .iter()
            .find(|(_, cap)| cap.name.contains(name))
            .map(|(s, _)| s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_substrate_detection() {
        let selector = SubstrateSelector::detect().await.unwrap();

        // CPU should always be available (pure Rust)
        assert!(selector.available_count() >= 1);

        let names = selector.available_names();
        assert!(names.iter().any(|n| n.contains("CPU")));

        println!("Available substrates: {:?}", names);
    }

    #[tokio::test]
    async fn test_edge_deployment_selection() {
        let selector = SubstrateSelector::detect().await.unwrap();
        let hints = WorkloadHints::edge_deployment();

        let selected = selector.select(&hints).unwrap();

        // Should select NPU if available (2W), otherwise CPU
        println!("Edge deployment selected: {}", selected.name());

        // NPU should win if available (2W < 5W budget)
        // CPU is backup (25W > 5W budget, but may be only option)
    }

    #[tokio::test]
    async fn test_batch_processing_selection() {
        let selector = SubstrateSelector::detect().await.unwrap();
        let hints = WorkloadHints::batch_processing();

        let selected = selector.select(&hints).unwrap();

        // Should select GPU if available (highest throughput)
        println!("Batch processing selected: {}", selected.name());
    }

    #[tokio::test]
    async fn test_streaming_selection() {
        let selector = SubstrateSelector::detect().await.unwrap();
        let hints = WorkloadHints::streaming();

        let selected = selector.select(&hints).unwrap();

        // Should select NPU if available (low power + streaming optimized)
        println!("Streaming selected: {}", selected.name());
    }
}
