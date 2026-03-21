// SPDX-License-Identifier: AGPL-3.0-only
//! Fault Tolerance for Result Aggregation
//!
//! Handles partial failures in distributed GPU execution

use std::time::Instant;

/// Set of partial results with fault tolerance
pub struct PartialResultSet {
    expected_count: usize,
    results: Vec<PartialResultStatus>,
}

/// Status of a partial result
#[derive(Debug, Clone)]
pub enum PartialResultStatus {
    /// Successful result
    Success(PartialResult),

    /// Failed execution.
    Failed {
        /// Tower that failed.
        tower_id: String,
        /// Error message.
        error: String,
    },

    /// Timeout waiting for result.
    Timeout {
        /// Tower that timed out.
        tower_id: String,
    },

    /// Pending (not yet received).
    Pending {
        /// Tower ID.
        tower_id: String,
    },
}

/// A successful partial result from a tower.
#[derive(Debug, Clone)]
pub struct PartialResult {
    /// Tower that produced the result.
    pub tower_id: String,
    /// Sequence number for ordering.
    pub sequence: usize,
    /// Result data.
    pub data: Vec<u8>,
    /// Size in bytes.
    pub size_bytes: usize,
}

/// Recovery strategy for handling failures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Aggregate with available partial results (lossy but fast)
    AggregatePartial,

    /// Retry failed towers (slower but complete)
    RetryFailed,

    /// Failover to backup towers (requires spare capacity)
    Failover,

    /// Abort entire job (strict correctness)
    Abort,
}

impl PartialResultSet {
    /// Creates a new partial result set with expected count and initial results.
    pub const fn new(expected_count: usize, results: Vec<PartialResultStatus>) -> Self {
        Self {
            expected_count,
            results,
        }
    }

    /// Returns true if we have sufficient results to proceed (≥50% success).
    pub fn is_sufficient(&self) -> bool {
        let success_count = self.successful_count();
        let min_required = self.expected_count.div_ceil(2); // At least 50%
        success_count >= min_required
    }

    /// Returns successful partial results.
    pub fn successful_results(&self) -> Vec<&PartialResult> {
        self.results
            .iter()
            .filter_map(|status| match status {
                PartialResultStatus::Success(result) => Some(result),
                _ => None,
            })
            .collect()
    }

    /// Returns the count of successful results.
    pub fn successful_count(&self) -> usize {
        self.results
            .iter()
            .filter(|status| matches!(status, PartialResultStatus::Success(_)))
            .count()
    }

    /// Returns the count of failed or timed-out results.
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|status| {
                matches!(
                    status,
                    PartialResultStatus::Failed { .. } | PartialResultStatus::Timeout { .. }
                )
            })
            .count()
    }

    /// Returns the expected total result count.
    pub const fn expected_count(&self) -> usize {
        self.expected_count
    }

    /// Check if there are any failures
    pub fn has_failures(&self) -> bool {
        self.failed_count() > 0
    }

    /// Determine best recovery strategy
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        let success_rate = self.successful_count() as f32 / self.expected_count as f32;

        match success_rate {
            // >= 75% success: aggregate partial
            r if r >= 0.75 => RecoveryStrategy::AggregatePartial,

            // 50-75% success: retry failed if possible
            r if r >= 0.50 => RecoveryStrategy::RetryFailed,

            // 25-50% success: try failover
            r if r >= 0.25 => RecoveryStrategy::Failover,

            // < 25% success: abort
            _ => RecoveryStrategy::Abort,
        }
    }

    /// Get failure messages
    pub fn failure_messages(&self) -> Vec<String> {
        self.results
            .iter()
            .filter_map(|status| match status {
                PartialResultStatus::Failed { tower_id, error } => {
                    Some(format!("Tower {tower_id} failed: {error}"))
                }
                PartialResultStatus::Timeout { tower_id } => {
                    Some(format!("Tower {tower_id} timed out"))
                }
                _ => None,
            })
            .collect()
    }

    /// Get IDs of failed towers (for retry)
    pub fn failed_tower_ids(&self) -> Vec<String> {
        self.results
            .iter()
            .filter_map(|status| match status {
                PartialResultStatus::Failed { tower_id, .. } => Some(tower_id.clone()),
                PartialResultStatus::Timeout { tower_id } => Some(tower_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Check if result is complete (no failures)
    pub fn is_complete(&self) -> bool {
        self.successful_count() == self.expected_count
    }
}

/// Builder for creating partial result sets with timeout tracking
pub struct PartialResultCollector {
    expected_count: usize,
    results: Vec<PartialResultStatus>,
    timeout: std::time::Duration,
    started_at: Instant,
}

impl PartialResultCollector {
    /// Create a new collector
    pub fn new(expected_count: usize, timeout: std::time::Duration) -> Self {
        Self {
            expected_count,
            results: Vec::with_capacity(expected_count),
            timeout,
            started_at: Instant::now(),
        }
    }

    /// Create a collector with an explicit start time.
    ///
    /// Useful when reconstructing collector state from a checkpoint or when
    /// the collection window began before the collector object was created.
    pub fn new_with_start(
        expected_count: usize,
        timeout: std::time::Duration,
        started_at: Instant,
    ) -> Self {
        Self {
            expected_count,
            results: Vec::with_capacity(expected_count),
            timeout,
            started_at,
        }
    }

    /// Add a successful result
    pub fn add_success(&mut self, result: PartialResult) {
        self.results.push(PartialResultStatus::Success(result));
    }

    /// Add a failure
    pub fn add_failure(&mut self, tower_id: String, error: String) {
        self.results
            .push(PartialResultStatus::Failed { tower_id, error });
    }

    /// Add a timeout
    pub fn add_timeout(&mut self, tower_id: String) {
        self.results.push(PartialResultStatus::Timeout { tower_id });
    }

    /// Check if timeout has been exceeded
    pub fn is_timeout_exceeded(&self) -> bool {
        self.started_at.elapsed() > self.timeout
    }

    /// Check if all results received
    pub const fn is_complete(&self) -> bool {
        self.results.len() >= self.expected_count
    }

    /// Build the final result set
    pub fn build(self) -> PartialResultSet {
        PartialResultSet::new(self.expected_count, self.results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_result_set_sufficient() {
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
            PartialResultStatus::Failed {
                tower_id: "tower3".to_string(),
                error: "Connection lost".to_string(),
            },
        ];

        let set = PartialResultSet::new(3, results);

        assert!(set.is_sufficient()); // 2/3 = 67% > 50%
        assert_eq!(set.successful_count(), 2);
        assert_eq!(set.failed_count(), 1);
        assert!(set.has_failures());
        assert!(!set.is_complete());
    }

    #[test]
    fn test_recovery_strategy_selection() {
        // 80% success -> AggregatePartial
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
            PartialResultStatus::Success(PartialResult {
                tower_id: "t4".to_string(),
                sequence: 3,
                data: vec![],
                size_bytes: 0,
            }),
            PartialResultStatus::Failed {
                tower_id: "t5".to_string(),
                error: "Error".to_string(),
            },
        ];

        let set = PartialResultSet::new(5, results);
        assert_eq!(set.recovery_strategy(), RecoveryStrategy::AggregatePartial);
    }

    #[test]
    fn test_partial_result_collector() {
        let mut collector = PartialResultCollector::new(3, std::time::Duration::from_secs(10));

        collector.add_success(PartialResult {
            tower_id: "tower1".to_string(),
            sequence: 0,
            data: vec![1, 2, 3],
            size_bytes: 3,
        });

        collector.add_failure("tower2".to_string(), "Connection lost".to_string());
        collector.add_timeout("tower3".to_string());

        assert!(collector.is_complete());

        let set = collector.build();
        assert_eq!(set.successful_count(), 1);
        assert_eq!(set.failed_count(), 2);
    }
}
