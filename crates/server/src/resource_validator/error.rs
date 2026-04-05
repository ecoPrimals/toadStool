// SPDX-License-Identifier: AGPL-3.0-or-later
//! Errors produced while validating resources against system capabilities.

use crate::resource_estimator::EstimationError;

/// Validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// Resource estimation failed.
    #[error("Estimation failed: {0}")]
    EstimationFailed(#[from] EstimationError),

    /// System capability query failed.
    #[error("System query failed: {0}")]
    SystemQueryFailed(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}
