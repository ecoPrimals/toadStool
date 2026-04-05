// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Orchestration network configuration
//!
//! Network configuration for the coordination / service-mesh stack: traffic management,
//! DNS discovery, security policies, and ingress/egress rules.

mod configurator;
mod types;

// Re-export all public types and functions
pub use configurator::*;
pub use types::*;
