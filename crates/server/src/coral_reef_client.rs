// SPDX-License-Identifier: AGPL-3.0-or-later
//! coralReef shader compiler client for capability-based discovery and IPC.
//!
//! Discovers coralReef at runtime via capability-based discovery, then proxies
//! `shader.compile.*` requests through JSON-RPC to coralReef's `shader.compile.*` methods.
//! Gracefully degrades when coralReef is not available.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

/// coralReef health response from `shader.compile.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoralReefHealth {
    /// Compiler name.
    pub name: String,
    /// Version string.
    pub version: String,
    /// Status string.
    pub status: String,
    /// Supported GPU architectures.
    #[serde(default)]
    pub supported_archs: Vec<String>,
}

/// Compile response from `shader.compile.spirv` / `shader.compile.wgsl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResponse {
    /// Compiled binary.
    #[serde(default)]
    pub binary: Option<Vec<u8>>,
    /// Binary size in bytes.
    #[serde(default)]
    pub size: Option<u64>,
    /// Target architecture.
    #[serde(default)]
    pub arch: Option<String>,
    /// Compilation status.
    #[serde(default)]
    pub status: Option<String>,
    /// Target device this binary was compiled for (card index).
    #[serde(default)]
    pub target_device: Option<u32>,
}

/// Per-device compilation request for multi-GPU arrays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDeviceCompileRequest {
    /// WGSL source to compile.
    pub wgsl_source: String,
    /// Target devices (card indices) to compile for.
    pub target_devices: Vec<DeviceTarget>,
    /// Optimization level (0-3).
    #[serde(default)]
    pub opt_level: Option<u32>,
}

/// A specific GPU device to compile for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTarget {
    /// DRM card index (e.g. 0 for card0).
    pub card_index: u32,
    /// GPU architecture hint (e.g. `"gfx1030"`, `"sm89"`).
    #[serde(default)]
    pub arch: Option<String>,
    /// `PCIe` group for topology-aware placement.
    #[serde(default)]
    pub pcie_group: Option<String>,
}

/// Multi-device compile response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDeviceCompileResponse {
    /// Per-device compile results, keyed by card index.
    pub results: Vec<CompileResponse>,
    /// Number of successful compilations.
    pub success_count: u32,
    /// Total devices targeted.
    pub total_count: u32,
}

/// Client for coralReef shader compiler primal.
///
/// Uses capability-based discovery to find coralReef at runtime.
/// Falls back gracefully when the compiler is not available.
pub struct CoralReefClient {
    /// Lazily discovered client (None = discovery not yet attempted, Some(None) = not found)
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
    /// 2. `CORALREEF_URL` env var (socket path or address)
    /// 3. XDG runtime dir manifest: `$XDG_RUNTIME_DIR/ecoPrimals/coralreef-core.json`
    /// 4. Capability-based socket: `$XDG_RUNTIME_DIR/biomeos/coralreef.sock`
    async fn discover() -> Option<UnixJsonRpcClient> {
        if let Ok(addr) = std::env::var("TOADSTOOL_SHADER_COMPILER_ADDR") {
            let path = PathBuf::from(&addr);
            if path.exists() {
                debug!(path = %path.display(), "coralReef discovered via TOADSTOOL_SHADER_COMPILER_ADDR");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        if let Ok(url) = std::env::var("CORALREEF_URL") {
            let path = PathBuf::from(&url);
            if path.exists() {
                debug!(path = %path.display(), "coralReef discovered via CORALREEF_URL");
                return Some(UnixJsonRpcClient::new(path));
            }
        }

        // XDG runtime manifest discovery
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let manifest = PathBuf::from(&runtime_dir)
                .join("ecoPrimals")
                .join("coralreef-core.json");
            if let Some(socket) = read_socket_from_manifest(&manifest)
                && socket.exists()
            {
                debug!(path = %socket.display(), "coralReef discovered via XDG manifest");
                return Some(UnixJsonRpcClient::new(socket));
            }

            // Direct socket path
            let sock = PathBuf::from(&runtime_dir)
                .join("biomeos")
                .join("coralreef.sock");
            if sock.exists() {
                debug!(path = %sock.display(), "coralReef discovered via biomeos socket");
                return Some(UnixJsonRpcClient::new(sock));
            }
        }

        debug!("coralReef not discovered — shader compilation will use naga-only pipeline");
        None
    }

    /// Get the underlying client, discovering on first call.
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

    /// Call `shader.compile.status` to check coralReef status and supported architectures.
    pub async fn health(&self) -> Option<CoralReefHealth> {
        let client = self.client().await?;
        match client
            .call_typed::<CoralReefHealth>("shader.compile.status", serde_json::json!({}))
            .await
        {
            Ok(health) => Some(health),
            Err(e) => {
                warn!(error = %e, "coralReef health check failed");
                None
            }
        }
    }

    /// Compile WGSL source to native binary via `shader.compile.wgsl`.
    ///
    /// Optionally targets a specific device (card index) for per-GPU ISA
    /// optimization. When `target_device` is provided, coralReef compiles
    /// for that device's architecture; when `None`, it compiles for the
    /// default/generic architecture.
    pub async fn compile_wgsl(
        &self,
        source: &str,
        arch: Option<&str>,
        opt_level: Option<u32>,
        target_device: Option<u32>,
    ) -> Option<serde_json::Value> {
        let client = self.client().await?;
        let mut params = serde_json::json!({
            "wgsl_source": source,
        });
        if let Some(a) = arch {
            params["arch"] = serde_json::json!(a);
        }
        if let Some(o) = opt_level {
            params["opt_level"] = serde_json::json!(o);
        }
        if let Some(d) = target_device {
            params["target_device"] = serde_json::json!(d);
        }
        match client.call("shader.compile.wgsl", params).await {
            Ok(result) => Some(result),
            Err(e) => {
                warn!(error = %e, "coralReef WGSL compilation failed");
                None
            }
        }
    }

    /// Compile WGSL source for multiple target devices simultaneously.
    ///
    /// Enables per-GPU ISA optimization for arrays of heterogeneous GPUs
    /// (e.g. 4x RTX 3050 behind a `PCIe` switch, each getting its own
    /// optimized binary).
    pub async fn compile_wgsl_multi(
        &self,
        request: &MultiDeviceCompileRequest,
    ) -> Option<MultiDeviceCompileResponse> {
        let client = self.client().await?;
        let params = serde_json::to_value(request).ok()?;
        match client.call("shader.compile.wgsl.multi", params).await {
            Ok(result) => serde_json::from_value(result).ok(),
            Err(e) => {
                warn!(error = %e, "coralReef multi-device WGSL compilation failed");
                None
            }
        }
    }

    /// Compile SPIR-V binary to native binary via `shader.compile.spirv`.
    pub async fn compile_spirv(
        &self,
        spirv_words: &[u32],
        arch: Option<&str>,
    ) -> Option<serde_json::Value> {
        let client = self.client().await?;
        let mut params = serde_json::json!({
            "spirv_words": spirv_words,
        });
        if let Some(a) = arch {
            params["arch"] = serde_json::json!(a);
        }
        match client.call("shader.compile.spirv", params).await {
            Ok(result) => Some(result),
            Err(e) => {
                warn!(error = %e, "coralReef SPIR-V compilation failed");
                None
            }
        }
    }
}

/// Read socket path from a coralReef discovery manifest JSON file.
fn read_socket_from_manifest(manifest_path: &std::path::Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    // Try transports.jsonrpc.path first (coralReef manifest format)
    json.get("transports")
        .and_then(|t| t.get("jsonrpc"))
        .and_then(|j| j.get("path"))
        .and_then(|p| p.as_str())
        .map(PathBuf::from)
        .or_else(|| {
            // Fallback: top-level "socket" field
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

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{"name":"coralReef","version":"0.10.0","status":"healthy","supported_archs":["sm70","sm89","gfx1030"]}"#;
        let health: CoralReefHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.name, "coralReef");
        assert_eq!(health.supported_archs.len(), 3);
    }

    #[test]
    fn test_compile_response_deserialization() {
        let json = r#"{"size":1024,"arch":"sm70","status":"success"}"#;
        let resp: CompileResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.size, Some(1024));
        assert_eq!(resp.arch.as_deref(), Some("sm70"));
    }

    #[tokio::test]
    async fn test_client_not_available_without_coralreef() {
        let client = CoralReefClient::new();
        assert!(!client.is_available().await);
    }

    #[tokio::test]
    async fn test_health_returns_none_without_coralreef() {
        let client = CoralReefClient::new();
        assert!(client.health().await.is_none());
    }

    #[tokio::test]
    async fn test_compile_wgsl_returns_none_without_coralreef() {
        let client = CoralReefClient::new();
        let result = client.compile_wgsl("fn main() {}", None, None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_compile_wgsl_with_target_device() {
        let client = CoralReefClient::new();
        let result = client
            .compile_wgsl("fn main() {}", Some("gfx1030"), Some(2), Some(0))
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_compile_wgsl_multi_returns_none_without_coralreef() {
        let client = CoralReefClient::new();
        let request = MultiDeviceCompileRequest {
            wgsl_source: "fn main() {}".to_string(),
            target_devices: vec![
                DeviceTarget {
                    card_index: 0,
                    arch: Some("gfx1030".to_string()),
                    pcie_group: None,
                },
                DeviceTarget {
                    card_index: 1,
                    arch: Some("sm89".to_string()),
                    pcie_group: Some("0000:00:01.0".to_string()),
                },
            ],
            opt_level: Some(2),
        };
        assert!(client.compile_wgsl_multi(&request).await.is_none());
    }

    #[test]
    fn test_multi_device_compile_request_serde() {
        let request = MultiDeviceCompileRequest {
            wgsl_source: "fn main() {}".to_string(),
            target_devices: vec![DeviceTarget {
                card_index: 0,
                arch: Some("gfx1030".to_string()),
                pcie_group: None,
            }],
            opt_level: Some(2),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gfx1030"));
        let parsed: MultiDeviceCompileRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_devices.len(), 1);
        assert_eq!(parsed.target_devices[0].card_index, 0);
    }

    #[test]
    fn test_multi_device_compile_response_serde() {
        let json = r#"{"results":[{"size":1024,"arch":"gfx1030","status":"success","target_device":0}],"success_count":1,"total_count":1}"#;
        let resp: MultiDeviceCompileResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.success_count, 1);
        assert_eq!(resp.results[0].target_device, Some(0));
    }

    #[tokio::test]
    async fn test_compile_spirv_returns_none_without_coralreef() {
        let client = CoralReefClient::new();
        let result = client.compile_spirv(&[0x07230203], None).await;
        assert!(result.is_none());
    }
}
