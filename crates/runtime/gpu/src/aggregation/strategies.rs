// SPDX-License-Identifier: AGPL-3.0-only
//! Aggregation Strategies
//!
//! Different strategies for combining partial results from distributed execution

use super::{AggregatedResult, AggregationMetadata, PartialResultSet};
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Strategy for aggregating partial results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// Concatenate results in order
    Concatenate,

    /// Merge matrix chunks into final matrix
    MatrixMerge,

    /// Reduce results (sum, min, max, etc.)
    Reduction(ReductionOp),

    /// Average across results
    Average,

    /// Custom aggregation (user-defined)
    Custom,
}

/// Reduction operations for aggregating partial results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReductionOp {
    /// Sum reduction.
    Sum,
    /// Minimum reduction.
    Min,
    /// Maximum reduction.
    Max,
    /// Product reduction.
    Product,
    /// Logical AND reduction.
    And,
    /// Logical OR reduction.
    Or,
}

/// Result aggregator with fault tolerance.
pub struct ResultAggregator {
    /// Aggregation strategy.
    strategy: AggregationStrategy,
}

impl ResultAggregator {
    /// Creates a new aggregator with the specified strategy.
    pub const fn new(strategy: AggregationStrategy) -> Self {
        Self { strategy }
    }

    /// Aggregate partial results into final result
    ///
    /// Handles partial failures gracefully using recovery strategies
    ///
    /// # Errors
    ///
    /// Returns [`ToadStoolError`] when there are insufficient successful partial results, or aggregation fails.
    pub async fn aggregate(
        &self,
        partial_results: PartialResultSet,
    ) -> ToadStoolResult<AggregatedResult> {
        let start_time = std::time::Instant::now();

        // Check if we can proceed with available results
        if !partial_results.is_sufficient() {
            return Err(ToadStoolError::runtime(format!(
                "Insufficient results: {} successful out of {} expected",
                partial_results.successful_count(),
                partial_results.expected_count()
            )));
        }

        // Determine recovery strategy if needed
        let recovery_strategy = if partial_results.has_failures() {
            Some(partial_results.recovery_strategy())
        } else {
            None
        };

        // Aggregate based on strategy
        let data = match self.strategy {
            AggregationStrategy::Concatenate => self.concatenate_results(&partial_results).await?,
            AggregationStrategy::MatrixMerge => self.merge_matrix_results(&partial_results).await?,
            AggregationStrategy::Reduction(op) => self.reduce_results(&partial_results, op).await?,
            AggregationStrategy::Average => self.average_results(&partial_results).await?,
            AggregationStrategy::Custom => {
                return Err(ToadStoolError::runtime(
                    "Custom aggregation requires custom aggregator implementation",
                ));
            }
        };

        let aggregation_time = start_time.elapsed();

        // Collect warnings for failed results
        let warnings = partial_results.failure_messages();

        Ok(AggregatedResult {
            data,
            metadata: AggregationMetadata {
                expected_count: partial_results.expected_count(),
                successful_count: partial_results.successful_count(),
                failed_count: partial_results.failed_count(),
                aggregation_time,
                recovery_strategy,
            },
            warnings,
        })
    }

    /// Concatenate results in order
    async fn concatenate_results(&self, results: &PartialResultSet) -> ToadStoolResult<Vec<u8>> {
        let successful = results.successful_results();
        let mut output = Vec::new();

        // Sort by sequence number to maintain order
        let mut sorted: Vec<_> = successful.iter().collect();
        sorted.sort_by_key(|r| r.sequence);

        // Concatenate (zero-copy where possible)
        for result in sorted {
            output.extend_from_slice(&result.data);
        }

        Ok(output)
    }

    /// Merge matrix chunks into final matrix
    async fn merge_matrix_results(&self, results: &PartialResultSet) -> ToadStoolResult<Vec<u8>> {
        // Matrix merging logic would go here
        // For now, use concatenation as fallback
        self.concatenate_results(results).await
    }

    /// Reduce results using specified operation
    async fn reduce_results(
        &self,
        results: &PartialResultSet,
        op: ReductionOp,
    ) -> ToadStoolResult<Vec<u8>> {
        let successful = results.successful_results();
        if successful.is_empty() {
            return Ok(Vec::new());
        }

        // For simplicity, assume results are f32 arrays
        let first = &successful[0].data;
        let elem_count = first.len() / 4; // f32 = 4 bytes

        if elem_count == 0 {
            return Ok(Vec::new());
        }

        let mut result = vec![0.0f32; elem_count];

        // Initialize based on operation
        match op {
            ReductionOp::Sum => {} // Start at 0
            ReductionOp::Product => result.fill(1.0),
            ReductionOp::Min => result.fill(f32::MAX),
            ReductionOp::Max => result.fill(f32::MIN),
            ReductionOp::And | ReductionOp::Or => {}
        }

        // Apply reduction operation
        for partial in successful {
            if partial.data.len() != first.len() {
                continue; // Skip mismatched sizes
            }

            // Convert bytes to f32
            let values: Vec<f32> = partial
                .data
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            // Apply operation
            for (i, &val) in values.iter().enumerate() {
                if i >= elem_count {
                    break;
                }

                result[i] = match op {
                    ReductionOp::Sum => result[i] + val,
                    ReductionOp::Product => result[i] * val,
                    ReductionOp::Min => result[i].min(val),
                    ReductionOp::Max => result[i].max(val),
                    ReductionOp::And => {
                        if result[i] != 0.0 && val != 0.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    ReductionOp::Or => {
                        if result[i] != 0.0 || val != 0.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                };
            }
        }

        // Convert back to bytes
        let bytes: Vec<u8> = result.iter().flat_map(|&f| f.to_le_bytes()).collect();

        Ok(bytes)
    }

    /// Average results across all partial results
    #[allow(clippy::cast_precision_loss)] // count as f32 for mean
    async fn average_results(&self, results: &PartialResultSet) -> ToadStoolResult<Vec<u8>> {
        // First sum, then divide
        let summed = self.reduce_results(results, ReductionOp::Sum).await?;
        let count = results.successful_count() as f32;

        if count == 0.0 {
            return Ok(Vec::new());
        }

        // Divide by count (`count` from `successful_count` as f32)
        let values: Vec<f32> = summed
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) / count)
            .collect();

        let bytes: Vec<u8> = values.iter().flat_map(|&f| f.to_le_bytes()).collect();

        Ok(bytes)
    }
}

impl Default for ResultAggregator {
    fn default() -> Self {
        Self::new(AggregationStrategy::Concatenate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregation::fault_tolerance::{PartialResult, PartialResultStatus};

    fn create_test_results() -> PartialResultSet {
        let results = vec![
            PartialResultStatus::Success(PartialResult {
                tower_id: "tower1".to_string(),
                sequence: 0,
                data: vec![1, 2, 3, 4],
                size_bytes: 4,
            }),
            PartialResultStatus::Success(PartialResult {
                tower_id: "tower2".to_string(),
                sequence: 1,
                data: vec![5, 6, 7, 8],
                size_bytes: 4,
            }),
        ];

        PartialResultSet::new(2, results)
    }

    #[tokio::test]
    async fn test_concatenate_strategy() {
        let aggregator = ResultAggregator::new(AggregationStrategy::Concatenate);
        let results = create_test_results();

        let aggregated = aggregator.aggregate(results).await.unwrap();
        assert_eq!(aggregated.data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(aggregated.is_complete());
    }

    #[tokio::test]
    async fn test_reduction_sum() {
        let aggregator = ResultAggregator::new(AggregationStrategy::Reduction(ReductionOp::Sum));
        let results = create_test_results();

        let aggregated = aggregator.aggregate(results).await.unwrap();
        assert!(aggregated.is_complete());
    }
}
