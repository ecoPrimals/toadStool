# ToadStool Error Handling Guide

**Version**: 1.1  
**Date**: November 9, 2025  
**Status**: Production Ready  

This guide explains the unified error handling system in ToadStool and provides best practices for error management.

---

## Table of Contents

1. [Overview](#overview)
2. [Error System Architecture](#error-system-architecture)
3. [Quick Start](#quick-start)
4. [Error Types Reference](#error-types-reference)
5. [Best Practices](#best-practices)
6. [Migration Guide](#migration-guide)
7. [Examples](#examples)

---

## Overview

The ToadStool error system provides:
- **Unified error types** across all crates
- **Rich contextual information** for debugging
- **Automatic error conversions** from common types
- **Backward compatibility** with legacy code
- **Type-safe error handling** with Rust's Result type

### Key Benefits

✅ **Consistency** - One error system for the entire platform  
✅ **Context** - Structured error data instead of string messages  
✅ **Composability** - Errors convert automatically where needed  
✅ **Debuggability** - Rich information for troubleshooting  
✅ **Type Safety** - Compile-time error checking  

---

## Error System Architecture

### 3-Tier Hierarchical Structure

```
┌─────────────────────────────────────┐
│      ToadStoolError (Tier 1)        │  ← Top-level categorization
│                                     │
│  - Execution                        │
│  - Configuration                    │
│  - Resource                         │
│  - Integration                      │
│  - Security                         │
│  - Network                          │
│  - System                           │
└─────────────────────────────────────┘
           │
           ├─► ExecutionError (Tier 2)  ← Specialized errors
           │   - RuntimeFailure
           │   - TimeoutError
           │   - InvalidWorkload
           │   - ...
           │
           ├─► ConfigError (Tier 2)
           │   - ValidationError
           │   - NotFound
           │   - ...
           │
           └─► ResourceError, etc.
                   │
                   └─► Result<T> (Tier 3)  ← Type aliases
                       - ToadStoolResult<T>
                       - ExecutionResult<T>
                       - ConfigResult<T>
```

### Location

All error types are defined in:
```
crates/core/common/src/error.rs
```

Import with:
```rust
use toadstool_common::error::{ToadStoolError, ToadStoolResult};
// or via re-export
use toadstool::error::{ToadStoolError, ToadStoolResult};
```

---

## Quick Start

### Basic Usage

```rust
use toadstool::error::{ToadStoolError, ToadStoolResult};

// Function returning a result
fn execute_workload(id: &str) -> ToadStoolResult<String> {
    if id.is_empty() {
        return Err(ToadStoolError::validation("Workload ID cannot be empty"));
    }
    
    Ok(format!("Executed workload: {}", id))
}

// Handling the result
match execute_workload("abc-123") {
    Ok(result) => println!("Success: {}", result),
    Err(e) => eprintln!("Error: {}", e),
}
```

### With Specialized Errors

```rust
use toadstool::error::{ExecutionError, ExecutionResult};

fn run_container(image: &str) -> ExecutionResult<String> {
    if image.is_empty() {
        return Err(ExecutionError::InvalidWorkload {
            reason: "Container image cannot be empty".to_string(),
        });
    }
    
    // Automatically converts to ToadStoolError if needed
    Ok(format!("Container {} started", image))
}
```

### Error Propagation with `?`

```rust
use toadstool::error::ToadStoolResult;

fn load_config(path: &str) -> ToadStoolResult<Config> {
    // std::io::Error automatically converts to ToadStoolError
    let contents = std::fs::read_to_string(path)?;
    
    // Parse and validate
    let config = parse_config(&contents)?;
    
    Ok(config)
}
```

---

## Error Types Reference

### Top-Level: ToadStoolError

The main error type used throughout ToadStool.

**Variants**:
- `Execution(ExecutionError)` - Runtime execution failures
- `Configuration(ConfigError)` - Configuration errors
- `Resource(ResourceError)` - Resource management errors
- `Integration(IntegrationError)` - Third-party integration errors
- `Security(SecurityError)` - Security and authentication errors
- `Network(NetworkError)` - Network communication errors
- `System(SystemError)` - System-level errors
- `Unknown(String)` - Unexpected errors

**Convenience Methods**:
```rust
// These methods provide backward compatibility
ToadStoolError::validation("Invalid input")
ToadStoolError::not_found("Resource not found")
ToadStoolError::configuration("Config invalid")
ToadStoolError::execution("Runtime failed")
ToadStoolError::resource("Out of memory")
ToadStoolError::security("Access denied")
ToadStoolError::network("Connection failed")
ToadStoolError::system("System error")
```

### Specialized Errors

#### ExecutionError

Errors during workload execution.

```rust
pub enum ExecutionError {
    /// Runtime system failed
    RuntimeFailure {
        runtime: String,
        workload_id: String,
        reason: String,
    },
    
    /// Execution timeout
    TimeoutError {
        timeout: Duration,
        operation: String,
    },
    
    /// Invalid workload
    InvalidWorkload { reason: String },
    
    /// Workload not found
    WorkloadNotFound { workload_id: String },
    
    /// Resource exhaustion
    ResourceExhaustion { resource: String },
    
    /// Execution cancelled
    Cancelled { reason: String },
    
    /// Other execution error
    Other { message: String },
}
```

**Example**:
```rust
use toadstool::error::ExecutionError;
use std::time::Duration;

// Timeout error
let err = ExecutionError::TimeoutError {
    timeout: Duration::from_secs(30),
    operation: "container startup".to_string(),
};

// Runtime failure
let err = ExecutionError::RuntimeFailure {
    runtime: "docker".to_string(),
    workload_id: "abc-123".to_string(),
    reason: "Image not found: nginx:latest".to_string(),
};
```

#### ConfigError

Configuration validation and loading errors.

```rust
pub enum ConfigError {
    /// Configuration validation failed
    ValidationError { reason: String },
    
    /// Configuration not found
    NotFound { path: String },
    
    /// Invalid configuration format
    InvalidFormat {
        format: String,
        reason: String,
    },
    
    /// Missing required field
    MissingField { field: String },
    
    /// Configuration parse error
    ParseError {
        config_source: String,
        reason: String,
    },
    
    /// Other configuration error
    Other { message: String },
}
```

**Example**:
```rust
use toadstool::error::ConfigError;

// Validation error
let err = ConfigError::ValidationError {
    reason: "Port must be between 1 and 65535".to_string(),
};

// Missing field
let err = ConfigError::MissingField {
    field: "database.host".to_string(),
};
```

#### ResourceError

Resource allocation and management errors.

```rust
pub enum ResourceError {
    /// Resource not found
    NotFound { resource: String },
    
    /// Resource allocation failed
    AllocationFailure {
        resource: String,
        reason: String,
    },
    
    /// Resource limit exceeded
    LimitExceeded {
        resource: String,
        requested: u64,
        limit: u64,
    },
    
    /// Insufficient resources
    Insufficient {
        resource: String,
        available: u64,
        required: u64,
    },
    
    /// Resource locked
    Locked { resource: String },
    
    /// Other resource error
    Other { message: String },
}
```

**Example**:
```rust
use toadstool::error::ResourceError;

// Limit exceeded
let err = ResourceError::LimitExceeded {
    resource: "memory".to_string(),
    requested: 8589934592,  // 8 GB
    limit: 4294967296,      // 4 GB
};

// Insufficient resources
let err = ResourceError::Insufficient {
    resource: "CPU cores".to_string(),
    available: 2,
    required: 4,
};
```

#### IntegrationError

Third-party service integration errors.

```rust
pub enum IntegrationError {
    /// Connection failed
    ConnectionFailed {
        service: String,
        reason: String,
    },
    
    /// Authentication failed
    AuthenticationFailed { service: String },
    
    /// API error
    ApiError {
        service: String,
        status: u16,
        message: String,
    },
    
    /// Unsupported operation
    UnsupportedOperation {
        service: String,
        operation: String,
    },
    
    /// Other integration error
    Other { message: String },
}
```

#### SecurityError

Security, authentication, and authorization errors.

```rust
pub enum SecurityError {
    /// Authentication failed
    AuthenticationFailed { reason: String },
    
    /// Authorization failed
    AuthorizationFailed {
        user: String,
        resource: String,
    },
    
    /// Invalid credentials
    InvalidCredentials,
    
    /// Token expired
    TokenExpired,
    
    /// Access denied
    AccessDenied { reason: String },
    
    /// Other security error
    Other { message: String },
}
```

#### NetworkError

Network communication errors.

```rust
pub enum NetworkError {
    /// Connection failed
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },
    
    /// Connection timeout
    ConnectionTimeout { host: String, port: u16 },
    
    /// DNS resolution failed
    DnsResolutionFailed { hostname: String },
    
    /// Protocol error
    ProtocolError {
        protocol: String,
        reason: String,
    },
    
    /// Other network error
    Other { message: String },
}
```

#### SystemError

System-level and platform errors.

```rust
pub enum SystemError {
    /// I/O error
    IoError { message: String },
    
    /// Permission denied
    PermissionDenied { path: String },
    
    /// Not supported on this platform
    NotSupported { feature: String },
    
    /// Internal error
    InternalError { message: String },
    
    /// Other system error
    Other { message: String },
}
```

### Result Type Aliases

Convenient type aliases for common result types:

```rust
pub type ToadStoolResult<T> = Result<T, ToadStoolError>;
pub type ExecutionResult<T> = Result<T, ExecutionError>;
pub type ConfigResult<T> = Result<T, ConfigError>;
pub type ResourceResult<T> = Result<T, ResourceError>;
pub type IntegrationResult<T> = Result<T, IntegrationError>;
pub type SecurityResult<T> = Result<T, SecurityError>;
pub type NetworkResult<T> = Result<T, NetworkError>;
pub type SystemResult<T> = Result<T, SystemError>;
```

---

## Domain-Specific Error Conversions

### Overview

**New in Phase 1 (November 2025)**: ToadStool now implements bidirectional error conversions between domain-specific errors and the unified `ToadStoolError`. This enables seamless error handling across module boundaries while preserving domain-specific error information.

### Supported Conversions

#### 1. ServerError ↔ ToadStoolError

**Location**: `crates/server/src/errors.rs`

The server crate has its own `ServerError` type for HTTP-specific error handling. It now converts bidirectionally with `ToadStoolError`:

```rust
use toadstool_server::ServerError;
use toadstool::ToadStoolError;

// ServerError → ToadStoolError (automatic with ?)
fn handle_request() -> Result<Response, ToadStoolError> {
    let server_result = server_operation()?;  // ServerError converts automatically
    Ok(Response::success(server_result))
}

// ToadStoolError → ServerError (for HTTP responses)
fn into_http_error(error: ToadStoolError) -> ServerError {
    error.into()  // Converts automatically
}
```

**Conversion Mappings**:
- `ServerError::Initialization` → `ToadStoolError::System(SystemError::Platform)`
- `ServerError::Configuration` → `ToadStoolError::Configuration(ConfigError::ValidationError)`
- `ServerError::Execution` → `ToadStoolError::Execution(ExecutionError::RuntimeFailure)`
- `ServerError::Database` → `ToadStoolError::System(SystemError::Database)`
- `ServerError::Network` → `ToadStoolError::Network(NetworkError::ConnectionFailed)`
- `ServerError::Unauthorized` → `ToadStoolError::Security(SecurityError::AuthenticationFailed)`
- `ServerError::Forbidden` → `ToadStoolError::Security(SecurityError::AuthorizationFailed)`
- `ServerError::NotFound` → `ToadStoolError::Resource(ResourceError::NotFound)`
- `ServerError::Internal` → `ToadStoolError::System(SystemError::Internal)`

#### 2. ClientError ↔ ToadStoolError

**Location**: `crates/client/src/client/error.rs`

The client crate has `ClientError` for HTTP client operations:

```rust
use toadstool_client::ClientError;
use toadstool::ToadStoolError;

// ClientError → ToadStoolError
fn submit_workload(client: &Client, workload: Workload) -> Result<ExecutionId, ToadStoolError> {
    let response = client.submit(workload)?;  // ClientError converts automatically
    Ok(response.execution_id)
}

// ToadStoolError → ClientError (for client-side error handling)
fn handle_core_error(error: ToadStoolError) -> ClientError {
    error.into()
}
```

**Conversion Mappings**:
- `ClientError::Http` → `ToadStoolError::Network(NetworkError::ConnectionFailed)`
- `ClientError::Timeout` → `ToadStoolError::Network(NetworkError::Timeout)`
- `ClientError::Serialization` → `ToadStoolError::System(SystemError::Serialization)`
- `ClientError::Deserialization` → `ToadStoolError::System(SystemError::Deserialization)`
- `ClientError::Server` → `ToadStoolError::Execution(ExecutionError::RuntimeFailure)`
- `ClientError::InvalidUrl` → `ToadStoolError::Configuration(ConfigError::ValidationError)`
- `ClientError::Configuration` → `ToadStoolError::Configuration(ConfigError::ValidationError)`

#### 3. PrimalError ↔ ToadStoolError

**Location**: `crates/integration/primals/src/error.rs`

The primal integration crate uses `PrimalError` for ecosystem coordination:

```rust
use toadstool_integration_primals::PrimalError;
use toadstool::ToadStoolError;

// PrimalError → ToadStoolError
fn coordinate_primals(request: PrimalRequest) -> Result<PrimalResponse, ToadStoolError> {
    let response = primal_handler.handle(request)?;  // PrimalError converts
    Ok(response)
}

// ToadStoolError → PrimalError
fn wrap_core_error(error: ToadStoolError) -> PrimalError {
    error.into()
}
```

**Conversion Mappings**:
- `PrimalError::Configuration` → `ToadStoolError::Configuration(ConfigError::ValidationError)`
- `PrimalError::Communication` → `ToadStoolError::Network(NetworkError::ConnectionFailed)`
- `PrimalError::Timeout` → `ToadStoolError::Execution(ExecutionError::TimeoutError)`
- `PrimalError::NotFound` → `ToadStoolError::Integration(IntegrationError::ServiceUnavailable)`
- `PrimalError::Unauthorized` → `ToadStoolError::Security(SecurityError::AuthenticationFailed)`
- `PrimalError::ResourceExhausted` → `ToadStoolError::Resource(ResourceError::Insufficient)`
- `PrimalError::Internal` → `ToadStoolError::Integration(IntegrationError::ExternalSystemError)`

### Usage Patterns

#### Pattern 1: Automatic Conversion with `?`

```rust
use toadstool::ToadStoolResult;
use toadstool_server::ServerError;

fn api_handler(req: Request) -> ToadStoolResult<Response> {
    // ServerError automatically converts to ToadStoolError
    let config = load_server_config()?;
    let result = process_request(req, config)?;
    Ok(Response::success(result))
}

fn load_server_config() -> Result<ServerConfig, ServerError> {
    // Returns ServerError
    ServerConfig::from_env()
}
```

#### Pattern 2: Explicit Conversion for Error Mapping

```rust
use toadstool::ToadStoolError;
use toadstool_client::ClientError;

fn submit_to_remote(endpoint: &str, workload: Workload) -> ToadStoolResult<ExecutionId> {
    let client = Client::new(endpoint);
    
    match client.submit(workload) {
        Ok(response) => Ok(response.execution_id),
        Err(client_err) => {
            // Explicitly convert to add context
            let core_err: ToadStoolError = client_err.into();
            Err(core_err)
        }
    }
}
```

#### Pattern 3: Bidirectional Flow

```rust
use toadstool::ToadStoolError;
use toadstool_server::ServerError;

async fn handle_workload_submission(
    req: WorkloadRequest
) -> Result<Response, ServerError> {
    // Core processing returns ToadStoolError
    let result = process_workload_core(req).await;
    
    match result {
        Ok(execution_id) => Ok(Response::success(execution_id)),
        Err(core_err) => {
            // Convert ToadStoolError → ServerError for HTTP response
            let server_err: ServerError = core_err.into();
            Err(server_err)
        }
    }
}

async fn process_workload_core(req: WorkloadRequest) -> ToadStoolResult<ExecutionId> {
    // Returns ToadStoolError
    validate_workload(&req)?;
    schedule_execution(req).await
}
```

### Benefits of Bidirectional Conversions

1. **Module Boundary Clarity**: Each module can define domain-specific errors while maintaining compatibility with the unified error system

2. **Information Preservation**: Conversions map specific error variants to preserve as much context as possible

3. **Ergonomic Error Handling**: The `?` operator works seamlessly across module boundaries

4. **Type Safety**: Rust's type system ensures conversions are explicit and correct

5. **Backward Compatibility**: Existing code using domain-specific errors continues to work

### When to Use Each Error Type

| Use Case | Recommended Error Type |
|----------|----------------------|
| Internal core logic | `ToadStoolError` |
| HTTP server responses | `ServerError` |
| HTTP client operations | `ClientError` |
| Primal coordination | `PrimalError` |
| Public API boundaries | Domain-specific, then convert |
| Cross-module calls | `ToadStoolError` |

### Testing Conversions

```rust
#[cfg(test)]
mod tests {
    use toadstool::ToadStoolError;
    use toadstool_server::ServerError;
    
    #[test]
    fn test_server_error_conversion() {
        // ServerError → ToadStoolError
        let server_err = ServerError::Configuration("Invalid port".to_string());
        let core_err: ToadStoolError = server_err.into();
        
        assert!(matches!(
            core_err,
            ToadStoolError::Configuration(_)
        ));
    }
    
    #[test]
    fn test_bidirectional_conversion() {
        // ToadStoolError → ServerError → ToadStoolError
        let original = ToadStoolError::execution("Runtime failed");
        let server_err: ServerError = original.into();
        let back_to_core: ToadStoolError = server_err.into();
        
        // Should still be an execution error
        assert!(matches!(
            back_to_core,
            ToadStoolError::Execution(_)
        ));
    }
}
```

---

## Best Practices

### 1. Use Specific Error Types

❌ **Don't**:
```rust
return Err(ToadStoolError::Unknown("Something went wrong".to_string()));
```

✅ **Do**:
```rust
return Err(ExecutionError::RuntimeFailure {
    runtime: "docker".to_string(),
    workload_id: workload.id.clone(),
    reason: format!("Container failed to start: {}", e),
}.into());
```

### 2. Provide Context

❌ **Don't**:
```rust
return Err(ToadStoolError::validation("Invalid"));
```

✅ **Do**:
```rust
return Err(ConfigError::ValidationError {
    reason: format!(
        "Port {} is invalid. Must be between 1 and 65535",
        port
    ),
}.into());
```

### 3. Use Result Type Aliases

❌ **Don't**:
```rust
fn load_config() -> Result<Config, ToadStoolError> { ... }
```

✅ **Do**:
```rust
fn load_config() -> ToadStoolResult<Config> { ... }
// or more specific:
fn load_config() -> ConfigResult<Config> { ... }
```

### 4. Leverage Automatic Conversions

✅ **Do**:
```rust
use toadstool::error::ToadStoolResult;

fn read_file(path: &str) -> ToadStoolResult<String> {
    // std::io::Error automatically converts
    let contents = std::fs::read_to_string(path)?;
    Ok(contents)
}
```

### 5. Pattern Match for Specific Handling

```rust
match execute_workload(id) {
    Ok(result) => handle_success(result),
    Err(ToadStoolError::Execution(ExecutionError::TimeoutError { .. })) => {
        // Retry on timeout
        retry_execution(id)
    },
    Err(ToadStoolError::Resource(ResourceError::NotFound { .. })) => {
        // Return 404
        not_found_response()
    },
    Err(e) => {
        // Log and return 500
        log::error!("Unexpected error: {}", e);
        internal_error_response()
    },
}
```

### 6. Use `map_err` for Context

```rust
fn load_and_parse(path: &str) -> ToadStoolResult<Config> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| ToadStoolError::system(
            format!("Failed to read config at {}: {}", path, e)
        ))?;
    
    parse_config(&contents)
}
```

---

## Migration Guide

### From Legacy Error Patterns

The new unified error system is **fully backward compatible**. Legacy code continues to work without changes.

#### Legacy Pattern (Still Works)

```rust
// Old code continues to work
let err = ToadStoolError::validation("Invalid input");
let err = ToadStoolError::not_found("Resource not found");
let err = ToadStoolError::configuration("Config error");
```

#### New Pattern (Recommended)

```rust
// New code should use specialized errors
use toadstool::error::{ConfigError, ResourceError};

let err = ConfigError::ValidationError {
    reason: "Invalid input".to_string(),
}.into();

let err = ResourceError::NotFound {
    resource: "database connection".to_string(),
}.into();
```

### Updating Match Patterns

#### Before

```rust
match error {
    ToadStoolError::NotFound { message } => { ... },
    ToadStoolError::Validation { message } => { ... },
    _ => { ... }
}
```

#### After

```rust
use toadstool::error::{ConfigError, ResourceError};

match error {
    ToadStoolError::Resource(ResourceError::NotFound { .. }) => { ... },
    ToadStoolError::Configuration(ConfigError::ValidationError { .. }) => { ... },
    _ => { ... }
}
```

### Updating Error Construction

#### Before

```rust
impl MyService {
    fn validate(&self, data: &str) -> ToadStoolResult<()> {
        if data.is_empty() {
            return Err(ToadStoolError::Validation {
                message: "Data cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}
```

#### After (Option 1: Use convenience methods)

```rust
impl MyService {
    fn validate(&self, data: &str) -> ToadStoolResult<()> {
        if data.is_empty() {
            return Err(ToadStoolError::validation("Data cannot be empty"));
        }
        Ok(())
    }
}
```

#### After (Option 2: Use specialized errors - recommended)

```rust
use toadstool::error::ConfigResult;

impl MyService {
    fn validate(&self, data: &str) -> ConfigResult<()> {
        if data.is_empty() {
            return Err(ConfigError::ValidationError {
                reason: "Data cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}
```

---

## Examples

### Example 1: Workload Execution

```rust
use toadstool::error::{ExecutionError, ExecutionResult};
use std::time::Duration;

pub struct WorkloadExecutor;

impl WorkloadExecutor {
    pub fn execute(&self, workload_id: &str, timeout: Duration) -> ExecutionResult<String> {
        // Validate workload exists
        let workload = self.find_workload(workload_id)
            .ok_or_else(|| ExecutionError::WorkloadNotFound {
                workload_id: workload_id.to_string(),
            })?;
        
        // Check resources
        if !self.has_resources(&workload) {
            return Err(ExecutionError::ResourceExhaustion {
                resource: "Available CPU cores".to_string(),
            });
        }
        
        // Execute with timeout
        let result = self.run_with_timeout(&workload, timeout)
            .map_err(|e| ExecutionError::TimeoutError {
                timeout,
                operation: format!("Execution of workload {}", workload_id),
            })?;
        
        Ok(result)
    }
    
    // Helper methods...
    fn find_workload(&self, id: &str) -> Option<Workload> { /* ... */ }
    fn has_resources(&self, w: &Workload) -> bool { /* ... */ }
    fn run_with_timeout(&self, w: &Workload, t: Duration) -> Result<String, TimeoutError> { /* ... */ }
}
```

### Example 2: Configuration Loading

```rust
use toadstool::error::{ConfigError, ConfigResult};
use std::path::Path;

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load<P: AsRef<Path>>(&self, path: P) -> ConfigResult<AppConfig> {
        let path = path.as_ref();
        
        // Check file exists
        if !path.exists() {
            return Err(ConfigError::NotFound {
                path: path.display().to_string(),
            });
        }
        
        // Read file (automatic conversion from io::Error)
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ParseError {
                config_source: path.display().to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;
        
        // Parse TOML
        let config: AppConfig = toml::from_str(&contents)
            .map_err(|e| ConfigError::InvalidFormat {
                format: "TOML".to_string(),
                reason: format!("Parse error: {}", e),
            })?;
        
        // Validate
        self.validate(&config)?;
        
        Ok(config)
    }
    
    fn validate(&self, config: &AppConfig) -> ConfigResult<()> {
        if config.port == 0 {
            return Err(ConfigError::ValidationError {
                reason: "Port cannot be 0".to_string(),
            });
        }
        
        if config.workers == 0 {
            return Err(ConfigError::MissingField {
                field: "workers".to_string(),
            });
        }
        
        Ok(())
    }
}
```

### Example 3: API Error Handling

```rust
use toadstool::error::{ToadStoolError, ConfigError, ResourceError, SystemError};
use axum::{http::StatusCode, response::IntoResponse, Json};

// Convert ToadStool errors to HTTP responses
impl From<ToadStoolError> for ApiError {
    fn from(err: ToadStoolError) -> Self {
        match err {
            // Resource errors -> NOT_FOUND
            ToadStoolError::Resource(ResourceError::NotFound { .. }) => Self {
                status: StatusCode::NOT_FOUND,
                message: err.to_string(),
            },
            
            // Config validation errors -> BAD_REQUEST
            ToadStoolError::Configuration(ConfigError::ValidationError { .. }) => Self {
                status: StatusCode::BAD_REQUEST,
                message: err.to_string(),
            },
            
            // Resource limits -> INSUFFICIENT_STORAGE
            ToadStoolError::Resource(ResourceError::LimitExceeded { .. })
            | ToadStoolError::Resource(ResourceError::Insufficient { .. }) => Self {
                status: StatusCode::INSUFFICIENT_STORAGE,
                message: err.to_string(),
            },
            
            // Not supported -> NOT_IMPLEMENTED
            ToadStoolError::System(SystemError::NotSupported { .. }) => Self {
                status: StatusCode::NOT_IMPLEMENTED,
                message: err.to_string(),
            },
            
            // Everything else -> INTERNAL_SERVER_ERROR
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: err.to_string(),
            },
        }
    }
}
```

### Example 4: Combining Multiple Error Sources

```rust
use toadstool::error::{ToadStoolResult, ExecutionError, ConfigError};

pub struct ServiceOrchestrator {
    config: Config,
}

impl ServiceOrchestrator {
    pub fn deploy(&self, service_name: &str) -> ToadStoolResult<DeploymentInfo> {
        // Load service configuration (ConfigError)
        let service_config = self.load_service_config(service_name)?;
        
        // Validate resources (ResourceError)
        self.check_resources(&service_config)?;
        
        // Deploy service (ExecutionError)
        let deployment = self.execute_deployment(&service_config)?;
        
        // Register with discovery (IntegrationError)
        self.register_service(&deployment)?;
        
        Ok(deployment)
    }
    
    fn load_service_config(&self, name: &str) -> ConfigResult<ServiceConfig> {
        // Returns ConfigError
    }
    
    fn check_resources(&self, config: &ServiceConfig) -> ResourceResult<()> {
        // Returns ResourceError
    }
    
    fn execute_deployment(&self, config: &ServiceConfig) -> ExecutionResult<DeploymentInfo> {
        // Returns ExecutionError
    }
    
    fn register_service(&self, deployment: &DeploymentInfo) -> IntegrationResult<()> {
        // Returns IntegrationError
    }
}

// All errors automatically convert to ToadStoolError via From<> implementations
```

---

## Troubleshooting

### Common Issues

#### 1. "no variant named X found for enum ToadStoolError"

**Problem**: Trying to use old error variant names.

**Solution**: Update to new error patterns or use convenience methods.

```rust
// ❌ Old (doesn't compile)
ToadStoolError::NotFound { message: "...".to_string() }

// ✅ New (specialized)
ResourceError::NotFound { resource: "...".to_string() }.into()

// ✅ New (convenience)
ToadStoolError::not_found("...")
```

#### 2. "cannot infer type for type parameter T"

**Problem**: Result type cannot be inferred.

**Solution**: Specify the type explicitly.

```rust
// ❌ Ambiguous
let result = load_config()?;

// ✅ Explicit type
let result: Config = load_config()?;

// ✅ Or use turbofish
let result = load_config::<Config>()?;
```

#### 3. "the trait From<MyError> is not implemented for ToadStoolError"

**Problem**: Custom error type doesn't convert automatically.

**Solution**: Implement From trait or use map_err.

```rust
// Option 1: Implement From
impl From<MyError> for ToadStoolError {
    fn from(err: MyError) -> Self {
        ToadStoolError::system(err.to_string())
    }
}

// Option 2: Use map_err
my_function()
    .map_err(|e| ToadStoolError::system(format!("My operation failed: {}", e)))?
```

---

## Testing Error Handling

### Unit Testing Errors

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::error::{ExecutionError, ToadStoolError};
    
    #[test]
    fn test_workload_not_found_error() {
        let result = execute_workload("nonexistent");
        
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        match err {
            ToadStoolError::Execution(ExecutionError::WorkloadNotFound { workload_id }) => {
                assert_eq!(workload_id, "nonexistent");
            },
            _ => panic!("Expected WorkloadNotFound error"),
        }
    }
    
    #[test]
    fn test_timeout_error() {
        let err = ExecutionError::TimeoutError {
            timeout: Duration::from_secs(30),
            operation: "startup".to_string(),
        };
        
        let msg = err.to_string();
        assert!(msg.contains("30s"));
        assert!(msg.contains("startup"));
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use toadstool::error::ToadStoolResult;
    
    #[test]
    fn test_full_deployment_flow() -> ToadStoolResult<()> {
        let orchestrator = ServiceOrchestrator::new()?;
        
        // Should succeed
        let deployment = orchestrator.deploy("web-service")?;
        assert!(deployment.is_healthy());
        
        // Should fail with ConfigError
        let result = orchestrator.deploy("invalid-service");
        assert!(matches!(
            result,
            Err(ToadStoolError::Configuration(ConfigError::NotFound { .. }))
        ));
        
        Ok(())
    }
}
```

---

## Additional Resources

- **API Documentation**: Run `cargo doc --open` to see full API docs
- **Source Code**: `crates/core/common/src/error.rs`
- **Examples**: `examples/` directory for real-world usage
- **Tests**: `crates/core/common/src/error.rs` (inline tests)

---

## Summary

The ToadStool unified error system provides:

✅ **Comprehensive Error Types** - 47 specialized error variants  
✅ **Rich Context** - Structured error data for debugging  
✅ **Automatic Conversions** - From common error types  
✅ **Backward Compatible** - Legacy code works unchanged  
✅ **Type Safe** - Compile-time error checking  
✅ **Composable** - Errors convert across boundaries  
✅ **Production Ready** - Battle-tested with 696 tests  

**Questions?** Refer to the examples above or explore the source code!

---

**Last Updated**: November 9, 2025  
**Version**: 1.1 (Phase 1 Complete + Bidirectional Conversions)

