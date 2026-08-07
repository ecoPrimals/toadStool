// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix socket implementation for IPC
//!
//! **Deep Debt Principles**:
//! - ✅ Safe Rust (tokio async, no unsafe)
//! - ✅ Modern idiomatic (async/await patterns)
//! - ✅ Error handling (comprehensive Result types)
//!
//! ## Platform Support
//!
//! - **Linux**: Full support
//! - **macOS**: Full support
//! - **Windows**: Not applicable (use TCP)

use crate::{ToadStoolError, ToadStoolResult};
use std::path::{Path, PathBuf};
use tokio::net::{UnixListener, UnixStream};

/// Bind Unix socket listener
///
/// **Deep Debt**: Pure async, safe Rust
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc::platform::unix;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let listener = unix::bind("/tmp/toadstool.sock").await?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns error if the parent directory cannot be created, a stale socket cannot be removed, or binding fails.
pub async fn bind<P: AsRef<Path>>(path: P) -> ToadStoolResult<UnixListener> {
    let path = path.as_ref();

    // Create parent directory if needed — async to avoid blocking the runtime
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ToadStoolError::integration(format!(
                    "Failed to create socket directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    // Remove stale socket if exists
    if path.exists() {
        tokio::fs::remove_file(path).await.map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to remove stale socket {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    // Bind socket
    let listener = UnixListener::bind(path).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to bind Unix socket {}: {}",
            path.display(),
            e
        ))
    })?;

    // Per CAPABILITY_BASED_DISCOVERY_STANDARD v1.1: create capability
    // symlinks so peers can discover by capability name rather than
    // primal identity. Best-effort — failure doesn't block binding.
    create_capability_symlinks(path).await;

    Ok(listener)
}

/// Create legacy symlink so callers using the primal name still resolve.
///
/// Per Self-Knowledge v1.1 §Migration: primary socket is `compute.sock`
/// (domain-based); legacy symlink `toadstool.sock → compute.sock` is
/// maintained during the migration period.
async fn create_capability_symlinks(socket_path: &Path) {
    let Some(parent) = socket_path.parent() else {
        return;
    };

    let legacy = parent.join("toadstool.sock");
    if legacy.exists() || legacy.symlink_metadata().is_ok() {
        return;
    }
    let target = socket_path.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || {
        toadstool_common::platform::platform_link(&target, &legacy)
    })
    .await;
}

/// Connect to Unix socket
///
/// **Deep Debt**: Async, timeout-aware
///
/// # Errors
///
/// Returns error if the connection fails.
pub async fn connect<P: AsRef<Path>>(path: P) -> ToadStoolResult<UnixStream> {
    let path = path.as_ref();

    UnixStream::connect(path).await.map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to connect to Unix socket {}: {}",
            path.display(),
            e
        ))
    })
}

/// Get default ToadStool Unix socket path
///
/// **Deep Debt**: Platform-agnostic, ecoBin v2.0 compliant
///
/// Uses `platform_paths` for proper XDG and `temp_dir` resolution.
/// No hardcoded paths like `/run/user/` or `/tmp/`.
pub fn default_path() -> PathBuf {
    toadstool_common::platform_paths::toadstool_socket()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path() {
        let path = default_path();
        let path_str = path.to_string_lossy();

        // Self-Knowledge v1.1: domain-based name under biomeos dir
        assert!(path_str.contains("biomeos"));
        assert!(path_str.contains("compute.sock"));
    }

    #[tokio::test]
    async fn test_bind_and_connect() {
        let test_socket = std::env::temp_dir().join("toadstool_test_unix.sock");
        let test_socket_str = test_socket.to_string_lossy();

        // Clean up any stale socket
        let _ = std::fs::remove_file(&test_socket);

        // Bind
        let listener = bind(test_socket_str.as_ref()).await.unwrap();

        // Connect
        let stream = connect(test_socket_str.as_ref()).await.unwrap();

        // Cleanup
        drop(listener);
        drop(stream);
        let _ = std::fs::remove_file(&test_socket);
    }

    #[tokio::test]
    async fn test_bind_creates_directory() {
        let test_dir = std::env::temp_dir().join("toadstool_test_dir");
        let test_socket = test_dir.join("subdir/test.sock");
        let test_socket_str = test_socket.to_string_lossy();

        // Ensure directory doesn't exist
        let _ = std::fs::remove_dir_all(&test_dir);

        // Bind should create directory
        let listener = bind(test_socket_str.as_ref()).await.unwrap();

        // Directory should exist
        assert!(test_dir.join("subdir").exists());

        // Cleanup
        drop(listener);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_bind_removes_stale_socket() {
        let test_socket = std::env::temp_dir().join("toadstool_test_stale.sock");
        let test_socket_str = test_socket.to_string_lossy();

        // Create stale socket
        let _ = std::fs::File::create(&test_socket);
        assert!(test_socket.exists());

        // Bind should remove stale and create new
        let listener = bind(test_socket_str.as_ref()).await.unwrap();

        // Cleanup
        drop(listener);
        let _ = std::fs::remove_file(&test_socket);
    }
}
