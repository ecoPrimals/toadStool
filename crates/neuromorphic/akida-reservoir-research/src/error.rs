// SPDX-License-Identifier: AGPL-3.0-only
//! Error types for reservoir computing research

use thiserror::Error;

/// Result type alias for reservoir research operations
pub type Result<T> = std::result::Result<T, ReservoirError>;

/// Errors that can occur during reservoir computing operations
#[derive(Debug, Error)]
pub enum ReservoirError {
    /// I/O or system error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid state or configuration
    #[error("{0}")]
    InvalidState(String),

    /// Numerical/computation error
    #[error("{0}")]
    Numerical(String),

    /// Thread/join error
    #[error("{0}")]
    Thread(String),
}
