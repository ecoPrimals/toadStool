//! Fault Injection Tests - Error scenarios and graceful degradation
//!
//! **Purpose**: Validate error handling under failure conditions
//! **Coverage**: Invalid inputs, boundary cases, error propagation
//! **Deep Debt**: Graceful errors (Result), no panics

pub mod invalid_inputs;
pub mod boundary_cases;
pub mod error_propagation;
