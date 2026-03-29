// SPDX-License-Identifier: AGPL-3.0-only
//! coral-ember / GlowPlug VFIO client for GPU passthrough IPC.
//!
//! Discovers `coral-ember` at runtime via capability-based discovery, then
//! proxies `ember.list`, `ember.vfio_fds`, `ember.swap`, `ember.reacquire`,
//! and `ember.status` requests through JSON-RPC over a Unix domain socket.
//!
//! The FD-passing path (`ember.vfio_fds`) uses `SCM_RIGHTS` and is handled
//! by coral-glowplug directly — this client covers the metadata / lifecycle
//! RPCs that toadStool needs for hardware orchestration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

/// Fallback ember socket path when no env or XDG candidate exists (override with `CORALREEF_EMBER_DEFAULT_SOCKET`).
fn fallback_ember_socket_path() -> PathBuf {
    std::env::var("CORALREEF_EMBER_DEFAULT_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/run/coralreef/ember.sock"))
}

/// Device entry returned by `ember.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceList {
    /// PCI BDF addresses of held devices.
    pub devices: Vec<String>,
}

/// Status response from `ember.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberStatus {
    /// Held device BDFs.
    pub devices: Vec<String>,
    /// Daemon uptime in seconds.
    pub uptime_secs: u64,
}

/// Swap result from `ember.swap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberSwapResult {
    /// BDF of the swapped device.
    pub bdf: String,
    /// New personality after swap (e.g. `"vfio"`, `"nouveau"`, `"unbound"`).
    pub personality: String,
}

/// Reacquire result from `ember.reacquire`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberReacquireResult {
    /// BDF of the reacquired device.
    pub bdf: String,
}

/// Client for coral-ember VFIO daemon.
///
/// Uses capability-based discovery to find coral-ember at runtime.
/// Falls back gracefully when the daemon is not running.
pub struct GlowPlugClient {
    inner: OnceCell<Option<UnixJsonRpcClient>>,
}

impl Default for GlowPlugClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GlowPlugClient {
    /// Creates a new GlowPlug client (lazy discovery on first use).
    pub fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }

    /// Attempt to discover and connect to coral-ember.
    ///
    /// Discovery order:
    /// 1. `CORALREEF_EMBER_SOCKET` env var (explicit socket path)
    /// 2. XDG runtime dir: `$XDG_RUNTIME_DIR/coralreef/ember.sock`
    /// 3. `CORALREEF_EMBER_DEFAULT_SOCKET` or built-in fallback path (last resort)
    async fn discover() -> Option<UnixJsonRpcClient> {
        if let Ok(addr) = std::env::var("CORALREEF_EMBER_SOCKET") {
            let path = PathBuf::from(&addr);
            if path.exists() {
                debug!(path = %path.display(), "coral-ember discovered via CORALREEF_EMBER_SOCKET");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let sock = PathBuf::from(&runtime_dir)
                .join("coralreef")
                .join("ember.sock");
            if sock.exists() {
                debug!(path = %sock.display(), "coral-ember discovered via XDG runtime");
                return Some(UnixJsonRpcClient::new(sock));
            }
        }

        let default = fallback_ember_socket_path();
        if default.exists() {
            debug!(path = %default.display(), "coral-ember discovered at default path");
            return Some(UnixJsonRpcClient::new(default));
        }

        debug!("coral-ember not discovered — VFIO passthrough unavailable");
        None
    }

    async fn client(&self) -> Option<&UnixJsonRpcClient> {
        self.inner
            .get_or_init(|| async { Self::discover().await })
            .await
            .as_ref()
    }

    /// Public accessor for the underlying JSON-RPC client.
    pub async fn client_ref(&self) -> Option<&UnixJsonRpcClient> {
        self.client().await
    }

    /// Whether coral-ember is available (discovered and socket exists).
    pub async fn is_available(&self) -> bool {
        self.client().await.is_some()
    }

    /// List all devices currently held by ember.
    pub async fn list_devices(&self) -> Option<EmberDeviceList> {
        let client = self.client().await?;
        match client
            .call_typed::<EmberDeviceList>("ember.list", serde_json::json!({}))
            .await
        {
            Ok(list) => Some(list),
            Err(e) => {
                warn!(error = %e, "ember.list failed");
                None
            }
        }
    }

    /// Query ember daemon status (held devices + uptime).
    pub async fn status(&self) -> Option<EmberStatus> {
        let client = self.client().await?;
        match client
            .call_typed::<EmberStatus>("ember.status", serde_json::json!({}))
            .await
        {
            Ok(status) => Some(status),
            Err(e) => {
                warn!(error = %e, "ember.status failed");
                None
            }
        }
    }

    /// Request a driver swap for a device.
    ///
    /// `target` is the driver to bind (e.g. `"vfio"`, `"nouveau"`, `"amdgpu"`, `"unbound"`).
    pub async fn swap_device(&self, bdf: &str, target: &str) -> Option<EmberSwapResult> {
        let client = self.client().await?;
        match client
            .call_typed::<EmberSwapResult>(
                "ember.swap",
                serde_json::json!({"bdf": bdf, "target": target}),
            )
            .await
        {
            Ok(result) => Some(result),
            Err(e) => {
                warn!(bdf, target, error = %e, "ember.swap failed");
                None
            }
        }
    }

    /// Reacquire VFIO hold on a device after a swap back to vfio-pci.
    pub async fn reacquire(&self, bdf: &str) -> Option<EmberReacquireResult> {
        let client = self.client().await?;
        match client
            .call_typed::<EmberReacquireResult>("ember.reacquire", serde_json::json!({"bdf": bdf}))
            .await
        {
            Ok(result) => Some(result),
            Err(e) => {
                warn!(bdf, error = %e, "ember.reacquire failed");
                None
            }
        }
    }
}

/// Shared GlowPlug client wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared GlowPlug client instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let client = GlowPlugClient::new();
        assert!(!client.inner.initialized());
    }

    #[test]
    fn shared_client_creation() {
        let client = create_glowplug_client();
        assert!(Arc::strong_count(&client) == 1);
    }

    #[tokio::test]
    async fn not_available_without_ember() {
        let client = GlowPlugClient::new();
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn list_returns_none_without_ember() {
        let client = GlowPlugClient::new();
        assert!(client.list_devices().await.is_none());
    }

    #[tokio::test]
    async fn status_returns_none_without_ember() {
        let client = GlowPlugClient::new();
        assert!(client.status().await.is_none());
    }

    #[tokio::test]
    async fn swap_returns_none_without_ember() {
        let client = GlowPlugClient::new();
        assert!(
            client
                .swap_device("0000:01:00.0", "nouveau")
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn reacquire_returns_none_without_ember() {
        let client = GlowPlugClient::new();
        assert!(client.reacquire("0000:01:00.0").await.is_none());
    }

    #[test]
    fn ember_device_list_deserialization() {
        let json = r#"{"devices":["0000:01:00.0","0000:03:00.0"]}"#;
        let list: EmberDeviceList = serde_json::from_str(json).unwrap();
        assert_eq!(list.devices.len(), 2);
    }

    #[test]
    fn ember_status_deserialization() {
        let json = r#"{"devices":["0000:01:00.0"],"uptime_secs":3600}"#;
        let status: EmberStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.uptime_secs, 3600);
        assert_eq!(status.devices.len(), 1);
    }

    #[test]
    fn ember_swap_result_deserialization() {
        let json = r#"{"bdf":"0000:01:00.0","personality":"nouveau"}"#;
        let result: EmberSwapResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.personality, "nouveau");
    }

    #[test]
    fn ember_reacquire_result_deserialization() {
        let json = r#"{"bdf":"0000:01:00.0"}"#;
        let result: EmberReacquireResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.bdf, "0000:01:00.0");
    }
}
