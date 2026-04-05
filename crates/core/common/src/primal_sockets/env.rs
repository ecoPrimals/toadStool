// SPDX-License-Identifier: AGPL-3.0-or-later
//! Environment snapshot for socket path resolution

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
    /// `BIOMEOS_FAMILY_ID` or `TOADSTOOL_FAMILY` for family-scoped paths
    pub biomeos_family_id: Option<String>,
    /// `BIOMEOS_CRYPTO_SOCKET` explicit path for the crypto capability
    pub biomeos_crypto_socket: Option<String>,
    /// `BIOMEOS_COORDINATION_SOCKET` explicit path for the coordination capability
    pub biomeos_coordination_socket: Option<String>,
    /// `BIOMEOS_STORAGE_SOCKET` explicit path for the storage capability
    pub biomeos_storage_socket: Option<String>,
    /// `BIOMEOS_ROUTING_SOCKET` explicit path for routing / MCP-style workloads
    pub biomeos_routing_socket: Option<String>,
    /// Security capability socket: `TOADSTOOL_SECURITY_SOCKET`, then legacy `BEARDOG_SOCKET`
    pub legacy_security_socket: Option<String>,
    /// Coordination capability socket: `TOADSTOOL_COORDINATION_SOCKET`, then legacy `SONGBIRD_SOCKET`
    pub legacy_coordination_socket: Option<String>,
    /// Storage capability socket: `TOADSTOOL_STORAGE_SOCKET`, then legacy `NESTGATE_SOCKET`
    pub legacy_storage_socket: Option<String>,
    /// Routing / intelligence capability socket: `TOADSTOOL_INTELLIGENCE_SOCKET`, then legacy `SQUIRREL_SOCKET`
    pub legacy_intelligence_socket: Option<String>,
    /// `TOADSTOOL_SOCKET` override for ToadStool main socket
    pub toadstool_socket: Option<String>,
    /// `BIOMEOS_SOCKET_PATH` override for Nucleus socket
    pub biomeos_socket_path: Option<String>,
    /// `NUCLEUS_SOCKET` override for Nucleus socket
    pub nucleus_socket: Option<String>,
}

impl SocketPathEnv {
    /// Capture current environment (production use)
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            xdg_runtime_dir: std::env::var("XDG_RUNTIME_DIR").ok(),
            user: std::env::var("USER").ok(),
            biomeos_family_id: std::env::var("BIOMEOS_FAMILY_ID")
                .or_else(|_| std::env::var("TOADSTOOL_FAMILY"))
                .ok(),
            biomeos_crypto_socket: std::env::var("BIOMEOS_CRYPTO_SOCKET").ok(),
            biomeos_coordination_socket: std::env::var("BIOMEOS_COORDINATION_SOCKET").ok(),
            biomeos_storage_socket: std::env::var("BIOMEOS_STORAGE_SOCKET").ok(),
            biomeos_routing_socket: std::env::var("BIOMEOS_ROUTING_SOCKET").ok(),
            // legacy env fallbacks (product-era names) after capability-prefixed vars
            legacy_security_socket: std::env::var("TOADSTOOL_SECURITY_SOCKET")
                .or_else(|_| std::env::var("BEARDOG_SOCKET")) // legacy
                .ok(),
            legacy_coordination_socket: std::env::var("TOADSTOOL_COORDINATION_SOCKET")
                .or_else(|_| std::env::var("SONGBIRD_SOCKET")) // legacy
                .ok(),
            legacy_storage_socket: std::env::var("TOADSTOOL_STORAGE_SOCKET")
                .or_else(|_| std::env::var("NESTGATE_SOCKET")) // legacy
                .ok(),
            legacy_intelligence_socket: std::env::var("TOADSTOOL_INTELLIGENCE_SOCKET")
                .or_else(|_| std::env::var("SQUIRREL_SOCKET")) // legacy
                .ok(),
            toadstool_socket: std::env::var("TOADSTOOL_SOCKET").ok(),
            biomeos_socket_path: std::env::var("BIOMEOS_SOCKET_PATH").ok(),
            nucleus_socket: std::env::var("NUCLEUS_SOCKET").ok(),
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
