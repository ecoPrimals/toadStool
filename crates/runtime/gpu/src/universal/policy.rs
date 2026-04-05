// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload requirements and capability matching / scoring.

use super::detection::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

impl ComputeCapabilities {
    /// Check if capabilities meet requirements
    pub fn meets_requirements(&self, requirements: &ComputeRequirements) -> bool {
        // Check parallelism
        if self.parallelism.max_parallel_threads < requirements.min_parallel_threads {
            return false;
        }

        // Check memory
        if self.memory.total_bytes < requirements.memory_bytes {
            return false;
        }

        // Check precision
        if !self.precision.supports(requirements.precision) {
            return false;
        }

        // Check operations
        for op in &requirements.operations {
            if !self.operations.supports(op) {
                return false;
            }
        }

        true
    }

    /// Calculate capability score for a workload (0.0 = poor, 1.0 = perfect)
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // normalized scoring ratios
    pub fn score_for_workload(&self, requirements: &ComputeRequirements) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // Parallelism score (more is better, but diminishing returns)
        let parallelism_ratio =
            self.parallelism.max_parallel_threads as f64 / requirements.min_parallel_threads as f64;
        score += (parallelism_ratio.min(2.0) / 2.0).min(1.0);
        factors += 1;

        // Memory score (exact match is best)
        let memory_ratio = self.memory.total_bytes as f64 / requirements.memory_bytes as f64;
        score += if (1.0..=2.0).contains(&memory_ratio) {
            1.0
        } else if memory_ratio < 1.0 {
            memory_ratio
        } else {
            0.5 // Too much memory wastes resources
        };
        factors += 1;

        // Precision score (exact match or better)
        score += if self.precision.supports(requirements.precision) {
            1.0
        } else {
            0.0
        };
        factors += 1;

        // Operation score (all required ops supported)
        let supported_ops = requirements
            .operations
            .iter()
            .filter(|op| self.operations.supports(op))
            .count();
        score += supported_ops as f64 / requirements.operations.len().max(1) as f64;
        factors += 1;

        score / factors as f64
    }
}

impl PrecisionCapabilities {
    /// Check if precision is supported
    pub const fn supports(&self, precision: Precision) -> bool {
        match precision {
            Precision::Fp16 => self.fp16,
            Precision::Fp32 => self.fp32,
            Precision::Fp64 => self.fp64,
            Precision::Int8 => self.int8,
            Precision::Int16 => self.int16,
            Precision::Int32 => self.int32,
            Precision::Int64 => self.int64,
            Precision::Mixed => self.mixed_precision,
        }
    }
}

impl OperationCapabilities {
    /// Check if operation is supported
    pub fn supports(&self, operation: &Operation) -> bool {
        match operation {
            Operation::GeneralCompute => self.general_compute,
            Operation::MatrixMultiply => self.matrix_multiply,
            Operation::TensorOps => self.tensor_ops,
            Operation::Convolution => self.convolution,
            Operation::Fft => self.fft,
            Operation::Reduction => self.reduction_ops,
            Operation::Atomic => self.atomic_ops,
            Operation::BranchHeavy => matches!(
                self.branching_efficiency,
                BranchingEfficiency::High | BranchingEfficiency::Medium
            ),
            Operation::Custom(name) => self.custom_ops.contains(name),
        }
    }
}

/// Required precision for computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    /// 16-bit float.
    Fp16,
    /// 32-bit float.
    Fp32,
    /// 64-bit float.
    Fp64,
    /// 8-bit integer.
    Int8,
    /// 16-bit integer.
    Int16,
    /// 32-bit integer.
    Int32,
    /// 64-bit integer.
    Int64,
    /// Mixed precision.
    Mixed,
}

/// Required operations for computation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// General compute.
    GeneralCompute,
    /// Matrix multiply.
    MatrixMultiply,
    /// Tensor operations.
    TensorOps,
    /// Convolution.
    Convolution,
    /// FFT.
    Fft,
    /// Reduction.
    Reduction,
    /// Atomic operations.
    Atomic,
    /// Branch-heavy code.
    BranchHeavy,
    /// Custom operation.
    Custom(String),
}

/// Compute workload requirements (WHAT the workload needs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequirements {
    /// Minimum parallel threads needed
    pub min_parallel_threads: u64,

    /// Memory required (bytes)
    pub memory_bytes: u64,

    /// Required precision
    pub precision: Precision,

    /// Required operations
    pub operations: Vec<Operation>,

    /// Estimated number of operations (for performance prediction)
    /// If None, scheduler will estimate based on workload characteristics
    pub estimated_operations: Option<u64>,

    /// Maximum acceptable execution time
    pub max_execution_time: Option<Duration>,

    /// Preferred memory access pattern
    pub preferred_access_pattern: Option<MemoryAccessPattern>,
}

impl Default for ComputeRequirements {
    fn default() -> Self {
        Self {
            min_parallel_threads: 1,
            memory_bytes: 1024,
            precision: Precision::Fp32,
            operations: vec![Operation::GeneralCompute],
            estimated_operations: None,
            max_execution_time: None,
            preferred_access_pattern: None,
        }
    }
}
