// SPDX-License-Identifier: AGPL-3.0-only
//! Environment snapshot for socket path resolution

/// Environment snapshot for socket path resolution.
///
/// Production code creates this via `SocketPathEnv::from_env()`.
/// Tests create this with explicit values - no env var mutation needed.
#[derive(Debug, Clone, Default)]
pub struct SocketPathEnv {
    pub xdg_runtime_dir: Option<String>,
    pub user: Option<String>,
    pub biomeos_family_id: Option<String>,
    pub beardog_socket: Option<String>,
    pub songbird_socket: Option<String>,
    pub nestgate_socket: Option<String>,
    pub squirrel_socket: Option<String>,
    pub toadstool_socket: Option<String>,
    pub biomeos_socket_path: Option<String>,
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
