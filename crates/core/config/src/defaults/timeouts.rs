// SPDX-License-Identifier: AGPL-3.0-only
//! Timeout defaults (in milliseconds)
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::timeouts;
//! use std::time::Duration;
//!
//! // Create timeout durations
//! let exec_timeout = Duration::from_millis(timeouts::EXECUTION_MS);
//! let conn_timeout = Duration::from_millis(timeouts::CONNECTION_MS);
//! let req_timeout = Duration::from_millis(timeouts::REQUEST_MS);
//!
//! // Validate timeout values
//! assert!(exec_timeout.as_secs() > 0);
//! assert!(conn_timeout.as_secs() > 0);
//! assert!(req_timeout.as_secs() > 0);
//! ```
//!
//! See also: `durations` module for helper functions that return `Duration` values directly.

/// Default execution timeout for tasks
pub const EXECUTION_MS: u64 = 30_000;

/// Default health check interval
pub const HEALTH_CHECK_MS: u64 = 5_000;

/// Default connection timeout
pub const CONNECTION_MS: u64 = 5_000;

/// Default request timeout
pub const REQUEST_MS: u64 = 30_000;

/// Default idle timeout
pub const IDLE_MS: u64 = 60_000;

/// Default discovery timeout
pub const DISCOVERY_MS: u64 = 5_000;

/// Default discovery interval
pub const DISCOVERY_INTERVAL_MS: u64 = 30_000;

/// Default keepalive timeout (in seconds)
pub const KEEPALIVE_SEC: u64 = 60;
