// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination service client - Capability-based discovery
//!
//! **Design Philosophy (Infant Discovery)**:
//! - Async-first: Non-blocking operations
//! - Resilient: Retry logic, circuit breaker patterns
//! - Observable: Metrics and health checks
//! - Zero hardcoding: Endpoints discovered at runtime by capability
//! - Multi-vendor: Works with ANY coordination service (Songbird, Consul, etcd, K8s, etc.)

mod discovery;
mod rpc;

#[cfg(test)]
mod tests;

pub use discovery::CoordinationDiscovery;
pub use rpc::CoordinationClient;
