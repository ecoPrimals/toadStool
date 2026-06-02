// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shader compiler client for capability-based discovery and IPC (`shader` capability).
//!
//! Discovers a shader compilation provider at runtime; the caller never names a specific primal.
//! Used by the dispatch handler to check compiler availability and coordinate compile-then-dispatch
//! pipelines. Gracefully degrades when no provider is available.
//!
//! ## Live re-discovery via `ipc.watch`
//!
//! When songBird reports a new `shader` capability registration via `ipc.watch`,
//! the background watcher calls [`VisualizationClient::invalidate`] to trigger
//! re-discovery on the next `is_available()` / `client_ref()` call. This replaces
//! the previous `OnceCell`-based permanent cache that could never recover if
//! coralReef registered after toadStool startup.
//!
//! Discovery tiers (per `wateringHole/CAPABILITY_BASED_DISCOVERY_STANDARD.md`):
//! - **Tier 0:** `TOADSTOOL_SHADER_COMPILER_ADDR` (explicit override; evaluated in the blocking fallback after Tier 1 does not yield a usable socket).
//! - **Tier 1:** Coordination plane — `capability.discover("shader")` via `CapabilityProvider::discover`.
//! - **Tier 2:** Domain capability socket — `$XDG_RUNTIME_DIR/biomeos/shader.sock`.
//! - **Tier 3:** `$XDG_RUNTIME_DIR/ecoPrimals/shader_compile.sock`, then capability-named scan under `biomeos/` for `*.sock` with stem prefix `shader`.

use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::constants::primal_identity::capability;
use toadstool_common::primal_sockets::{SocketPathEnv, resolve_capability_socket_fallback};
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tokio::sync::RwLock;
use tracing::debug;

/// Client for the native shader compilation pipeline (discovered via the `shader` capability).
///
/// Uses capability-based discovery with live invalidation support. When songBird's
/// `ipc.watch` reports a new shader provider, `invalidate()` clears the cache so
/// the next call triggers fresh discovery.
pub struct VisualizationClient {
    inner: RwLock<CachedClient>,
}

#[derive(Default)]
struct CachedClient {
    client: Option<UnixJsonRpcClient>,
    initialized: bool,
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
            inner: RwLock::new(CachedClient::default()),
        }
    }

    /// Creates a client that is explicitly unavailable (no discovery attempted).
    /// Used in tests that require deterministic behavior regardless of the host environment.
    #[cfg(test)]
    pub fn unavailable() -> Self {
        Self {
            inner: RwLock::new(CachedClient {
                client: None,
                initialized: true,
            }),
        }
    }

    /// Invalidate the cached client, forcing re-discovery on next access.
    ///
    /// Called by the `ipc.watch` background watcher when songBird reports a new
    /// shader capability registration (e.g. coralReef coming online after toadStool).
    pub async fn invalidate(&self) {
        let mut cache = self.inner.write().await;
        if cache.initialized {
            debug!("visualization client cache invalidated — will re-discover on next access");
            cache.client = None;
            cache.initialized = false;
        }
    }

    /// Attempt to discover a shader compiler via capability-based discovery.
    async fn discover() -> Option<UnixJsonRpcClient> {
        match toadstool_common::capability_provider::CapabilityProvider::discover(
            toadstool_common::primal_identity::Capability::Custom {
                name: capability::SHADER_COMPILER.to_string(),
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

        tokio::task::spawn_blocking(Self::discover_blocking)
            .await
            .ok()
            .flatten()
    }

    /// Filesystem fallback for [`discover`]: Tier 0 (`TOADSTOOL_SHADER_COMPILER_ADDR`), Tier 2
    /// (`biomeos/shader.sock`), Tier 3 (`ecoPrimals/shader_compile.sock`, then `shader*.sock` scan).
    fn discover_blocking() -> Option<UnixJsonRpcClient> {
        let env = SocketPathEnv::from_env();

        if let Ok(addr) = std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_SHADER_COMPILER_ADDR) {
            let path = PathBuf::from(&addr);
            if path.exists() {
                debug!(path = %path.display(), "shader compiler discovered via TOADSTOOL_SHADER_COMPILER_ADDR");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        let capability_sock = resolve_capability_socket_fallback(capability::SHADER_COMPILER, &env);
        if capability_sock.exists() {
            debug!(path = %capability_sock.display(), "shader compiler discovered via capability socket fallback");
            return Some(UnixJsonRpcClient::new(capability_sock));
        }

        let runtime_dir = std::env::var(toadstool_common::interned_strings::socket_env::XDG_RUNTIME_DIR).ok()?;
        let runtime = PathBuf::from(&runtime_dir);
        let biomeos = runtime.join("biomeos");

        let eco_sock = runtime.join("ecoPrimals").join("shader_compile.sock");
        if eco_sock.exists() {
            debug!(path = %eco_sock.display(), "shader compiler discovered via ecoPrimals capability socket");
            return Some(UnixJsonRpcClient::new(eco_sock));
        }

        if let Some(sock) = scan_dir_for_socket(&biomeos, capability::SHADER_COMPILER) {
            debug!(path = %sock.display(), "shader compiler discovered via capability socket scan");
            return Some(UnixJsonRpcClient::new(sock));
        }

        debug!("No shader compiler discovered — compilation will use naga-only pipeline");
        None
    }

    async fn ensure_initialized(&self) -> tokio::sync::RwLockReadGuard<'_, CachedClient> {
        {
            let cache = self.inner.read().await;
            if cache.initialized {
                return cache;
            }
        }

        {
            let mut cache = self.inner.write().await;
            if !cache.initialized {
                cache.client = Self::discover().await;
                cache.initialized = true;
                if cache.client.is_some() {
                    tracing::info!("shader compiler discovered and cached");
                }
            }
        }

        self.inner.read().await
    }

    /// Public accessor for the underlying client (for dispatch handler).
    pub async fn client_ref(&self) -> Option<ClientGuard<'_>> {
        let guard = self.ensure_initialized().await;
        if guard.client.is_some() {
            Some(ClientGuard(guard))
        } else {
            None
        }
    }

    /// Whether a shader compiler was discovered and is reachable.
    pub async fn is_available(&self) -> bool {
        let guard = self.ensure_initialized().await;
        guard.client.is_some()
    }
}

/// RAII guard that holds the read lock and provides access to the client.
pub struct ClientGuard<'a>(tokio::sync::RwLockReadGuard<'a, CachedClient>);

impl<'a> ClientGuard<'a> {
    pub fn get(&self) -> &UnixJsonRpcClient {
        self.0.client.as_ref().expect("ClientGuard only constructed when client is Some")
    }
}

impl<'a> std::ops::Deref for ClientGuard<'a> {
    type Target = UnixJsonRpcClient;
    fn deref(&self) -> &Self::Target {
        self.get()
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
        let _client = VisualizationClient::new();
    }

    #[test]
    fn test_shared_client_creation() {
        let client = create_visualization_client();
        assert!(Arc::strong_count(&client) == 1);
    }

    #[tokio::test]
    async fn test_client_not_available_without_compiler() {
        temp_env::async_with_vars(
            [
                ("XDG_RUNTIME_DIR", Some("/nonexistent/test/path")),
                ("TOADSTOOL_SHADER_COMPILER_ADDR", None::<&str>),
            ],
            async {
                let client = VisualizationClient::new();
                assert!(!client.is_available().await);
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_invalidate_resets_cache() {
        let client = VisualizationClient::unavailable();
        assert!(!client.is_available().await);
        client.invalidate().await;
        let cache = client.inner.read().await;
        assert!(!cache.initialized);
    }

    #[test]
    fn test_scan_dir_for_socket_finds_prefixed() {
        let dir = std::env::temp_dir().join("test_scan_shader_capability");
        let _ = std::fs::create_dir_all(&dir);
        let sock = dir.join("shader-compile-default.sock");
        std::fs::write(&sock, b"").unwrap();

        let result = scan_dir_for_socket(&dir, "shader");
        assert_eq!(result, Some(sock));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_dir_for_socket_no_match() {
        let dir = std::env::temp_dir().join("test_scan_no_shader");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("other.sock"), b"").unwrap();

        let result = scan_dir_for_socket(&dir, "shader");
        assert!(result.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_dir_for_socket_missing_dir() {
        let result = scan_dir_for_socket(std::path::Path::new("/nonexistent/dir"), "shader");
        assert!(result.is_none());
    }
}
