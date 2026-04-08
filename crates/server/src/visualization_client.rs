// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shader compiler client for capability-based discovery and IPC (`shader` capability).
//!
//! Discovers a shader compilation provider at runtime; the caller never names a specific primal.
//! Used by the dispatch handler to check compiler availability and coordinate compile-then-dispatch
//! pipelines. Gracefully degrades when no provider is available.

use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tokio::sync::OnceCell;
use tracing::debug;

/// Client for the native shader compilation pipeline (discovered via the `shader` capability).
///
/// Uses capability-based discovery; falls back gracefully when no compiler is available.
pub struct VisualizationClient {
    inner: OnceCell<Option<UnixJsonRpcClient>>,
}

impl Default for VisualizationClient {
    fn default() -> Self {
        Self::new()
    }
}

impl VisualizationClient {
    /// Creates a new shader-compiler client.
    pub fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }

    /// Attempt to discover a shader compiler via capability-based discovery.
    ///
    /// Per `wateringHole/CAPABILITY_BASED_DISCOVERY_STANDARD.md`, toadStool
    /// discovers the "shader" capability, not a specific primal. The caller
    /// never needs to know which primal implements the capability.
    ///
    /// Discovery tiers:
    /// 0. `TOADSTOOL_SHADER_COMPILER_ADDR` env var (explicit override)
    /// 1. Coordination-plane: `capability.discover("shader")` via biomeOS (Tier 1)
    /// 2. Capability socket: `$XDG_RUNTIME_DIR/biomeos/shader.sock` (Tier 2)
    /// 3. ecoPrimals capability: `$XDG_RUNTIME_DIR/ecoPrimals/shader_compile.sock`
    /// 4. Legacy identity fallback: visualization service env vars, on-disk manifest, filename scan
    async fn discover() -> Option<UnixJsonRpcClient> {
        // Tier 1: Ask the coordination service who provides "shader" capability.
        // This is the preferred path per CAPABILITY_BASED_DISCOVERY_STANDARD.md.
        match toadstool_common::capability_provider::CapabilityProvider::discover(
            toadstool_common::primal_identity::Capability::Custom {
                name: "shader".to_string(),
                version: "1.0".to_string(),
            },
        )
        .await
        {
            Ok(provider) if provider.socket_path().exists() => {
                debug!(
                    path = %provider.socket_path().display(),
                    "shader compiler discovered via coordination-plane capability.discover"
                );
                return Some(UnixJsonRpcClient::new(provider.socket_path()));
            }
            Ok(_) => {
                debug!("coordination returned shader provider but socket not found on disk");
            }
            Err(_) => {
                debug!("coordination-plane discovery unavailable, falling back to filesystem");
            }
        }

        // Tiers 0, 2-4: filesystem probing on the blocking pool
        tokio::task::spawn_blocking(Self::discover_blocking)
            .await
            .ok()
            .flatten()
    }

    fn discover_blocking() -> Option<UnixJsonRpcClient> {
        // Tier 0: Explicit override (always wins)
        if let Ok(addr) = std::env::var("TOADSTOOL_SHADER_COMPILER_ADDR") {
            let path = PathBuf::from(&addr);
            if path.exists() {
                debug!(path = %path.display(), "shader compiler discovered via TOADSTOOL_SHADER_COMPILER_ADDR");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
        let runtime = PathBuf::from(&runtime_dir);
        let biomeos = runtime.join("biomeos");

        // Tier 2: Capability-named socket (per discovery standard v1.1)
        let capability_sock = biomeos.join("shader.sock");
        if capability_sock.exists() {
            debug!(path = %capability_sock.display(), "shader compiler discovered via capability socket");
            return Some(UnixJsonRpcClient::new(capability_sock));
        }

        // Tier 3: ecoPrimals capability socket
        let eco_sock = runtime.join("ecoPrimals").join("shader_compile.sock");
        if eco_sock.exists() {
            debug!(path = %eco_sock.display(), "shader compiler discovered via ecoPrimals capability socket");
            return Some(UnixJsonRpcClient::new(eco_sock));
        }

        // Tier 4: Legacy identity-based fallbacks (backward compat)
        for env_name in ["CORALREEF_SOCKET", "CORALREEF_URL"] {
            if let Ok(val) = std::env::var(env_name) {
                let path = PathBuf::from(&val);
                if path.exists() {
                    debug!(path = %path.display(), env = env_name, "shader compiler discovered via legacy env");
                    return Some(UnixJsonRpcClient::new(path));
                }
            }
        }

        let manifest = runtime.join("ecoPrimals").join("coralreef-core.json");
        if let Some(socket) = read_socket_from_manifest(&manifest)
            && socket.exists()
        {
            debug!(path = %socket.display(), "shader compiler discovered via legacy manifest");
            return Some(UnixJsonRpcClient::new(socket));
        }

        if let Some(sock) = scan_dir_for_socket(&biomeos, "shader") {
            debug!(path = %sock.display(), "shader compiler discovered via capability socket scan");
            return Some(UnixJsonRpcClient::new(sock));
        }

        if let Some(sock) = scan_dir_for_socket(&biomeos, "coralreef") {
            debug!(path = %sock.display(), "shader compiler discovered via legacy on-disk socket name");
            return Some(UnixJsonRpcClient::new(sock));
        }

        debug!("No shader compiler discovered — compilation will use naga-only pipeline");
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

    /// Whether a shader compiler was discovered and is reachable.
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

/// Read socket path from a legacy on-disk discovery manifest (JSON).
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

/// Shared shader-compiler client wrapped in `Arc` for handler use.
pub type SharedVisualizationClient = Arc<VisualizationClient>;

/// Create a shared shader-compiler client instance.
pub fn create_visualization_client() -> SharedVisualizationClient {
    Arc::new(VisualizationClient::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = VisualizationClient::new();
        assert!(!client.inner.initialized());
    }

    #[test]
    fn test_shared_client_creation() {
        let client = create_visualization_client();
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
    async fn test_client_not_available_without_compiler() {
        let client = VisualizationClient::new();
        assert!(!client.is_available().await);
    }

    #[test]
    fn test_scan_dir_for_socket_finds_prefixed() {
        let dir = std::env::temp_dir().join("test_scan_shader_legacy");
        let _ = std::fs::create_dir_all(&dir);
        let sock = dir.join("coralreef-core-default.sock");
        std::fs::write(&sock, b"").unwrap();

        let result = scan_dir_for_socket(&dir, "coralreef");
        assert_eq!(result, Some(sock));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_dir_for_socket_no_match() {
        let dir = std::env::temp_dir().join("test_scan_no_cr");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("other.sock"), b"").unwrap();

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
