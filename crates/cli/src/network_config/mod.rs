// SPDX-License-Identifier: AGPL-3.0-only
//! # Songbird Network Configuration Module
//!
//! This module provides comprehensive network configuration for Songbird service mesh
//! integration, including traffic management, DNS service discovery, security policies,
//! and ingress/egress rules.

mod configurator;
mod types;

// Re-export all public types and functions
pub use configurator::*;
pub use types::*;
