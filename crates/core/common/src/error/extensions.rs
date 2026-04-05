// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error extensions and wrappers
//!
//! This module provides extensions like `ToadStoolErrorWithCode` that add
//! structured error codes to ToadStool errors, along with comprehensive tests.

use super::types::ToadStoolError;
#[cfg(test)]
use super::types::{
    ConfigError, ExecutionError, IntegrationError, NetworkError, ResourceError, SecurityError,
    SystemError, ToadStoolResult,
};
use crate::error_codes::ErrorCode;

// ============================================================================
// Error Code Integration (Tier 4)
// ============================================================================

/// ToadStool error enriched with a structured error code
#[derive(Debug)]
pub struct ToadStoolErrorWithCode {
    /// The underlying error
    pub error: ToadStoolError,
    /// Optional structured error code
    pub code: Option<ErrorCode>,
}

impl ToadStoolErrorWithCode {
    /// Get the error code if present
    #[must_use]
    pub const fn error_code(&self) -> Option<&ErrorCode> {
        self.code.as_ref()
    }

    /// Get the error code string if present
    #[must_use]
    pub fn error_code_str(&self) -> Option<&str> {
        self.code.as_ref().map(|c| c.code)
    }

    /// Get the error category if code is present
    #[must_use]
    pub fn category_str(&self) -> Option<&str> {
        self.code
            .as_ref()
            .map(super::super::error_codes::ErrorCode::category_str)
    }

    /// Get remediation suggestion if available
    #[must_use]
    pub fn remediation(&self) -> Option<&str> {
        self.code.as_ref().and_then(|c| c.remediation)
    }
}

impl std::fmt::Display for ToadStoolErrorWithCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(code) = &self.code {
            write!(f, "[{}] {}", code.code, self.error)
        } else {
            write!(f, "{}", self.error)
        }
    }
}

impl std::error::Error for ToadStoolErrorWithCode {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<ToadStoolError> for ToadStoolErrorWithCode {
    fn from(error: ToadStoolError) -> Self {
        Self { error, code: None }
    }
}

impl From<ToadStoolErrorWithCode> for ToadStoolError {
    fn from(error: ToadStoolErrorWithCode) -> Self {
        error.error
    }
}

/// Result type using error codes
pub type ToadStoolResultWithCode<T> = Result<T, ToadStoolErrorWithCode>;

#[cfg(test)]
mod tests {
    use super::super::context::ToadStoolErrorExt;
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_execution_error_runtime_failure() {
        let err = ExecutionError::runtime_failure("container", "workload-123", "Image not found");
        assert!(err.to_string().contains("container"));
        assert!(err.to_string().contains("workload-123"));
        assert!(err.to_string().contains("Image not found"));
    }

    #[test]
    fn test_config_error_not_found() {
        let err = ConfigError::not_found("/etc/toadstool/config.toml");
        assert!(err.to_string().contains("/etc/toadstool/config.toml"));
    }

    #[test]
    fn test_resource_error_limit_exceeded() {
        let err = ResourceError::limit_exceeded("memory", "2GB", "1GB");
        assert!(err.to_string().contains("memory"));
        assert!(err.to_string().contains("2GB"));
        assert!(err.to_string().contains("1GB"));
    }

    #[test]
    fn test_integration_error_service_unavailable() {
        let err = IntegrationError::service_unavailable("nestgate", "Connection refused");
        assert!(err.to_string().contains("nestgate"));
        assert!(err.to_string().contains("Connection refused"));
    }

    #[test]
    fn test_security_error_permission_denied() {
        let err = SecurityError::permission_denied("read file", "Insufficient permissions");
        assert!(err.to_string().contains("read file"));
        assert!(err.to_string().contains("Insufficient permissions"));
    }

    #[test]
    fn test_network_error_timeout() {
        let err = NetworkError::timeout("http://example.com", Duration::from_secs(30));
        assert!(err.to_string().contains("example.com"));
        assert!(err.to_string().contains("30s"));
    }

    #[test]
    fn test_system_error_not_supported() {
        let err = SystemError::not_supported("GPU compute", "No GPU available");
        assert!(err.to_string().contains("GPU compute"));
        assert!(err.to_string().contains("No GPU available"));
    }

    #[test]
    fn test_toadstool_error_from_execution() {
        let exec_err = ExecutionError::workload_failure("test-123", "Failed");
        let err: ToadStoolError = exec_err.into();
        assert!(err.to_string().contains("Execution error"));
    }

    #[test]
    fn test_toadstool_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ToadStoolError = io_err.into();
        assert!(err.to_string().contains("System error"));
    }

    #[test]
    fn test_result_type_ok() {
        fn returns_ok() -> String {
            "success".to_string()
        }
        assert_eq!(returns_ok(), "success");
    }

    #[test]
    fn test_result_type_err() {
        fn returns_err() -> ToadStoolResult<String> {
            Err(ExecutionError::workload_failure("test", "failed").into())
        }
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_debug() {
        let err = ExecutionError::timeout(Duration::from_secs(10), "startup");
        let debug = format!("{err:?}");
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn test_nested_error_conversion() {
        let exec_err = ExecutionError::ResourceExhaustion {
            resource: "CPU".to_string(),
        };
        let toadstool_err: ToadStoolError = exec_err.into();
        let message = toadstool_err.to_string();
        assert!(message.contains("Execution error"));
        assert!(message.contains("CPU"));
    }

    #[test]
    fn test_error_with_code() {
        use crate::error_codes::codes;

        let error = ToadStoolError::runtime("Test error").with_code(codes::EXEC_RUNTIME_001);

        assert!(error.error_code().is_some());
        assert_eq!(error.error_code_str(), Some("EXEC-RUNTIME-001"));
        assert_eq!(error.category_str(), Some("execution"));
        assert!(error.remediation().is_some());
    }

    #[test]
    fn test_error_with_code_display() {
        use crate::error_codes::codes;

        let error =
            ToadStoolError::runtime("Initialization failed").with_code(codes::EXEC_RUNTIME_001);

        let display = error.to_string();
        assert!(display.contains("EXEC-RUNTIME-001"));
        assert!(display.contains("Initialization failed"));
    }

    #[test]
    fn test_error_conversion() {
        use crate::error_codes::codes;

        let error_with_code = ToadStoolError::runtime("Test").with_code(codes::EXEC_RUNTIME_001);

        // Convert to ToadStoolError
        let plain_error: ToadStoolError = error_with_code.into();
        assert!(plain_error.to_string().contains("Test"));
    }

    #[test]
    fn test_error_without_code() {
        let error = ToadStoolError::runtime("Test error");
        let error_with_code: ToadStoolErrorWithCode = error.into();
        assert!(error_with_code.error_code().is_none());
    }

    /// Test: Convenience method - configuration error
    #[test]
    fn test_convenience_configuration() {
        let err = ToadStoolError::configuration("Invalid TOML");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("Invalid TOML"));
    }

    /// Test: Convenience method - security error
    #[test]
    fn test_convenience_security() {
        let err = ToadStoolError::security("Unauthorized access");
        assert!(err.to_string().contains("Security error"));
        assert!(err.to_string().contains("Unauthorized access"));
    }

    /// Test: Convenience method - resource error
    #[test]
    fn test_convenience_resource() {
        let err = ToadStoolError::resource("Out of memory");
        assert!(err.to_string().contains("Resource error"));
        assert!(err.to_string().contains("Out of memory"));
    }

    /// Test: Convenience method - network error
    #[test]
    fn test_convenience_network() {
        let err = ToadStoolError::network("Connection refused");
        assert!(err.to_string().contains("Network error"));
        assert!(err.to_string().contains("Connection refused"));
    }

    /// Test: Convenience method - validation error
    #[test]
    fn test_convenience_validation() {
        let err = ToadStoolError::validation("Port must be 1-65535");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("Port must be 1-65535"));
    }

    /// Test: Convenience method - `not_found` error
    #[test]
    fn test_convenience_not_found() {
        let err = ToadStoolError::not_found("workload-123");
        assert!(err.to_string().contains("Not found"));
        assert!(err.to_string().contains("workload-123"));
    }

    /// Test: Convenience method - `permission_denied` error
    #[test]
    fn test_convenience_permission_denied() {
        let err = ToadStoolError::permission_denied("Cannot write to /etc");
        assert!(err.to_string().contains("Security error"));
        assert!(err.to_string().contains("Cannot write to /etc"));
    }

    /// Test: Convenience method - `not_supported` error
    #[test]
    fn test_convenience_not_supported() {
        let err = ToadStoolError::not_supported("CUDA on ARM");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("CUDA on ARM"));
    }

    /// Test: Convenience method - timeout error
    #[test]
    fn test_convenience_timeout() {
        let err = ToadStoolError::timeout("database query");
        assert!(err.to_string().contains("Execution error"));
        assert!(err.to_string().contains("database query"));
    }

    /// Test: Error chain preserves context
    #[test]
    fn test_error_chain_context() {
        let inner_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let toadstool_err: ToadStoolError = inner_err.into();

        assert!(toadstool_err.to_string().contains("System error"));
        assert!(std::error::Error::source(&toadstool_err).is_some());
    }

    /// Test: Multiple error conversions
    #[test]
    fn test_error_conversions() {
        // ExecutionError -> ToadStoolError
        let exec_err = ExecutionError::workload_failure("test", "failed");
        let _: ToadStoolError = exec_err.into();

        // ConfigError -> ToadStoolError
        let config_err = ConfigError::not_found("config.toml");
        let _: ToadStoolError = config_err.into();

        // ResourceError -> ToadStoolError
        let resource_err = ResourceError::NotFound {
            resource: "memory".to_string(),
            id: "resource-123".to_string(),
        };
        let _: ToadStoolError = resource_err.into();
    }
}
