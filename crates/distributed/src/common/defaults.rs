// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared default constants for distributed subsystem configuration.
//!
//! Centralises "magic numbers" that were previously duplicated across
//! `security`, `crypto_integration`, `coordination_integration`,
//! `core::coordinator`, and `universal::scheduler`.

/// Default timeout for service discovery (milliseconds).
pub const DISCOVERY_TIMEOUT_MS: u64 = 5000;

/// Default interval between health checks (seconds).
pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

/// Default health check interval (milliseconds) — used by scheduler/load-balancing.
pub const HEALTH_CHECK_INTERVAL_MS: u64 = 5000;

/// Default startup timeout (milliseconds) for hosted instances.
pub const STARTUP_TIMEOUT_MS: u64 = 30_000;

/// Default number of consecutive failures before failover is triggered.
pub const FAILOVER_THRESHOLD: u32 = 3;

/// Default maximum retry attempts for transient failures.
pub const MAX_RETRIES: u32 = 3;

/// Default number of consecutive failures before circuit opens.
pub const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// Default maximum recursive hosting depth.
pub const MAX_HOSTING_DEPTH: u32 = 3;

/// Default resource sharing ratio (fraction of capacity made available).
pub const SHARING_RATIO: f64 = 0.8;

/// Default priority boost multiplier for high-priority workloads.
pub const PRIORITY_BOOST: f64 = 1.2;
