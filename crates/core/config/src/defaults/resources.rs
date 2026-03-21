// SPDX-License-Identifier: AGPL-3.0-only
//! Resource limits
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::resources;
//!
//! // Get resource defaults
//! let workers = resources::WORKER_THREADS;
//! let max_conns = resources::MAX_CONNECTIONS;
//!
//! // Use Kubernetes-style resource specifications
//! let cpu_limit = resources::SIDECAR_CPU_LIMIT;      // "200m" = 200 millicores
//! let mem_limit = resources::SIDECAR_MEMORY_LIMIT;  // "256Mi" = 256 mebibytes
//!
//! // Validate resource values
//! assert!(workers > 0);
//! assert!(max_conns > 0);
//! assert!(!cpu_limit.is_empty());
//! assert!(!mem_limit.is_empty());
//! ```

/// Default worker thread count
pub const WORKER_THREADS: usize = 4;

/// Default max connections
pub const MAX_CONNECTIONS: usize = 1000;

/// Default retry count
pub const RETRY_COUNT: u32 = 3;

/// Default sidecar CPU limit
pub const SIDECAR_CPU_LIMIT: &str = "200m";

/// Default sidecar memory limit
pub const SIDECAR_MEMORY_LIMIT: &str = "256Mi";

/// Default sidecar CPU request
pub const SIDECAR_CPU_REQUEST: &str = "100m";

/// Default sidecar memory request
pub const SIDECAR_MEMORY_REQUEST: &str = "128Mi";
