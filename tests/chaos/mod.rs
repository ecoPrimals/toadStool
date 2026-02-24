//! Chaos Test Module
//!
//! Declares all chaos test submodules. Tests in submodules are auto-discovered by Cargo.

pub mod helpers;
pub mod fault_injection;
pub mod resilience_tests;
pub mod real_fault_injection;
pub mod network_failures_month2;
pub mod network_chaos_e2e;
pub mod fault_injection_recovery_e2e;
pub mod timeout_scenarios_month2;
pub mod resource_exhaustion_month2;
pub mod real_network_partition;
pub mod resource_exhaustion_e2e;

// Re-export helpers for convenience
pub use helpers::*;

