//! Helper constructors for creating errors ergonomically
//!
//! This module provides convenient constructor methods for error types,
//! allowing easy creation with type inference and `Into` conversions.

use super::types::*;
use std::time::Duration;

// ============================================================================
// Helper Functions for Common Patterns
// ============================================================================

impl ExecutionError {
    /// Create a runtime failure error
    pub fn runtime_failure(
        runtime: impl Into<String>,
        workload_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::RuntimeFailure {
            runtime: runtime.into(),
            workload_id: workload_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a workload failure error
    pub fn workload_failure(workload_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::WorkloadFailure {
            workload_id: workload_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(duration: Duration, operation: impl Into<String>) -> Self {
        Self::Timeout {
            duration,
            operation: operation.into(),
        }
    }
}

impl ConfigError {
    /// Create a not found error
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }

    /// Create a parse error
    pub fn parse_error(reason: impl Into<String>) -> Self {
        Self::ParseError {
            reason: reason.into(),
        }
    }

    /// Create a validation error
    pub fn validation_error(reason: impl Into<String>) -> Self {
        Self::ValidationError {
            reason: reason.into(),
        }
    }
}

impl ResourceError {
    /// Create an allocation failure error
    pub fn allocation_failure(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AllocationFailure {
            resource: resource.into(),
            reason: reason.into(),
        }
    }

    /// Create a limit exceeded error
    pub fn limit_exceeded(
        resource: impl Into<String>,
        requested: impl Into<String>,
        limit: impl Into<String>,
    ) -> Self {
        Self::LimitExceeded {
            resource: resource.into(),
            requested: requested.into(),
            limit: limit.into(),
        }
    }
}

impl IntegrationError {
    /// Create a service unavailable error
    pub fn service_unavailable(service: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            reason: reason.into(),
        }
    }

    /// Create a connection failed error
    pub fn connection_failed(service: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            service: service.into(),
            reason: reason.into(),
        }
    }
}

impl SecurityError {
    /// Create a permission denied error
    pub fn permission_denied(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a policy violation error
    pub fn policy_violation(policy: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PolicyViolation {
            policy: policy.into(),
            reason: reason.into(),
        }
    }
}

impl NetworkError {
    /// Create a connection failed error
    pub fn connection_failed(endpoint: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            endpoint: endpoint.into(),
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(endpoint: impl Into<String>, duration: Duration) -> Self {
        Self::Timeout {
            endpoint: endpoint.into(),
            duration,
        }
    }
}

impl SystemError {
    /// Create an I/O error
    pub fn io(reason: impl Into<String>) -> Self {
        Self::Io {
            reason: reason.into(),
        }
    }

    /// Create a file system error
    pub fn file_system(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::FileSystem {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a not supported error
    pub fn not_supported(feature: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::NotSupported {
            feature: feature.into(),
            reason: reason.into(),
        }
    }
}

