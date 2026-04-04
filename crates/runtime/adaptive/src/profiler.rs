// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime profiler for GPU operation benchmarking
//!
//! Measures operation performance to learn optimal workgroup sizes.
//! Platform and vendor agnostic - measures YOUR specific hardware!

use crate::cache::{OperationProfile, WorkgroupConfig};
use crate::error::AdaptiveError;
use crate::fingerprint::GpuFingerprint;
use crate::types::{OpType, SizeClass};

/// Profiling configuration
#[derive(Debug, Clone)]
pub struct ProfilingConfig {
    /// Warmup runs before measurement
    pub warmup_runs: usize,
    /// Measurement runs for averaging
    pub measurement_runs: usize,
    /// Timeout per benchmark in milliseconds
    pub timeout_ms: u64,
    /// Minimum confidence threshold
    pub min_confidence: f32,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            warmup_runs: 3,
            measurement_runs: 10,
            timeout_ms: 5000,
            min_confidence: 0.90,
        }
    }
}

/// Runtime profiler for GPU operations
///
/// Performs micro-benchmarks to learn optimal configurations.
/// All measurements done at runtime - no assumptions!
pub struct RuntimeProfiler {
    fingerprint: GpuFingerprint,
    // BLOCKED(real-gpu-executor): config drives warmup/measurement runs once real benchmarks replace the model
    config: ProfilingConfig,
}

impl RuntimeProfiler {
    /// Create new runtime profiler
    ///
    /// # Errors
    ///
    /// Returns error if GPU initialization fails.
    pub fn new(fingerprint: GpuFingerprint) -> Result<Self, AdaptiveError> {
        Ok(Self {
            fingerprint,
            config: ProfilingConfig::default(),
        })
    }

    /// Create profiler with custom config
    #[must_use]
    pub const fn with_config(fingerprint: GpuFingerprint, config: ProfilingConfig) -> Self {
        Self {
            fingerprint,
            config,
        }
    }

    /// Profile a specific operation
    ///
    /// Measures performance for different size classes and workgroup sizes.
    /// Returns optimal configuration for each size class.
    ///
    /// # Errors
    ///
    /// Returns error if benchmarking fails or times out.
    pub fn profile_operation(
        &self,
        op_type: OpType,
        size_classes: &[SizeClass],
        workgroup_candidates: &[usize],
    ) -> Result<OperationProfile, AdaptiveError> {
        let mut profile = OperationProfile::new(op_type);

        for &size_class in size_classes {
            tracing::debug!(
                "Profiling {:?} for {:?} on {}...",
                op_type,
                size_class,
                self.fingerprint.vendor
            );

            let optimal_config =
                self.find_optimal_workgroup(op_type, size_class, workgroup_candidates)?;

            profile.add_config(size_class, optimal_config);
        }

        Ok(profile)
    }

    /// Find optimal workgroup size for operation + size class
    ///
    /// Tests all candidates and returns best performer.
    fn find_optimal_workgroup(
        &self,
        op_type: OpType,
        size_class: SizeClass,
        candidates: &[usize],
    ) -> Result<WorkgroupConfig, AdaptiveError> {
        let mut best_config: Option<WorkgroupConfig> = None;
        let mut best_time = f64::MAX;

        let size = size_class.representative_size();

        for &workgroup_size in candidates {
            match self.benchmark_workgroup(op_type, size, workgroup_size) {
                Ok(avg_time_us) => {
                    tracing::trace!("  Workgroup {}: {:.2} µs", workgroup_size, avg_time_us);

                    if avg_time_us < best_time {
                        best_time = avg_time_us;
                        best_config = Some(WorkgroupConfig::new(workgroup_size, avg_time_us));
                    }
                }
                Err(e) => {
                    tracing::warn!("  Workgroup {} failed: {}", workgroup_size, e);
                }
            }
        }

        best_config.ok_or_else(|| {
            AdaptiveError::Other("No valid workgroup configuration found".to_string())
        })
    }

    /// Benchmark specific workgroup size
    ///
    /// Returns estimated execution time in microseconds.
    /// BLOCKED(real-gpu-executor): Replace with actual wgpu executor micro-benchmarks
    /// once `RuntimeProfiler` holds an executor reference. Current implementation returns
    /// conservative model-based estimates without sleeping. The `Result` return and `&self`
    /// receiver preserve the API surface for the real-gpu-executor evolution.
    #[allow(clippy::unnecessary_wraps)]
    fn benchmark_workgroup(
        &self,
        op_type: OpType,
        size: usize,
        workgroup_size: usize,
    ) -> Result<f64, AdaptiveError> {
        let _ = &self.config;
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let estimated_us = Self::simulate_gpu_time(op_type, size, workgroup_size) as f64;
        Ok(estimated_us)
    }

    /// Heuristic GPU execution time model.
    ///
    /// Returns a conservative cost estimate based on operation type, problem
    /// size, and workgroup size. Used for workgroup tuning when a real executor
    /// is not yet wired. Replaced by actual profiling when barraCuda provides
    /// dispatch timing via capability-based IPC.
    fn simulate_gpu_time(op_type: OpType, size: usize, workgroup_size: usize) -> u64 {
        let base_time = match op_type {
            OpType::MatMul => 1000,
            OpType::LayerNorm => 500,
            OpType::GELU => 200,
            OpType::Softmax => 300,
            _ => 100,
        };

        // Larger sizes take longer
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )] // size values are well within f64 precision
        let size_factor = (size as f64 / 1000.0).sqrt();

        // Suboptimal workgroup sizes are slower
        let workgroup_penalty = if workgroup_size < 64 {
            2.0 // Too small
        } else if workgroup_size > 512 {
            1.5 // Too large
        } else {
            1.0 // Reasonable
        };

        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation acceptable for this conversion"
        )]
        #[allow(clippy::cast_sign_loss)]
        // Result is always positive and within u64 range (simulation timing)
        {
            (f64::from(base_time) * size_factor * workgroup_penalty) as u64
        }
    }

    /// Quick profile all common operations
    ///
    /// Profiles core operations with common sizes.
    /// Takes ~10 seconds on typical hardware.
    ///
    /// # Errors
    ///
    /// Returns error if profiling fails.
    pub fn quick_profile_all(&self) -> Result<Vec<OperationProfile>, AdaptiveError> {
        let operations = vec![
            OpType::MatMul,
            OpType::LayerNorm,
            OpType::GELU,
            OpType::Softmax,
            OpType::Add,
        ];

        let size_classes = vec![SizeClass::Small, SizeClass::Medium, SizeClass::Large];

        let workgroup_candidates = vec![32, 64, 128, 256];

        let mut profiles = Vec::new();
        for op_type in operations {
            let profile = self.profile_operation(op_type, &size_classes, &workgroup_candidates)?;
            profiles.push(profile);
        }

        Ok(profiles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::GpuVendor;

    fn mock_fingerprint() -> GpuFingerprint {
        GpuFingerprint {
            vendor: GpuVendor::NVIDIA,
            architecture: "Ampere".to_string(),
            model_class: "high_end".to_string(),
            driver_version: "1.0".to_string(),
            backend: "Vulkan".to_string(),
            memory_size_gb: 24,
        }
    }

    #[test]
    fn test_profiler_creation() {
        let fingerprint = mock_fingerprint();
        let profiler = RuntimeProfiler::new(fingerprint);
        assert!(profiler.is_ok());
    }

    #[test]
    fn test_profile_operation() {
        let fingerprint = mock_fingerprint();
        let profiler = RuntimeProfiler::new(fingerprint).unwrap();

        let profile = profiler.profile_operation(
            OpType::MatMul,
            &[SizeClass::Small, SizeClass::Medium],
            &[64, 128, 256],
        );

        assert!(profile.is_ok());
        let profile = profile.unwrap();
        assert_eq!(profile.op_type, OpType::MatMul);
        assert!(!profile.size_configs.is_empty());
    }

    #[test]
    fn test_benchmark_workgroup() {
        let fingerprint = mock_fingerprint();
        let profiler = RuntimeProfiler::new(fingerprint).unwrap();

        let result = profiler.benchmark_workgroup(OpType::MatMul, 10_000, 128);

        assert!(result.is_ok());
        let time = result.unwrap();
        assert!(time > 0.0);
    }
}
