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
pub const TOADSTOOL_TARPC_SOCKET: &str = "TOADSTOOL_TARPC_SOCKET";
pub const BIOMEOS_SOCKET_PATH: &str = "BIOMEOS_SOCKET_PATH";
pub const NUCLEUS_SOCKET: &str = "NUCLEUS_SOCKET";
pub const BIOMEOS_INSECURE: &str = "BIOMEOS_INSECURE";

// Connection hint env vars (URL/endpoint discovery chain).
// Capability-based vars are preferred; legacy primal-identity vars are fallbacks.

pub const TOADSTOOL_COORDINATION_ENDPOINT: &str = "TOADSTOOL_COORDINATION_ENDPOINT";
pub const COORDINATION_URL: &str = "COORDINATION_URL";
pub const COORDINATION_ENDPOINT: &str = "COORDINATION_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `COORDINATION_URL`.
pub const LEGACY_SONGBIRD_URL: &str = "SONGBIRD_URL";
/// **Deprecated** (identity-based). Prefer `COORDINATION_ENDPOINT`.
pub const LEGACY_SONGBIRD_ENDPOINT: &str = "SONGBIRD_ENDPOINT";

pub const TOADSTOOL_SECURITY_ENDPOINT: &str = "TOADSTOOL_SECURITY_ENDPOINT";
pub const SECURITY_URL: &str = "SECURITY_URL";
pub const SECURITY_ENDPOINT: &str = "SECURITY_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `SECURITY_URL`.
pub const LEGACY_BEARDOG_URL: &str = "BEARDOG_URL";
/// **Deprecated** (identity-based). Prefer `SECURITY_ENDPOINT`.
pub const LEGACY_BEARDOG_ENDPOINT: &str = "BEARDOG_ENDPOINT";

pub const TOADSTOOL_STORAGE_ENDPOINT: &str = "TOADSTOOL_STORAGE_ENDPOINT";
pub const STORAGE_URL: &str = "STORAGE_URL";
pub const STORAGE_ENDPOINT: &str = "STORAGE_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `STORAGE_URL`.
pub const LEGACY_NESTGATE_URL: &str = "NESTGATE_URL";
/// **Deprecated** (identity-based). Prefer `STORAGE_ENDPOINT`.
pub const LEGACY_NESTGATE_ENDPOINT: &str = "NESTGATE_ENDPOINT";

pub const TOADSTOOL_AI_ENDPOINT: &str = "TOADSTOOL_AI_ENDPOINT";
pub const TOADSTOOL_INTELLIGENCE_ENDPOINT: &str = "TOADSTOOL_INTELLIGENCE_ENDPOINT";
pub const AI_PROCESSING_ENDPOINT: &str = "AI_PROCESSING_ENDPOINT";
pub const TOADSTOOL_AI_PROCESSING_ENDPOINT: &str = "TOADSTOOL_AI_PROCESSING_ENDPOINT";
pub const INTELLIGENCE_URL: &str = "INTELLIGENCE_URL";
pub const INTELLIGENCE_ENDPOINT: &str = "INTELLIGENCE_ENDPOINT";
pub const AI_PROCESSING_URL: &str = "AI_PROCESSING_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_URL`.
pub const LEGACY_SQUIRREL_URL: &str = "SQUIRREL_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_ENDPOINT`.
pub const LEGACY_SQUIRREL_ENDPOINT: &str = "SQUIRREL_ENDPOINT";

// HTTP URL overrides (parallel naming to `*_ENDPOINT`; used by discovery fallbacks and demos).
pub const TOADSTOOL_COORDINATION_URL: &str = "TOADSTOOL_COORDINATION_URL";
pub const TOADSTOOL_SECURITY_URL: &str = "TOADSTOOL_SECURITY_URL";
pub const TOADSTOOL_STORAGE_URL: &str = "TOADSTOOL_STORAGE_URL";

// Capability service ports (HTTP fallback / local bind hints).
pub const TOADSTOOL_COORDINATION_PORT: &str = "TOADSTOOL_COORDINATION_PORT";
pub const TOADSTOOL_SECURITY_PORT: &str = "TOADSTOOL_SECURITY_PORT";
pub const TOADSTOOL_STORAGE_PORT: &str = "TOADSTOOL_STORAGE_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_COORDINATION_PORT`.
pub const TOADSTOOL_SONGBIRD_PORT: &str = "TOADSTOOL_SONGBIRD_PORT";

// Discovery bootstrap and bind hints.
pub const TOADSTOOL_MDNS_ENABLE: &str = "TOADSTOOL_MDNS_ENABLE";
pub const TOADSTOOL_MDNS_REQUIRE: &str = "TOADSTOOL_MDNS_REQUIRE";
pub const TOADSTOOL_DISCOVERY_FALLBACKS: &str = "TOADSTOOL_DISCOVERY_FALLBACKS";
pub const TOADSTOOL_ENV: &str = "TOADSTOOL_ENV";
pub const TOADSTOOL_BIND_HOST: &str = "TOADSTOOL_BIND_HOST";
pub const BIND_HOST: &str = "BIND_HOST";
pub const TOADSTOOL_BIND_ADDRESS: &str = "TOADSTOOL_BIND_ADDRESS";
pub const TOADSTOOL_TCP_BIND_ADDRESS: &str = "TOADSTOOL_TCP_BIND_ADDRESS";
pub const TOADSTOOL_STANDALONE: &str = "TOADSTOOL_STANDALONE";

// Auth tokens (coordination plane registration).
pub const COORDINATION_AUTH_TOKEN: &str = "COORDINATION_AUTH_TOKEN";
/// **Deprecated** (identity-based). Prefer `COORDINATION_AUTH_TOKEN`.
pub const LEGACY_SONGBIRD_AUTH_TOKEN: &str = "SONGBIRD_AUTH_TOKEN";

// Optional capability endpoint env names (not socket-specific).
pub const AUTHENTICATION_URL: &str = "AUTHENTICATION_URL";
pub const NLP_URL: &str = "NLP_URL";
pub const TOADSTOOL_ENTROPY_SERVICE_URL: &str = "TOADSTOOL_ENTROPY_SERVICE_URL";
