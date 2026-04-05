// SPDX-License-Identifier: AGPL-3.0-or-later
//! Base configuration types used across the codebase
//!
//! This module provides common patterns for configuration structs,
//! enabling code reuse and consistency across different configuration types.
//!
//! # Design Pattern
//!
//! These base types use composition via `#[serde(flatten)]` to allow
//! configuration structs to embed common patterns while maintaining
//! their own specific fields.
//!
//! # Example
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use toadstool_common::config_bases::{TimeoutConfig, HealthCheckConfig};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct MyServiceConfig {
//!     pub service_name: String,
//!     #[serde(flatten)]
//!     pub timeouts: TimeoutConfig,
//!     #[serde(flatten)]
//!     pub health_check: HealthCheckConfig,
//! }
//! ```
//!
//! Implementation is split into focused submodules (`timeout`, `health`,
//! `resources_validation`, `endpoint_retry_pool`, `cache_telemetry`) and
//! re-exported here so callers keep a stable `config_bases::*` surface.
//!
//! - `timeout`: `TimeoutConfig` and duration defaults for connections and I/O.
//! - `health`: `HealthCheckConfig`, `HttpHealthCheckConfig`.
//! - `resources_validation`: `ResourceLimit`, `BaseResourceConfig`, `ValidationConfig`.
//! - `endpoint_retry_pool`: `BackendEndpoint`, `RetryConfig`, `ConnectionPoolConfig`.
//! - `cache_telemetry`: `CacheConfig`, `TelemetryConfig`.

/// Shared `serde` default functions referenced by attribute paths in submodules.
pub(in crate::config_bases) mod serde_defaults {
    pub(in crate::config_bases) const fn default_true() -> bool {
        true
    }
}

mod cache_telemetry;
mod endpoint_retry_pool;
mod health;
mod resources_validation;
mod timeout;

pub use cache_telemetry::{CacheConfig, TelemetryConfig};
pub use endpoint_retry_pool::{BackendEndpoint, ConnectionPoolConfig, RetryConfig};
pub use health::{HealthCheckConfig, HttpHealthCheckConfig};
pub use resources_validation::{BaseResourceConfig, ResourceLimit, ValidationConfig};
pub use timeout::TimeoutConfig;

#[cfg(test)]
mod tests;
