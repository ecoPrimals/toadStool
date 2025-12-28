//! Type conversions from standard library and external types
//!
//! This module provides `From` implementations to convert common error types
//! from the standard library and external crates into ToadStool error types.

use super::types::{SystemError, ToadStoolError};

// ============================================================================
// Standard Error Conversions
// ============================================================================

impl From<std::io::Error> for ToadStoolError {
    fn from(err: std::io::Error) -> Self {
        SystemError::Io {
            reason: err.to_string(),
        }
        .into()
    }
}

impl From<serde_json::Error> for ToadStoolError {
    fn from(err: serde_json::Error) -> Self {
        SystemError::Serialization {
            reason: err.to_string(),
        }
        .into()
    }
}

impl From<std::io::Error> for SystemError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            reason: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for SystemError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            reason: err.to_string(),
        }
    }
}
