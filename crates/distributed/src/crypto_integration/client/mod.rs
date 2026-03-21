// SPDX-License-Identifier: AGPL-3.0-only
//! Crypto service client - Capability-based discovery
//!
//! **Design Philosophy (Infant Discovery)**:
//! - Async-first: Non-blocking operations
//! - Resilient: Retry logic, circuit breaker patterns
//! - Observable: Metrics and health checks
//! - Zero hardcoding: Endpoints discovered at runtime by capability
//! - Multi-vendor: Works with ANY crypto service (BearDog, Vault, KMS, etc.)
//!
//! Submodules: [`discovery`] for capability-based service lookup; [`operations`] for RPC calls;
//! the `tests` submodule (crate tests only) covers configuration and wire-format types.

mod discovery;
mod operations;

#[cfg(test)]
mod tests;

pub use discovery::CryptoServiceDiscovery;
pub use operations::CryptoServiceClient;
