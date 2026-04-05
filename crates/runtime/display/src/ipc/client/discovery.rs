// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix and TCP endpoint discovery for [`super::DisplayClient`].

use std::net::SocketAddr;
use std::path::PathBuf;

use super::IpcEndpoint;
use crate::DisplayError;

impl super::DisplayClient {
    /// Discover IPC endpoint (Unix OR TCP)
    ///
    /// **Capability-based**: Tries multiple discovery methods!
    pub(super) fn discover_endpoint() -> crate::Result<IpcEndpoint> {
        // 1. Try Unix socket paths (optimal)
        let socket_paths = Self::get_socket_paths();
        for path in socket_paths {
            if path.exists() {
                tracing::debug!("   Unix socket found: {}", path.display());
                return Ok(IpcEndpoint::UnixSocket(path));
            }
        }

        // 2. Try TCP discovery file (fallback mode)
        if let Ok(endpoint) = Self::discover_tcp_endpoint() {
            tracing::debug!("   TCP endpoint discovered from file");
            return Ok(endpoint);
        }

        Err(DisplayError::IpcError(
            "Could not discover display server endpoint (tried Unix sockets and TCP discovery)"
                .to_string(),
        ))
    }

    /// Get candidate Unix socket paths
    ///
    /// **XDG-compliant**: Uses `PlatformPaths` for consistent path resolution
    pub(super) fn get_socket_paths() -> Vec<PathBuf> {
        use toadstool_common::platform_paths::{PathEnv, PlatformPaths};

        let mut paths = Vec::new();
        let env = PathEnv::from_env();
        let platform_paths = PlatformPaths::new(&env);

        // Primary: PlatformPaths socket directory (XDG_RUNTIME_DIR or fallback)
        paths.push(platform_paths.toadstool_socket_dir().join("display.sock"));

        // Secondary: HOME/.local/share
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("toadstool");
            path.push("display.sock");
            paths.push(path);
        }

        // Tertiary: temp_dir fallback (platform-agnostic)
        paths.push(platform_paths.toadstool_temp_dir().join("display.sock"));

        paths
    }

    /// Discover TCP endpoint from discovery file
    ///
    /// **Fallback mode**: Reads TCP port from server's discovery file
    pub(super) fn discover_tcp_endpoint() -> crate::Result<IpcEndpoint> {
        let discovery_files = Self::get_tcp_discovery_file_candidates();

        for file in discovery_files {
            if let Ok(contents) = std::fs::read_to_string(&file)
                && let Some(addr_str) = contents.trim().strip_prefix("tcp:")
                && let Ok(addr) = addr_str.parse::<SocketAddr>()
            {
                tracing::debug!("   TCP discovery file: {}", file.display());
                return Ok(IpcEndpoint::TcpLocal(addr));
            }
        }

        Err(DisplayError::IpcError(
            "No TCP discovery file found".to_string(),
        ))
    }

    /// Get candidate TCP discovery file paths
    ///
    /// **XDG-compliant**: Uses `PlatformPaths` for consistent path resolution
    pub(super) fn get_tcp_discovery_file_candidates() -> Vec<PathBuf> {
        use toadstool_common::platform_paths::{PathEnv, PlatformPaths};

        let mut paths = Vec::new();
        let env = PathEnv::from_env();
        let platform_paths = PlatformPaths::new(&env);

        // Primary: XDG_RUNTIME_DIR via PlatformPaths
        paths.push(platform_paths.runtime_dir().join("toadstool-ipc-port"));

        // Secondary: HOME/.local/share
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("toadstool-ipc-port");
            paths.push(path);
        }

        // Tertiary: temp_dir fallback (platform-agnostic)
        paths.push(std::env::temp_dir().join("toadstool-ipc-port"));

        paths
    }
}
