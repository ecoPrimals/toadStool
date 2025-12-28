//! # Unified Error System for ToadStool Platform
//!
//! This module provides a comprehensive, hierarchical error system for the entire ToadStool platform.
//! It consolidates all error types into a cohesive 3-tier architecture:
//!
//! - **Tier 1**: `ToadStoolError` - Top-level error enum with high-level categories
//! - **Tier 2**: Specialized errors (`ExecutionError`, `ConfigError`, etc.) - Domain-specific errors
//! - **Tier 3**: Result type aliases for convenient error handling
//!
//! ## Design Principles
//!
//! 1. **Single Source of Truth**: All ToadStool errors flow through this module
//! 2. **Proper Error Chaining**: Errors preserve context through the call stack
//! 3. **Clear Categorization**: Errors are organized by domain (execution, config, resource, etc.)
//! 4. **Rich Context**: Errors include relevant information for debugging
//! 5. **Easy Conversion**: Automatic conversions from common error types
//!
//! ## Module Organization
//!
//! - [`types`] - Error type definitions (enums, structs, result aliases)
//! - [`constructors`] - Helper constructors for ergonomic error creation
//! - [`conversions`] - From implementations for external types
//! - [`context`] - Context builders and error chaining
//! - [`extensions`] - Optional extensions and wrappers
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use toadstool_common::error::{ToadStoolError, ToadStoolResult, ExecutionError};
//!
//! fn execute_workload(id: &str) -> ToadStoolResult<String> {
//!     // Use the error system
//!     Err(ToadStoolError::execution(ExecutionError::RuntimeFailure {
//!         runtime: "container".to_string(),
//!         workload_id: id.to_string(),
//!         reason: "Image not found".to_string(),
//!     }))
//! }
//! ```
//!
//! ## Backward Compatibility
//!
//! All public types and functions from the original `error.rs` are re-exported
//! at the module root, ensuring complete backward compatibility.

// Module declarations
pub mod constructors;
pub mod context;
pub mod conversions;
pub mod extensions;
pub mod types;

// Re-export all public types for backward compatibility
pub use types::{
    ConfigError, ConfigResult, ExecutionError, ExecutionResult, IntegrationError,
    IntegrationResult, NetworkError, NetworkResult, ResourceError, ResourceResult, SecurityError,
    SecurityResult, SystemError, SystemResult, ToadStoolError, ToadStoolResult,
};

pub use context::ToadStoolErrorExt;
pub use extensions::{ToadStoolErrorWithCode, ToadStoolResultWithCode};
