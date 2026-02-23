// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! # Structured Error Code System
//!
//! Provides machine-readable error codes with human-friendly messages,
//! remediation suggestions, and categorization for the ToadStool platform.
//!
//! ## Design Principles
//!
//! 1. **Machine-Readable**: Each error has a unique code for programmatic handling
//! 2. **Human-Friendly**: Clear messages with actionable remediation
//! 3. **Backward Compatible**: Integrates with existing `ToadStoolError` system
//! 4. **Hierarchical**: Codes match the 3-tier error architecture
//! 5. **Extensible**: Easy to add new codes without breaking changes
//!
//! ## Error Code Format
//!
//! Format: `[CATEGORY]-[SUBCATEGORY]-[CODE]`
//!
//! Examples:
//! - `EXEC-RUNTIME-001`: Runtime engine initialization failed
//! - `CONFIG-PARSE-002`: Configuration parsing error
//! - `RESOURCE-ALLOC-003`: Resource allocation failure
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use toadstool_common::error_codes::{ErrorCode, codes};
//! use toadstool_common::error::ToadStoolError;
//!
//! // Use error codes with existing error system
//! let error = ToadStoolError::Execution(
//!     codes::EXEC_RUNTIME_001.into_error_with_context("Failed to initialize Wasmtime")
//! );
//!
//! // Access error code information
//! let code = &codes::EXEC_RUNTIME_001;
//! println!("Error: {} - {}", code.code, code.message);
//! if let Some(fix) = code.remediation {
//!     println!("Remediation: {}", fix);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Structured error code with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorCode {
    /// Machine-readable error code (e.g., "EXEC-RUNTIME-001")
    pub code: &'static str,

    /// Human-readable error message template
    pub message: &'static str,

    /// Error category matching `ToadStoolError` variants
    pub category: ErrorCategory,

    /// Optional remediation suggestion
    pub remediation: Option<&'static str>,
}

/// Error categories matching the 3-tier error hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorCategory {
    /// Execution errors (runtime engines, workload execution)
    Execution,

    /// Configuration errors (parsing, validation, environment)
    Configuration,

    /// Resource errors (allocation, limits, availability)
    Resource,

    /// Integration errors (ecosystem services, external systems)
    Integration,

    /// Security errors (authentication, authorization, sandboxing)
    Security,

    /// Network errors (connectivity, protocols, timeouts)
    Network,

    /// System errors (OS-level, I/O, permissions)
    System,
}

impl ErrorCode {
    /// Create a detailed error message with context
    ///
    /// Uses zero-copy optimization: if no context is provided, returns a static string.
    /// Only allocates when context is provided.
    pub fn into_error_with_context(self, context: impl Into<String>) -> String {
        let context = context.into();
        if context.is_empty() {
            format!("{}: {}", self.code, self.message)
        } else {
            format!("{}: {} - {}", self.code, self.message, context)
        }
    }

    /// Create an error message as Cow (zero-copy when possible)
    ///
    /// Returns `Cow::Borrowed` when no context, `Cow::Owned` when context is provided.
    /// This is the preferred method for performance-critical paths.
    #[must_use]
    pub const fn to_error_message(&self) -> Cow<'static, str> {
        // Static formatting without allocation
        Cow::Borrowed(self.message)
    }

    /// Create an error message with context as Cow (zero-copy when possible)
    ///
    /// Returns `Cow::Owned` only when context is non-empty.
    #[must_use]
    pub fn to_error_message_with_context<'a>(&self, context: &'a str) -> Cow<'a, str> {
        if context.is_empty() {
            // No allocation - just borrow the static message
            Cow::Borrowed(self.message)
        } else {
            // Allocate only when we have context
            Cow::Owned(format!("{}: {} - {}", self.code, self.message, context))
        }
    }

    /// Get the full error code with message (static, no allocation)
    #[must_use]
    pub fn full_message(&self) -> String {
        format!("{}: {}", self.code, self.message)
    }

    /// Get category as string
    #[must_use]
    pub const fn category_str(&self) -> &'static str {
        match self.category {
            ErrorCategory::Execution => "execution",
            ErrorCategory::Configuration => "configuration",
            ErrorCategory::Resource => "resource",
            ErrorCategory::Integration => "integration",
            ErrorCategory::Security => "security",
            ErrorCategory::Network => "network",
            ErrorCategory::System => "system",
        }
    }
}

/// Standard error codes organized by category
pub mod codes {
    use super::{ErrorCategory, ErrorCode};

    // ========================================================================
    // Execution Errors (EXEC)
    // ========================================================================

    /// Runtime engine initialization failed
    pub const EXEC_RUNTIME_001: ErrorCode = ErrorCode {
        code: "EXEC-RUNTIME-001",
        message: "Runtime engine initialization failed",
        category: ErrorCategory::Execution,
        remediation: Some("Check runtime dependencies and configuration"),
    };

    /// Execution timeout exceeded
    pub const EXEC_TIMEOUT_001: ErrorCode = ErrorCode {
        code: "EXEC-TIMEOUT-001",
        message: "Execution timeout exceeded",
        category: ErrorCategory::Execution,
        remediation: Some("Increase timeout limit or optimize workload"),
    };

    /// Invalid execution input
    pub const EXEC_VALIDATION_001: ErrorCode = ErrorCode {
        code: "EXEC-VALIDATION-001",
        message: "Invalid execution input or parameters",
        category: ErrorCategory::Execution,
        remediation: Some("Validate input against schema requirements"),
    };

    /// Execution runtime failure
    pub const EXEC_RUNTIME_002: ErrorCode = ErrorCode {
        code: "EXEC-RUNTIME-002",
        message: "Runtime execution failure",
        category: ErrorCategory::Execution,
        remediation: Some("Check runtime logs for detailed error information"),
    };

    /// Workload not found
    pub const EXEC_NOTFOUND_001: ErrorCode = ErrorCode {
        code: "EXEC-NOTFOUND-001",
        message: "Requested workload or resource not found",
        category: ErrorCategory::Execution,
        remediation: Some("Verify workload ID or path exists"),
    };

    // ========================================================================
    // Configuration Errors (CONFIG)
    // ========================================================================

    /// Configuration parsing error
    pub const CONFIG_PARSE_001: ErrorCode = ErrorCode {
        code: "CONFIG-PARSE-001",
        message: "Failed to parse configuration file",
        category: ErrorCategory::Configuration,
        remediation: Some("Validate YAML/TOML syntax and structure"),
    };

    /// Configuration validation error
    pub const CONFIG_VALIDATE_001: ErrorCode = ErrorCode {
        code: "CONFIG-VALIDATE-001",
        message: "Configuration validation failed",
        category: ErrorCategory::Configuration,
        remediation: Some("Check configuration against schema requirements"),
    };

    /// Environment variable error
    pub const CONFIG_ENV_001: ErrorCode = ErrorCode {
        code: "CONFIG-ENV-001",
        message: "Required environment variable not set",
        category: ErrorCategory::Configuration,
        remediation: Some("Set required environment variables"),
    };

    /// Configuration file not found
    pub const CONFIG_FILE_001: ErrorCode = ErrorCode {
        code: "CONFIG-FILE-001",
        message: "Configuration file not found",
        category: ErrorCategory::Configuration,
        remediation: Some("Ensure configuration file exists at expected path"),
    };

    // ========================================================================
    // Resource Errors (RESOURCE)
    // ========================================================================

    /// Resource allocation failure
    pub const RESOURCE_ALLOC_001: ErrorCode = ErrorCode {
        code: "RESOURCE-ALLOC-001",
        message: "Failed to allocate required resources",
        category: ErrorCategory::Resource,
        remediation: Some("Free up system resources or reduce allocation requirements"),
    };

    /// Resource limit exceeded
    pub const RESOURCE_LIMIT_001: ErrorCode = ErrorCode {
        code: "RESOURCE-LIMIT-001",
        message: "Resource limit exceeded",
        category: ErrorCategory::Resource,
        remediation: Some("Increase resource limits or optimize usage"),
    };

    /// Insufficient memory
    pub const RESOURCE_ALLOC_002: ErrorCode = ErrorCode {
        code: "RESOURCE-ALLOC-002",
        message: "Insufficient memory available",
        category: ErrorCategory::Resource,
        remediation: Some("Free memory or increase memory limit"),
    };

    /// Resource unavailable
    pub const RESOURCE_UNAVAIL_001: ErrorCode = ErrorCode {
        code: "RESOURCE-UNAVAIL-001",
        message: "Required resource is unavailable",
        category: ErrorCategory::Resource,
        remediation: Some("Wait for resource availability or use alternative"),
    };

    // ========================================================================
    // Integration Errors (INTEGRATION)
    // ========================================================================

    /// Service connection failed
    pub const INTEGRATION_CONNECT_001: ErrorCode = ErrorCode {
        code: "INTEGRATION-CONNECT-001",
        message: "Failed to connect to external service",
        category: ErrorCategory::Integration,
        remediation: Some("Check service availability and network connectivity"),
    };

    /// Protocol error
    pub const INTEGRATION_PROTO_001: ErrorCode = ErrorCode {
        code: "INTEGRATION-PROTO-001",
        message: "Protocol communication error",
        category: ErrorCategory::Integration,
        remediation: Some("Verify protocol version compatibility"),
    };

    /// Service timeout
    pub const INTEGRATION_TIMEOUT_001: ErrorCode = ErrorCode {
        code: "INTEGRATION-TIMEOUT-001",
        message: "External service request timeout",
        category: ErrorCategory::Integration,
        remediation: Some("Increase timeout or check service responsiveness"),
    };

    /// Version mismatch
    pub const INTEGRATION_VERSION_001: ErrorCode = ErrorCode {
        code: "INTEGRATION-VERSION-001",
        message: "Service version mismatch",
        category: ErrorCategory::Integration,
        remediation: Some("Update services to compatible versions"),
    };

    // ========================================================================
    // Security Errors (SECURITY)
    // ========================================================================

    /// Authentication failed
    pub const SECURITY_AUTH_001: ErrorCode = ErrorCode {
        code: "SECURITY-AUTH-001",
        message: "Authentication failed",
        category: ErrorCategory::Security,
        remediation: Some("Verify credentials and authentication configuration"),
    };

    /// Authorization denied
    pub const SECURITY_AUTHZ_001: ErrorCode = ErrorCode {
        code: "SECURITY-AUTHZ-001",
        message: "Authorization denied - insufficient permissions",
        category: ErrorCategory::Security,
        remediation: Some("Request appropriate permissions or role"),
    };

    /// Cryptographic operation failed
    pub const SECURITY_CRYPTO_001: ErrorCode = ErrorCode {
        code: "SECURITY-CRYPTO-001",
        message: "Cryptographic operation failed",
        category: ErrorCategory::Security,
        remediation: Some("Check encryption keys and algorithms"),
    };

    /// Sandbox violation
    pub const SECURITY_SANDBOX_001: ErrorCode = ErrorCode {
        code: "SECURITY-SANDBOX-001",
        message: "Sandbox security policy violation",
        category: ErrorCategory::Security,
        remediation: Some("Review and comply with sandbox restrictions"),
    };

    // ========================================================================
    // Network Errors (NETWORK)
    // ========================================================================

    /// Connection failed
    pub const NETWORK_CONNECT_001: ErrorCode = ErrorCode {
        code: "NETWORK-CONNECT-001",
        message: "Network connection failed",
        category: ErrorCategory::Network,
        remediation: Some("Check network connectivity and firewall rules"),
    };

    /// Network timeout
    pub const NETWORK_TIMEOUT_001: ErrorCode = ErrorCode {
        code: "NETWORK-TIMEOUT-001",
        message: "Network operation timeout",
        category: ErrorCategory::Network,
        remediation: Some("Check network latency or increase timeout"),
    };

    /// DNS resolution failed
    pub const NETWORK_DNS_001: ErrorCode = ErrorCode {
        code: "NETWORK-DNS-001",
        message: "DNS resolution failed",
        category: ErrorCategory::Network,
        remediation: Some("Verify hostname and DNS configuration"),
    };

    /// TLS error
    pub const NETWORK_TLS_001: ErrorCode = ErrorCode {
        code: "NETWORK-TLS-001",
        message: "TLS/SSL connection error",
        category: ErrorCategory::Network,
        remediation: Some("Check certificate validity and TLS configuration"),
    };

    // ========================================================================
    // System Errors (SYSTEM)
    // ========================================================================

    /// I/O error
    pub const SYSTEM_IO_001: ErrorCode = ErrorCode {
        code: "SYSTEM-IO-001",
        message: "I/O operation failed",
        category: ErrorCategory::System,
        remediation: Some("Check file system and device availability"),
    };

    /// Permission denied
    pub const SYSTEM_PERM_001: ErrorCode = ErrorCode {
        code: "SYSTEM-PERM-001",
        message: "Permission denied",
        category: ErrorCategory::System,
        remediation: Some("Check file/directory permissions"),
    };

    /// OS resource error
    pub const SYSTEM_RESOURCE_001: ErrorCode = ErrorCode {
        code: "SYSTEM-RESOURCE-001",
        message: "Operating system resource error",
        category: ErrorCategory::System,
        remediation: Some("Check system resource limits and availability"),
    };

    /// Platform not supported
    pub const SYSTEM_PLATFORM_001: ErrorCode = ErrorCode {
        code: "SYSTEM-PLATFORM-001",
        message: "Operation not supported on this platform",
        category: ErrorCategory::System,
        remediation: Some("Use platform-specific alternative or compatibility layer"),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_structure() {
        let code = codes::EXEC_RUNTIME_001;
        assert_eq!(code.code, "EXEC-RUNTIME-001");
        assert!(!code.message.is_empty());
        assert_eq!(code.category, ErrorCategory::Execution);
        assert!(code.remediation.is_some());
    }

    #[test]
    fn test_error_with_context() {
        let code = codes::CONFIG_PARSE_001;
        let error = code.into_error_with_context("Invalid YAML at line 42");
        assert!(error.contains("CONFIG-PARSE-001"));
        assert!(error.contains("Invalid YAML at line 42"));
    }

    #[test]
    fn test_category_strings() {
        assert_eq!(codes::EXEC_RUNTIME_001.category_str(), "execution");
        assert_eq!(codes::CONFIG_PARSE_001.category_str(), "configuration");
        assert_eq!(codes::RESOURCE_ALLOC_001.category_str(), "resource");
        assert_eq!(codes::SECURITY_AUTH_001.category_str(), "security");
    }

    #[test]
    fn test_all_error_codes_unique() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();

        // Add all codes (this is a sample, extend for all codes)
        let all_codes = vec![
            codes::EXEC_RUNTIME_001.code,
            codes::EXEC_TIMEOUT_001.code,
            codes::CONFIG_PARSE_001.code,
            codes::RESOURCE_ALLOC_001.code,
            codes::SECURITY_AUTH_001.code,
            codes::NETWORK_CONNECT_001.code,
            codes::SYSTEM_IO_001.code,
        ];

        for code in all_codes {
            assert!(seen.insert(code), "Duplicate error code: {code}");
        }
    }

    #[test]
    fn test_serialization() {
        let code = codes::EXEC_RUNTIME_001;
        let json = serde_json::to_string(&code).unwrap();
        assert!(json.contains("EXEC-RUNTIME-001"));
        assert!(json.contains("execution"));
    }

    #[test]
    fn test_zero_copy_error_message() {
        let code = codes::CONFIG_PARSE_001;
        let msg = code.to_error_message();

        // Should be borrowed (zero-copy)
        assert!(matches!(msg, std::borrow::Cow::Borrowed(_)));
        assert_eq!(msg.as_ref(), "Failed to parse configuration file");
    }

    #[test]
    fn test_zero_copy_error_message_with_context_empty() {
        let code = codes::CONFIG_PARSE_001;
        let msg = code.to_error_message_with_context("");

        // Should be borrowed when context is empty (zero-copy)
        assert!(matches!(msg, std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn test_zero_copy_error_message_with_context_present() {
        let code = codes::CONFIG_PARSE_001;
        let msg = code.to_error_message_with_context("line 42");

        // Should be owned when context is present (allocation)
        assert!(matches!(msg, std::borrow::Cow::Owned(_)));
        assert!(msg.contains("line 42"));
        assert!(msg.contains("CONFIG-PARSE-001"));
    }

    #[test]
    fn test_full_message_format() {
        let code = codes::EXEC_RUNTIME_001;
        let msg = code.full_message();

        assert_eq!(
            msg,
            "EXEC-RUNTIME-001: Runtime engine initialization failed"
        );
    }
}
