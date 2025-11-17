//! Connection management for ecosystem services
//!
//! This module handles maintaining connections to ecosystem services,
//! including connection pooling, retry logic, and state tracking.

use std::net::SocketAddr;

use anyhow::{Context, Result};

// ConnectionManager removed - was defined but never constructed.
// Connection state is managed directly in EcosystemIntegrator
// via the connections HashMap field for better ergonomics.

/// Get local address for registration
pub fn get_local_address() -> Result<SocketAddr> {
    // Default to localhost with a standard port
    "127.0.0.1:8084"
        .parse()
        .context("Failed to parse local address")
}
