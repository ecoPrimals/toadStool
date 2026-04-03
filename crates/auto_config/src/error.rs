// SPDX-License-Identifier: AGPL-3.0-only

/// Result type for auto-configuration operations.
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;

/// Errors that can occur during auto-configuration.
#[derive(Debug, thiserror::Error)]
pub enum ToadStoolError {
    /// Configuration validation or application failed.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Hardware detection failed.
    #[error("Hardware detection error: {0}")]
    Hardware(String),

    /// Network discovery or connectivity failed.
    #[error("Network error: {0}")]
    Network(String),

    /// Ecosystem service discovery failed.
    #[error("Ecosystem discovery error: {0}")]
    EcosystemDiscovery(String),

    /// I/O error during config file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing failed.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// External HTTP not supported; use Songbird for external HTTP.
    #[error("External HTTP not supported - use Songbird for external HTTP")]
    ExternalHttpNotSupported,

    /// Other auto-configuration error.
    #[error("Other error: {0}")]
    Other(String),
}

impl ToadStoolError {
    /// Creates a configuration error.
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration(message.into())
    }

    /// Creates a hardware detection error.
    pub fn hardware<S: Into<String>>(message: S) -> Self {
        Self::Hardware(message.into())
    }

    /// Creates a network error.
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network(message.into())
    }

    /// Creates an ecosystem discovery error.
    pub fn ecosystem_discovery<S: Into<String>>(message: S) -> Self {
        Self::EcosystemDiscovery(message.into())
    }

    /// Creates an other error.
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
