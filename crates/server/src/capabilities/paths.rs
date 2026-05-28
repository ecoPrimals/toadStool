// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery path helpers for capability announcement and peer discovery

use std::path::PathBuf;

/// Runtime base directory: `XDG_RUNTIME_DIR` or platform temp dir.
pub(super) fn runtime_base_dir() -> PathBuf {
    std::env::var(toadstool_common::interned_strings::socket_env::XDG_RUNTIME_DIR)
        .map_or_else(|_| std::env::temp_dir(), PathBuf::from)
}

/// Get discovery directory (canonical path)
///
/// Prefers `XDG_RUNTIME_DIR`, falls back to platform temp directory.
pub(super) fn discovery_directory() -> PathBuf {
    runtime_base_dir().join("ecoPrimals").join("discovery")
}

/// Get ecoPrimals root directory (ecosystem-compatible discovery path)
///
/// Some primals scan `$XDG_RUNTIME_DIR/ecoPrimals/` directly for discovery
/// entries, so we dual-write to this root alongside `ecoPrimals/discovery/`.
pub(super) fn ecoprimals_root_directory() -> PathBuf {
    runtime_base_dir().join("ecoPrimals")
}

/// Get default socket path for this primal
pub(super) fn default_socket_path(primal_id: &str) -> PathBuf {
    runtime_base_dir()
        .join("ecoPrimals")
        .join("sockets")
        .join(format!("{primal_id}.sock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_base_dir_returns_path() {
        let dir = runtime_base_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn discovery_dir_is_under_ecoprimals() {
        let dir = discovery_directory();
        assert!(dir.to_string_lossy().contains("ecoPrimals"));
        assert!(dir.to_string_lossy().contains("discovery"));
    }

    #[test]
    fn ecoprimals_root_is_parent_of_discovery() {
        let root = ecoprimals_root_directory();
        let disc = discovery_directory();
        assert!(disc.starts_with(&root));
    }

    #[test]
    fn socket_path_includes_primal_id() {
        let path = default_socket_path("toadstool-abc123");
        assert!(path.to_string_lossy().contains("toadstool-abc123.sock"));
        assert!(path.to_string_lossy().contains("sockets"));
    }
}
