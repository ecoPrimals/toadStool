// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::error::Error as StdError;

// --- From trait: ToadStoolError from inner errors ---

#[test]
fn from_execution_error() {
    let inner = ExecutionError::InvalidRequest {
        reason: "bad".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Execution error:"));
    assert!(err.to_string().contains("Invalid execution request"));
}

#[test]
fn from_config_error() {
    let inner = ConfigError::NotFound {
        path: "/x".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Configuration error:"));
    assert!(err.to_string().contains("/x"));
}

#[test]
fn from_resource_error() {
    let inner = ResourceError::NotFound {
        resource: "foo".to_string(),
        id: "1".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Resource error:"));
    assert!(err.to_string().contains("foo"));
}

#[test]
fn from_integration_error() {
    let inner = IntegrationError::ServiceUnavailable {
        service: "svc".to_string(),
        reason: "down".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Integration error:"));
    assert!(err.to_string().contains("svc"));
}

#[test]
fn from_security_error() {
    let inner = SecurityError::PermissionDenied {
        operation: "op".to_string(),
        reason: "nope".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Security error:"));
    assert!(err.to_string().contains("op"));
}

#[test]
fn from_network_error() {
    let inner = NetworkError::IoError {
        reason: "io".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("Network error:"));
    assert!(err.to_string().contains("io"));
}

#[test]
fn from_system_error() {
    let inner = SystemError::Internal {
        reason: "bug".to_string(),
    };
    let err: ToadStoolError = inner.into();
    assert!(err.to_string().contains("System error:"));
    assert!(err.to_string().contains("bug"));
}

// --- ToadStoolError display ---

#[test]
fn toadstool_error_display_includes_inner() {
    let err: ToadStoolError = ExecutionError::WorkloadFailure {
        workload_id: "w1".to_string(),
        reason: "crashed".to_string(),
    }
    .into();
    assert!(err.to_string().contains("Workload 'w1' failed"));
    assert!(err.to_string().contains("crashed"));
}

// --- ExecutionError display ---

#[test]
fn execution_error_runtime_failure_display() {
    let err = ExecutionError::RuntimeFailure {
        runtime: "wgpu".to_string(),
        workload_id: "w1".to_string(),
        reason: "OOM".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("wgpu"));
    assert!(s.contains("w1"));
    assert!(s.contains("OOM"));
}

#[test]
fn execution_error_timeout_display() {
    let err = ExecutionError::Timeout {
        duration: Duration::from_secs(5),
        operation: "infer".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("5s"));
    assert!(s.contains("infer"));
}

#[test]
fn execution_error_engine_unavailable_display() {
    let err = ExecutionError::EngineUnavailable {
        engine: "cuda".to_string(),
        reason: "no driver".to_string(),
    };
    assert!(err.to_string().contains("cuda"));
    assert!(err.to_string().contains("no driver"));
}

// --- ConfigError display ---

#[test]
fn config_error_not_found_display() {
    let err = ConfigError::NotFound {
        path: "/etc/app.toml".to_string(),
    };
    assert!(err.to_string().contains("/etc/app.toml"));
}

#[test]
fn config_error_invalid_value_display() {
    let err = ConfigError::InvalidValue {
        field: "timeout".to_string(),
        value: "-1".to_string(),
        reason: "must be positive".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("timeout"));
    assert!(s.contains("-1"));
    assert!(s.contains("must be positive"));
}

// --- ResourceError display ---

#[test]
fn resource_error_limit_exceeded_display() {
    let err = ResourceError::LimitExceeded {
        resource: "memory".to_string(),
        requested: "16GB".to_string(),
        limit: "8GB".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("memory"));
    assert!(s.contains("16GB"));
    assert!(s.contains("8GB"));
}

#[test]
fn resource_error_insufficient_display() {
    let err = ResourceError::Insufficient {
        resource: "cores".to_string(),
        needed: "8".to_string(),
        available: "4".to_string(),
    };
    assert!(err.to_string().contains("cores"));
    assert!(err.to_string().contains('8'));
    assert!(err.to_string().contains('4'));
}

// --- IntegrationError display ---

#[test]
fn integration_error_operation_failed_display() {
    let err = IntegrationError::OperationFailed {
        service: "db".to_string(),
        operation: "query".to_string(),
        reason: "timeout".to_string(),
    };
    let s = err.to_string();
    assert!(s.contains("db"));
    assert!(s.contains("query"));
    assert!(s.contains("timeout"));
}

#[test]
fn integration_error_discovery_failed_display() {
    let err = IntegrationError::DiscoveryFailed {
        service: "api".to_string(),
        reason: "no hosts".to_string(),
    };
    assert!(err.to_string().contains("api"));
    assert!(err.to_string().contains("no hosts"));
}

// --- SecurityError display ---

#[test]
fn security_error_policy_violation_display() {
    let err = SecurityError::PolicyViolation {
        policy: "sanity".to_string(),
        reason: "check failed".to_string(),
    };
    assert!(err.to_string().contains("sanity"));
    assert!(err.to_string().contains("check failed"));
}

#[test]
fn security_error_token_error_display() {
    let err = SecurityError::TokenError {
        reason: "expired".to_string(),
    };
    assert!(err.to_string().contains("expired"));
}

// --- NetworkError display ---

#[test]
fn network_error_connection_failed_display() {
    let err = NetworkError::ConnectionFailed {
        endpoint: "localhost:8080".to_string(),
        reason: "refused".to_string(),
    };
    assert!(err.to_string().contains("localhost:8080"));
    assert!(err.to_string().contains("refused"));
}

#[test]
fn network_error_timeout_display() {
    let err = NetworkError::Timeout {
        endpoint: "example.com".to_string(),
        duration: Duration::from_secs(30),
    };
    let s = err.to_string();
    assert!(s.contains("example.com"));
    assert!(s.contains("30s"));
}

#[test]
fn network_error_dns_error_display() {
    let err = NetworkError::DnsError {
        hostname: "bad.host".to_string(),
        reason: "NXDOMAIN".to_string(),
    };
    assert!(err.to_string().contains("bad.host"));
    assert!(err.to_string().contains("NXDOMAIN"));
}

// --- SystemError display ---

#[test]
fn system_error_file_system_display() {
    let err = SystemError::FileSystem {
        path: "/tmp/foo".to_string(),
        reason: "permission denied".to_string(),
    };
    assert!(err.to_string().contains("/tmp/foo"));
    assert!(err.to_string().contains("permission denied"));
}

#[test]
fn system_error_not_supported_display() {
    let err = SystemError::NotSupported {
        feature: "FHE".to_string(),
        reason: "no NPU".to_string(),
    };
    assert!(err.to_string().contains("FHE"));
    assert!(err.to_string().contains("no NPU"));
}

// --- ToadStoolResult type alias ---

#[test]
fn toadstool_result_ok() {
    let r: ToadStoolResult<i32> = Ok(42);
    assert!(r.is_ok());
    // Validate the value through match rather than unwrap
    match r {
        Ok(v) => assert_eq!(v, 42),
        Err(e) => panic!("expected Ok, got Err: {e:?}"),
    }
}

#[test]
fn toadstool_result_err() {
    let r: ToadStoolResult<i32> = Err(ExecutionError::InvalidRequest {
        reason: "bad".to_string(),
    }
    .into());
    assert!(r.is_err());
    // Validate the error through match rather than unwrap_err
    match r {
        Ok(_) => panic!("expected Err"),
        Err(e) => assert!(e.to_string().contains("bad")),
    }
}

#[test]
fn other_result_aliases_work() {
    let _: ExecutionResult<()> = Err(ExecutionError::InvalidRequest {
        reason: "x".to_string(),
    });
    let _: ConfigResult<()> = Err(ConfigError::MissingField {
        field: "x".to_string(),
    });
    let _: ResourceResult<()> = Err(ResourceError::AllocationFailure {
        resource: "x".to_string(),
        reason: "y".to_string(),
    });
}

// --- Error source / downcasting ---

#[test]
fn error_source_returns_inner() {
    let inner = ExecutionError::ResourceExhaustion {
        resource: "memory".to_string(),
    };
    let top: ToadStoolError = inner.into();
    let dyn_err = &top as &dyn StdError;
    let source = dyn_err.source();
    assert!(source.is_some());
    let src = source.unwrap();
    assert!(src.to_string().contains("memory"));
}
