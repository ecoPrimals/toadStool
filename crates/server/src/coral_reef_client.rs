// SPDX-License-Identifier: AGPL-3.0-only
//! coralReef shader compiler client for capability-based discovery and IPC.
//!
//! Discovers coralReef at runtime via capability-based discovery.
//! Used by the dispatch handler to check coralReef availability and
//! coordinate compile-then-dispatch pipelines. Gracefully degrades
//! when coralReef is not available.

use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tokio::sync::OnceCell;
use tracing::debug;

/// Client for coralReef shader compiler primal.
///
/// Uses capability-based discovery to find coralReef at runtime.
/// Falls back gracefully when the compiler is not available.
pub struct CoralReefClient {
    inner: OnceCell<Option<UnixJsonRpcClient>>,
}

impl Default for CoralReefClient {
    fn default() -> Self {
        Self::new()
    }
}

impl CoralReefClient {
    /// Creates a new coralReef client.
    pub fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }

    /// Attempt to discover and connect to coralReef.
    ///
    /// Discovery order:
    /// 1. `TOADSTOOL_SHADER_COMPILER_ADDR` env var (explicit socket path)
    /// 2. `CORALREEF_SOCKET` env var (socket path — preferred over deprecated `CORALREEF_URL`)
    /// 3. `CORALREEF_URL` env var (deprecated: treated as socket path, not HTTP)
    /// 4. XDG runtime dir manifest: `$XDG_RUNTIME_DIR/ecoPrimals/coralreef-core.json`
    /// 5. Socket directory scan: `$XDG_RUNTIME_DIR/biomeos/coralreef*.sock` (matches
    ///    any coralReef naming variant, e.g. `coralreef-core-default.sock`)
    /// 6. ecoPrimals socket fallback: `$XDG_RUNTIME_DIR/ecoPrimals/shader_compile.sock`
    async fn discover() -> Option<UnixJsonRpcClient> {
        if let Ok(addr) = std::env::var("TOADSTOOL_SHADER_COMPILER_ADDR") {
            let path = PathBuf::from(&addr);
            if path.exists() {
                debug!(path = %path.display(), "coralReef discovered via TOADSTOOL_SHADER_COMPILER_ADDR");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        for env_name in ["CORALREEF_SOCKET", "CORALREEF_URL"] {
            if let Ok(val) = std::env::var(env_name) {
                let path = PathBuf::from(&val);
                if path.exists() {
                    debug!(path = %path.display(), env = env_name, "coralReef discovered via env");
                    return Some(UnixJsonRpcClient::new(path));
                }
            }
        }

        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let runtime = PathBuf::from(&runtime_dir);

            let manifest = runtime.join("ecoPrimals").join("coralreef-core.json");
            if let Some(socket) = read_socket_from_manifest(&manifest)
                && socket.exists()
            {
                debug!(path = %socket.display(), "coralReef discovered via XDG manifest");
                return Some(UnixJsonRpcClient::new(socket));
            }

            if let Some(sock) = scan_dir_for_socket(&runtime.join("biomeos"), "coralreef") {
                debug!(path = %sock.display(), "coralReef discovered via biomeos socket directory scan");
                return Some(UnixJsonRpcClient::new(sock));
            }

            let capability_sock = runtime.join("ecoPrimals").join("shader_compile.sock");
            if capability_sock.exists() {
                debug!(path = %capability_sock.display(), "coralReef discovered via ecoPrimals capability socket");
                return Some(UnixJsonRpcClient::new(capability_sock));
            }
        }

        debug!("coralReef not discovered — shader compilation will use naga-only pipeline");
        None
    }

    async fn client(&self) -> Option<&UnixJsonRpcClient> {
        self.inner
            .get_or_init(|| async { Self::discover().await })
            .await
            .as_ref()
    }

    /// Public accessor for the underlying client (for dispatch handler).
    pub async fn client_ref(&self) -> Option<&UnixJsonRpcClient> {
        self.client().await
    }

    /// Whether coralReef is available (discovered and reachable).
    pub async fn is_available(&self) -> bool {
        self.client().await.is_some()
    }
}

/// Scan a directory for a Unix socket whose file stem starts with `prefix`.
///
/// Returns the first matching `.sock` path found (sorted for determinism).
fn scan_dir_for_socket(dir: &std::path::Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut matches: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("sock")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(prefix))
        })
        .collect();
    matches.sort();
    matches.into_iter().next()
}

/// Read socket path from a coralReef discovery manifest JSON file.
fn read_socket_from_manifest(manifest_path: &std::path::Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.get("transports")
        .and_then(|t| t.get("jsonrpc"))
        .and_then(|j| j.get("path"))
        .and_then(|p| p.as_str())
        .map(PathBuf::from)
        .or_else(|| {
            json.get("socket")
                .and_then(|s| s.as_str())
                .map(PathBuf::from)
        })
}

/// Shared coralReef client wrapped in Arc for handler use.
pub type SharedCoralReefClient = Arc<CoralReefClient>;

/// Create a shared coralReef client instance.
pub fn create_coral_reef_client() -> SharedCoralReefClient {
    Arc::new(CoralReefClient::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CoralReefClient::new();
        assert!(!client.inner.initialized());
    }

    #[test]
    fn test_shared_client_creation() {
        let client = create_coral_reef_client();
        assert!(Arc::strong_count(&client) == 1);
    }

    #[test]
    fn test_manifest_parsing_with_transports() {
        let json = r#"{"transports":{"jsonrpc":{"path":"/tmp/coralreef.sock"}}}"#;
        let tmp = std::env::temp_dir().join("test_manifest_cr.json");
        std::fs::write(&tmp, json).unwrap();
        let result = read_socket_from_manifest(&tmp);
        assert_eq!(result, Some(PathBuf::from("/tmp/coralreef.sock")));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_manifest_parsing_with_socket_fallback() {
        let json = r#"{"socket":"/tmp/fallback.sock"}"#;
        let tmp = std::env::temp_dir().join("test_manifest_fb.json");
        std::fs::write(&tmp, json).unwrap();
        let result = read_socket_from_manifest(&tmp);
        assert_eq!(result, Some(PathBuf::from("/tmp/fallback.sock")));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_manifest_parsing_missing_file() {
        let result = read_socket_from_manifest(std::path::Path::new("/nonexistent/file.json"));
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_client_not_available_without_coralreef() {
        let client = CoralReefClient::new();
        assert!(!client.is_available().await);
    }

    #[test]
    fn test_scan_dir_for_socket_finds_prefixed() {
        let dir = std::env::temp_dir().join("test_scan_coralreef");
        let _ = std::fs::create_dir_all(&dir);
        let sock = dir.join("coralreef-core-default.sock");
        std::fs::write(&sock, b"").unwrap();

        let result = scan_dir_for_socket(&dir, "coralreef");
        assert_eq!(result, Some(sock.clone()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_dir_for_socket_no_match() {
        let dir = std::env::temp_dir().join("test_scan_no_cr");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("beardog.sock"), b"").unwrap();

        let result = scan_dir_for_socket(&dir, "coralreef");
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_dir_for_socket_missing_dir() {
        let result = scan_dir_for_socket(std::path::Path::new("/nonexistent/dir"), "coralreef");
        assert!(result.is_none());
    }
}
