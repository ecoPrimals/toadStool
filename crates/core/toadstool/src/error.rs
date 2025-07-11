//! Error types for ToadStool Universal Compute Platform

use thiserror::Error;

/// Main error type for ToadStool operations
#[derive(Error, Debug)]
pub enum ToadStoolError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Runtime error: {message}")]
    Runtime { message: String },
    
    #[error("Security error: {message}")]
    Security { message: String },
    
    #[error("Resource error: {message}")]
    Resource { message: String },
    
    #[error("Network error: {message}")]
    Network { message: String },
    
    #[error("IO error: {message}")]
    Io { message: String },
    
    #[error("Validation error: {message}")]
    Validation { message: String },
    
    #[error("Not found: {message}")]
    NotFound { message: String },
    
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },
    
    #[error("Not supported: {message}")]
    NotSupported { message: String },
    
    #[error("Timeout: {message}")]
    Timeout { message: String },
    
    #[error("Parsing error: {message}")]
    Parsing { message: String },
    
    #[error("Ecosystem error: {message}")]
    Ecosystem { message: String },
    
    #[error("BiomeOS error: {message}")]
    BiomeOS { message: String },
    
    #[error("OS Layer error: {message}")]
    OSLayer { message: String },
    
    #[error("Execution error: {message}")]
    Execution { message: String },
    
    #[error("Other error: {message}")]
    Other { message: String },
    
    #[error("Integration error: {message}")]
    Integration { message: String },
}

/// Result type for ToadStool operations
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;

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
    
    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }
    
    /// Create a not found error
    pub fn not_found<S: Into<String>>(message: S) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }
    
    /// Create a permission denied error
    pub fn permission_denied<S: Into<String>>(message: S) -> Self {
        Self::PermissionDenied {
            message: message.into(),
        }
    }
    
    /// Create a not supported error
    pub fn not_supported<S: Into<String>>(message: S) -> Self {
        Self::NotSupported {
            message: message.into(),
        }
    }
    
    /// Create a timeout error
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }
    
    /// Create a parsing error
    pub fn parsing<S: Into<String>>(message: S) -> Self {
        Self::Parsing {
            message: message.into(),
        }
    }
    
    /// Create an ecosystem error
    pub fn ecosystem<S: Into<String>>(message: S) -> Self {
        Self::Ecosystem {
            message: message.into(),
        }
    }
    
    /// Create a biomeOS error
    pub fn biomeos<S: Into<String>>(message: S) -> Self {
        Self::BiomeOS {
            message: message.into(),
        }
    }
    
    /// Create an OS layer error
    pub fn os_layer<S: Into<String>>(message: S) -> Self {
        Self::OSLayer {
            message: message.into(),
        }
    }
    
    /// Create an execution error
    pub fn execution<S: Into<String>>(message: S) -> Self {
        Self::Execution {
            message: message.into(),
        }
    }
    
    /// Create an other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
    
    /// Create an integration error
    pub fn integration<S: Into<String>>(message: S) -> Self {
        Self::Integration {
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for ToadStoolError {
    fn from(err: std::io::Error) -> Self {
        Self::io(err.to_string())
    }
}

impl From<serde_json::Error> for ToadStoolError {
    fn from(err: serde_json::Error) -> Self {
        Self::parsing(err.to_string())
    }
}

#[cfg(feature = "networking")]
impl From<reqwest::Error> for ToadStoolError {
    fn from(err: reqwest::Error) -> Self {
        Self::network(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ToadStoolError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::timeout(err.to_string())
    }
}
