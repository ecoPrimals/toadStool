// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for bearDog integration

use thiserror::Error;

/// Errors from bearDog entropy integration
#[derive(Debug, Error)]
pub enum SecurityError {
    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error with context
    #[error("{0}")]
    Other(String),
}
