//! Server error types and handling
//!
//! This module provides server-specific error types that integrate with the unified
//! ToadStool error system. ServerError is a thin wrapper that maps to ToadStoolError
//! categories while providing server-specific context.

use toadstool::ToadStoolError;
use toadstool_common::error::{
    ConfigError, ExecutionError, NetworkError, ResourceError, SecurityError, SystemError,
};

/// Placeholder strings for error conversions (when specific context is unknown)
const UNKNOWN: &str = "unknown";
const SYSTEM_RESOURCE: &str = "system";
const SERVER_OPERATION: &str = "server_operation";

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

    #[error("Not found: {0}")]
    NotFound(String),

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
                    engine: UNKNOWN.to_string(),
                    reason: msg,
                })
            }
            ServerError::ResourceExhaustion(msg) => {
                ToadStoolError::Resource(ResourceError::AllocationFailure {
                    resource: SYSTEM_RESOURCE.to_string(),
                    reason: msg,
                })
            }
            ServerError::Authentication(msg) => {
                ToadStoolError::Security(SecurityError::AuthenticationFailed { reason: msg })
            }
            ServerError::Authorization(msg) => {
                ToadStoolError::Security(SecurityError::PermissionDenied {
                    operation: SERVER_OPERATION.to_string(),
                    reason: msg,
                })
            }
            ServerError::Configuration(msg) => {
                ToadStoolError::Configuration(ConfigError::ValidationError { reason: msg })
            }
            ServerError::Network(msg) => ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: UNKNOWN.to_string(),
                reason: msg,
            }),
            ServerError::Execution(msg) => {
                ToadStoolError::Execution(ExecutionError::WorkloadFailure {
                    workload_id: UNKNOWN.to_string(),
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

pub type ServerResult<T> = Result<T, ServerError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ServerError Display Tests
    // ========================================================================

    #[test]
    fn test_initialization_error_display() {
        let error = ServerError::Initialization("failed to start".to_string());
        assert_eq!(
            error.to_string(),
            "Server initialization failed: failed to start"
        );
    }

    #[test]
    fn test_runtime_engine_error_display() {
        let error = ServerError::RuntimeEngine("engine crashed".to_string());
        assert_eq!(error.to_string(), "Runtime engine error: engine crashed");
    }

    #[test]
    fn test_resource_exhaustion_error_display() {
        let error = ServerError::ResourceExhaustion("out of memory".to_string());
        assert_eq!(error.to_string(), "Resource exhausted: out of memory");
    }

    #[test]
    fn test_authentication_error_display() {
        let error = ServerError::Authentication("invalid credentials".to_string());
        assert_eq!(
            error.to_string(),
            "Authentication failed: invalid credentials"
        );
    }

    #[test]
    fn test_authorization_error_display() {
        let error = ServerError::Authorization("insufficient permissions".to_string());
        assert_eq!(
            error.to_string(),
            "Authorization failed: insufficient permissions"
        );
    }

    #[test]
    fn test_configuration_error_display() {
        let error = ServerError::Configuration("invalid config".to_string());
        assert_eq!(error.to_string(), "Invalid configuration: invalid config");
    }

    #[test]
    fn test_network_error_display() {
        let error = ServerError::Network("connection refused".to_string());
        assert_eq!(error.to_string(), "Network error: connection refused");
    }

    #[test]
    fn test_execution_error_display() {
        let error = ServerError::Execution("workload failed".to_string());
        assert_eq!(error.to_string(), "Execution failed: workload failed");
    }

    #[test]
    fn test_not_found_error_display() {
        let error = ServerError::NotFound("workload xyz".to_string());
        assert_eq!(error.to_string(), "Not found: workload xyz");
    }

    #[test]
    fn test_internal_error_display() {
        let error = ServerError::Internal("unexpected state".to_string());
        assert_eq!(error.to_string(), "Internal server error: unexpected state");
    }

    // ========================================================================
    // ServerError → ToadStoolError Conversion Tests
    // ========================================================================

    #[test]
    fn test_initialization_to_toadstool_error() {
        let server_error = ServerError::Initialization("init failed".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::System(_) => {} // Expected
            _ => panic!("Expected System error"),
        }
    }

    #[test]
    fn test_runtime_engine_to_toadstool_error() {
        let server_error = ServerError::RuntimeEngine("engine error".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Execution(_) => {} // Expected
            _ => panic!("Expected Execution error"),
        }
    }

    #[test]
    fn test_resource_exhaustion_to_toadstool_error() {
        let server_error = ServerError::ResourceExhaustion("OOM".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Resource(_) => {} // Expected
            _ => panic!("Expected Resource error"),
        }
    }

    #[test]
    fn test_authentication_to_toadstool_error() {
        let server_error = ServerError::Authentication("auth failed".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Security(_) => {} // Expected
            _ => panic!("Expected Security error"),
        }
    }

    #[test]
    fn test_authorization_to_toadstool_error() {
        let server_error = ServerError::Authorization("no permission".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Security(_) => {} // Expected
            _ => panic!("Expected Security error"),
        }
    }

    #[test]
    fn test_configuration_to_toadstool_error() {
        let server_error = ServerError::Configuration("bad config".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Configuration(_) => {} // Expected
            _ => panic!("Expected Configuration error"),
        }
    }

    #[test]
    fn test_network_to_toadstool_error() {
        let server_error = ServerError::Network("connection lost".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Network(_) => {} // Expected
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_execution_to_toadstool_error() {
        let server_error = ServerError::Execution("exec failed".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::Execution(_) => {} // Expected
            _ => panic!("Expected Execution error"),
        }
    }

    #[test]
    fn test_internal_to_toadstool_error() {
        let server_error = ServerError::Internal("internal error".to_string());
        let toadstool_error: ToadStoolError = server_error.into();
        match toadstool_error {
            ToadStoolError::System(_) => {} // Expected
            _ => panic!("Expected System error"),
        }
    }

    // ========================================================================
    // ToadStoolError → ServerError Conversion Tests
    // ========================================================================

    #[test]
    fn test_toadstool_execution_to_server_error() {
        let toadstool_error = ToadStoolError::Execution(ExecutionError::WorkloadFailure {
            workload_id: "test".to_string(),
            reason: "failed".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Execution(_) => {} // Expected
            _ => panic!("Expected Execution error"),
        }
    }

    #[test]
    fn test_toadstool_configuration_to_server_error() {
        let toadstool_error = ToadStoolError::Configuration(ConfigError::ValidationError {
            reason: "invalid".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Configuration(_) => {} // Expected
            _ => panic!("Expected Configuration error"),
        }
    }

    #[test]
    fn test_toadstool_resource_to_server_error() {
        let toadstool_error = ToadStoolError::Resource(ResourceError::AllocationFailure {
            resource: "cpu".to_string(),
            reason: "unavailable".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::ResourceExhaustion(_) => {} // Expected
            _ => panic!("Expected ResourceExhaustion error"),
        }
    }

    #[test]
    fn test_toadstool_security_to_server_error() {
        let toadstool_error = ToadStoolError::Security(SecurityError::AuthenticationFailed {
            reason: "bad token".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Authentication(_) => {} // Expected
            _ => panic!("Expected Authentication error"),
        }
    }

    #[test]
    fn test_toadstool_network_to_server_error() {
        let toadstool_error = ToadStoolError::Network(NetworkError::ConnectionFailed {
            endpoint: "localhost".to_string(),
            reason: "refused".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Network(_) => {} // Expected
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_toadstool_system_to_server_error() {
        let toadstool_error = ToadStoolError::System(SystemError::Internal {
            reason: "panic".to_string(),
        });
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Internal(_) => {} // Expected
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_toadstool_runtime_to_server_error() {
        let toadstool_error = ToadStoolError::Runtime("task panicked".to_string());
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Execution(_) => {}
            _ => panic!("Expected Execution error"),
        }
    }

    #[test]
    fn test_toadstool_not_found_to_server_error() {
        let toadstool_error = ToadStoolError::NotFound("workload-abc".to_string());
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::NotFound(_) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_not_found_roundtrip() {
        let original = ServerError::NotFound("thing-123".to_string());
        let toadstool: ToadStoolError = original.into();
        assert!(matches!(toadstool, ToadStoolError::NotFound(_)));
        assert!(toadstool.to_string().contains("thing-123"));
    }

    #[test]
    fn test_toadstool_integration_to_server_error() {
        let toadstool_error = ToadStoolError::Integration(
            toadstool_common::error::IntegrationError::ServiceUnavailable {
                service: "test".to_string(),
                reason: "timeout".to_string(),
            },
        );
        let server_error: ServerError = toadstool_error.into();
        match server_error {
            ServerError::Internal(_) => {} // Expected (Integration maps to Internal)
            _ => panic!("Expected Internal error"),
        }
    }

    // ========================================================================
    // ServerResult Type Alias Test
    // ========================================================================

    #[test]
    fn test_server_result_ok() {
        let result: ServerResult<i32> = Ok(42);
        let Ok(value) = result else {
            panic!("Expected Ok");
        };
        assert_eq!(value, 42);
    }

    #[test]
    fn test_server_result_err() {
        let result: ServerResult<i32> = Err(ServerError::Internal("error".to_string()));
        assert!(result.is_err());
    }
}
