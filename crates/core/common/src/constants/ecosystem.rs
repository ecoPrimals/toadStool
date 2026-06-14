// SPDX-License-Identifier: AGPL-3.0-or-later
//! Well-Known Ecosystem Identifiers
//!
//! These are names of ecosystem services that ToadStool integrates with.
//! Used only in integration modules — core logic discovers by capability.
//!
//! ## Design Principle
//!
//! ToadStool has **self-knowledge only**. It discovers other primals at runtime
//! through capability-based discovery. This module exists solely for integration
//! points (e.g., parsing manifest files, protocol strings) where knowing the
//! conventional names is unavoidable.

/// Canonical capability identifier strings for ecosystem integration.
///
/// Use these for discovery, config keys, and socket basename resolution — not legacy primal names.
pub mod capabilities {
    /// Coordination / discovery / mesh
    pub const COORDINATION: &str = "coordination";
    /// Cryptography / signing / PKI
    pub const CRYPTO: &str = "crypto";
    /// Storage / artifacts / pipelines
    pub const STORAGE: &str = "storage";
    /// Routing / MCP-style AI workloads
    pub const ROUTING: &str = "routing";
}

/// OS-release identifiers used for deployment-layer detection.
pub mod os_identifiers {
    /// biomeOS host OS identifier (matched in `/etc/os-release`).
    pub const BIOMEOS: &str = "biomeOS";
    /// SteamOS host OS identifier (matched in `/etc/os-release`).
    pub const STEAMOS: &str = "SteamOS";
}

/// Protocol strings for `NodeType` (coordination discovery API).
/// Legacy wire-format labels removed S314 (BEARDOG, SONGBIRD, NESTGATE had zero production callers).
/// Prefer `capabilities::*` constants for discovery.
pub mod node_type {
    /// Hardware infrastructure primal
    pub const TOADSTOOL: &str = "ToadStool";
}
