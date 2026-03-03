// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validation threshold constants
//!
//! These constants define minimum and maximum values for various configuration
//! parameters to ensure safe and reasonable operation.
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::validation;
//!
//! // Validate cache configuration
//! fn validate_cache_size(size: usize) -> Result<(), String> {
//!     if size < validation::MIN_CACHE_SIZE {
//!         return Err(format!("Cache size {} below minimum {}", size, validation::MIN_CACHE_SIZE));
//!     }
//!     if size > validation::MAX_CACHE_SIZE {
//!         return Err(format!("Cache size {} exceeds maximum {}", size, validation::MAX_CACHE_SIZE));
//!     }
//!     Ok(())
//! }
//!
//! // Validate worker thread count
//! fn validate_workers(count: usize) -> Result<(), String> {
//!     if count < validation::MIN_WORKER_THREADS {
//!         return Err(format!("Worker count {} below minimum {}", count, validation::MIN_WORKER_THREADS));
//!     }
//!     if count > validation::MAX_WORKER_THREADS {
//!         return Err(format!("Worker count {} exceeds maximum {}", count, validation::MAX_WORKER_THREADS));
//!     }
//!     Ok(())
//! }
//! ```

/// Minimum cache size (entries)
pub const MIN_CACHE_SIZE: usize = 100;

/// Maximum cache size (entries)
pub const MAX_CACHE_SIZE: usize = 100_000;

/// Minimum cache TTL (seconds)
pub const MIN_CACHE_TTL_SECS: u64 = 60;

/// Maximum cache TTL (seconds)
pub const MAX_CACHE_TTL_SECS: u64 = 86_400; // 24 hours

/// Minimum flush interval (seconds)
pub const MIN_FLUSH_INTERVAL_SECS: u64 = 10;

/// Maximum flush interval (seconds)
pub const MAX_FLUSH_INTERVAL_SECS: u64 = 3600; // 1 hour

/// Minimum worker thread count
pub const MIN_WORKER_THREADS: usize = 1;

/// Maximum worker thread count
pub const MAX_WORKER_THREADS: usize = 128;

/// Minimum connection pool size
pub const MIN_POOL_SIZE: usize = 1;

/// Maximum connection pool size
pub const MAX_POOL_SIZE: usize = 10_000;

/// Minimum timeout value (milliseconds)
pub const MIN_TIMEOUT_MS: u64 = 100;

/// Maximum timeout value (milliseconds)
pub const MAX_TIMEOUT_MS: u64 = 3_600_000; // 1 hour

/// Minimum retry attempts
pub const MIN_RETRY_ATTEMPTS: u32 = 0;

/// Maximum retry attempts
pub const MAX_RETRY_ATTEMPTS: u32 = 10;

/// Minimum port number
pub const MIN_PORT: u16 = 1024; // Avoid privileged ports

/// Maximum port number
pub const MAX_PORT: u16 = 65535;
