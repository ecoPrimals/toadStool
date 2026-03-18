// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for `ToadStool` Universal Compute Platform
//!
//! This module re-exports the unified error system from `toadstool-common`.
//!
//! ## Migration to Unified Error System
//!
//! The error system has been unified in `toadstool-common::error`.
//! All error types and convenience methods are now available from the common crate.
//!
//! **Usage**:
//! ```rust,ignore
//! use toadstool::ToadStoolError;
//!
//! // Convenience methods work as before
//! let err = ToadStoolError::validation("Invalid input");
//! let err = ToadStoolError::not_found("Resource not found");
//! let err = ToadStoolError::security("Permission denied");
//!
//! // Or use specialized errors for better context
//! use toadstool::error::{ExecutionError, ConfigError};
//! let err = ExecutionError::RuntimeFailure {
//!     runtime: "container".to_string(),
//!     workload_id: "abc-123".to_string(),
//!     reason: "Image not found".to_string(),
//! };
//! ```
//!
//! ## Backward Compatibility
//!
//! All legacy error construction methods are preserved as convenience methods
//! on `ToadStoolError`. Existing code continues to work without changes.

// Re-export the unified error system from common
pub use toadstool_common::error::{
    ConfigError, ConfigResult, ExecutionError, ExecutionResult, IntegrationError,
    IntegrationResult, NetworkError, NetworkResult, ResourceError, ResourceResult, SecurityError,
    SecurityResult, SystemError, SystemResult, ToadStoolError, ToadStoolResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_error_import() {
        // Test that we can use the unified error types
        let exec_err = ExecutionError::WorkloadFailure {
            workload_id: "test-123".to_string(),
            reason: "Test failure".to_string(),
        };
        let err: ToadStoolError = exec_err.into();
        assert!(err.to_string().contains("Execution error"));
        assert!(err.to_string().contains("test-123"));
    }

    #[test]
    fn test_backward_compat_configuration() {
        let err = ToadStoolError::configuration("invalid config");
        assert!(err.to_string().contains("invalid config"));
    }

    #[test]
    fn test_backward_compat_validation() {
        let err = ToadStoolError::validation("bad value");
        assert!(err.to_string().contains("bad value"));
    }

    #[test]
    fn test_backward_compat_security() {
        let err = ToadStoolError::security("access denied");
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn test_backward_compat_resource() {
        let err = ToadStoolError::resource("out of memory");
        assert!(err.to_string().contains("out of memory"));
    }

    #[test]
    fn test_backward_compat_network() {
        let err = ToadStoolError::network("connection refused");
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_backward_compat_execution() {
        let err = ToadStoolError::execution("job failed");
        assert!(err.to_string().contains("job failed"));
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> ToadStoolResult<String> {
            Ok("success".to_string())
        }

        let result = returns_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_result_type_error() {
        fn returns_error() -> ToadStoolResult<String> {
            Err(ToadStoolError::runtime("failed"))
        }

        let result = returns_error();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ToadStoolError = io_err.into();
        assert!(err.to_string().contains("System error"));
    }

    #[test]
    fn test_error_conversion_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid").unwrap_err();
        let err: ToadStoolError = json_err.into();
        assert!(err.to_string().contains("System error"));
    }
}
