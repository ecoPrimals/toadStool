// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

/// Distributed execution retry configuration
///
/// Domain-specific retry configuration for distributed workload execution.
/// Includes execution-specific retry conditions and backoff strategies.
///
/// For simple retry logic, use `toadstool::config_bases::RetryConfig` instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Backoff strategy for retries
    pub backoff_strategy: BackoffStrategy,
    /// Conditions that trigger retries
    pub retry_conditions: Vec<RetryCondition>,
}

impl Default for DistributedRetryConfig {
    fn default() -> Self {
        const DEFAULT_BACKOFF_BASE_MS: u64 = 1_000;
        const DEFAULT_BACKOFF_MAX_MS: u64 = 30_000;
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                base_ms: DEFAULT_BACKOFF_BASE_MS,
                max_ms: DEFAULT_BACKOFF_MAX_MS,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::ResourceUnavailable,
                RetryCondition::TemporaryFailure,
            ],
        }
    }
}

/// Backoff strategies for retry logic in distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed {
        /// Delay in milliseconds.
        delay_ms: u64,
    },
    /// Linear increase: initial + n * increment.
    Linear {
        /// Initial delay in ms.
        initial_ms: u64,
        /// Increment per retry in ms.
        increment_ms: u64,
    },
    /// Exponential backoff with base and max.
    Exponential {
        /// Base delay in ms.
        base_ms: u64,
        /// Max delay in ms.
        max_ms: u64,
    },
    /// Exponential backoff with jitter to avoid thundering herd.
    ExponentialJittered {
        /// Base delay in ms.
        base_ms: u64,
        /// Max delay in ms.
        max_ms: u64,
    },
}

/// Conditions that trigger job retry in distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// Network connectivity or timeout error.
    NetworkError,
    /// Resource (CPU, memory, GPU) temporarily unavailable.
    ResourceUnavailable,
    /// Generic transient failure.
    TemporaryFailure,
    /// Remote service returned 503 or similar.
    ServiceUnavailable,
    /// Custom condition for extensibility.
    Custom(String),
}
