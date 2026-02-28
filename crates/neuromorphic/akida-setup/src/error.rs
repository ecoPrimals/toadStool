//! Error types for Akida setup

use thiserror::Error;

/// Result type alias for setup operations
pub type Result<T> = std::result::Result<T, SetupError>;

/// Errors that can occur during Akida setup
#[derive(Debug, Error)]
pub enum SetupError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Glob pattern error
    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),

    /// Invalid glob pattern
    #[error("Invalid glob pattern: {0}")]
    Pattern(#[from] glob::PatternError),

    /// Setup/configuration error
    #[error("{0}")]
    Setup(String),
}
