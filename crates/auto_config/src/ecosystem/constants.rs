// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability keys and well-known hostnames for ecosystem discovery.

/// Capability identifiers for discovery (`WateringHole` sovereignty)
pub mod capability_keys {
    /// Discovery / coordination capability key.
    pub const DISCOVERY: &str = "discovery";
    /// Cryptographic operations capability key.
    pub const CRYPTO: &str = "crypto";
    /// Storage capability key.
    pub const STORAGE: &str = "storage";
    /// Compute / AI capability key.
    pub const COMPUTE: &str = "compute";
    /// Orchestration capability key.
    pub const ORCHESTRATION: &str = "orchestration";
    /// Self / ToadStool identity key.
    pub const SELF: &str = "self";
}

/// Well-known hostnames probed during ecosystem discovery.
/// These are mDNS/.local or public endpoints; none carry primal identity.
pub mod wellknown_hosts {
    /// Public API host used for discovery probes.
    pub const API_HOST: &str = "api.toadstool.dev";
    /// Local services mDNS-style hostname.
    pub const SERVICES_LOCAL: &str = "services.local";
    /// Ecosystem local hostname.
    pub const ECOSYSTEM_LOCAL: &str = "ecosystem.local";

    /// All well-known hosts scanned in order.
    pub const ALL: &[&str] = &[API_HOST, SERVICES_LOCAL, ECOSYSTEM_LOCAL];
}
