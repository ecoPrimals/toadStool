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
