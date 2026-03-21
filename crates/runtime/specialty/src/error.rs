// SPDX-License-Identifier: AGPL-3.0-only

use toadstool::ToadStoolError;

/// Errors that can occur during specialty runtime operations.
#[derive(Debug, thiserror::Error)]
pub enum SpecialtyRuntimeError {
    /// The requested legacy system type is not supported.
    #[error("System not supported: {0}")]
    SystemNotSupported(String),

    /// The requested architecture is not supported for cross-compilation.
    #[error("Architecture not supported: {0}")]
    ArchitectureNotSupported(String),

    /// Cross-compilation or build failed.
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    /// Communication with a legacy system or emulator failed.
    #[error("Communication error: {0}")]
    CommunicationError(String),

    /// Emulation of a legacy architecture failed.
    #[error("Emulation error: {0}")]
    EmulationError(String),

    /// Invalid or inconsistent configuration.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Operation exceeded the allowed time limit.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Underlying I/O operation failed.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Catch-all for other runtime errors.
    #[error("Other error: {0}")]
    Other(String),
}

impl From<SpecialtyRuntimeError> for ToadStoolError {
    fn from(err: SpecialtyRuntimeError) -> Self {
        ToadStoolError::runtime(err.to_string())
    }
}
