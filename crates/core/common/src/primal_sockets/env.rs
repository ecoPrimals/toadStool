// SPDX-License-Identifier: AGPL-3.0-or-later
//! Environment snapshot for socket path resolution

use crate::interned_strings::socket_env;

/// Environment snapshot for socket path resolution.
///
/// Production code creates this via `SocketPathEnv::from_env()`.
/// Tests create this with explicit values - no env var mutation needed.
#[derive(Debug, Clone, Default)]
pub struct SocketPathEnv {
    /// `XDG_RUNTIME_DIR` for socket directory resolution
    pub xdg_runtime_dir: Option<String>,
    /// `USER` for fallback path construction
    pub user: Option<String>,
    /// Family ID: `TOADSTOOL_FAMILY_ID` → `TOADSTOOL_FAMILY` → `BIOMEOS_FAMILY_ID`
    pub biomeos_family_id: Option<String>,
    /// `BIOMEOS_CRYPTO_SOCKET` explicit path for the crypto capability
    pub biomeos_crypto_socket: Option<String>,
    /// `BIOMEOS_COORDINATION_SOCKET` explicit path for the coordination capability
    pub biomeos_coordination_socket: Option<String>,
    /// `BIOMEOS_STORAGE_SOCKET` explicit path for the storage capability
    pub biomeos_storage_socket: Option<String>,
    /// `BIOMEOS_ROUTING_SOCKET` explicit path for routing / MCP-style workloads
    pub biomeos_routing_socket: Option<String>,
    /// `DISCOVERY_SOCKET` — highest-precedence override for coordination / discovery
    /// capability resolution (set by `composition_nucleus.sh` to point at Songbird).
    pub discovery_socket: Option<String>,
    /// Security capability socket: `TOADSTOOL_SECURITY_SOCKET`, then deprecated legacy `BEARDOG_SOCKET`
    /// (prefer `BIOMEOS_CRYPTO_SOCKET` / capability discovery; see `interned_strings::socket_env`).
    pub legacy_security_socket: Option<String>,
    /// Coordination capability socket: `TOADSTOOL_COORDINATION_SOCKET`, then deprecated legacy `SONGBIRD_SOCKET`
    /// (prefer `BIOMEOS_COORDINATION_SOCKET` / capability discovery).
    pub legacy_coordination_socket: Option<String>,
    /// Storage capability socket: `TOADSTOOL_STORAGE_SOCKET`, then deprecated legacy `NESTGATE_SOCKET`
    /// (prefer `BIOMEOS_STORAGE_SOCKET` / capability discovery).
    pub legacy_storage_socket: Option<String>,
    /// Routing / intelligence: `TOADSTOOL_INTELLIGENCE_SOCKET`, then deprecated legacy `SQUIRREL_SOCKET`
    /// (prefer `BIOMEOS_ROUTING_SOCKET` / capability discovery).
    pub legacy_intelligence_socket: Option<String>,
    /// `TOADSTOOL_SOCKET` override for ToadStool JSON-RPC socket (primary)
    pub toadstool_socket: Option<String>,
    /// `TOADSTOOL_TARPC_SOCKET` override for ToadStool tarpc socket (hot-path)
    pub toadstool_tarpc_socket: Option<String>,
    /// `BIOMEOS_SOCKET_PATH` override for Nucleus socket
    pub biomeos_socket_path: Option<String>,
    /// `NUCLEUS_SOCKET` override for Nucleus socket
    pub nucleus_socket: Option<String>,
    /// `BIOMEOS_INSECURE` — when "1", disables BTSP (dev only; conflicts with `FAMILY_ID`)
    pub biomeos_insecure: Option<String>,
    /// Coordination service URL/path hint (not socket-specific): `TOADSTOOL_COORDINATION_ENDPOINT`,
    /// `COORDINATION_URL`, `COORDINATION_ENDPOINT`, legacy `SONGBIRD_URL` / `SONGBIRD_ENDPOINT`.
    pub coordination_connection_hint: Option<String>,
    /// Security / crypto URL/path hint: `TOADSTOOL_SECURITY_ENDPOINT`, `SECURITY_URL`,
    /// `SECURITY_ENDPOINT`, legacy `BEARDOG_URL` / `BEARDOG_ENDPOINT`.
    pub security_connection_hint: Option<String>,
    /// Storage URL/path hint: `TOADSTOOL_STORAGE_ENDPOINT`, `STORAGE_URL`, `STORAGE_ENDPOINT`,
    /// legacy `NESTGATE_URL` / `NESTGATE_ENDPOINT`.
    pub storage_connection_hint: Option<String>,
    /// AI / routing / intelligence URL/path hint: `TOADSTOOL_AI_ENDPOINT`, `AI_PROCESSING_ENDPOINT`,
    /// `TOADSTOOL_INTELLIGENCE_ENDPOINT`, legacy `SQUIRREL_URL` / `SQUIRREL_ENDPOINT`, etc.
    pub routing_connection_hint: Option<String>,
}

impl SocketPathEnv {
    /// Capture current environment (production use)
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            xdg_runtime_dir: std::env::var(socket_env::XDG_RUNTIME_DIR).ok(),
            user: std::env::var(socket_env::USER).ok(),
            biomeos_family_id: std::env::var(socket_env::TOADSTOOL_FAMILY_ID)
                .or_else(|_| std::env::var(socket_env::TOADSTOOL_FAMILY))
                .or_else(|_| std::env::var(socket_env::BIOMEOS_FAMILY_ID))
                .ok(),
            biomeos_crypto_socket: std::env::var(socket_env::BIOMEOS_CRYPTO_SOCKET).ok(),
            biomeos_coordination_socket: std::env::var(socket_env::BIOMEOS_COORDINATION_SOCKET)
                .ok(),
            biomeos_storage_socket: std::env::var(socket_env::BIOMEOS_STORAGE_SOCKET).ok(),
            biomeos_routing_socket: std::env::var(socket_env::BIOMEOS_ROUTING_SOCKET).ok(),
            discovery_socket: std::env::var(socket_env::DISCOVERY_SOCKET).ok(),
            // Deprecated identity-based fallbacks (after `TOADSTOOL_*` / `BIOMEOS_*` capability vars).
            // Prefer `BIOMEOS_*_SOCKET` and runtime capability discovery — see `interned_strings::socket_env`.
            legacy_security_socket: std::env::var(socket_env::TOADSTOOL_SECURITY_SOCKET)
                .or_else(|_| std::env::var(socket_env::LEGACY_BEARDOG_SOCKET_ENV))
                .ok(),
            legacy_coordination_socket: std::env::var(socket_env::TOADSTOOL_COORDINATION_SOCKET)
                .or_else(|_| std::env::var(socket_env::LEGACY_SONGBIRD_SOCKET_ENV))
                .ok(),
            legacy_storage_socket: std::env::var(socket_env::TOADSTOOL_STORAGE_SOCKET)
                .or_else(|_| std::env::var(socket_env::LEGACY_NESTGATE_SOCKET_ENV))
                .ok(),
            legacy_intelligence_socket: std::env::var(socket_env::TOADSTOOL_INTELLIGENCE_SOCKET)
                .or_else(|_| std::env::var(socket_env::LEGACY_SQUIRREL_SOCKET_ENV))
                .ok(),
            toadstool_socket: std::env::var(socket_env::TOADSTOOL_SOCKET).ok(),
            toadstool_tarpc_socket: std::env::var(socket_env::TOADSTOOL_TARPC_SOCKET).ok(),
            biomeos_socket_path: std::env::var(socket_env::BIOMEOS_SOCKET_PATH).ok(),
            nucleus_socket: std::env::var(socket_env::NUCLEUS_SOCKET).ok(),
            biomeos_insecure: std::env::var(socket_env::BIOMEOS_INSECURE).ok(),
            coordination_connection_hint: std::env::var(
                socket_env::TOADSTOOL_COORDINATION_ENDPOINT,
            )
            .or_else(|_| std::env::var(socket_env::COORDINATION_URL))
            .or_else(|_| std::env::var(socket_env::COORDINATION_ENDPOINT))
            .or_else(|_| std::env::var(socket_env::LEGACY_SONGBIRD_URL))
            .or_else(|_| std::env::var(socket_env::LEGACY_SONGBIRD_ENDPOINT))
            .ok(),
            security_connection_hint: std::env::var(socket_env::TOADSTOOL_SECURITY_ENDPOINT)
                .or_else(|_| std::env::var(socket_env::SECURITY_URL))
                .or_else(|_| std::env::var(socket_env::SECURITY_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::LEGACY_BEARDOG_URL))
                .or_else(|_| std::env::var(socket_env::LEGACY_BEARDOG_ENDPOINT))
                .ok(),
            storage_connection_hint: std::env::var(socket_env::TOADSTOOL_STORAGE_ENDPOINT)
                .or_else(|_| std::env::var(socket_env::STORAGE_URL))
                .or_else(|_| std::env::var(socket_env::STORAGE_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::LEGACY_NESTGATE_URL))
                .or_else(|_| std::env::var(socket_env::LEGACY_NESTGATE_ENDPOINT))
                .ok(),
            routing_connection_hint: std::env::var(socket_env::TOADSTOOL_AI_ENDPOINT)
                .or_else(|_| std::env::var(socket_env::TOADSTOOL_INTELLIGENCE_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::AI_PROCESSING_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::TOADSTOOL_AI_PROCESSING_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::INTELLIGENCE_URL))
                .or_else(|_| std::env::var(socket_env::INTELLIGENCE_ENDPOINT))
                .or_else(|_| std::env::var(socket_env::AI_PROCESSING_URL))
                .or_else(|_| std::env::var(socket_env::LEGACY_SQUIRREL_URL))
                .or_else(|_| std::env::var(socket_env::LEGACY_SQUIRREL_ENDPOINT))
                .ok(),
        }
    }

    /// Create for testing with a specific runtime dir
    #[cfg(test)]
    #[must_use]
    pub fn with_runtime_dir(dir: &str) -> Self {
        Self {
            xdg_runtime_dir: Some(dir.to_string()),
            ..Default::default()
        }
    }
}
