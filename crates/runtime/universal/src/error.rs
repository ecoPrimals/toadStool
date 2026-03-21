// SPDX-License-Identifier: AGPL-3.0-only
//! Universal runtime error types

use thiserror::Error;

/// Errors from the substrate layer (simplified compute interface)
#[derive(Debug, Error)]
pub enum SubstrateError {
    /// I/O error (e.g. file, network).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Runtime error.
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Other error.
    #[error("{0}")]
    Other(String),
}
