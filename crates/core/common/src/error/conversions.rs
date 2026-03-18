// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type conversions from standard library and external types
//!
//! This module provides `From` implementations to convert common error types
//! from the standard library and external crates into ToadStool error types.

use super::types::{ExecutionError, SystemError, ToadStoolError};

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

// ============================================================================
// Tokio Error Conversions
// ============================================================================

impl From<tokio::time::error::Elapsed> for ToadStoolError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ExecutionError::Timeout {
            duration: std::time::Duration::from_secs(0),
            operation: err.to_string(),
        }
        .into()
    }
}

impl From<tokio::task::JoinError> for ToadStoolError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Runtime(format!("Task join failed: {err}"))
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for ToadStoolError {
    fn from(err: tokio::sync::broadcast::error::RecvError) -> Self {
        Self::Runtime(format!("Broadcast receive failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_io_error_to_toadstool_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let toadstool_err: ToadStoolError = io_err.into();

        assert!(toadstool_err.to_string().contains("file not found"));
        assert!(matches!(toadstool_err, ToadStoolError::System(_)));
    }

    #[test]
    fn test_from_serde_json_error_to_toadstool_error() {
        let invalid_json = "invalid json {{{";
        let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
        let serde_err = result.unwrap_err();
        let toadstool_err: ToadStoolError = serde_err.into();

        assert!(matches!(toadstool_err, ToadStoolError::System(_)));
    }

    #[test]
    fn test_from_io_error_to_system_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let system_err: SystemError = io_err.into();

        assert!(matches!(system_err, SystemError::Io { .. }));
        assert!(system_err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_from_serde_json_error_to_system_error() {
        let invalid_json = r#"{"invalid": }"#;
        let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
        let serde_err = result.unwrap_err();
        let system_err: SystemError = serde_err.into();

        assert!(matches!(system_err, SystemError::Serialization { .. }));
    }
}
