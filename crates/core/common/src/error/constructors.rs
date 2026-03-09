// SPDX-License-Identifier: AGPL-3.0-only
//! Helper constructors for creating errors ergonomically
//!
//! This module provides convenient constructor methods for error types,
//! allowing easy creation with type inference and `Into` conversions.

use super::types::{
    ConfigError, ExecutionError, IntegrationError, NetworkError, ResourceError, SecurityError,
    SystemError,
};
use std::time::Duration;

// ============================================================================
// Helper Functions for Common Patterns
// ============================================================================

impl ExecutionError {
    /// Create a runtime failure error
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }

    /// Create a parse error
    #[must_use]
    pub fn parse_error(reason: impl Into<String>) -> Self {
        Self::ParseError {
            reason: reason.into(),
        }
    }

    /// Create a validation error
    #[must_use]
    pub fn validation_error(reason: impl Into<String>) -> Self {
        Self::ValidationError {
            reason: reason.into(),
        }
    }
}

impl ResourceError {
    /// Create an allocation failure error
    #[must_use]
    pub fn allocation_failure(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AllocationFailure {
            resource: resource.into(),
            reason: reason.into(),
        }
    }

    /// Create a limit exceeded error
    #[must_use]
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
    #[must_use]
    pub fn service_unavailable(service: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            reason: reason.into(),
        }
    }

    /// Create a connection failed error
    #[must_use]
    pub fn connection_failed(service: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            service: service.into(),
            reason: reason.into(),
        }
    }
}

impl SecurityError {
    /// Create a permission denied error
    #[must_use]
    pub fn permission_denied(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a policy violation error
    #[must_use]
    pub fn policy_violation(policy: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PolicyViolation {
            policy: policy.into(),
            reason: reason.into(),
        }
    }
}

impl NetworkError {
    /// Create a connection failed error
    #[must_use]
    pub fn connection_failed(endpoint: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            endpoint: endpoint.into(),
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    #[must_use]
    pub fn timeout(endpoint: impl Into<String>, duration: Duration) -> Self {
        Self::Timeout {
            endpoint: endpoint.into(),
            duration,
        }
    }
}

impl SystemError {
    /// Create an I/O error
    #[must_use]
    pub fn io(reason: impl Into<String>) -> Self {
        Self::Io {
            reason: reason.into(),
        }
    }

    /// Create a file system error
    #[must_use]
    pub fn file_system(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::FileSystem {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a not supported error
    #[must_use]
    pub fn not_supported(feature: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::NotSupported {
            feature: feature.into(),
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_error_runtime_failure() {
        let err = ExecutionError::runtime_failure("container", "wk-1", "image not found");
        assert!(matches!(err, ExecutionError::RuntimeFailure { .. }));
        assert!(err.to_string().contains("container"));
        assert!(err.to_string().contains("wk-1"));
        assert!(err.to_string().contains("image not found"));
    }

    #[test]
    fn test_execution_error_workload_failure() {
        let err = ExecutionError::workload_failure("wk-2", "oom killed");
        assert!(matches!(err, ExecutionError::WorkloadFailure { .. }));
        assert!(err.to_string().contains("wk-2"));
        assert!(err.to_string().contains("oom killed"));
    }

    #[test]
    fn test_execution_error_timeout() {
        let err = ExecutionError::timeout(Duration::from_secs(30), "fetch manifest");
        assert!(matches!(err, ExecutionError::Timeout { .. }));
        assert!(err.to_string().contains("Timeout"));
        assert!(err.to_string().contains("fetch manifest"));
    }

    #[test]
    fn test_config_error_not_found() {
        let err = ConfigError::not_found("/etc/toadstool/config.toml");
        assert!(matches!(err, ConfigError::NotFound { .. }));
        assert!(err.to_string().contains("/etc/toadstool/config.toml"));
    }

    #[test]
    fn test_config_error_parse_error() {
        let err = ConfigError::parse_error("invalid TOML syntax");
        assert!(matches!(err, ConfigError::ParseError { .. }));
        assert!(err.to_string().contains("invalid TOML syntax"));
    }

    #[test]
    fn test_config_error_validation_error() {
        let err = ConfigError::validation_error("port must be positive");
        assert!(matches!(err, ConfigError::ValidationError { .. }));
        assert!(err.to_string().contains("port must be positive"));
    }

    #[test]
    fn test_resource_error_allocation_failure() {
        let err = ResourceError::allocation_failure("GPU", "out of memory");
        assert!(matches!(err, ResourceError::AllocationFailure { .. }));
        assert!(err.to_string().contains("GPU"));
        assert!(err.to_string().contains("out of memory"));
    }

    #[test]
    fn test_resource_error_limit_exceeded() {
        let err = ResourceError::limit_exceeded("memory", "16GB", "8GB");
        assert!(matches!(err, ResourceError::LimitExceeded { .. }));
        assert!(err.to_string().contains("memory"));
        assert!(err.to_string().contains("16GB"));
        assert!(err.to_string().contains("8GB"));
    }

    #[test]
    fn test_integration_error_service_unavailable() {
        let err = IntegrationError::service_unavailable("beardog", "connection refused");
        assert!(matches!(err, IntegrationError::ServiceUnavailable { .. }));
        assert!(err.to_string().contains("beardog"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_integration_error_connection_failed() {
        let err = IntegrationError::connection_failed("songbird", "timeout");
        assert!(matches!(err, IntegrationError::ConnectionFailed { .. }));
        assert!(err.to_string().contains("songbird"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_security_error_permission_denied() {
        let err = SecurityError::permission_denied("read file", "not in allowlist");
        assert!(matches!(err, SecurityError::PermissionDenied { .. }));
        assert!(err.to_string().contains("read file"));
        assert!(err.to_string().contains("not in allowlist"));
    }

    #[test]
    fn test_security_error_policy_violation() {
        let err = SecurityError::policy_violation("sandbox", "exec not allowed");
        assert!(matches!(err, SecurityError::PolicyViolation { .. }));
        assert!(err.to_string().contains("sandbox"));
        assert!(err.to_string().contains("exec not allowed"));
    }

    #[test]
    fn test_network_error_connection_failed() {
        let err = NetworkError::connection_failed("localhost:8080", "connection refused");
        assert!(matches!(err, NetworkError::ConnectionFailed { .. }));
        assert!(err.to_string().contains("localhost:8080"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn test_network_error_timeout() {
        let err = NetworkError::timeout("example.com:443", Duration::from_secs(5));
        assert!(matches!(err, NetworkError::Timeout { .. }));
        assert!(err.to_string().contains("example.com:443"));
        assert!(err.to_string().contains("5s"));
    }

    #[test]
    fn test_system_error_io() {
        let err = SystemError::io("disk full");
        assert!(matches!(err, SystemError::Io { .. }));
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_system_error_file_system() {
        let err = SystemError::file_system("/data/file", "permission denied");
        assert!(matches!(err, SystemError::FileSystem { .. }));
        assert!(err.to_string().contains("/data/file"));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_system_error_not_supported() {
        let err = SystemError::not_supported("FHE", "CPU only");
        assert!(matches!(err, SystemError::NotSupported { .. }));
        assert!(err.to_string().contains("FHE"));
        assert!(err.to_string().contains("CPU only"));
    }

    #[test]
    fn test_constructor_into_conversions() {
        let err = ExecutionError::runtime_failure(String::from("k8s"), "wk".to_string(), "failed");
        assert!(matches!(err, ExecutionError::RuntimeFailure { .. }));
    }
}
