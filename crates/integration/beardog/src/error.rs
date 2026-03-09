// SPDX-License-Identifier: AGPL-3.0-only
//! Error types for bearDog integration

use thiserror::Error;

/// Errors from bearDog entropy integration
#[derive(Debug, Error)]
pub enum BeardogError {
    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error with context
    #[error("{0}")]
    Other(String),
}
