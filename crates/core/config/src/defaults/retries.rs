// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retry and resilience defaults
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::retries;
//! use std::time::Duration;
//!
//! // Configure retry logic
//! let max_attempts = retries::MAX_ATTEMPTS;
//! let initial_backoff = Duration::from_millis(retries::BACKOFF_MS);
//! let max_backoff = Duration::from_millis(retries::MAX_BACKOFF_MS);
//! let multiplier = retries::BACKOFF_MULTIPLIER;
//!
//! // Validate retry configuration
//! assert!(max_attempts > 0);
//! assert!(initial_backoff.as_millis() > 0);
//! assert!(max_backoff >= initial_backoff);
//! assert!(multiplier > 1.0);
//! ```

/// Default maximum retry attempts
pub const MAX_ATTEMPTS: u32 = 3;

/// Default retry backoff duration (in milliseconds)
pub const BACKOFF_MS: u64 = 1_000;

/// Default exponential backoff multiplier
pub const BACKOFF_MULTIPLIER: f64 = 2.0;

/// Default maximum backoff duration (in milliseconds)
pub const MAX_BACKOFF_MS: u64 = 30_000;
