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

