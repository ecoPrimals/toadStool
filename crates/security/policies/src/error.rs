// SPDX-License-Identifier: AGPL-3.0-only
//! Policy error types

use std::path::PathBuf;

/// Policy-related errors.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// I/O error during policy file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Policy violation detected.
    #[error("policy violation: {0}")]
    Violation(String),

    /// Invalid policy configuration.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Policy validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Policy file not found at the given path.
    #[error("policy file not found: {0}")]
    PolicyNotFound(PathBuf),

    /// Failed to parse policy file.
    #[error("failed to parse policy: {0}")]
    ParseError(String),

    /// Other policy-related error.
    #[error("{0}")]
    Other(String),
}
