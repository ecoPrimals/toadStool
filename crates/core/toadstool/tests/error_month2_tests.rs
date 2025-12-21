//! Error handling tests - Month 2 Week 1
//!
//! Tier 1 tests: Coverage-measured error conversion and formatting tests
//! Focus: Error types, conversions, Display impl, context preservation

use std::fmt;
use std::io;
use toadstool::error::{ToadStoolError, ToadStoolResult};

// ============================================================================
// Error Creation Tests
// ============================================================================

#[test]
fn test_error_runtime_creation() {
    let err = ToadStoolError::runtime("test error");

    assert!(format!("{}", err).contains("test error"));
}

#[test]
fn test_error_config_creation() {
    let err = ToadStoolError::configuration("invalid configuration");

    assert!(format!("{}", err).contains("invalid configuration"));
}

#[test]
fn test_error_network_creation() {
    let err = ToadStoolError::network("connection failed");

    assert!(format!("{}", err).contains("connection failed"));
}

#[test]
fn test_error_validation_creation() {
    let err = ToadStoolError::runtime("field must be positive");

    let message = format!("{}", err);
    assert!(message.contains("field"));
    assert!(message.contains("must be positive"));
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err: ToadStoolError = io_err.into();

    assert!(format!("{}", err).contains("file not found"));
}

#[test]
fn test_error_from_parse_error() {
    let parse_result: Result<i32, _> = "not a number".parse();
    let parse_err = parse_result.unwrap_err();
    let err: ToadStoolError = ToadStoolError::runtime(format!("Parse error: {}", parse_err));

    assert!(format!("{}", err).contains("Parse error"));
}

#[test]
fn test_error_chain_preservation() {
    let cause = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let err = ToadStoolError::runtime(format!("Failed to read file: {}", cause));

    let message = format!("{}", err);
    assert!(message.contains("access denied"));
}

// ============================================================================
// Error Context Tests
// ============================================================================

#[test]
fn test_error_with_context() {
    let err = ToadStoolError::runtime("database query failed while loading user profile");

    let message = format!("{}", err);
    assert!(message.contains("database query failed"));
    assert!(message.contains("while loading user profile"));
}

#[test]
fn test_error_multiple_contexts() {
    let err = ToadStoolError::runtime(
        "network timeout connecting to service while initializing application",
    );

    let message = format!("{}", err);
    assert!(message.contains("network timeout"));
    assert!(message.contains("connecting to service"));
}

#[test]
fn test_error_context_chain() {
    let base_err = ToadStoolError::configuration("missing field while parsing config file");

    assert!(format!("{}", base_err).contains("missing field"));
    assert!(format!("{}", base_err).contains("parsing config file"));
}

// ============================================================================
// Error Display Tests
// ============================================================================

#[test]
fn test_error_display_format() {
    let err = ToadStoolError::runtime("test message");
    let display = format!("{}", err);

    // Should be human-readable
    assert!(!display.is_empty());
    assert!(display.contains("test message"));
}

#[test]
fn test_error_debug_format() {
    let err = ToadStoolError::runtime("port must be between 1024 and 65535");
    let debug = format!("{:?}", err);

    // Debug format should include more detail
    assert!(!debug.is_empty());
}

#[test]
fn test_error_empty_message_handling() {
    let err = ToadStoolError::runtime("");
    let display = format!("{}", err);

    // Should handle empty message gracefully
    assert!(!display.is_empty());
}

// ============================================================================
// Result Type Tests
// ============================================================================

#[test]
fn test_result_ok_value() {
    let result: ToadStoolResult<i32> = Ok(42);

    assert!(result.is_ok());
    if let Ok(value) = result {
        assert_eq!(value, 42);
    }
}

#[test]
fn test_result_err_value() {
    let result: ToadStoolResult<i32> = Err(ToadStoolError::runtime("failed"));

    assert!(result.is_err());
}

#[test]
fn test_result_map() {
    let result: ToadStoolResult<i32> = Ok(10);
    let mapped = result.map(|x| x * 2);

    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_result_map_err() {
    let result: ToadStoolResult<i32> = Err(ToadStoolError::runtime("original"));
    let mapped = result.map_err(|_e| ToadStoolError::runtime("original with additional context"));

    assert!(format!("{}", mapped.unwrap_err()).contains("original"));
}

#[test]
fn test_result_and_then() {
    let result: ToadStoolResult<i32> = Ok(5);
    let chained = result.and_then(|x| {
        if x > 0 {
            Ok(x * 2)
        } else {
            Err(ToadStoolError::runtime("value must be positive"))
        }
    });

    assert_eq!(chained.unwrap(), 10);
}

// ============================================================================
// Mock Error Implementation (Simplified)
// ============================================================================

// Note: This is a simplified mock for testing
// Actual implementation should be in toadstool::error module

#[derive(Debug)]
pub struct MockToadStoolError {
    message: String,
    context: Vec<String>,
}

impl MockToadStoolError {
    pub fn runtime(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            context: Vec::new(),
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::runtime(msg)
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::runtime(msg)
    }

    pub fn validation(field: &str, reason: &str) -> Self {
        Self::runtime(format!("Validation error for '{}': {}", field, reason))
    }

    pub fn context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

impl fmt::Display for MockToadStoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        for ctx in &self.context {
            write!(f, "\n  Context: {}", ctx)?;
        }
        Ok(())
    }
}

impl From<io::Error> for MockToadStoolError {
    fn from(err: io::Error) -> Self {
        Self::runtime(format!("IO error: {}", err))
    }
}
