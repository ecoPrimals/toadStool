// SPDX-License-Identifier: AGPL-3.0-or-later
//! Server error conversion tests - calling actual production code
//!
//! These tests directly call the From trait implementations in server/src/errors.rs
//! to increase llvm-cov coverage

use toadstool::ToadStoolError;
use toadstool_common::error::{
    ConfigError, ExecutionError, NetworkError, ResourceError, SecurityError, SystemError,
};
use toadstool_server::ServerError;

// ============================================================================
// ServerError Construction Tests (calls enum constructors)
// ============================================================================

#[test]
fn test_server_error_initialization() {
    let error = ServerError::Initialization("Failed to start".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Server initialization failed"));
    assert!(msg.contains("Failed to start"));
}

#[test]
fn test_server_error_runtime_engine() {
    let error = ServerError::RuntimeEngine("Engine crashed".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Runtime engine error"));
    assert!(msg.contains("Engine crashed"));
}

#[test]
fn test_server_error_resource_exhaustion() {
    let error = ServerError::ResourceExhaustion("Out of memory".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Resource exhausted"));
    assert!(msg.contains("Out of memory"));
}

#[test]
fn test_server_error_authentication() {
    let error = ServerError::Authentication("Invalid token".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Authentication failed"));
    assert!(msg.contains("Invalid token"));
}

#[test]
fn test_server_error_authorization() {
    let error = ServerError::Authorization("Access denied".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Authorization failed"));
    assert!(msg.contains("Access denied"));
}

#[test]
fn test_server_error_configuration() {
    let error = ServerError::Configuration("Invalid config".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Invalid configuration"));
    assert!(msg.contains("Invalid config"));
}

#[test]
fn test_server_error_network() {
    let error = ServerError::Network("Connection timeout".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Network error"));
    assert!(msg.contains("Connection timeout"));
}

#[test]
fn test_server_error_execution() {
    let error = ServerError::Execution("Workload failed".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Execution failed"));
    assert!(msg.contains("Workload failed"));
}

#[test]
fn test_server_error_internal() {
    let error = ServerError::Internal("Panic occurred".to_string());
    let msg = format!("{error}");
    assert!(msg.contains("Internal server error"));
    assert!(msg.contains("Panic occurred"));
}

#[test]
fn test_server_error_debug() {
    let error = ServerError::Internal("test".to_string());
    let debug_str = format!("{error:?}");
    assert!(debug_str.contains("Internal"));
}

// ============================================================================
// ServerError → ToadStoolError Conversion Tests (calls From<ServerError>)
// ============================================================================

#[test]
fn test_conversion_initialization_to_toadstool_error() {
    // Calls From<ServerError> for ToadStoolError
    let server_error = ServerError::Initialization("init failed".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::System(SystemError::Platform { reason }) => {
            assert_eq!(reason, "init failed");
        }
        _ => panic!("Expected System::Platform error"),
    }
}

#[test]
fn test_conversion_runtime_engine_to_toadstool_error() {
    let server_error = ServerError::RuntimeEngine("engine error".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Execution(ExecutionError::EngineUnavailable { engine, reason }) => {
            assert_eq!(engine, "runtime engine (identifier not available)");
            assert_eq!(reason, "engine error");
        }
        _ => panic!("Expected Execution::EngineUnavailable error"),
    }
}

#[test]
fn test_conversion_resource_exhaustion_to_toadstool_error() {
    let server_error = ServerError::ResourceExhaustion("no memory".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Resource(ResourceError::AllocationFailure { resource, reason }) => {
            assert_eq!(resource, "system resource (type not specified)");
            assert_eq!(reason, "no memory");
        }
        _ => panic!("Expected Resource::AllocationFailure error"),
    }
}

#[test]
fn test_conversion_authentication_to_toadstool_error() {
    let server_error = ServerError::Authentication("bad auth".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Security(SecurityError::AuthenticationFailed { reason }) => {
            assert_eq!(reason, "bad auth");
        }
        _ => panic!("Expected Security::AuthenticationFailed error"),
    }
}

#[test]
fn test_conversion_authorization_to_toadstool_error() {
    let server_error = ServerError::Authorization("no permission".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Security(SecurityError::PermissionDenied { operation, reason }) => {
            assert_eq!(operation, "requested operation (not specified)");
            assert_eq!(reason, "no permission");
        }
        _ => panic!("Expected Security::PermissionDenied error"),
    }
}

#[test]
fn test_conversion_configuration_to_toadstool_error() {
    let server_error = ServerError::Configuration("bad config".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Configuration(ConfigError::ValidationError { reason }) => {
            assert_eq!(reason, "bad config");
        }
        _ => panic!("Expected Configuration::ValidationError error"),
    }
}

#[test]
fn test_conversion_network_to_toadstool_error() {
    let server_error = ServerError::Network("connection failed".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Network(NetworkError::ConnectionFailed { endpoint, reason }) => {
            assert_eq!(endpoint, "connection target (endpoint not specified)");
            assert_eq!(reason, "connection failed");
        }
        _ => panic!("Expected Network::ConnectionFailed error"),
    }
}

#[test]
fn test_conversion_execution_to_toadstool_error() {
    let server_error = ServerError::Execution("workload crash".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::Execution(ExecutionError::WorkloadFailure {
            workload_id,
            reason,
        }) => {
            assert_eq!(workload_id, "workload (identifier not available)");
            assert_eq!(reason, "workload crash");
        }
        _ => panic!("Expected Execution::WorkloadFailure error"),
    }
}

#[test]
fn test_conversion_internal_to_toadstool_error() {
    let server_error = ServerError::Internal("internal error".to_string());
    let toadstool_error: ToadStoolError = server_error.into();

    match toadstool_error {
        ToadStoolError::System(SystemError::Internal { reason }) => {
            assert_eq!(reason, "internal error");
        }
        _ => panic!("Expected System::Internal error"),
    }
}

// ============================================================================
// ToadStoolError → ServerError Conversion Tests (calls From<ToadStoolError>)
// ============================================================================

#[test]
fn test_conversion_toadstool_execution_to_server_error() {
    let toadstool_error = ToadStoolError::Execution(ExecutionError::WorkloadFailure {
        workload_id: "test-123".to_string(),
        reason: "failed".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Execution(msg) => {
            assert!(msg.contains("Execution"));
        }
        _ => panic!("Expected ServerError::Execution"),
    }
}

#[test]
fn test_conversion_toadstool_configuration_to_server_error() {
    let toadstool_error = ToadStoolError::Configuration(ConfigError::ValidationError {
        reason: "invalid".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Configuration(msg) => {
            assert!(msg.contains("Configuration"));
        }
        _ => panic!("Expected ServerError::Configuration"),
    }
}

#[test]
fn test_conversion_toadstool_resource_to_server_error() {
    let toadstool_error = ToadStoolError::Resource(ResourceError::AllocationFailure {
        resource: "cpu".to_string(),
        reason: "exhausted".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::ResourceExhaustion(msg) => {
            assert!(msg.contains("Resource"));
        }
        _ => panic!("Expected ServerError::ResourceExhaustion"),
    }
}

#[test]
fn test_conversion_toadstool_security_to_server_error() {
    let toadstool_error = ToadStoolError::Security(SecurityError::AuthenticationFailed {
        reason: "bad token".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Authentication(msg) => {
            assert!(msg.contains("Security"));
        }
        _ => panic!("Expected ServerError::Authentication"),
    }
}

#[test]
fn test_conversion_toadstool_network_to_server_error() {
    let toadstool_error = ToadStoolError::Network(NetworkError::ConnectionFailed {
        endpoint: "localhost:8080".to_string(),
        reason: "timeout".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Network(msg) => {
            assert!(msg.contains("Network"));
        }
        _ => panic!("Expected ServerError::Network"),
    }
}

#[test]
fn test_conversion_toadstool_system_to_server_error() {
    let toadstool_error = ToadStoolError::System(SystemError::Internal {
        reason: "panic".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Internal(msg) => {
            assert!(msg.contains("System"));
        }
        _ => panic!("Expected ServerError::Internal"),
    }
}

#[test]
fn test_conversion_toadstool_integration_to_server_error() {
    use toadstool_common::error::IntegrationError;

    let toadstool_error = ToadStoolError::Integration(IntegrationError::ServiceUnavailable {
        service: "songbird".to_string(),
        reason: "down".to_string(),
    });
    let server_error: ServerError = toadstool_error.into();

    match server_error {
        ServerError::Internal(msg) => {
            assert!(msg.contains("Integration"));
        }
        _ => panic!("Expected ServerError::Internal for Integration errors"),
    }
}

// ============================================================================
// Round-trip Conversion Tests
// ============================================================================

#[test]
fn test_roundtrip_server_to_toadstool_to_server() {
    let original = ServerError::Network("test error".to_string());
    let toadstool: ToadStoolError = original.into();
    let back_to_server: ServerError = toadstool.into();

    match back_to_server {
        ServerError::Network(msg) => {
            assert!(msg.contains("Network"));
        }
        _ => panic!("Round-trip failed"),
    }
}

#[test]
fn test_error_as_source() {
    use std::error::Error;

    let error = ServerError::Execution("test".to_string());
    // Calls Error trait implementation
    assert!(error.source().is_none());
}

#[test]
fn test_error_display_trait() {
    let error = ServerError::Authentication("invalid credentials".to_string());
    let display_string = format!("{error}");
    assert_eq!(display_string, "Authentication failed: invalid credentials");
}

// ============================================================================
// ServerResult Type Alias Tests
// ============================================================================

#[test]
fn test_server_result_ok() {
    use toadstool_server::ServerResult;

    let result: ServerResult<i32> = Ok(42);
    assert!(result.is_ok());
    // Extract value properly instead of unwrapping literal Ok
    if let Ok(val) = result {
        assert_eq!(val, 42);
    }
}

#[test]
fn test_server_result_err() {
    use toadstool_server::ServerResult;

    let result: ServerResult<i32> = Err(ServerError::Internal("failed".to_string()));
    assert!(result.is_err());
}

// Coverage: These tests call actual production code in errors.rs:
// - All 9 ServerError enum variants construction
// - Display trait implementation (format!)
// - Debug trait implementation
// - From<ServerError> for ToadStoolError (all 9 branches)
// - From<ToadStoolError> for ServerError (all 7 branches)
// - Error trait implementation
// - ServerResult type alias usage
