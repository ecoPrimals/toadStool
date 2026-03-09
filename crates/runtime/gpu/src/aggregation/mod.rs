// SPDX-License-Identifier: AGPL-3.0-only
//! Result Aggregation with Fault Tolerance
//!
//! Aggregates partial results from distributed GPU execution with:
//! - Fault tolerance (handles partial failures)
//! - Multiple aggregation strategies
//! - Zero-copy where possible
//! - Async, non-blocking operations

pub mod fault_tolerance;
pub mod merger;
pub mod strategies;

pub use fault_tolerance::{PartialResultSet, PartialResultStatus, RecoveryStrategy};
pub use merger::{MatrixMerger, ScalarReducer, VectorMerger};
pub use strategies::{AggregationStrategy, ResultAggregator};

/// Aggregated result from distributed execution
#[derive(Debug, Clone)]
pub struct AggregatedResult {
    /// Final aggregated data
    pub data: Vec<u8>,

    /// Metadata about aggregation
    pub metadata: AggregationMetadata,

    /// Warnings (if any partial failures)
    pub warnings: Vec<String>,
}

/// Metadata about the aggregation process
#[derive(Debug, Clone)]
pub struct AggregationMetadata {
    /// Total partial results expected
    pub expected_count: usize,

    /// Successful partial results received
    pub successful_count: usize,

    /// Failed partial results
    pub failed_count: usize,

    /// Total aggregation time
    pub aggregation_time: std::time::Duration,

    /// Recovery strategy used (if any)
    pub recovery_strategy: Option<RecoveryStrategy>,
}

impl AggregatedResult {
    /// Check if aggregation was fully successful
    pub fn is_complete(&self) -> bool {
        self.metadata.failed_count == 0
    }

    /// Check if aggregation used recovery
    pub fn used_recovery(&self) -> bool {
        self.metadata.recovery_strategy.is_some()
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.metadata.expected_count == 0 {
            return 0.0;
        }
        (self.metadata.successful_count as f32 / self.metadata.expected_count as f32) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // test values are exact literals
    fn test_aggregated_result_complete() {
        let result = AggregatedResult {
            data: vec![1, 2, 3],
            metadata: AggregationMetadata {
                expected_count: 4,
                successful_count: 4,
                failed_count: 0,
                aggregation_time: std::time::Duration::from_millis(10),
                recovery_strategy: None,
            },
            warnings: vec![],
        };

        assert!(result.is_complete());
        assert!(!result.used_recovery());
        assert_eq!(result.completion_percentage(), 100.0);
    }

    #[test]
    #[allow(clippy::float_cmp)] // test values are exact literals
    fn test_aggregated_result_partial() {
        let result = AggregatedResult {
            data: vec![1, 2, 3],
            metadata: AggregationMetadata {
                expected_count: 4,
                successful_count: 3,
                failed_count: 1,
                aggregation_time: std::time::Duration::from_millis(10),
                recovery_strategy: Some(RecoveryStrategy::AggregatePartial),
            },
            warnings: vec!["Tower 2 failed".to_string()],
        };

        assert!(!result.is_complete());
        assert!(result.used_recovery());
        assert_eq!(result.completion_percentage(), 75.0);
    }
}
