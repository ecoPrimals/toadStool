// SPDX-License-Identifier: AGPL-3.0-only
//! Data types for coordination: registration, discovery, and distributed locks.
//!
//! These structs are the stable surface for CLI and integrator code that registers
//! services, interprets discovery results, or holds lock handles from the
//! coordination capability layer.

use std::collections::HashMap;

/// Service information for registration
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    /// Service name
    pub name: String,
    /// Capability IDs this service provides
    pub capabilities: Vec<String>,
    /// Service endpoint URL
    pub endpoint: String,
    /// Optional metadata key-value pairs
    pub metadata: HashMap<String, String>,
}

/// Registration token from coordination service
#[derive(Debug, Clone)]
pub struct RegistrationToken {
    /// Opaque token for heartbeat and unregister
    pub token: String,
}

/// Peer service information from discovery
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer service name
    pub name: String,
    /// Peer endpoint URL
    pub endpoint: String,
    /// Capabilities the peer provides
    pub capabilities: Vec<String>,
    /// Health status (healthy, unhealthy, unknown)
    pub health: String,
}

/// Distributed lock handle for coordination
#[derive(Debug, Clone)]
pub struct LockHandle {
    /// Lock name (resource being locked)
    pub lock_name: String,
    /// Unique lock ID for release
    pub lock_id: String,
}
