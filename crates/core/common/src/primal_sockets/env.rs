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
    /// `BEARDOG_SOCKET` override for crypto capability
    pub beardog_socket: Option<String>,
    /// `SONGBIRD_SOCKET` override for coordination capability
    pub songbird_socket: Option<String>,
    /// `NESTGATE_SOCKET` override for storage capability
    pub nestgate_socket: Option<String>,
    /// `SQUIRREL_SOCKET` override for AI capability
    pub squirrel_socket: Option<String>,
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
            beardog_socket: std::env::var("BEARDOG_SOCKET").ok(),
            songbird_socket: std::env::var("SONGBIRD_SOCKET").ok(),
            nestgate_socket: std::env::var("NESTGATE_SOCKET").ok(),
            squirrel_socket: std::env::var("SQUIRREL_SOCKET").ok(),
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
