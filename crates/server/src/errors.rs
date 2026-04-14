// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server error types and handling
//!
//! This module provides server-specific error types that integrate with the unified
//! ToadStool error system. ServerError is a thin wrapper that maps to ToadStoolError
//! categories while providing server-specific context.

use toadstool::ToadStoolError;
use toadstool_common::error::{
    ConfigError, ExecutionError, NetworkError, ResourceError, SecurityError, SystemError,
};

/// Fallback strings for error conversions when specific context cannot be extracted
/// from ServerError. Used when converting ServerError → ToadStoolError; the target
/// variant requires structured fields (engine, resource, endpoint, etc.) that
/// ServerError does not carry. Chosen to produce clear, actionable error output.
const FALLBACK_ENGINE: &str = "runtime engine (identifier not available)";
const FALLBACK_RESOURCE: &str = "system resource (type not specified)";
const FALLBACK_OPERATION: &str = "requested operation (not specified)";
const FALLBACK_ENDPOINT: &str = "connection target (endpoint not specified)";
const FALLBACK_WORKLOAD: &str = "workload (identifier not available)";

/// `ToadStool` server errors
///
/// This is a server-specific error type that maps directly to ToadStoolError categories.
/// It provides backward compatibility while allowing seamless integration with the
/// unified error system.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// Server initialization failed.
    #[error("Server initialization failed: {0}")]
    Initialization(String),

    /// Runtime engine error.
    #[error("Runtime engine error: {0}")]
    RuntimeEngine(String),

    /// Resource exhausted.
    #[error("Resource exhausted: {0}")]
    ResourceExhaustion(String),

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization failed.
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    Configuration(String),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Execution failed.
    #[error("Execution failed: {0}")]
    Execution(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    Internal(String),
}

// ============================================================================
// Conversions: ServerError → ToadStoolError
// ============================================================================

impl From<ServerError> for ToadStoolError {
    fn from(error: ServerError) -> Self {
        match error {
            ServerError::Initialization(msg) => {
                ToadStoolError::System(SystemError::Platform { reason: msg })
            }
            ServerError::RuntimeEngine(msg) => {
                ToadStoolError::Execution(ExecutionError::EngineUnavailable {
                    engine: FALLBACK_ENGINE.into(),
                    reason: msg,
                })
            }
            ServerError::ResourceExhaustion(msg) => {
                ToadStoolError::Resource(ResourceError::AllocationFailure {
                    resource: FALLBACK_RESOURCE.into(),
                    reason: msg,
                })
            }
            ServerError::Authentication(msg) => {
                ToadStoolError::Security(SecurityError::AuthenticationFailed { reason: msg })
            }
            ServerError::Authorization(msg) => {
                ToadStoolError::Security(SecurityError::PermissionDenied {
                    operation: FALLBACK_OPERATION.into(),
                    reason: msg,
                })
            }
            ServerError::Configuration(msg) => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: msg })
            }
            ServerError::Network(msg) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: FALLBACK_ENDPOINT.into(),
                reason: msg,
            }),
            ServerError::Execution(msg) => {
                ToadStoolError::Execution(ExecutionError::WorkloadFailure {
                    workload_id: FALLBACK_WORKLOAD.into(),
                    reason: msg,
                })
            }
            ServerError::NotFound(msg) => ToadStoolError::NotFound(msg),
            ServerError::Internal(msg) => {
                ToadStoolError::System(SystemError::Internal { reason: msg })
            }
        }
    }
}

// ============================================================================
// Conversions: ToadStoolError → ServerError
// ============================================================================

impl From<ToadStoolError> for ServerError {
    fn from(error: ToadStoolError) -> Self {
        match error {
            ToadStoolError::Execution(_) => ServerError::Execution(error.to_string()),
            ToadStoolError::Configuration(_) => ServerError::Configuration(error.to_string()),
            ToadStoolError::Resource(_) => ServerError::ResourceExhaustion(error.to_string()),
            ToadStoolError::Security(_) => ServerError::Authentication(error.to_string()),
            ToadStoolError::Network(_) => ServerError::Network(error.to_string()),
            ToadStoolError::System(_) => ServerError::Internal(error.to_string()),
            ToadStoolError::Integration(_) => ServerError::Internal(error.to_string()),
            // Lightweight variants added in Session 24 (error_context.rs helpers)
            ToadStoolError::Runtime(_) => ServerError::Execution(error.to_string()),
            ToadStoolError::NotFound(_) => ServerError::NotFound(error.to_string()),
        }
    }
}

/// Result type alias for server operations.
pub type ServerResult<T> = Result<T, ServerError>;

#[cfg(test)]
#[path = "errors_tests.rs"]
mod tests;
