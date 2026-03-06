// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for Akida model operations

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Akida model operations
pub type Result<T> = std::result::Result<T, AkidaModelError>;

/// Errors that can occur during model parsing and loading
#[derive(Debug, Error)]
pub enum AkidaModelError {
    /// File not found or cannot be read
    #[error("Model file not found: {path}")]
    FileNotFound {
        /// Path that was attempted
        path: PathBuf,
    },

    /// Invalid `FlatBuffers` magic bytes
    #[error("Invalid FlatBuffers header: expected magic bytes \\x80D\\x04\\x10")]
    InvalidHeader,

    /// Unsupported model version
    #[error("Unsupported model version: {version} (expected 2.18.x)")]
    UnsupportedVersion {
        /// Version string from model
        version: String,
    },

    /// Model parsing failed
    #[error("Failed to parse model: {reason}")]
    ParseError {
        /// Reason for failure
        reason: String,
    },

    /// Invalid layer configuration
    #[error("Invalid layer: {reason}")]
    InvalidLayer {
        /// Reason for failure
        reason: String,
    },

    /// I/O error
    #[error("I/O error: {source}")]
    Io {
        /// Underlying I/O error
        #[from]
        source: std::io::Error,
    },

    /// Model loading failed
    #[error("Model loading failed: {reason}")]
    LoadingFailed {
        /// Reason for failure
        reason: String,
    },
}

impl AkidaModelError {
    /// Create a parse error
    pub fn parse_error(reason: impl Into<String>) -> Self {
        Self::ParseError {
            reason: reason.into(),
        }
    }

    /// Create an invalid layer error
    pub fn invalid_layer(reason: impl Into<String>) -> Self {
        Self::InvalidLayer {
            reason: reason.into(),
        }
    }

    /// Create a loading error
    pub fn loading_failed(reason: impl Into<String>) -> Self {
        Self::LoadingFailed {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error() {
        let err = AkidaModelError::parse_error("invalid format");
        assert!(matches!(err, AkidaModelError::ParseError { .. }));
        assert!(err.to_string().contains("invalid format"));
    }

    #[test]
    fn test_invalid_layer() {
        let err = AkidaModelError::invalid_layer("wrong dimensions");
        assert!(matches!(err, AkidaModelError::InvalidLayer { .. }));
        assert!(err.to_string().contains("wrong dimensions"));
    }

    #[test]
    fn test_loading_failed() {
        let err = AkidaModelError::loading_failed("out of memory");
        assert!(matches!(err, AkidaModelError::LoadingFailed { .. }));
        assert!(err.to_string().contains("out of memory"));
    }

    #[test]
    fn test_file_not_found_display() {
        let err = AkidaModelError::FileNotFound {
            path: PathBuf::from("/nonexistent/model.ebnf"),
        };
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn test_unsupported_version_display() {
        let err = AkidaModelError::UnsupportedVersion {
            version: "1.0".to_string(),
        };
        assert!(err.to_string().contains("Unsupported"));
        assert!(err.to_string().contains("1.0"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: AkidaModelError = io_err.into();
        assert!(matches!(err, AkidaModelError::Io { .. }));
    }
}
