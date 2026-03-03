// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for adaptive runtime

use thiserror::Error;

/// Errors from the adaptive optimization system
#[derive(Debug, Error)]
pub enum AdaptiveError {
    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error with context
    #[error("{0}")]
    Other(String),
}
