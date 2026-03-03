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
    pub const SONGBIRD: &str = "songbird";
    pub const BEARDOG: &str = "beardog";
    pub const NESTGATE: &str = "nestgate";
    pub const SQUIRREL: &str = "squirrel";
    pub const BIOMEOS: &str = "biomeos";
}

/// Protocol strings for `NodeType` (Songbird discovery API).
/// Used when parsing discovery responses from coordination services.
pub mod node_type {
    pub const TOADSTOOL: &str = "ToadStool";
    pub const NESTGATE: &str = "NestGate";
    pub const BEARDOG: &str = "BearDog";
    pub const SONGBIRD: &str = "Songbird";
}
