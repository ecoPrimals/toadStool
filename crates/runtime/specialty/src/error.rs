// SPDX-License-Identifier: AGPL-3.0-only

use toadstool::ToadStoolError;

#[derive(Debug, thiserror::Error)]
pub enum SpecialtyRuntimeError {
    #[error("System not supported: {0}")]
    SystemNotSupported(String),

    #[error("Architecture not supported: {0}")]
    ArchitectureNotSupported(String),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Emulation error: {0}")]
    EmulationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<SpecialtyRuntimeError> for ToadStoolError {
    fn from(err: SpecialtyRuntimeError) -> Self {
        ToadStoolError::runtime(err.to_string())
    }
}
