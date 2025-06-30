//! Error types and result handling for ToadStool
//!
//! This module defines all error types that can occur during ToadStool operations,
//! providing structured error handling across all components.


use thiserror::Error;

/// Result type alias for ToadStool operations
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;

/// Main error type for ToadStool operations
#[derive(Error, Debug, Clone)]
pub enum ToadStoolError {
    /// Configuration related errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Runtime execution errors
    #[error("Runtime error: {message}")]
    Runtime { message: String },

    /// Security and sandboxing errors
    #[error("Security error: {message}")]
    Security { message: String },

    /// Resource management errors
    #[error("Resource error: {message}")]
    Resource { message: String },

    /// Integration and ecosystem communication errors
    #[error("Integration error: {message}")]
    Integration { message: String },

    /// Network and communication errors
    #[error("Network error: {message}")]
    Network { message: String },

    /// File system and I/O errors
    #[error("IO error: {message}")]
    Io { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Timeout errors
    #[error("Timeout error: operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Not supported errors
    #[error("Not supported: {message}")]
    NotSupported { message: String },

    /// Not found errors
    #[error("Not found: {message}")]
    NotFound { message: String },

    /// Already exists errors
    #[error("Already exists: {message}")]
    AlreadyExists { message: String },

    /// Permission denied errors
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// Internal errors (should not happen in normal operation)
    #[error("Internal error: {message}")]
    Internal { message: String },

    /// External service errors
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },
}

impl ToadStoolError {
    /// Create a configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a runtime error
    pub fn runtime<S: Into<String>>(message: S) -> Self {
        Self::Runtime {
            message: message.into(),
        }
    }

    /// Create a security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::Security {
            message: message.into(),
        }
    }

    /// Create a resource error
    pub fn resource<S: Into<String>>(message: S) -> Self {
        Self::Resource {
            message: message.into(),
        }
    }

    /// Create an integration error
    pub fn integration<S: Into<String>>(message: S) -> Self {
        Self::Integration {
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create an IO error
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a not supported error
    pub fn not_supported<S: Into<String>>(message: S) -> Self {
        Self::NotSupported {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    /// Create an already exists error
    pub fn already_exists<S: Into<String>>(message: S) -> Self {
        Self::AlreadyExists {
            message: message.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied<S: Into<String>>(message: S) -> Self {
        Self::PermissionDenied {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create an external service error
    pub fn external_service<S1: Into<String>, S2: Into<String>>(service: S1, message: S2) -> Self {
        Self::ExternalService {
            service: service.into(),
            message: message.into(),
        }
    }

    /// Check if this is a transient error that might succeed on retry
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Network { .. } | Self::Timeout { .. } | Self::ExternalService { .. }
        )
    }

    /// Check if this is a configuration-related error
    pub fn is_configuration(&self) -> bool {
        matches!(self, Self::Configuration { .. })
    }

    /// Check if this is a security-related error
    pub fn is_security(&self) -> bool {
        matches!(self, Self::Security { .. } | Self::PermissionDenied { .. })
    }
}

// Standard error conversions
impl From<std::io::Error> for ToadStoolError {
    fn from(err: std::io::Error) -> Self {
        Self::io(err.to_string())
    }
}

impl From<serde_json::Error> for ToadStoolError {
    fn from(err: serde_json::Error) -> Self {
        Self::serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for ToadStoolError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::serialization(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ToadStoolError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::timeout(0) // Timeout duration unknown
    }
}

impl From<anyhow::Error> for ToadStoolError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal(err.to_string())
    }
} 