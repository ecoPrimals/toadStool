// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compute error types.

/// Compute errors
#[derive(Debug, thiserror::Error)]
pub enum ComputeError {
    /// Workload not supported by this compute unit.
    #[error("Workload not supported by this compute unit")]
    UnsupportedWorkload,

    /// Memory allocation failed.
    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),

    /// Execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Backend error.
    #[error("Backend error: {0}")]
    BackendError(String),

    /// No suitable compute unit found for workload.
    #[error("No suitable compute unit found for workload")]
    NoSuitableUnit,
}
