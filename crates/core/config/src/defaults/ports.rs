// SPDX-License-Identifier: AGPL-3.0-or-later
//! Port range defaults for dynamic allocation
//!
//! # Example
//!
//! ```rust
//! use toadstool_config::defaults::ports;
//!
//! // Allocate port range for containers
//! let start_port = ports::CONTAINER_START;
//! let end_port = ports::CONTAINER_END;
//! for port in start_port..=end_port {
//!     // Use port...
//! }
//!
//! // Service mesh sidecar ports
//! let proxy_port = ports::SIDECAR_LISTEN;
//! let admin_port = ports::SIDECAR_ADMIN;
//! ```

/// Default starting port for container allocations
pub const CONTAINER_START: u16 = 3000;

/// Default ending port for container allocations
pub const CONTAINER_END: u16 = 3999;

/// Default starting port for general port range
pub const RANGE_START: u16 = 8080;

/// Default ending port for general port range
pub const RANGE_END: u16 = 8999;

/// Default service mesh sidecar listen port
pub const SIDECAR_LISTEN: u16 = 15001;

/// Default service mesh sidecar admin port
pub const SIDECAR_ADMIN: u16 = 15000;

// ---------------------------------------------------------------------------
// Discovery / IPC cold-start fallbacks (canonical definitions in `toadstool-common`)
// ---------------------------------------------------------------------------

pub use toadstool_common::constants::discovery_ports::{
    DISCOVERY_HTTP_FALLBACK, DISCOVERY_LOCALHOST_FALLBACK_BASE, DISPLAY_IPC_FALLBACK,
};
