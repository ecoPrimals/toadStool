// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error type definitions for ToadStool platform
//!
//! This module contains all error enum definitions organized by domain.
//! It provides a comprehensive 3-tier error hierarchy:
//!
//! - **Tier 1**: `ToadStoolError` - Top-level error enum with high-level categories
//! - **Tier 2**: Specialized errors (`ExecutionError`, `ConfigError`, etc.)
//! - **Tier 3**: Result type aliases for convenient error handling

use std::time::Duration;
use thiserror::Error;

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

    /// Runtime execution failure (lightweight variant for direct matching)
    ///
    /// Preferred over wrapping in `Execution(ExecutionError::WorkloadFailure {...})`
    /// when the call-site only needs a string description.
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// Resource or entity not found (lightweight variant for direct matching)
    ///
    /// Preferred over wrapping in `Resource(ResourceError::NotFound {...})`
    /// when the call-site only needs a string description.
    #[error("Not found: {0}")]
    NotFound(String),
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
        /// Name of the runtime engine that failed
        runtime: String,
        /// Identifier of the workload that failed
        workload_id: String,
        /// Human-readable failure reason from the runtime
        reason: String,
    },

    /// Workload execution failed
    #[error("Workload '{workload_id}' failed: {reason}")]
    WorkloadFailure {
        /// Identifier of the workload that failed
        workload_id: String,
        /// Human-readable failure reason
        reason: String,
    },

    /// Operation timed out
    #[error("Timeout after {duration:?} for operation '{operation}'")]
    Timeout {
        /// Duration that elapsed before the timeout
        duration: Duration,
        /// Name or description of the operation that timed out
        operation: String,
    },

    /// Resources exhausted during execution
    #[error("Resource '{resource}' exhausted during execution")]
    ResourceExhaustion {
        /// Type of resource that was exhausted (e.g. memory, GPU)
        resource: String,
    },

    /// Unsupported workload type
    #[error("Workload type '{workload_type}' is not supported on this platform")]
    UnsupportedWorkloadType {
        /// The workload type that is not supported
        workload_type: String,
    },

    /// Runtime engine not available
    #[error("Runtime engine '{engine}' is not available: {reason}")]
    EngineUnavailable {
        /// Name of the runtime engine that is unavailable
        engine: String,
        /// Reason the engine cannot be used
        reason: String,
    },

    /// Invalid execution request
    #[error("Invalid execution request: {reason}")]
    InvalidRequest {
        /// Description of why the request is invalid
        reason: String,
    },
}

/// Errors related to configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    NotFound {
        /// Path to the configuration file that was not found
        path: String,
    },

    /// Configuration parsing failed
    #[error("Failed to parse configuration: {reason}")]
    ParseError {
        /// Description of the parse failure
        reason: String,
    },

    /// Configuration validation failed
    #[error("Configuration validation failed: {reason}")]
    ValidationError {
        /// Description of the validation failure
        reason: String,
    },

    /// Missing required configuration field
    #[error("Missing required configuration field: {field}")]
    MissingField {
        /// Name of the required field that is missing
        field: String,
    },

    /// Invalid configuration value
    #[error("Invalid value for '{field}': {value} ({reason})")]
    InvalidValue {
        /// Name of the configuration field
        field: String,
        /// The invalid value that was provided
        value: String,
        /// Explanation of why the value is invalid
        reason: String,
    },

    /// Configuration loading failed
    #[error("Failed to load configuration from '{config_source}': {reason}")]
    LoadError {
        /// Source from which configuration was being loaded (file path, URL, etc.)
        config_source: String,
        /// Description of the load failure
        reason: String,
    },

    /// Environment variable error
    #[error("Environment variable '{name}' error: {reason}")]
    EnvVarError {
        /// Name of the environment variable
        name: String,
        /// Description of the error (missing, invalid format, etc.)
        reason: String,
    },
}

/// Errors related to resource management
#[derive(Error, Debug)]
pub enum ResourceError {
    /// Resource allocation failed
    #[error("Failed to allocate {resource}: {reason}")]
    AllocationFailure {
        /// Type of resource that failed to allocate
        resource: String,
        /// Description of the allocation failure
        reason: String,
    },

    /// Resource limit exceeded
    #[error("Resource limit exceeded for '{resource}': requested {requested}, limit {limit}")]
    LimitExceeded {
        /// Type of resource whose limit was exceeded
        resource: String,
        /// Amount that was requested
        requested: String,
        /// Configured limit that was exceeded
        limit: String,
    },

    /// Resource not found
    #[error("Resource '{resource}' with id '{id}' not found")]
    NotFound {
        /// Type of resource that was not found
        resource: String,
        /// Identifier of the resource
        id: String,
    },

    /// Resource monitoring error
    #[error("Failed to monitor resource '{resource}': {reason}")]
    MonitoringError {
        /// Type of resource that could not be monitored
        resource: String,
        /// Description of the monitoring failure
        reason: String,
    },

    /// Insufficient resources
    #[error("Insufficient {resource} available: need {needed}, have {available}")]
    Insufficient {
        /// Type of resource that is insufficient
        resource: String,
        /// Amount required
        needed: String,
        /// Amount currently available
        available: String,
    },

    /// Resource cleanup failed
    #[error("Failed to cleanup resource '{resource}': {reason}")]
    CleanupError {
        /// Type of resource that failed to cleanup
        resource: String,
        /// Description of the cleanup failure
        reason: String,
    },
}

/// Errors related to ecosystem integration
#[derive(Error, Debug)]
pub enum IntegrationError {
    /// Service unavailable
    #[error("Service '{service}' is unavailable: {reason}")]
    ServiceUnavailable {
        /// Name or identifier of the unavailable service
        service: String,
        /// Reason the service is unavailable
        reason: String,
    },

    /// Service connection failed
    #[error("Failed to connect to service '{service}': {reason}")]
    ConnectionFailed {
        /// Name or identifier of the service
        service: String,
        /// Description of the connection failure
        reason: String,
    },

    /// Service authentication failed
    #[error("Authentication failed for service '{service}': {reason}")]
    AuthenticationFailed {
        /// Name or identifier of the service
        service: String,
        /// Description of the authentication failure
        reason: String,
    },

    /// Service operation failed
    #[error("Operation '{operation}' failed on service '{service}': {reason}")]
    OperationFailed {
        /// Name or identifier of the service
        service: String,
        /// Name of the operation that failed
        operation: String,
        /// Description of the operation failure
        reason: String,
    },

    /// Service discovery failed
    #[error("Failed to discover service '{service}': {reason}")]
    DiscoveryFailed {
        /// Name or identifier of the service that could not be discovered
        service: String,
        /// Description of the discovery failure
        reason: String,
    },

    /// Invalid service response
    #[error("Invalid response from service '{service}': {reason}")]
    InvalidResponse {
        /// Name or identifier of the service
        service: String,
        /// Description of why the response is invalid
        reason: String,
    },
}

/// Errors related to security
#[derive(Error, Debug)]
pub enum SecurityError {
    /// Permission denied
    #[error("Permission denied for '{operation}': {reason}")]
    PermissionDenied {
        /// Operation that was denied
        operation: String,
        /// Reason for the denial
        reason: String,
    },

    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed {
        /// Description of the authentication failure
        reason: String,
    },

    /// Authorization failed
    #[error("Authorization failed for '{resource}': {reason}")]
    AuthorizationFailed {
        /// Resource or action that was not authorized
        resource: String,
        /// Reason for the authorization failure
        reason: String,
    },

    /// Security policy violation
    #[error("Security policy '{policy}' violated: {reason}")]
    PolicyViolation {
        /// Name or identifier of the policy that was violated
        policy: String,
        /// Description of the violation
        reason: String,
    },

    /// Sandbox violation
    #[error("Sandbox violation: {reason}")]
    SandboxViolation {
        /// Description of the sandbox violation
        reason: String,
    },

    /// Invalid credentials
    #[error("Invalid credentials: {reason}")]
    InvalidCredentials {
        /// Description of why the credentials are invalid
        reason: String,
    },

    /// Token error
    #[error("Token error: {reason}")]
    TokenError {
        /// Description of the token error (expired, malformed, etc.)
        reason: String,
    },
}

/// Errors related to networking
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Connection failed
    #[error("Connection to '{endpoint}' failed: {reason}")]
    ConnectionFailed {
        /// Network endpoint that could not be reached
        endpoint: String,
        /// Description of the connection failure
        reason: String,
    },

    /// Connection timeout
    #[error("Connection to '{endpoint}' timed out after {duration:?}")]
    Timeout {
        /// Network endpoint that timed out
        endpoint: String,
        /// Duration that elapsed before timeout
        duration: Duration,
    },

    /// Network I/O error
    #[error("Network I/O error: {reason}")]
    IoError {
        /// Description of the I/O failure
        reason: String,
    },

    /// DNS resolution failed
    #[error("DNS resolution failed for '{hostname}': {reason}")]
    DnsError {
        /// Hostname that could not be resolved
        hostname: String,
        /// Description of the resolution failure
        reason: String,
    },

    /// Invalid endpoint
    #[error("Invalid endpoint '{endpoint}': {reason}")]
    InvalidEndpoint {
        /// The invalid endpoint string
        endpoint: String,
        /// Description of why the endpoint is invalid
        reason: String,
    },

    /// Protocol error
    #[error("Protocol error: {reason}")]
    ProtocolError {
        /// Description of the protocol violation or error
        reason: String,
    },

    /// TLS/SSL error
    #[error("TLS/SSL error: {reason}")]
    TlsError {
        /// Description of the TLS/SSL handshake or certificate error
        reason: String,
    },
}

/// Errors related to system operations
#[derive(Error, Debug)]
pub enum SystemError {
    /// I/O error
    #[error("I/O error: {reason}")]
    Io {
        /// Description of the I/O failure
        reason: String,
    },

    /// File system error
    #[error("File system error on '{path}': {reason}")]
    FileSystem {
        /// Path where the file system error occurred
        path: String,
        /// Description of the file system failure
        reason: String,
    },

    /// Platform error
    #[error("Platform error: {reason}")]
    Platform {
        /// Description of the platform-specific failure
        reason: String,
    },

    /// Process error
    #[error("Process error: {reason}")]
    Process {
        /// Description of the process failure (spawn, signal, etc.)
        reason: String,
    },

    /// Serialization error
    #[error("Serialization error: {reason}")]
    Serialization {
        /// Description of the serialization/deserialization failure
        reason: String,
    },

    /// Not supported on this platform
    #[error("'{feature}' is not supported on this platform: {reason}")]
    NotSupported {
        /// Name of the feature that is not supported
        feature: String,
        /// Reason it is not supported (e.g. missing hardware)
        reason: String,
    },

    /// Internal error
    #[error("Internal error: {reason}")]
    Internal {
        /// Description of the internal error (bugs, invariants violated)
        reason: String,
    },
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    // --- From trait: ToadStoolError from inner errors ---

    #[test]
    fn from_execution_error() {
        let inner = ExecutionError::InvalidRequest {
            reason: "bad".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Execution error:"));
        assert!(err.to_string().contains("Invalid execution request"));
    }

    #[test]
    fn from_config_error() {
        let inner = ConfigError::NotFound {
            path: "/x".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Configuration error:"));
        assert!(err.to_string().contains("/x"));
    }

    #[test]
    fn from_resource_error() {
        let inner = ResourceError::NotFound {
            resource: "foo".to_string(),
            id: "1".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Resource error:"));
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn from_integration_error() {
        let inner = IntegrationError::ServiceUnavailable {
            service: "svc".to_string(),
            reason: "down".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Integration error:"));
        assert!(err.to_string().contains("svc"));
    }

    #[test]
    fn from_security_error() {
        let inner = SecurityError::PermissionDenied {
            operation: "op".to_string(),
            reason: "nope".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Security error:"));
        assert!(err.to_string().contains("op"));
    }

    #[test]
    fn from_network_error() {
        let inner = NetworkError::IoError {
            reason: "io".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("Network error:"));
        assert!(err.to_string().contains("io"));
    }

    #[test]
    fn from_system_error() {
        let inner = SystemError::Internal {
            reason: "bug".to_string(),
        };
        let err: ToadStoolError = inner.into();
        assert!(err.to_string().contains("System error:"));
        assert!(err.to_string().contains("bug"));
    }

    // --- ToadStoolError display ---

    #[test]
    fn toadstool_error_display_includes_inner() {
        let err: ToadStoolError = ExecutionError::WorkloadFailure {
            workload_id: "w1".to_string(),
            reason: "crashed".to_string(),
        }
        .into();
        assert!(err.to_string().contains("Workload 'w1' failed"));
        assert!(err.to_string().contains("crashed"));
    }

    // --- ExecutionError display ---

    #[test]
    fn execution_error_runtime_failure_display() {
        let err = ExecutionError::RuntimeFailure {
            runtime: "wgpu".to_string(),
            workload_id: "w1".to_string(),
            reason: "OOM".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("wgpu"));
        assert!(s.contains("w1"));
        assert!(s.contains("OOM"));
    }

    #[test]
    fn execution_error_timeout_display() {
        let err = ExecutionError::Timeout {
            duration: Duration::from_secs(5),
            operation: "infer".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("5s"));
        assert!(s.contains("infer"));
    }

    #[test]
    fn execution_error_engine_unavailable_display() {
        let err = ExecutionError::EngineUnavailable {
            engine: "cuda".to_string(),
            reason: "no driver".to_string(),
        };
        assert!(err.to_string().contains("cuda"));
        assert!(err.to_string().contains("no driver"));
    }

    // --- ConfigError display ---

    #[test]
    fn config_error_not_found_display() {
        let err = ConfigError::NotFound {
            path: "/etc/app.toml".to_string(),
        };
        assert!(err.to_string().contains("/etc/app.toml"));
    }

    #[test]
    fn config_error_invalid_value_display() {
        let err = ConfigError::InvalidValue {
            field: "timeout".to_string(),
            value: "-1".to_string(),
            reason: "must be positive".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("timeout"));
        assert!(s.contains("-1"));
        assert!(s.contains("must be positive"));
    }

    // --- ResourceError display ---

    #[test]
    fn resource_error_limit_exceeded_display() {
        let err = ResourceError::LimitExceeded {
            resource: "memory".to_string(),
            requested: "16GB".to_string(),
            limit: "8GB".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("memory"));
        assert!(s.contains("16GB"));
        assert!(s.contains("8GB"));
    }

    #[test]
    fn resource_error_insufficient_display() {
        let err = ResourceError::Insufficient {
            resource: "cores".to_string(),
            needed: "8".to_string(),
            available: "4".to_string(),
        };
        assert!(err.to_string().contains("cores"));
        assert!(err.to_string().contains('8'));
        assert!(err.to_string().contains('4'));
    }

    // --- IntegrationError display ---

    #[test]
    fn integration_error_operation_failed_display() {
        let err = IntegrationError::OperationFailed {
            service: "db".to_string(),
            operation: "query".to_string(),
            reason: "timeout".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("db"));
        assert!(s.contains("query"));
        assert!(s.contains("timeout"));
    }

    #[test]
    fn integration_error_discovery_failed_display() {
        let err = IntegrationError::DiscoveryFailed {
            service: "api".to_string(),
            reason: "no hosts".to_string(),
        };
        assert!(err.to_string().contains("api"));
        assert!(err.to_string().contains("no hosts"));
    }

    // --- SecurityError display ---

    #[test]
    fn security_error_policy_violation_display() {
        let err = SecurityError::PolicyViolation {
            policy: "sanity".to_string(),
            reason: "check failed".to_string(),
        };
        assert!(err.to_string().contains("sanity"));
        assert!(err.to_string().contains("check failed"));
    }

    #[test]
    fn security_error_token_error_display() {
        let err = SecurityError::TokenError {
            reason: "expired".to_string(),
        };
        assert!(err.to_string().contains("expired"));
    }

    // --- NetworkError display ---

    #[test]
    fn network_error_connection_failed_display() {
        let err = NetworkError::ConnectionFailed {
            endpoint: "localhost:8080".to_string(),
            reason: "refused".to_string(),
        };
        assert!(err.to_string().contains("localhost:8080"));
        assert!(err.to_string().contains("refused"));
    }

    #[test]
    fn network_error_timeout_display() {
        let err = NetworkError::Timeout {
            endpoint: "example.com".to_string(),
            duration: Duration::from_secs(30),
        };
        let s = err.to_string();
        assert!(s.contains("example.com"));
        assert!(s.contains("30s"));
    }

    #[test]
    fn network_error_dns_error_display() {
        let err = NetworkError::DnsError {
            hostname: "bad.host".to_string(),
            reason: "NXDOMAIN".to_string(),
        };
        assert!(err.to_string().contains("bad.host"));
        assert!(err.to_string().contains("NXDOMAIN"));
    }

    // --- SystemError display ---

    #[test]
    fn system_error_file_system_display() {
        let err = SystemError::FileSystem {
            path: "/tmp/foo".to_string(),
            reason: "permission denied".to_string(),
        };
        assert!(err.to_string().contains("/tmp/foo"));
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn system_error_not_supported_display() {
        let err = SystemError::NotSupported {
            feature: "FHE".to_string(),
            reason: "no NPU".to_string(),
        };
        assert!(err.to_string().contains("FHE"));
        assert!(err.to_string().contains("no NPU"));
    }

    // --- ToadStoolResult type alias ---

    #[test]
    fn toadstool_result_ok() {
        let r: ToadStoolResult<i32> = Ok(42);
        assert!(r.is_ok());
        // Validate the value through match rather than unwrap
        match r {
            Ok(v) => assert_eq!(v, 42),
            Err(e) => panic!("expected Ok, got Err: {e:?}"),
        }
    }

    #[test]
    fn toadstool_result_err() {
        let r: ToadStoolResult<i32> = Err(ExecutionError::InvalidRequest {
            reason: "bad".to_string(),
        }
        .into());
        assert!(r.is_err());
        // Validate the error through match rather than unwrap_err
        match r {
            Ok(_) => panic!("expected Err"),
            Err(e) => assert!(e.to_string().contains("bad")),
        }
    }

    #[test]
    fn other_result_aliases_work() {
        let _: ExecutionResult<()> = Err(ExecutionError::InvalidRequest {
            reason: "x".to_string(),
        });
        let _: ConfigResult<()> = Err(ConfigError::MissingField {
            field: "x".to_string(),
        });
        let _: ResourceResult<()> = Err(ResourceError::AllocationFailure {
            resource: "x".to_string(),
            reason: "y".to_string(),
        });
    }

    // --- Error source / downcasting ---

    #[test]
    fn error_source_returns_inner() {
        let inner = ExecutionError::ResourceExhaustion {
            resource: "memory".to_string(),
        };
        let top: ToadStoolError = inner.into();
        let dyn_err = &top as &dyn StdError;
        let source = dyn_err.source();
        assert!(source.is_some());
        let src = source.unwrap();
        assert!(src.to_string().contains("memory"));
    }
}
