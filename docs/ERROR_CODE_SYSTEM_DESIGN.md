# ToadStool Error Code System Design

## Overview

This document outlines the design for a comprehensive structured error code system for ToadStool, building upon the existing 3-tier error hierarchy.

## Current Error System (Tier 1-3)

```
Tier 1: ToadStoolError (top-level variants)
├── Execution
├── Configuration  
├── Resource
├── Integration
├── Security
├── Network
└── System

Tier 2: Specialized Errors (ExecutionError, ConfigError, etc.)

Tier 3: Result Aliases (ToadStoolResult<T>, ExecutionResult<T>, etc.)
```

## Proposed Error Code System (Tier 4)

### Design Principles

1. **Machine-Readable**: Structured codes for programmatic error handling
2. **Human-Friendly**: Clear error messages with context
3. **Backward Compatible**: Works with existing error system
4. **Categorized**: Hierarchical codes matching error tiers
5. **Extensible**: Easy to add new codes without breaking changes

### Error Code Format

```
[CATEGORY]-[SUBCATEGORY]-[CODE]

Examples:
- EXEC-RUNTIME-001: Runtime engine initialization failed
- CONFIG-PARSE-002: Invalid YAML syntax
- RESOURCE-ALLOC-003: Insufficient memory
- SECURITY-AUTH-001: Authentication failed
```

### Code Categories

#### Execution Errors (EXEC)
- `EXEC-RUNTIME-*`: Runtime engine errors
- `EXEC-TIMEOUT-*`: Timeout-related errors
- `EXEC-VALIDATION-*`: Input validation failures
- `EXEC-STATE-*`: Invalid execution state

#### Configuration Errors (CONFIG)
- `CONFIG-PARSE-*`: Parsing/deserialization errors
- `CONFIG-VALIDATE-*`: Validation failures
- `CONFIG-ENV-*`: Environment variable errors
- `CONFIG-FILE-*`: File access errors

#### Resource Errors (RESOURCE)
- `RESOURCE-ALLOC-*`: Allocation failures
- `RESOURCE-LIMIT-*`: Limit exceeded
- `RESOURCE-UNAVAIL-*`: Resource unavailable
- `RESOURCE-CONFLICT-*`: Resource conflicts

#### Integration Errors (INTEGRATION)
- `INTEGRATION-CONNECT-*`: Connection failures
- `INTEGRATION-PROTO-*`: Protocol errors
- `INTEGRATION-VERSION-*`: Version mismatch
- `INTEGRATION-TIMEOUT-*`: Service timeouts

#### Security Errors (SECURITY)
- `SECURITY-AUTH-*`: Authentication errors
- `SECURITY-AUTHZ-*`: Authorization errors
- `SECURITY-CRYPTO-*`: Cryptographic errors
- `SECURITY-SANDBOX-*`: Sandbox violations

#### Network Errors (NETWORK)
- `NETWORK-CONNECT-*`: Connection errors
- `NETWORK-TIMEOUT-*`: Network timeouts
- `NETWORK-DNS-*`: DNS resolution errors
- `NETWORK-TLS-*`: TLS/SSL errors

#### System Errors (SYSTEM)
- `SYSTEM-IO-*`: I/O errors
- `SYSTEM-PERM-*`: Permission errors
- `SYSTEM-RESOURCE-*`: OS resource errors
- `SYSTEM-PLATFORM-*`: Platform-specific errors

## Implementation Plan

### Phase 1: Core Infrastructure (2-3 hours)

```rust
// crates/core/common/src/error_codes.rs

/// Error code structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorCode {
    /// Machine-readable code (e.g., "EXEC-RUNTIME-001")
    pub code: &'static str,
    /// Human-readable message template
    pub message: &'static str,
    /// Error category
    pub category: ErrorCategory,
    /// Suggested remediation
    pub remediation: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Execution,
    Configuration,
    Resource,
    Integration,
    Security,
    Network,
    System,
}

// Registry of all error codes
pub mod codes {
    use super::*;
    
    // Execution errors
    pub const EXEC_RUNTIME_001: ErrorCode = ErrorCode {
        code: "EXEC-RUNTIME-001",
        message: "Runtime engine initialization failed",
        category: ErrorCategory::Execution,
        remediation: Some("Check runtime dependencies and configuration"),
    };
    
    pub const EXEC_TIMEOUT_001: ErrorCode = ErrorCode {
        code: "EXEC-TIMEOUT-001", 
        message: "Execution timeout exceeded",
        category: ErrorCategory::Execution,
        remediation: Some("Increase timeout limit or optimize workload"),
    };
    
    // Configuration errors
    pub const CONFIG_PARSE_001: ErrorCode = ErrorCode {
        code: "CONFIG-PARSE-001",
        message: "Failed to parse configuration file",
        category: ErrorCategory::Configuration,
        remediation: Some("Validate YAML/TOML syntax"),
    };
    
    // ... more error codes ...
}
```

### Phase 2: Integration with Existing Errors (2-3 hours)

```rust
// Update existing error types to include error codes

impl ToadStoolError {
    /// Create error with structured code
    pub fn with_code(code: ErrorCode, context: impl Into<String>) -> Self {
        // Implementation that includes code in error
    }
    
    /// Get error code if available
    pub fn error_code(&self) -> Option<&ErrorCode> {
        // Return associated error code
    }
}

impl ExecutionError {
    pub fn runtime_init_failed(details: impl Into<String>) -> Self {
        Self::RuntimeFailure(
            codes::EXEC_RUNTIME_001,
            details.into()
        )
    }
}
```

### Phase 3: API Integration (2-3 hours)

```rust
// Update API error responses to include error codes

#[derive(Serialize)]
pub struct ApiErrorResponse {
    pub error: String,
    pub error_code: Option<String>,
    pub category: Option<String>,
    pub remediation: Option<String>,
    pub details: HashMap<String, serde_json::Value>,
}
```

### Phase 4: Documentation & Testing (2-3 hours)

- Generate error code catalog
- Add examples to each error code
- Write integration tests
- Update API documentation

## Benefits

1. **Improved Debugging**: Unique codes make errors easy to search/track
2. **Better Monitoring**: Group errors by code for metrics
3. **Client Experience**: Structured errors enable better client error handling
4. **Documentation**: Error codes serve as documentation
5. **i18n Ready**: Message templates can be localized

## Migration Strategy

1. Add error code system alongside existing errors (no breaking changes)
2. Gradually migrate hot paths to use error codes
3. Maintain backward compatibility with string-based errors
4. Document best practices for new code

## Success Metrics

- All Tier-1 error variants have associated error codes
- API responses include error codes
- Error catalog documentation exists
- Tests cover error code serialization/deserialization

## Timeline

- **Phase 1**: Core infrastructure (2-3 hours)
- **Phase 2**: Integration (2-3 hours)  
- **Phase 3**: API integration (2-3 hours)
- **Phase 4**: Documentation (2-3 hours)
- **Total**: 8-12 hours

## References

- Existing error system: `crates/core/common/src/error.rs`
- Configuration defaults: `crates/core/config/src/defaults.rs`
- API types: `crates/api/src/types.rs`

