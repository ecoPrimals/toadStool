// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Capability-Based Compute Abstractions
//!
//! This module provides hardware-agnostic compute abstractions where:
//! - Workloads describe WHAT they need (capabilities)
//! - Resources describe WHAT they can do (capabilities)
//! - Scheduler matches workloads to resources
//!
//! This enables:
//! - GPU, CPU, TPU, FPGA, Quantum, etc. as equal compute resources
//! - Automatic resource selection based on capabilities
//! - Future-proof architecture for unknown compute paradigms

pub mod detection;
pub mod execution;
pub mod policy;

pub use detection::*;
pub use execution::*;
pub use policy::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_matching() {
        let capabilities = ComputeCapabilities {
            parallelism: ParallelismCapabilities {
                max_parallel_threads: 4096,
                model: ParallelismModel::Simt { max_threads: 4096 },
                max_work_group_size: Some(256),
                simd_width: None,
                nested_parallelism: false,
            },
            memory: MemoryCapabilities {
                total_bytes: 8 * 1024 * 1024 * 1024,               // 8 GB
                bandwidth_bytes_per_sec: 300 * 1024 * 1024 * 1024, // 300 GB/s
                unified_memory: false,
                zero_copy: false,
                cache_levels: vec![],
                access_patterns: vec![MemoryAccessPattern::Coalesced],
            },
            precision: PrecisionCapabilities {
                fp16: true,
                fp32: true,
                fp64: false,
                int8: true,
                int16: true,
                int32: true,
                int64: false,
                mixed_precision: true,
            },
            operations: OperationCapabilities {
                general_compute: true,
                matrix_multiply: true,
                tensor_ops: true,
                convolution: true,
                fft: true,
                reduction_ops: true,
                atomic_ops: true,
                branching_efficiency: BranchingEfficiency::Medium,
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: 10_000_000_000_000.0, // 10 TFLOPS
                peak_iops: 20_000_000_000_000.0,
                power_watts: 250.0,
                startup_latency_us: 100,
                sustained_performance_percent: 80.0,
            },
            resource_type: "GPU".to_string(),
        };

        let requirements = ComputeRequirements {
            min_parallel_threads: 1024,
            memory_bytes: 1024 * 1024, // 1 MB
            precision: Precision::Fp32,
            operations: vec![Operation::MatrixMultiply],
            estimated_operations: Some(1024 * 1024),
            max_execution_time: None,
            preferred_access_pattern: None,
        };

        assert!(capabilities.meets_requirements(&requirements));

        let score = capabilities.score_for_workload(&requirements);
        assert!(score > 0.8); // Should be a good match
    }

    #[test]
    fn test_precision_support() {
        let precision = PrecisionCapabilities {
            fp16: false,
            fp32: true,
            fp64: true,
            int8: false,
            int16: true,
            int32: true,
            int64: true,
            mixed_precision: false,
        };

        assert!(!precision.supports(Precision::Fp16));
        assert!(precision.supports(Precision::Fp32));
        assert!(precision.supports(Precision::Fp64));
        assert!(precision.supports(Precision::Int32));
    }
}
