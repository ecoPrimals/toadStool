// SPDX-License-Identifier: AGPL-3.0-only
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

/// Well-known primal service names used in integration modules.
/// Core logic should discover by capability, not by name.
///
/// Prefer `interned_strings::capabilities::*` for discovery.
/// These exist only for protocol compatibility (parsing manifests, socket paths).
#[deprecated(
    since = "0.4.0",
    note = "Use interned_strings::capabilities::* for discovery. These are for protocol compat only."
)]
pub mod well_known {
    /// Coordination / discovery service (legacy name)
    pub const SONGBIRD: &str = "songbird";
    /// Cryptography / security service (legacy name)
    pub const BEARDOG: &str = "beardog";
    /// Storage / artifact service (legacy name)
    pub const NESTGATE: &str = "nestgate";
    /// Platform management service (legacy name)
    pub const SQUIRREL: &str = "squirrel";
    /// Ecosystem orchestration service (legacy name)
    pub const BIOMEOS: &str = "biomeos";
}

/// Protocol strings for `NodeType` (Songbird discovery API).
/// Used when parsing discovery responses from coordination services.
pub mod node_type {
    /// Hardware infrastructure primal
    pub const TOADSTOOL: &str = "ToadStool";
    /// Storage / artifact service
    pub const NESTGATE: &str = "NestGate";
    /// Cryptography / security service
    pub const BEARDOG: &str = "BearDog";
    /// Coordination / discovery service
    pub const SONGBIRD: &str = "Songbird";
}
