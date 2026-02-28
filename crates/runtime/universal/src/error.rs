//! Universal runtime error types

use thiserror::Error;

/// Errors from the substrate layer (simplified compute interface)
#[derive(Debug, Error)]
pub enum SubstrateError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("{0}")]
    Other(String),
}
