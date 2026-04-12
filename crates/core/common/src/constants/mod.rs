// SPDX-License-Identifier: AGPL-3.0-or-later
//! Centralized Constants Module
//!
//! This module provides a single source of truth for all hardcoded values
//! across the ToadStool codebase, improving maintainability and reducing
//! technical debt.
//!
//! ## Zero-Copy Optimization
//! String constants use `&'static str` for zero-cost sharing across the codebase.

pub mod compute;
pub mod discovery_ports;
pub mod display;
pub mod ecosystem;
pub mod jsonrpc;
pub mod network;
pub mod primal_identity;
pub mod resources;
pub mod timeouts;
pub mod versions;

// Re-export commonly used constants (narrowed from wildcards; submodules remain for full access)
pub use network::{
    DEFAULT_HOSTNAME, HTTP_PROTOCOL, HTTPS_PROTOCOL, LOCALHOST_IPV4, LOCALHOST_IPV6,
};
pub use primal_identity::{CAPABILITY_DOMAIN, PRIMAL_NAME};
