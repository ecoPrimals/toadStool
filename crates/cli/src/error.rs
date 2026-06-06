// SPDX-License-Identifier: AGPL-3.0-or-later
//! CLI error types, result alias, and context extension trait.

use thiserror::Error;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CliError {
    /// Biome not found by name or path
    #[error("Biome not found: {0}")]
    BiomeNotFound(String),

    /// Biome already exists when attempting to create
    #[error("Biome already exists: {0}")]
    BiomeAlreadyExists(String),

    /// Invalid configuration or manifest
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// I/O error during file or system operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// System or hardware error (e.g. NPU)
    #[error("System error: {0}")]
    System(String),

    /// Operation not yet implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Catch-all for other errors
    #[error("Other error: {0}")]
    Other(String),
}

impl From<base64::DecodeError> for CliError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::net::AddrParseError> for CliError {
    fn from(e: std::net::AddrParseError) -> Self {
        Self::Other(e.to_string())
    }
}

#[cfg(feature = "npu")]
impl From<akida_driver::AkidaError> for CliError {
    fn from(e: akida_driver::AkidaError) -> Self {
        Self::System(e.to_string())
    }
}

impl From<toadstool::ToadStoolError> for CliError {
    fn from(e: toadstool::ToadStoolError) -> Self {
        Self::Other(e.to_string())
    }
}

/// CLI result type alias. Use `Result<T>` for CliError, or `Result<T, E>` for other errors (e.g. serde).
pub type Result<T, E = CliError> = std::result::Result<T, E>;

/// Add context to errors (replacement for anyhow::Context)
pub trait CliContextExt<T> {
    /// Attach a context message to the error for better diagnostics
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

impl<T, E> CliContextExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| CliError::Other(format!("{context}: {e}")))
    }
}
