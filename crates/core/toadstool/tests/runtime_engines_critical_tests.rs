// SPDX-License-Identifier: AGPL-3.0-or-later
//! Critical Path Tests for Runtime Engines
//!
//! Tests for runtime engine functionality identified in audit:
//! - Runtime engine selection and initialization
//! - Native runtime execution
//! - WASM runtime execution
//! - Container runtime execution
//! - Runtime type detection and matching
//! - Resource allocation per runtime
//! - Error handling in runtime execution
//! - Runtime engine lifecycle
//! - Performance characteristics

mod runtime_engines_critical_tests {
    pub mod backends;
    pub mod configuration;
    pub mod error_handling;
    pub mod integration;
    pub mod lifecycle;
    pub mod performance;
    pub mod resource_allocation;
    pub mod selection;
}
