//! Error context helpers and builders
//!
//! This module provides convenience methods for creating errors with
//! contextual information. These methods are useful for backward compatibility
//! and for creating errors quickly without specifying all fields.

use super::types::*;
use std::time::Duration;

use crate::error_codes::ErrorCode;

// ============================================================================
// Convenience Methods on ToadStoolError for Backward Compatibility
// ============================================================================

impl ToadStoolError {
    /// Create a configuration error (convenience method)
    ///
    /// Delegates to `ConfigError::ValidationError`
    pub fn configuration(message: impl Into<String>) -> Self {
        ConfigError::ValidationError {
            reason: message.into(),
        }
        .into()
    }

    /// Create a runtime error (convenience method)
    ///
    /// Delegates to `ExecutionError::WorkloadFailure`
    pub fn runtime(message: impl Into<String>) -> Self {
        ExecutionError::WorkloadFailure {
            workload_id: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a security error (convenience method)
    ///
    /// Delegates to `SecurityError::PermissionDenied`
    pub fn security(message: impl Into<String>) -> Self {
        SecurityError::PermissionDenied {
            operation: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a resource error (convenience method)
    ///
    /// Delegates to `ResourceError::AllocationFailure`
    pub fn resource(message: impl Into<String>) -> Self {
        ResourceError::AllocationFailure {
            resource: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a network error (convenience method)
    ///
    /// Delegates to `NetworkError::ConnectionFailed`
    pub fn network(message: impl Into<String>) -> Self {
        NetworkError::ConnectionFailed {
            endpoint: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create an IO error (convenience method)
    ///
    /// Delegates to `SystemError::Io`
    pub fn io(message: impl Into<String>) -> Self {
        SystemError::Io {
            reason: message.into(),
        }
        .into()
    }

    /// Create a validation error (convenience method)
    ///
    /// Delegates to `ConfigError::ValidationError`
    pub fn validation(message: impl Into<String>) -> Self {
        ConfigError::ValidationError {
            reason: message.into(),
        }
        .into()
    }

    /// Create a not found error (convenience method)
    ///
    /// Delegates to `ResourceError::NotFound`
    pub fn not_found(message: impl Into<String>) -> Self {
        ResourceError::NotFound {
            resource: "unknown".to_string(),
            id: message.into(),
        }
        .into()
    }

    /// Create a permission denied error (convenience method)
    ///
    /// Delegates to `SecurityError::PermissionDenied`
    pub fn permission_denied(message: impl Into<String>) -> Self {
        SecurityError::PermissionDenied {
            operation: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a not supported error (convenience method)
    ///
    /// Delegates to `SystemError::NotSupported`
    pub fn not_supported(message: impl Into<String>) -> Self {
        SystemError::NotSupported {
            feature: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a timeout error (convenience method)
    ///
    /// Delegates to `ExecutionError::Timeout`
    pub fn timeout(message: impl Into<String>) -> Self {
        ExecutionError::Timeout {
            duration: Duration::from_secs(0),
            operation: message.into(),
        }
        .into()
    }

    /// Create a parsing error (convenience method)
    ///
    /// Delegates to `SystemError::Serialization`
    pub fn parsing(message: impl Into<String>) -> Self {
        SystemError::Serialization {
            reason: message.into(),
        }
        .into()
    }

    /// Create an ecosystem error (convenience method)
    ///
    /// Delegates to `IntegrationError::ServiceUnavailable`
    pub fn ecosystem(message: impl Into<String>) -> Self {
        IntegrationError::ServiceUnavailable {
            service: "ecosystem".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a biomeOS error (convenience method)
    ///
    /// Delegates to `IntegrationError::ServiceUnavailable`
    pub fn biomeos(message: impl Into<String>) -> Self {
        IntegrationError::ServiceUnavailable {
            service: "biomeos".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create an OS layer error (convenience method)
    ///
    /// Delegates to `SystemError::Platform`
    pub fn os_layer(message: impl Into<String>) -> Self {
        SystemError::Platform {
            reason: message.into(),
        }
        .into()
    }

    /// Create an execution error (convenience method)
    ///
    /// Delegates to `ExecutionError::WorkloadFailure`
    pub fn execution(message: impl Into<String>) -> Self {
        ExecutionError::WorkloadFailure {
            workload_id: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create an other/internal error (convenience method)
    ///
    /// Delegates to `SystemError::Internal`
    pub fn other(message: impl Into<String>) -> Self {
        SystemError::Internal {
            reason: message.into(),
        }
        .into()
    }

    /// Create an integration error (convenience method)
    ///
    /// Delegates to `IntegrationError::ServiceUnavailable`
    pub fn integration(message: impl Into<String>) -> Self {
        IntegrationError::ServiceUnavailable {
            service: "unknown".to_string(),
            reason: message.into(),
        }
        .into()
    }

    /// Create a deployment error (convenience method)
    ///
    /// Delegates to `ExecutionError::WorkloadFailure`
    pub fn deployment(message: impl Into<String>) -> Self {
        ExecutionError::WorkloadFailure {
            workload_id: "deployment".to_string(),
            reason: message.into(),
        }
        .into()
    }
}

// ============================================================================
// Error Code Integration (Tier 4)
// ============================================================================

/// Extension trait for attaching error codes to ToadStool errors
pub trait ToadStoolErrorExt: Sized {
    /// Attach an error code to this error
    fn with_code(self, code: ErrorCode) -> super::extensions::ToadStoolErrorWithCode;
}

impl ToadStoolErrorExt for ToadStoolError {
    fn with_code(self, code: ErrorCode) -> super::extensions::ToadStoolErrorWithCode {
        super::extensions::ToadStoolErrorWithCode {
            error: self,
            code: Some(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_codes::codes;

    #[test]
    fn test_configuration_empty_message() {
        let err = ToadStoolError::configuration("");
        let s = err.to_string();
        assert!(s.contains("Configuration error"));
    }

    #[test]
    fn test_configuration_long_message() {
        let msg = "x".repeat(1000);
        let err = ToadStoolError::configuration(&msg);
        assert!(err.to_string().contains("Configuration error"));
    }

    #[test]
    fn test_runtime() {
        let err = ToadStoolError::runtime("workload failed");
        let s = err.to_string();
        assert!(s.contains("Execution error"));
        assert!(s.contains("workload failed"));
    }

    #[test]
    fn test_security() {
        let err = ToadStoolError::security("access denied");
        assert!(err.to_string().contains("Security error"));
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn test_resource() {
        let err = ToadStoolError::resource("memory allocation");
        assert!(err.to_string().contains("Resource error"));
        assert!(err.to_string().contains("memory allocation"));
    }

    #[test]
    fn test_network() {
        let err = ToadStoolError::network("connection refused");
        assert!(err.to_string().contains("Network error"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_io() {
        let err = ToadStoolError::io("disk full");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_validation() {
        let err = ToadStoolError::validation("invalid port");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("invalid port"));
    }

    #[test]
    fn test_not_found() {
        let err = ToadStoolError::not_found("resource-123");
        assert!(err.to_string().contains("Resource error"));
        assert!(err.to_string().contains("resource-123"));
    }

    #[test]
    fn test_permission_denied() {
        let err = ToadStoolError::permission_denied("write to /etc");
        assert!(err.to_string().contains("Security error"));
        assert!(err.to_string().contains("write to /etc"));
    }

    #[test]
    fn test_not_supported() {
        let err = ToadStoolError::not_supported("CUDA");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("CUDA"));
    }

    #[test]
    fn test_timeout() {
        let err = ToadStoolError::timeout("db query");
        assert!(err.to_string().contains("Execution error"));
        assert!(err.to_string().contains("db query"));
    }

    #[test]
    fn test_parsing() {
        let err = ToadStoolError::parsing("malformed JSON");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("malformed JSON"));
    }

    #[test]
    fn test_ecosystem() {
        let err = ToadStoolError::ecosystem("nestgate unavailable");
        assert!(err.to_string().contains("Integration error"));
        assert!(err.to_string().contains("nestgate unavailable"));
    }

    #[test]
    fn test_biomeos() {
        let err = ToadStoolError::biomeos("biomeos down");
        assert!(err.to_string().contains("Integration error"));
        assert!(err.to_string().contains("biomeos down"));
    }

    #[test]
    fn test_os_layer() {
        let err = ToadStoolError::os_layer("platform error");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("platform error"));
    }

    #[test]
    fn test_execution() {
        let err = ToadStoolError::execution("workload crash");
        assert!(err.to_string().contains("Execution error"));
        assert!(err.to_string().contains("workload crash"));
    }

    #[test]
    fn test_other() {
        let err = ToadStoolError::other("internal error");
        assert!(err.to_string().contains("System error"));
        assert!(err.to_string().contains("internal error"));
    }

    #[test]
    fn test_integration() {
        let err = ToadStoolError::integration("service unavailable");
        assert!(err.to_string().contains("Integration error"));
        assert!(err.to_string().contains("service unavailable"));
    }

    #[test]
    fn test_deployment() {
        let err = ToadStoolError::deployment("deploy failed");
        assert!(err.to_string().contains("Execution error"));
        assert!(err.to_string().contains("deploy failed"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = ToadStoolError::runtime("test");
        let debug = format!("{err:?}");
        assert!(debug.contains("Execution"));
    }

    #[test]
    fn test_error_display_impl() {
        let err = ToadStoolError::configuration("config error");
        let display = err.to_string();
        assert!(!display.is_empty());
        assert!(display.contains("config error"));
    }

    #[test]
    fn test_with_code_builder_pattern() {
        let err = ToadStoolError::runtime("test").with_code(codes::EXEC_RUNTIME_001);
        assert!(err.error_code().is_some());
        assert_eq!(err.error_code_str(), Some("EXEC-RUNTIME-001"));
    }

    #[test]
    fn test_with_code_empty_message() {
        let err = ToadStoolError::configuration("").with_code(codes::CONFIG_PARSE_001);
        assert!(err.error_code().is_some());
    }

    #[test]
    fn test_convenience_accepts_string() {
        let msg: String = "owned string".to_string();
        let err = ToadStoolError::runtime(msg);
        assert!(err.to_string().contains("owned string"));
    }

    #[test]
    fn test_convenience_accepts_str() {
        let err = ToadStoolError::runtime("borrowed str");
        assert!(err.to_string().contains("borrowed str"));
    }
}
