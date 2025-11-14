//! # Unified Error System for ToadStool Platform
//!
//! This module provides a comprehensive, hierarchical error system for the entire ToadStool platform.
//! It consolidates all error types into a cohesive 3-tier architecture:
//!
//! - **Tier 1**: `ToadStoolError` - Top-level error enum with high-level categories
//! - **Tier 2**: Specialized errors (`ExecutionError`, `ConfigError`, etc.) - Domain-specific errors
//! - **Tier 3**: Result type aliases for convenient error handling
//!
//! ## Design Principles
//!
//! 1. **Single Source of Truth**: All ToadStool errors flow through this module
//! 2. **Proper Error Chaining**: Errors preserve context through the call stack
//! 3. **Clear Categorization**: Errors are organized by domain (execution, config, resource, etc.)
//! 4. **Rich Context**: Errors include relevant information for debugging
//! 5. **Easy Conversion**: Automatic conversions from common error types
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use toadstool_common::error::{ToadStoolError, ToadStoolResult, ExecutionError};
//!
//! fn execute_workload(id: &str) -> ToadStoolResult<String> {
//!     // Use the error system
//!     Err(ToadStoolError::execution(ExecutionError::RuntimeFailure {
//!         runtime: "container".to_string(),
//!         workload_id: id.to_string(),
//!         reason: "Image not found".to_string(),
//!     }))
//! }
//! ```

use std::time::Duration;
use thiserror::Error;

use crate::error_codes::ErrorCode;

// ============================================================================
// Tier 1: Top-Level Error Enum
// ============================================================================

/// Top-level error type for all ToadStool operations
///
/// This is the primary error type that all ToadStool functions should return.
/// It categorizes errors into high-level domains and wraps specialized error types.
#[derive(Error, Debug)]
pub enum ToadStoolError {
    /// Errors related to workload execution
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    /// Errors related to configuration loading, validation, and management
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),

    /// Errors related to resource allocation, monitoring, and management
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    /// Errors related to integration with ecosystem services
    #[error("Integration error: {0}")]
    Integration(#[from] IntegrationError),

    /// Errors related to security, authentication, and authorization
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    /// Errors related to networking and communication
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Errors related to system-level operations
    #[error("System error: {0}")]
    System(#[from] SystemError),
}

// ============================================================================
// Tier 2: Specialized Domain Errors
// ============================================================================

/// Errors related to workload execution
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// Runtime engine failed to execute workload
    #[error("Runtime '{runtime}' failed for workload '{workload_id}': {reason}")]
    RuntimeFailure {
        runtime: String,
        workload_id: String,
        reason: String,
    },

    /// Workload execution failed
    #[error("Workload '{workload_id}' failed: {reason}")]
    WorkloadFailure { workload_id: String, reason: String },

    /// Operation timed out
    #[error("Timeout after {duration:?} for operation '{operation}'")]
    Timeout {
        duration: Duration,
        operation: String,
    },

    /// Resources exhausted during execution
    #[error("Resource '{resource}' exhausted during execution")]
    ResourceExhaustion { resource: String },

    /// Unsupported workload type
    #[error("Workload type '{workload_type}' is not supported on this platform")]
    UnsupportedWorkloadType { workload_type: String },

    /// Runtime engine not available
    #[error("Runtime engine '{engine}' is not available: {reason}")]
    EngineUnavailable { engine: String, reason: String },

    /// Invalid execution request
    #[error("Invalid execution request: {reason}")]
    InvalidRequest { reason: String },
}

/// Errors related to configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    NotFound { path: String },

    /// Configuration parsing failed
    #[error("Failed to parse configuration: {reason}")]
    ParseError { reason: String },

    /// Configuration validation failed
    #[error("Configuration validation failed: {reason}")]
    ValidationError { reason: String },

    /// Missing required configuration field
    #[error("Missing required configuration field: {field}")]
    MissingField { field: String },

    /// Invalid configuration value
    #[error("Invalid value for '{field}': {value} ({reason})")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    /// Configuration loading failed
    #[error("Failed to load configuration from '{config_source}': {reason}")]
    LoadError {
        config_source: String,
        reason: String,
    },

    /// Environment variable error
    #[error("Environment variable '{name}' error: {reason}")]
    EnvVarError { name: String, reason: String },
}

/// Errors related to resource management
#[derive(Error, Debug)]
pub enum ResourceError {
    /// Resource allocation failed
    #[error("Failed to allocate {resource}: {reason}")]
    AllocationFailure { resource: String, reason: String },

    /// Resource limit exceeded
    #[error("Resource limit exceeded for '{resource}': requested {requested}, limit {limit}")]
    LimitExceeded {
        resource: String,
        requested: String,
        limit: String,
    },

    /// Resource not found
    #[error("Resource '{resource}' with id '{id}' not found")]
    NotFound { resource: String, id: String },

    /// Resource monitoring error
    #[error("Failed to monitor resource '{resource}': {reason}")]
    MonitoringError { resource: String, reason: String },

    /// Insufficient resources
    #[error("Insufficient {resource} available: need {needed}, have {available}")]
    Insufficient {
        resource: String,
        needed: String,
        available: String,
    },

    /// Resource cleanup failed
    #[error("Failed to cleanup resource '{resource}': {reason}")]
    CleanupError { resource: String, reason: String },
}

/// Errors related to ecosystem integration
#[derive(Error, Debug)]
pub enum IntegrationError {
    /// Service unavailable
    #[error("Service '{service}' is unavailable: {reason}")]
    ServiceUnavailable { service: String, reason: String },

    /// Service connection failed
    #[error("Failed to connect to service '{service}': {reason}")]
    ConnectionFailed { service: String, reason: String },

    /// Service authentication failed
    #[error("Authentication failed for service '{service}': {reason}")]
    AuthenticationFailed { service: String, reason: String },

    /// Service operation failed
    #[error("Operation '{operation}' failed on service '{service}': {reason}")]
    OperationFailed {
        service: String,
        operation: String,
        reason: String,
    },

    /// Service discovery failed
    #[error("Failed to discover service '{service}': {reason}")]
    DiscoveryFailed { service: String, reason: String },

    /// Invalid service response
    #[error("Invalid response from service '{service}': {reason}")]
    InvalidResponse { service: String, reason: String },
}

/// Errors related to security
#[derive(Error, Debug)]
pub enum SecurityError {
    /// Permission denied
    #[error("Permission denied for '{operation}': {reason}")]
    PermissionDenied { operation: String, reason: String },

    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    /// Authorization failed
    #[error("Authorization failed for '{resource}': {reason}")]
    AuthorizationFailed { resource: String, reason: String },

    /// Security policy violation
    #[error("Security policy '{policy}' violated: {reason}")]
    PolicyViolation { policy: String, reason: String },

    /// Sandbox violation
    #[error("Sandbox violation: {reason}")]
    SandboxViolation { reason: String },

    /// Invalid credentials
    #[error("Invalid credentials: {reason}")]
    InvalidCredentials { reason: String },

    /// Token error
    #[error("Token error: {reason}")]
    TokenError { reason: String },
}

/// Errors related to networking
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Connection failed
    #[error("Connection to '{endpoint}' failed: {reason}")]
    ConnectionFailed { endpoint: String, reason: String },

    /// Connection timeout
    #[error("Connection to '{endpoint}' timed out after {duration:?}")]
    Timeout {
        endpoint: String,
        duration: Duration,
    },

    /// Network I/O error
    #[error("Network I/O error: {reason}")]
    IoError { reason: String },

    /// DNS resolution failed
    #[error("DNS resolution failed for '{hostname}': {reason}")]
    DnsError { hostname: String, reason: String },

    /// Invalid endpoint
    #[error("Invalid endpoint '{endpoint}': {reason}")]
    InvalidEndpoint { endpoint: String, reason: String },

    /// Protocol error
    #[error("Protocol error: {reason}")]
    ProtocolError { reason: String },

    /// TLS/SSL error
    #[error("TLS/SSL error: {reason}")]
    TlsError { reason: String },
}

/// Errors related to system operations
#[derive(Error, Debug)]
pub enum SystemError {
    /// I/O error
    #[error("I/O error: {reason}")]
    Io { reason: String },

    /// File system error
    #[error("File system error on '{path}': {reason}")]
    FileSystem { path: String, reason: String },

    /// Platform error
    #[error("Platform error: {reason}")]
    Platform { reason: String },

    /// Process error
    #[error("Process error: {reason}")]
    Process { reason: String },

    /// Serialization error
    #[error("Serialization error: {reason}")]
    Serialization { reason: String },

    /// Not supported on this platform
    #[error("'{feature}' is not supported on this platform: {reason}")]
    NotSupported { feature: String, reason: String },

    /// Internal error
    #[error("Internal error: {reason}")]
    Internal { reason: String },
}

// ============================================================================
// Tier 3: Result Type Aliases
// ============================================================================

/// Result type for ToadStool operations
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;

/// Result type for execution operations
pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Result type for resource operations
pub type ResourceResult<T> = Result<T, ResourceError>;

/// Result type for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;

/// Result type for system operations
pub type SystemResult<T> = Result<T, SystemError>;

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
        SystemError::Io {
            reason: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for SystemError {
    fn from(err: serde_json::Error) -> Self {
        SystemError::Serialization {
            reason: err.to_string(),
        }
    }
}

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
    fn with_code(self, code: ErrorCode) -> ToadStoolErrorWithCode;
}

impl ToadStoolErrorExt for ToadStoolError {
    fn with_code(self, code: ErrorCode) -> ToadStoolErrorWithCode {
        ToadStoolErrorWithCode {
            error: self,
            code: Some(code),
        }
    }
}

/// ToadStool error enriched with a structured error code
#[derive(Debug)]
pub struct ToadStoolErrorWithCode {
    pub error: ToadStoolError,
    pub code: Option<ErrorCode>,
}

impl ToadStoolErrorWithCode {
    /// Get the error code if present
    pub fn error_code(&self) -> Option<&ErrorCode> {
        self.code.as_ref()
    }

    /// Get the error code string if present
    pub fn error_code_str(&self) -> Option<&str> {
        self.code.as_ref().map(|c| c.code)
    }

    /// Get the error category if code is present
    pub fn category_str(&self) -> Option<&str> {
        self.code.as_ref().map(|c| c.category_str())
    }

    /// Get remediation suggestion if available
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
    use super::*;

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
        fn returns_ok() -> ToadStoolResult<String> {
            Ok("success".to_string())
        }
        assert!(returns_ok().is_ok());
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
        let debug = format!("{:?}", err);
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
}
