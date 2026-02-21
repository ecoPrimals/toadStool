//! Fault injection and chaos engineering tests
//!
//! This module provides access to fault injection tests for the ToadStool platform.

#[path = "chaos/fault_injection.rs"]
mod fault_injection;

#[path = "chaos/resilience_tests.rs"]
mod resilience_tests;
