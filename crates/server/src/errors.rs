//! Server error types and handling
//!
//! This module provides server-specific error types that integrate with the unified
//! ToadStool error system. ServerError is a thin wrapper that maps to ToadStoolError
//! categories while providing server-specific context.

use toadstool::ToadStoolError;
use toadstool_common::error::{
    ConfigError, ExecutionError, NetworkError, ResourceError, SecurityError, SystemError,
};

/// `ToadStool` server errors
///
/// This is a server-specific error type that maps directly to ToadStoolError categories.
/// It provides backward compatibility while allowing seamless integration with the
/// unified error system.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Server initialization failed: {0}")]
    Initialization(String),

    #[error("Runtime engine error: {0}")]
    RuntimeEngine(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhaustion(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Execution failed: {0}")]
    Execution(String),

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
                    engine: "unknown".to_string(),
                    reason: msg,
                })
            }
            ServerError::ResourceExhaustion(msg) => {
                ToadStoolError::Resource(ResourceError::AllocationFailure {
                    resource: "system".to_string(),
                    reason: msg,
                })
            }
            ServerError::Authentication(msg) => {
                ToadStoolError::Security(SecurityError::AuthenticationFailed { reason: msg })
            }
            ServerError::Authorization(msg) => {
                ToadStoolError::Security(SecurityError::PermissionDenied {
                    operation: "server_operation".to_string(),
                    reason: msg,
                })
            }
            ServerError::Configuration(msg) => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: msg })
            }
            ServerError::Network(msg) => {
                ToadStoolError::Network(NetworkError::ConnectionFailed {
                    endpoint: "unknown".to_string(),
                    reason: msg,
                })
            }
            ServerError::Execution(msg) => {
                ToadStoolError::Execution(ExecutionError::WorkloadFailure {
                    workload_id: "unknown".to_string(),
                    reason: msg,
                })
            }
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
        }
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
