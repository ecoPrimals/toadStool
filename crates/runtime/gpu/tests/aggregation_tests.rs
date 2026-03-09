// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Tests for Result Aggregation with Fault Tolerance

use toadstool_runtime_gpu::aggregation::{
    fault_tolerance::{
        PartialResult, PartialResultCollector, PartialResultSet, PartialResultStatus,
        RecoveryStrategy,
    },
    merger::{MatrixChunk, MatrixMerger, ScalarReducer, VectorMerger},
    strategies::{AggregationStrategy, ReductionOp, ResultAggregator},
    AggregatedResult, AggregationMetadata,
};

// ============================================================================
// PARTIAL RESULT SET TESTS
// ============================================================================

#[test]
fn test_partial_result_set_all_success() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower2".to_string(),
            sequence: 1,
            data: vec![4, 5, 6],
            size_bytes: 3,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower3".to_string(),
            sequence: 2,
            data: vec![7, 8, 9],
            size_bytes: 3,
        }),
    ];

    let set = PartialResultSet::new(3, results);

    assert!(set.is_sufficient());
    assert_eq!(set.successful_count(), 3);
    assert_eq!(set.failed_count(), 0);
    assert!(!set.has_failures());
    assert!(set.is_complete());
}

#[test]
fn test_partial_result_set_with_failures() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Failed {
            tower_id: "tower2".to_string(),
            error: "Network timeout".to_string(),
        },
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower3".to_string(),
            sequence: 2,
            data: vec![7, 8, 9],
            size_bytes: 3,
        }),
    ];

    let set = PartialResultSet::new(3, results);

    assert!(set.is_sufficient()); // 2/3 = 67% > 50%
    assert_eq!(set.successful_count(), 2);
    assert_eq!(set.failed_count(), 1);
    assert!(set.has_failures());
    assert!(!set.is_complete());
}

#[test]
fn test_partial_result_set_insufficient() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "tower1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Failed {
            tower_id: "tower2".to_string(),
            error: "Connection lost".to_string(),
        },
        PartialResultStatus::Timeout {
            tower_id: "tower3".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "tower4".to_string(),
            error: "GPU error".to_string(),
        },
    ];

    let set = PartialResultSet::new(4, results);

    assert!(!set.is_sufficient()); // 1/4 = 25% < 50%
    assert_eq!(set.successful_count(), 1);
    assert_eq!(set.failed_count(), 3);
}

#[test]
fn test_recovery_strategy_high_success() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "t2".to_string(),
            sequence: 1,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "t3".to_string(),
            sequence: 2,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Failed {
            tower_id: "t4".to_string(),
            error: "Error".to_string(),
        },
    ];

    let set = PartialResultSet::new(4, results);
    assert_eq!(set.recovery_strategy(), RecoveryStrategy::AggregatePartial);
}

#[test]
fn test_recovery_strategy_medium_success() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "t2".to_string(),
            sequence: 1,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Failed {
            tower_id: "t3".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "t4".to_string(),
            error: "Error".to_string(),
        },
    ];

    let set = PartialResultSet::new(4, results);
    // 50% success -> RetryFailed
    assert_eq!(set.recovery_strategy(), RecoveryStrategy::RetryFailed);
}

#[test]
fn test_recovery_strategy_low_success() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Failed {
            tower_id: "t2".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "t3".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "t4".to_string(),
            error: "Error".to_string(),
        },
    ];

    let set = PartialResultSet::new(4, results);
    // 25% success -> Failover
    assert_eq!(set.recovery_strategy(), RecoveryStrategy::Failover);
}

#[test]
fn test_failure_messages() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Failed {
            tower_id: "t2".to_string(),
            error: "Network error".to_string(),
        },
        PartialResultStatus::Timeout {
            tower_id: "t3".to_string(),
        },
    ];

    let set = PartialResultSet::new(3, results);
    let messages = set.failure_messages();

    assert_eq!(messages.len(), 2);
    assert!(messages[0].contains("Tower t2 failed"));
    assert!(messages[1].contains("Tower t3 timed out"));
}

#[test]
fn test_failed_tower_ids() {
    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![],
            size_bytes: 0,
        }),
        PartialResultStatus::Failed {
            tower_id: "t2".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Timeout {
            tower_id: "t3".to_string(),
        },
    ];

    let set = PartialResultSet::new(3, results);
    let failed_ids = set.failed_tower_ids();

    assert_eq!(failed_ids.len(), 2);
    assert!(failed_ids.contains(&"t2".to_string()));
    assert!(failed_ids.contains(&"t3".to_string()));
}

// ============================================================================
// PARTIAL RESULT COLLECTOR TESTS
// ============================================================================

#[test]
fn test_collector_basic() {
    let mut collector = PartialResultCollector::new(3, std::time::Duration::from_secs(10));

    collector.add_success(PartialResult {
        tower_id: "t1".to_string(),
        sequence: 0,
        data: vec![1, 2, 3],
        size_bytes: 3,
    });

    collector.add_failure("t2".to_string(), "Error".to_string());
    collector.add_timeout("t3".to_string());

    assert!(collector.is_complete());

    let set = collector.build();
    assert_eq!(set.successful_count(), 1);
    assert_eq!(set.failed_count(), 2);
}

#[test]
fn test_collector_timeout_check() {
    // Create a collector whose start time is 100ms in the past, so any
    // non-zero timeout is already exceeded — no thread::sleep required.
    let past = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_millis(100))
        .unwrap_or_else(std::time::Instant::now);
    let collector =
        PartialResultCollector::new_with_start(3, std::time::Duration::from_millis(1), past);
    assert!(collector.is_timeout_exceeded());
}

// ============================================================================
// AGGREGATION STRATEGY TESTS
// ============================================================================

#[tokio::test]
async fn test_concatenate_strategy() {
    let aggregator = ResultAggregator::new(AggregationStrategy::Concatenate);

    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "t2".to_string(),
            sequence: 1,
            data: vec![4, 5, 6],
            size_bytes: 3,
        }),
    ];

    let set = PartialResultSet::new(2, results);
    let result = aggregator.aggregate(set).await.unwrap();

    assert_eq!(result.data, vec![1, 2, 3, 4, 5, 6]);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_concatenate_with_failures() {
    let aggregator = ResultAggregator::new(AggregationStrategy::Concatenate);

    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Failed {
            tower_id: "t2".to_string(),
            error: "Connection lost".to_string(),
        },
        PartialResultStatus::Success(PartialResult {
            tower_id: "t3".to_string(),
            sequence: 2,
            data: vec![7, 8, 9],
            size_bytes: 3,
        }),
    ];

    let set = PartialResultSet::new(3, results);
    let result = aggregator.aggregate(set).await.unwrap();

    // Should concatenate available results
    assert_eq!(result.data, vec![1, 2, 3, 7, 8, 9]);
    assert!(!result.is_complete());
    assert!(result.used_recovery());
    assert_eq!(result.warnings.len(), 1);
}

#[tokio::test]
async fn test_reduction_sum() {
    let aggregator = ResultAggregator::new(AggregationStrategy::Reduction(ReductionOp::Sum));

    // Create f32 data
    let data1: Vec<u8> = vec![1.0f32, 2.0f32, 3.0f32]
        .iter()
        .flat_map(|&f| f.to_le_bytes())
        .collect();

    let data2: Vec<u8> = vec![4.0f32, 5.0f32, 6.0f32]
        .iter()
        .flat_map(|&f| f.to_le_bytes())
        .collect();

    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: data1,
            size_bytes: 12,
        }),
        PartialResultStatus::Success(PartialResult {
            tower_id: "t2".to_string(),
            sequence: 1,
            data: data2,
            size_bytes: 12,
        }),
    ];

    let set = PartialResultSet::new(2, results);
    let result = aggregator.aggregate(set).await.unwrap();

    // Convert result back to f32
    let values: Vec<f32> = result
        .data
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    assert_eq!(values, vec![5.0, 7.0, 9.0]); // [1+4, 2+5, 3+6]
}

#[tokio::test]
async fn test_insufficient_results() {
    let aggregator = ResultAggregator::new(AggregationStrategy::Concatenate);

    let results = vec![
        PartialResultStatus::Success(PartialResult {
            tower_id: "t1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        }),
        PartialResultStatus::Failed {
            tower_id: "t2".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "t3".to_string(),
            error: "Error".to_string(),
        },
        PartialResultStatus::Failed {
            tower_id: "t4".to_string(),
            error: "Error".to_string(),
        },
    ];

    let set = PartialResultSet::new(4, results); // Only 25% success
    let result = aggregator.aggregate(set).await;

    assert!(result.is_err()); // Should fail due to insufficient results
}

// ============================================================================
// MERGER TESTS
// ============================================================================

#[test]
fn test_vector_concatenate() {
    let vectors = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];

    let result = VectorMerger::concatenate(vectors);
    assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn test_vector_add() {
    let vectors = vec![
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
        vec![7.0, 8.0, 9.0],
    ];

    let result = VectorMerger::add(vectors).unwrap();
    assert_eq!(result, vec![12.0, 15.0, 18.0]);
}

#[test]
fn test_vector_add_mismatch() {
    let vectors = vec![
        vec![1.0, 2.0],
        vec![3.0, 4.0, 5.0], // Different length
    ];

    let result = VectorMerger::add(vectors);
    assert!(result.is_err());
}

#[test]
fn test_vector_average() {
    let vectors = vec![vec![2.0, 4.0, 6.0], vec![4.0, 6.0, 8.0]];

    let result = VectorMerger::average(vectors).unwrap();
    assert_eq!(result, vec![3.0, 5.0, 7.0]);
}

#[test]
fn test_scalar_sum() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(ScalarReducer::sum(values), 15.0);
}

#[test]
fn test_scalar_min() {
    let values = vec![5.0, 2.0, 8.0, 1.0, 4.0];
    assert_eq!(ScalarReducer::min(values), Some(1.0));
}

#[test]
fn test_scalar_max() {
    let values = vec![5.0, 2.0, 8.0, 1.0, 4.0];
    assert_eq!(ScalarReducer::max(values), Some(8.0));
}

#[test]
fn test_scalar_average() {
    let values = vec![2.0, 4.0, 6.0, 8.0];
    assert_eq!(ScalarReducer::average(values), Some(5.0));
}

#[test]
fn test_scalar_product() {
    let values = vec![2.0, 3.0, 4.0];
    assert_eq!(ScalarReducer::product(values), 24.0);
}

#[test]
fn test_matrix_merger_simple() {
    let merger = MatrixMerger::new(2, 2);

    let chunks = vec![MatrixChunk {
        row_start: 0,
        row_end: 2,
        col_start: 0,
        col_end: 2,
        data: vec![1.0, 2.0, 3.0, 4.0],
    }];

    let result = merger.merge(chunks).unwrap();
    assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_matrix_merger_multiple_chunks() {
    let merger = MatrixMerger::new(2, 4);

    let chunks = vec![
        MatrixChunk {
            row_start: 0,
            row_end: 2,
            col_start: 0,
            col_end: 2,
            data: vec![1.0, 2.0, 3.0, 4.0],
        },
        MatrixChunk {
            row_start: 0,
            row_end: 2,
            col_start: 2,
            col_end: 4,
            data: vec![5.0, 6.0, 7.0, 8.0],
        },
    ];

    let result = merger.merge(chunks).unwrap();
    // Should form: [1 2 5 6]
    //              [3 4 7 8]
    assert_eq!(result, vec![1.0, 2.0, 5.0, 6.0, 3.0, 4.0, 7.0, 8.0]);
}

// ============================================================================
// AGGREGATED RESULT TESTS
// ============================================================================

#[test]
fn test_aggregated_result_completion_percentage() {
    let result = AggregatedResult {
        data: vec![],
        metadata: AggregationMetadata {
            expected_count: 10,
            successful_count: 7,
            failed_count: 3,
            aggregation_time: std::time::Duration::from_millis(100),
            recovery_strategy: Some(RecoveryStrategy::AggregatePartial),
        },
        warnings: vec![],
    };

    assert_eq!(result.completion_percentage(), 70.0);
}

#[test]
fn test_aggregated_result_zero_expected() {
    let result = AggregatedResult {
        data: vec![],
        metadata: AggregationMetadata {
            expected_count: 0,
            successful_count: 0,
            failed_count: 0,
            aggregation_time: std::time::Duration::from_millis(0),
            recovery_strategy: None,
        },
        warnings: vec![],
    };

    assert_eq!(result.completion_percentage(), 0.0);
}
