// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    missing_docs,
    reason = "each pub const is the env var name string; module docs describe usage"
)]
//! Environment variable names for runtime socket discovery (`SocketPathEnv::from_env`).
//!
//! Prefer capability-prefixed vars (`BIOMEOS_*_SOCKET`, `TOADSTOOL_*_SOCKET`). Legacy
//! `LEGACY_*_SOCKET_ENV` names are identity-based fallbacks only; see
//! `capabilities::*` and capability discovery in the wateringHole standards.

pub const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";
pub const USER: &str = "USER";

pub const TOADSTOOL_FAMILY_ID: &str = "TOADSTOOL_FAMILY_ID";
pub const TOADSTOOL_FAMILY: &str = "TOADSTOOL_FAMILY";
pub const BIOMEOS_FAMILY_ID: &str = "BIOMEOS_FAMILY_ID";

/// Preferred: explicit crypto / security socket path (capability-based).
pub const BIOMEOS_CRYPTO_SOCKET: &str = "BIOMEOS_CRYPTO_SOCKET";
/// Preferred: explicit coordination socket path (capability-based).
pub const BIOMEOS_COORDINATION_SOCKET: &str = "BIOMEOS_COORDINATION_SOCKET";
/// Preferred: explicit storage socket path (capability-based).
pub const BIOMEOS_STORAGE_SOCKET: &str = "BIOMEOS_STORAGE_SOCKET";
/// Preferred: explicit routing / MCP-style socket path (capability-based).
pub const BIOMEOS_ROUTING_SOCKET: &str = "BIOMEOS_ROUTING_SOCKET";

pub const TOADSTOOL_SECURITY_SOCKET: &str = "TOADSTOOL_SECURITY_SOCKET";
pub const TOADSTOOL_COORDINATION_SOCKET: &str = "TOADSTOOL_COORDINATION_SOCKET";
pub const TOADSTOOL_STORAGE_SOCKET: &str = "TOADSTOOL_STORAGE_SOCKET";
pub const TOADSTOOL_INTELLIGENCE_SOCKET: &str = "TOADSTOOL_INTELLIGENCE_SOCKET";

/// **Deprecated** (identity-based). Prefer `BIOMEOS_CRYPTO_SOCKET` or capability discovery
/// (`interned_strings::capabilities::CRYPTO`).
pub const LEGACY_BEARDOG_SOCKET_ENV: &str = "BEARDOG_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_COORDINATION_SOCKET` or
/// `interned_strings::capabilities::COORDINATION`.
pub const LEGACY_SONGBIRD_SOCKET_ENV: &str = "SONGBIRD_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_STORAGE_SOCKET` or
/// `interned_strings::capabilities::STORAGE`.
pub const LEGACY_NESTGATE_SOCKET_ENV: &str = "NESTGATE_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_ROUTING_SOCKET` or
/// `interned_strings::capabilities::ROUTING`.
pub const LEGACY_SQUIRREL_SOCKET_ENV: &str = "SQUIRREL_SOCKET";

pub const TOADSTOOL_SOCKET: &str = "TOADSTOOL_SOCKET";
pub const BIOMEOS_SOCKET_PATH: &str = "BIOMEOS_SOCKET_PATH";
pub const NUCLEUS_SOCKET: &str = "NUCLEUS_SOCKET";
pub const BIOMEOS_INSECURE: &str = "BIOMEOS_INSECURE";
