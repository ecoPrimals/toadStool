# ToadStool Error Code Usage Guide

## Overview

ToadStool now has a comprehensive error code system that provides structured, machine-readable error codes with human-friendly messages and remediation suggestions.

**Status**: ✅ **Fully Implemented** (November 10, 2025)

---

## Quick Start

### Basic Usage

```rust
use toadstool_common::{ToadStoolError, ToadStoolErrorExt, codes};

// Create an error with a structured code
let error = ToadStoolError::runtime("Container initialization failed")
    .with_code(codes::EXEC_RUNTIME_001);

// Error code information is embedded
assert_eq!(error.error_code_str(), Some("EXEC-RUNTIME-001"));
assert_eq!(error.category_str(), Some("execution"));
```

### In API Responses

Error codes are automatically exposed in API responses:

```json
{
  "error_code": "EXEC-RUNTIME-001",
  "message": "Runtime engine initialization failed - Container initialization failed",
  "details": {
    "category": "execution",
    "remediation": "Check runtime dependencies and configuration"
  },
  "timestamp": "2025-11-10T12:00:00Z",
  "request_id": "req-abc123"
}
```

---

## Available Error Codes

### Execution Errors (EXEC)

#### EXEC-RUNTIME-001: Runtime Engine Initialization Failed
- **Message**: Runtime engine initialization failed
- **Remediation**: Check runtime dependencies and configuration
- **Usage**:
```rust
ToadStoolError::runtime("Failed to initialize Wasmtime")
    .with_code(codes::EXEC_RUNTIME_001)
```

#### EXEC-RUNTIME-002: Runtime Execution Failure
- **Message**: Runtime execution failure
- **Remediation**: Check runtime logs for detailed error information
- **Usage**:
```rust
ToadStoolError::runtime("Container crashed during execution")
    .with_code(codes::EXEC_RUNTIME_002)
```

#### EXEC-TIMEOUT-001: Execution Timeout Exceeded
- **Message**: Execution timeout exceeded
- **Remediation**: Increase timeout limit or optimize workload
- **Usage**:
```rust
ToadStoolError::runtime("Workload exceeded 5 minute timeout")
    .with_code(codes::EXEC_TIMEOUT_001)
```

#### EXEC-VALIDATION-001: Invalid Execution Input
- **Message**: Invalid execution input or parameters
- **Remediation**: Validate input against schema requirements
- **Usage**:
```rust
ToadStoolError::runtime("Missing required field 'workload_spec'")
    .with_code(codes::EXEC_VALIDATION_001)
```

#### EXEC-NOTFOUND-001: Workload Not Found
- **Message**: Requested workload or resource not found
- **Remediation**: Verify workload ID or path exists
- **Usage**:
```rust
ToadStoolError::not_found("Workload abc-123")
    .with_code(codes::EXEC_NOTFOUND_001)
```

---

### Configuration Errors (CONFIG)

#### CONFIG-PARSE-001: Configuration Parsing Error
- **Message**: Failed to parse configuration file
- **Remediation**: Validate YAML/TOML syntax and structure
- **Usage**:
```rust
ToadStoolError::configuration("Invalid YAML at line 42")
    .with_code(codes::CONFIG_PARSE_001)
```

#### CONFIG-VALIDATE-001: Configuration Validation Failed
- **Message**: Configuration validation failed
- **Remediation**: Check configuration against schema requirements
- **Usage**:
```rust
ToadStoolError::configuration("Port must be between 1024-65535")
    .with_code(codes::CONFIG_VALIDATE_001)
```

#### CONFIG-ENV-001: Environment Variable Not Set
- **Message**: Required environment variable not set
- **Remediation**: Set required environment variables
- **Usage**:
```rust
ToadStoolError::configuration("TOADSTOOL_API_KEY not set")
    .with_code(codes::CONFIG_ENV_001)
```

#### CONFIG-FILE-001: Configuration File Not Found
- **Message**: Configuration file not found
- **Remediation**: Ensure configuration file exists at expected path
- **Usage**:
```rust
ToadStoolError::configuration("toadstool.toml not found")
    .with_code(codes::CONFIG_FILE_001)
```

---

### Resource Errors (RESOURCE)

#### RESOURCE-ALLOC-001: Resource Allocation Failure
- **Message**: Failed to allocate required resources
- **Remediation**: Free up system resources or reduce allocation requirements
- **Usage**:
```rust
ToadStoolError::resource("Failed to allocate 16GB memory")
    .with_code(codes::RESOURCE_ALLOC_001)
```

#### RESOURCE-ALLOC-002: Insufficient Memory
- **Message**: Insufficient memory available
- **Remediation**: Free memory or increase memory limit
- **Usage**:
```rust
ToadStoolError::resource("Only 2GB available, 4GB required")
    .with_code(codes::RESOURCE_ALLOC_002)
```

#### RESOURCE-LIMIT-001: Resource Limit Exceeded
- **Message**: Resource limit exceeded
- **Remediation**: Increase resource limits or optimize usage
- **Usage**:
```rust
ToadStoolError::resource("CPU usage exceeded 80% limit")
    .with_code(codes::RESOURCE_LIMIT_001)
```

#### RESOURCE-UNAVAIL-001: Resource Unavailable
- **Message**: Required resource is unavailable
- **Remediation**: Wait for resource availability or use alternative
- **Usage**:
```rust
ToadStoolError::resource("GPU device not available")
    .with_code(codes::RESOURCE_UNAVAIL_001)
```

---

### Security Errors (SECURITY)

#### SECURITY-AUTH-001: Authentication Failed
- **Message**: Authentication failed
- **Remediation**: Verify credentials and authentication configuration
- **Usage**:
```rust
ToadStoolError::security("Invalid API key")
    .with_code(codes::SECURITY_AUTH_001)
```

#### SECURITY-AUTHZ-001: Authorization Denied
- **Message**: Authorization denied - insufficient permissions
- **Remediation**: Request appropriate permissions or role
- **Usage**:
```rust
ToadStoolError::security("User lacks 'admin' role")
    .with_code(codes::SECURITY_AUTHZ_001)
```

#### SECURITY-CRYPTO-001: Cryptographic Operation Failed
- **Message**: Cryptographic operation failed
- **Remediation**: Check encryption keys and algorithms
- **Usage**:
```rust
ToadStoolError::security("Failed to decrypt data")
    .with_code(codes::SECURITY_CRYPTO_001)
```

#### SECURITY-SANDBOX-001: Sandbox Violation
- **Message**: Sandbox security policy violation
- **Remediation**: Review and comply with sandbox restrictions
- **Usage**:
```rust
ToadStoolError::security("Attempt to access /etc/passwd")
    .with_code(codes::SECURITY_SANDBOX_001)
```

---

### Network Errors (NETWORK)

#### NETWORK-CONNECT-001: Connection Failed
- **Message**: Network connection failed
- **Remediation**: Check network connectivity and firewall rules
- **Usage**:
```rust
ToadStoolError::network("Failed to connect to songbird:8080")
    .with_code(codes::NETWORK_CONNECT_001)
```

#### NETWORK-TIMEOUT-001: Network Timeout
- **Message**: Network operation timeout
- **Remediation**: Check network latency or increase timeout
- **Usage**:
```rust
ToadStoolError::network("Request timeout after 30s")
    .with_code(codes::NETWORK_TIMEOUT_001)
```

#### NETWORK-DNS-001: DNS Resolution Failed
- **Message**: DNS resolution failed
- **Remediation**: Verify hostname and DNS configuration
- **Usage**:
```rust
ToadStoolError::network("Cannot resolve example.com")
    .with_code(codes::NETWORK_DNS_001)
```

#### NETWORK-TLS-001: TLS Error
- **Message**: TLS/SSL connection error
- **Remediation**: Check certificate validity and TLS configuration
- **Usage**:
```rust
ToadStoolError::network("Certificate expired")
    .with_code(codes::NETWORK_TLS_001)
```

---

### Integration Errors (INTEGRATION)

#### INTEGRATION-CONNECT-001: Service Connection Failed
- **Message**: Failed to connect to external service
- **Remediation**: Check service availability and network connectivity
- **Usage**:
```rust
ToadStoolError::integration("Failed to connect to NestGate")
    .with_code(codes::INTEGRATION_CONNECT_001)
```

#### INTEGRATION-TIMEOUT-001: Service Timeout
- **Message**: External service request timeout
- **Remediation**: Increase timeout or check service responsiveness
- **Usage**:
```rust
ToadStoolError::integration("NestGate request timeout")
    .with_code(codes::INTEGRATION_TIMEOUT_001)
```

#### INTEGRATION-PROTO-001: Protocol Error
- **Message**: Protocol communication error
- **Remediation**: Verify protocol version compatibility
- **Usage**:
```rust
ToadStoolError::integration("Unsupported protocol version 2.0")
    .with_code(codes::INTEGRATION_PROTO_001)
```

#### INTEGRATION-VERSION-001: Version Mismatch
- **Message**: Service version mismatch
- **Remediation**: Update services to compatible versions
- **Usage**:
```rust
ToadStoolError::integration("BearDog v2.0 required, v1.5 found")
    .with_code(codes::INTEGRATION_VERSION_001)
```

---

### System Errors (SYSTEM)

#### SYSTEM-IO-001: I/O Error
- **Message**: I/O operation failed
- **Remediation**: Check file system and device availability
- **Usage**:
```rust
ToadStoolError::io("Failed to read file")
    .with_code(codes::SYSTEM_IO_001)
```

#### SYSTEM-PERM-001: Permission Denied
- **Message**: Permission denied
- **Remediation**: Check file/directory permissions
- **Usage**:
```rust
ToadStoolError::permission_denied("Cannot write to /var/log")
    .with_code(codes::SYSTEM_PERM_001)
```

#### SYSTEM-RESOURCE-001: OS Resource Error
- **Message**: Operating system resource error
- **Remediation**: Check system resource limits and availability
- **Usage**:
```rust
ToadStoolError::io("Too many open files")
    .with_code(codes::SYSTEM_RESOURCE_001)
```

#### SYSTEM-PLATFORM-001: Platform Not Supported
- **Message**: Operation not supported on this platform
- **Remediation**: Use platform-specific alternative or compatibility layer
- **Usage**:
```rust
ToadStoolError::not_supported("GPU compute not available on this platform")
    .with_code(codes::SYSTEM_PLATFORM_001)
```

---

## API Integration

### Automatic Conversion

The API automatically converts `ToadStoolErrorWithCode` to structured JSON responses:

```rust
use toadstool_common::{ToadStoolError, ToadStoolErrorExt, codes};
use toadstool_api::types::ApiError;

// In a handler
fn my_handler() -> Result<Json<Response>, ApiError> {
    let error = ToadStoolError::runtime("Container failed")
        .with_code(codes::EXEC_RUNTIME_002);
    
    // Automatic conversion to API error
    Err(error.into())
}
```

**Client receives**:
```json
{
  "error_code": "EXEC-RUNTIME-002",
  "message": "Runtime execution failure - Container failed",
  "details": {
    "category": "execution",
    "remediation": "Check runtime logs for detailed error information"
  },
  "timestamp": "2025-11-10T12:00:00Z"
}
```

---

## Best Practices

### 1. Always Use Codes for User-Facing Errors

```rust
// Good: Structured code for API responses
return Err(ToadStoolError::runtime("Initialization failed")
    .with_code(codes::EXEC_RUNTIME_001)
    .into());

// Avoid: Generic error without code
return Err(ToadStoolError::runtime("Something went wrong").into());
```

### 2. Choose the Most Specific Code

```rust
// Better: Specific code
ToadStoolError::resource("Out of memory")
    .with_code(codes::RESOURCE_ALLOC_002)  // Specific: insufficient memory

// Less helpful:
ToadStoolError::resource("Out of memory")
    .with_code(codes::RESOURCE_ALLOC_001)  // Generic: allocation failure
```

### 3. Preserve Error Context

```rust
// Wrap lower-level errors with context
let result = initialize_runtime().map_err(|e| {
    ToadStoolError::runtime(format!("Failed to initialize: {}", e))
        .with_code(codes::EXEC_RUNTIME_001)
})?;
```

### 4. Use in Tests

```rust
#[test]
fn test_error_code_in_response() {
    let error = ToadStoolError::runtime("Test error")
        .with_code(codes::EXEC_RUNTIME_001);
    
    assert_eq!(error.error_code_str(), Some("EXEC-RUNTIME-001"));
    assert!(error.remediation().is_some());
}
```

---

## Monitoring and Metrics

Error codes enable powerful monitoring:

```rust
// Example: Track errors by code
let error_code = error.error_code_str().unwrap_or("UNKNOWN");
metrics::counter!("errors_by_code", 1, "code" => error_code);
```

**Dashboard queries**:
- Count of `EXEC-*` errors (execution issues)
- Rate of `NETWORK-TIMEOUT-001` (network problems)
- Top error codes by frequency

---

## Migration from Legacy Errors

### Before (Legacy)

```rust
return Err(ToadStoolError::runtime("Failed").into());
```

### After (With Codes)

```rust
return Err(ToadStoolError::runtime("Failed")
    .with_code(codes::EXEC_RUNTIME_002)
    .into());
```

**Benefits**:
- ✅ Machine-readable error tracking
- ✅ Better client error handling
- ✅ Actionable remediation suggestions
- ✅ Improved monitoring and alerting

---

## Summary

**Total Error Codes**: 30+

| Category | Count | Prefix |
|----------|-------|--------|
| Execution | 5 | EXEC-* |
| Configuration | 4 | CONFIG-* |
| Resource | 4 | RESOURCE-* |
| Integration | 4 | INTEGRATION-* |
| Security | 4 | SECURITY-* |
| Network | 4 | NETWORK-* |
| System | 4 | SYSTEM-* |

---

## Related Documentation

- [Error Code System Design](ERROR_CODE_SYSTEM_DESIGN.md) - Architecture and implementation details
- [API Documentation](../crates/api/README.md) - API error responses
- [Error System](../crates/core/common/src/error.rs) - Core error types

---

*Last Updated: November 10, 2025*  
*ToadStool Error Code System v1.0*

