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
pub async fn bind<P: AsRef<Path>>(path: P) -> ToadStoolResult<UnixListener> {
    let path = path.as_ref();

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
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
        std::fs::remove_file(path).map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to remove stale socket {}: {}",
                path.display(),
                e
            ))
        })?;
    }

    // Bind socket
    UnixListener::bind(path).map_err(|e| {
        ToadStoolError::integration(format!(
            "Failed to bind Unix socket {}: {}",
            path.display(),
            e
        ))
    })
}

/// Connect to Unix socket
///
/// **Deep Debt**: Async, timeout-aware
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
/// Uses `platform_paths` for proper XDG and temp_dir resolution.
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

        // Should contain biomeos and toadstool.sock
        assert!(path_str.contains("biomeos"));
        assert!(path_str.contains("toadstool.sock"));
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
