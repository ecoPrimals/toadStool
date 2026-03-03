// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy error types

use std::path::PathBuf;

/// Policy-related errors
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("policy violation: {0}")]
    Violation(String),

    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("policy file not found: {0}")]
    PolicyNotFound(PathBuf),

    #[error("failed to parse policy: {0}")]
    ParseError(String),

    #[error("{0}")]
    Other(String),
}
