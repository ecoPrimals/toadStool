// SPDX-License-Identifier: AGPL-3.0-only
//! Cross-Gate Compute Delegation
//!
//! Routes compute jobs to the best available GPU across the mesh.
//!
//! ## Architecture
//!
//! - Plasmodium knows all gates and their GPU capabilities
//! - Job router selects gate by: VRAM available, model already loaded, queue depth
//! - Jobs forwarded via Unix socket or TCP to remote toadStool instances
//! - Results returned through the relay
//!
//! ## Example
//!
//! Gate2 (RTX 3090, 24GB) is better for large models. Tower (RTX 4070)
//! is better for quick inference. The router picks the right gate automatically.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

fn serialize_arc_str<S>(v: &Arc<str>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(v)
}

fn deserialize_arc_str<'de, D>(d: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(Arc::from(s))
}

/// GPU capabilities for a single gate
///
/// Uses `Arc<str>` for gate_id to avoid allocations on hot-path route decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateGpuInfo {
    /// Gate identifier (e.g., "tower", "gate2")
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub gate_id: Arc<str>,
    /// GPU model name (e.g., "RTX 4070", "RTX 3090")
    pub gpu_model: String,
    /// Total VRAM in MB
    pub vram_total_mb: u64,
    /// Available VRAM in MB
    pub vram_available_mb: u64,
    /// Models currently loaded in VRAM
    pub loaded_models: Vec<String>,
    /// Current queue depth (pending jobs)
    pub queue_depth: usize,
    /// Whether this gate is reachable via mesh
    pub reachable: bool,
    /// Remote endpoint for this gate (Unix socket path or host:port).
    /// Only present for remote gates — local gate has `None`.
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// Routing decision for a compute job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected gate ID (`Arc<str>` avoids allocation on hot path)
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub gate_id: Arc<str>,
    /// Reason for selection
    pub reason: RoutingReason,
    /// Estimated wait time in milliseconds
    pub estimated_wait_ms: u64,
}

/// Why a particular gate was selected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingReason {
    /// Model already loaded in VRAM (fastest)
    ModelLoaded,
    /// Most VRAM available for new model loading
    MostVramAvailable,
    /// Shortest queue (lowest wait time)
    ShortestQueue,
    /// Only gate available
    OnlyOption,
    /// Local execution (no mesh hop needed)
    Local,
}

/// Error from remote dispatch.
#[derive(Debug, thiserror::Error)]
pub enum RemoteDispatchError {
    /// Transport/connection error.
    #[error("transport error: {0}")]
    Transport(String),
    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialize(String),
    /// Remote gate returned an error.
    #[error("remote error: {0}")]
    Remote(String),
}

/// Dispatches a compute job to a remote toadStool gate via Unix socket or TCP.
///
/// Remote gates register their endpoint (socket path or host:port) via
/// `gate.update`. When the router selects a remote gate, the dispatcher
/// forwards the JSON-RPC `compute.submit` request.
pub struct RemoteDispatcher;

impl RemoteDispatcher {
    /// Forward a compute job to a remote gate.
    ///
    /// Attempts Unix socket first (if endpoint looks like a path), then TCP.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteDispatchError`] if the remote gate is unreachable,
    /// the JSON-RPC call fails, or the response cannot be parsed.
    pub async fn forward(
        endpoint: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        let path = Path::new(endpoint);
        if path.exists()
            && (endpoint.contains('/')
                || path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sock")))
        {
            return Self::forward_unix(path, method, params).await;
        }
        Self::forward_tcp(endpoint, method, params).await
    }

    async fn forward_unix(
        socket_path: &Path,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        let client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        client
            .call(method, params)
            .await
            .map_err(|e| RemoteDispatchError::Transport(e.to_string()))
    }

    async fn forward_tcp(
        endpoint: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        // Construct JSON-RPC request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let body = serde_json::to_vec(&request)
            .map_err(|e| RemoteDispatchError::Serialize(e.to_string()))?;

        // TCP connection + send + receive
        let mut stream = tokio::net::TcpStream::connect(endpoint)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP connect: {e}")))?;

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        stream
            .write_all(&body)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP write: {e}")))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|_| RemoteDispatchError::Transport("newline".into()))?;
        stream.shutdown().await.ok();

        let mut response_buf = Vec::new();
        stream
            .read_to_end(&mut response_buf)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP read: {e}")))?;

        let response: serde_json::Value = serde_json::from_slice(&response_buf)
            .map_err(|e| RemoteDispatchError::Serialize(format!("response parse: {e}")))?;

        if let Some(result) = response.get("result") {
            Ok(result.clone())
        } else if let Some(error) = response.get("error") {
            Err(RemoteDispatchError::Remote(error.to_string()))
        } else {
            Err(RemoteDispatchError::Remote("unexpected response".into()))
        }
    }
}

/// Cross-gate job router
///
/// Selects the best gate for a compute job based on GPU capabilities,
/// model locality, and queue depth.
#[derive(Debug, Clone)]
pub struct JobRouter {
    /// Known gates and their capabilities
    gates: HashMap<Arc<str>, GateGpuInfo>,
    /// Local gate ID (`Arc<str>` avoids allocation on route decisions)
    local_gate_id: Arc<str>,
}

impl JobRouter {
    /// Create a new job router for the given local gate
    #[must_use]
    pub fn new(local_gate_id: impl AsRef<str>) -> Self {
        Self {
            gates: HashMap::new(),
            local_gate_id: Arc::from(local_gate_id.as_ref()),
        }
    }

    /// Update gate capabilities (called when Plasmodium reports new state)
    pub fn update_gate(&mut self, info: GateGpuInfo) {
        self.gates.insert(Arc::clone(&info.gate_id), info);
    }

    /// Remove a gate (went offline)
    pub fn remove_gate(&mut self, gate_id: &str) {
        self.gates.remove(gate_id);
    }

    /// Route a job to the best available gate
    ///
    /// Selection priority:
    /// 1. Gate with model already loaded (avoids VRAM load time)
    /// 2. Gate with most available VRAM (can load model fastest)
    /// 3. Gate with shortest queue (lowest wait time)
    /// 4. Local gate (no mesh hop latency)
    #[must_use]
    pub fn route(&self, model: &str, vram_required_mb: u64) -> RoutingDecision {
        let reachable: Vec<&GateGpuInfo> = self.gates.values().filter(|g| g.reachable).collect();

        if reachable.is_empty() {
            return RoutingDecision {
                gate_id: Arc::clone(&self.local_gate_id),
                reason: RoutingReason::OnlyOption,
                estimated_wait_ms: 0,
            };
        }

        // 1. Check if model is already loaded somewhere
        if let Some(gate) = reachable
            .iter()
            .find(|g| g.loaded_models.iter().any(|m| m == model))
        {
            return RoutingDecision {
                gate_id: Arc::clone(&gate.gate_id),
                reason: RoutingReason::ModelLoaded,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: gate.queue_depth as u64 * 100, // rough estimate
            };
        }

        // 2. Gate with enough VRAM and most available
        let mut candidates: Vec<&&GateGpuInfo> = reachable
            .iter()
            .filter(|g| g.vram_available_mb >= vram_required_mb)
            .collect();

        if !candidates.is_empty() {
            candidates.sort_by(|a, b| b.vram_available_mb.cmp(&a.vram_available_mb));
            let best = candidates[0];
            return RoutingDecision {
                gate_id: Arc::clone(&best.gate_id),
                reason: RoutingReason::MostVramAvailable,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: best.queue_depth as u64 * 100,
            };
        }

        // 3. Shortest queue regardless of VRAM
        if let Some(gate) = reachable.iter().min_by_key(|g| g.queue_depth) {
            return RoutingDecision {
                gate_id: Arc::clone(&gate.gate_id),
                reason: RoutingReason::ShortestQueue,
                #[allow(clippy::cast_possible_truncation)]
                estimated_wait_ms: gate.queue_depth as u64 * 100,
            };
        }

        // 4. Fallback to local
        RoutingDecision {
            gate_id: Arc::clone(&self.local_gate_id),
            reason: RoutingReason::Local,
            estimated_wait_ms: 0,
        }
    }

    /// Get all known gates
    #[must_use]
    pub fn gates(&self) -> &HashMap<Arc<str>, GateGpuInfo> {
        &self.gates
    }

    /// Whether the given gate_id is a remote gate (not the local one).
    #[must_use]
    pub fn is_remote_gate(&self, gate_id: &str) -> bool {
        gate_id != self.local_gate_id.as_ref()
    }

    /// Get the endpoint for a remote gate.
    #[must_use]
    pub fn gate_endpoint(&self, gate_id: &str) -> Option<String> {
        self.gates.get(gate_id).and_then(|g| g.endpoint.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tower_gpu() -> GateGpuInfo {
        GateGpuInfo {
            gate_id: Arc::from("tower"),
            gpu_model: "RTX 4070".to_string(),
            vram_total_mb: 12288,
            vram_available_mb: 8000,
            loaded_models: vec!["tinyllama:latest".to_string()],
            queue_depth: 2,
            reachable: true,
            endpoint: None,
        }
    }

    fn gate2_gpu() -> GateGpuInfo {
        GateGpuInfo {
            gate_id: Arc::from("gate2"),
            gpu_model: "RTX 3090".to_string(),
            vram_total_mb: 24576,
            vram_available_mb: 20000,
            loaded_models: vec!["llama3:70b".to_string()],
            queue_depth: 0,
            reachable: true,
            endpoint: None,
        }
    }

    #[test]
    fn test_route_model_already_loaded() {
        let mut router = JobRouter::new("tower");
        router.update_gate(tower_gpu());
        router.update_gate(gate2_gpu());

        // tinyllama is loaded on tower
        let decision = router.route("tinyllama:latest", 2000);
        assert_eq!(decision.gate_id.as_ref(), "tower");
        assert!(matches!(decision.reason, RoutingReason::ModelLoaded));
    }

    #[test]
    fn test_route_large_model_to_big_gpu() {
        let mut router = JobRouter::new("tower");
        router.update_gate(tower_gpu());
        router.update_gate(gate2_gpu());

        // New model needing 16GB VRAM -> gate2 has 20GB available
        let decision = router.route("mixtral:8x7b", 16000);
        assert_eq!(decision.gate_id.as_ref(), "gate2");
        assert!(matches!(decision.reason, RoutingReason::MostVramAvailable));
    }

    #[test]
    fn test_route_no_gates_falls_back_local() {
        let router = JobRouter::new("tower");
        let decision = router.route("any_model", 4000);
        assert_eq!(decision.gate_id.as_ref(), "tower");
        assert!(matches!(decision.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn test_route_shortest_queue() {
        let mut router = JobRouter::new("tower");

        let mut tower = tower_gpu();
        tower.loaded_models.clear();
        tower.vram_available_mb = 1000; // Not enough
        tower.queue_depth = 5;

        let mut gate2 = gate2_gpu();
        gate2.loaded_models.clear();
        gate2.vram_available_mb = 1000; // Not enough
        gate2.queue_depth = 1;

        router.update_gate(tower);
        router.update_gate(gate2);

        // Neither has enough VRAM, pick shortest queue
        let decision = router.route("huge_model", 30000);
        assert_eq!(decision.gate_id.as_ref(), "gate2");
        assert!(matches!(decision.reason, RoutingReason::ShortestQueue));
    }

    #[test]
    fn test_update_and_remove_gate() {
        let mut router = JobRouter::new("tower");
        router.update_gate(tower_gpu());
        assert_eq!(router.gates().len(), 1);

        router.update_gate(gate2_gpu());
        assert_eq!(router.gates().len(), 2);

        router.remove_gate("gate2");
        assert_eq!(router.gates().len(), 1);
    }

    #[test]
    fn test_unreachable_gate_skipped() {
        let mut router = JobRouter::new("tower");

        let mut gate2 = gate2_gpu();
        gate2.reachable = false;
        router.update_gate(gate2);

        let tower = tower_gpu();
        router.update_gate(tower);

        // gate2 has more VRAM but is unreachable
        let decision = router.route("new_model", 4000);
        assert_eq!(decision.gate_id.as_ref(), "tower");
    }

    #[test]
    fn test_all_gates_unreachable_falls_back_to_local() {
        let mut router = JobRouter::new("local");

        let mut gate = tower_gpu();
        gate.reachable = false;
        router.update_gate(gate);

        let decision = router.route("any_model", 1000);
        assert_eq!(decision.gate_id.as_ref(), "local");
        assert!(matches!(decision.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn test_route_estimated_wait_ms_model_loaded() {
        let mut router = JobRouter::new("tower");
        let mut tower = tower_gpu();
        tower.queue_depth = 3;
        router.update_gate(tower);

        let decision = router.route("tinyllama:latest", 100);
        // Wait should be queue_depth * 100
        assert_eq!(decision.estimated_wait_ms, 300);
    }

    #[test]
    fn test_route_estimated_wait_ms_most_vram() {
        let mut router = JobRouter::new("tower");
        let mut gate2 = gate2_gpu();
        gate2.queue_depth = 4;
        gate2.loaded_models.clear();
        router.update_gate(gate2);

        let decision = router.route("new_model", 1000);
        assert!(matches!(decision.reason, RoutingReason::MostVramAvailable));
        assert_eq!(decision.estimated_wait_ms, 400);
    }

    #[test]
    fn test_update_gate_replaces_existing() {
        let mut router = JobRouter::new("tower");
        router.update_gate(tower_gpu());

        let mut updated = tower_gpu();
        updated.vram_available_mb = 1234;
        router.update_gate(updated);

        // Should still be one gate (updated in-place)
        assert_eq!(router.gates().len(), 1);
        assert_eq!(router.gates()["tower"].vram_available_mb, 1234);
    }

    #[test]
    fn test_remove_nonexistent_gate_is_noop() {
        let mut router = JobRouter::new("tower");
        router.remove_gate("nonexistent");
        assert_eq!(router.gates().len(), 0);
    }

    #[test]
    fn test_route_prefers_model_loaded_over_more_vram() {
        let mut router = JobRouter::new("tower");

        // tower has less VRAM available but model is loaded
        router.update_gate(tower_gpu()); // 8000 MB, tinyllama loaded
        router.update_gate(gate2_gpu()); // 20000 MB, no tinyllama

        let decision = router.route("tinyllama:latest", 2000);
        assert_eq!(decision.gate_id.as_ref(), "tower");
        assert!(matches!(decision.reason, RoutingReason::ModelLoaded));
    }

    #[test]
    fn test_routing_decision_serialization_roundtrip() {
        let decision = RoutingDecision {
            gate_id: Arc::from("gate2"),
            reason: RoutingReason::MostVramAvailable,
            estimated_wait_ms: 150,
        };
        let json = serde_json::to_string(&decision).expect("serialize");
        let parsed: RoutingDecision = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.gate_id.as_ref(), "gate2");
        assert!(matches!(parsed.reason, RoutingReason::MostVramAvailable));
        assert_eq!(parsed.estimated_wait_ms, 150);
    }

    #[test]
    fn test_routing_decision_serialization_all_reasons() {
        for (reason, expected_str) in [
            (RoutingReason::ModelLoaded, "model_loaded"),
            (RoutingReason::MostVramAvailable, "most_vram_available"),
            (RoutingReason::ShortestQueue, "shortest_queue"),
            (RoutingReason::OnlyOption, "only_option"),
            (RoutingReason::Local, "local"),
        ] {
            let decision = RoutingDecision {
                gate_id: Arc::from("test"),
                reason: reason.clone(),
                estimated_wait_ms: 0,
            };
            let json = serde_json::to_string(&decision).expect("serialize");
            assert!(
                json.contains(expected_str),
                "expected {expected_str} in {json}"
            );
            let parsed: RoutingDecision = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed.gate_id.as_ref(), "test");
        }
    }

    #[test]
    fn test_gate_gpu_info_serialization_roundtrip() {
        let info = GateGpuInfo {
            gate_id: Arc::from("tower"),
            gpu_model: "RTX 4070".to_string(),
            vram_total_mb: 12288,
            vram_available_mb: 8000,
            loaded_models: vec!["model1".to_string()],
            queue_depth: 2,
            reachable: true,
            endpoint: None,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let parsed: GateGpuInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.gate_id.as_ref(), "tower");
        assert_eq!(parsed.gpu_model, "RTX 4070");
        assert_eq!(parsed.vram_available_mb, 8000);
    }

    #[test]
    fn test_route_local_fallback_when_no_reachable() {
        let mut router = JobRouter::new("local-gate");
        let mut gate = tower_gpu();
        gate.reachable = false;
        router.update_gate(gate);

        let decision = router.route("any_model", 1000);
        assert_eq!(decision.gate_id.as_ref(), "local-gate");
        assert!(matches!(decision.reason, RoutingReason::OnlyOption));
    }

    #[test]
    fn test_is_remote_gate_returns_true_for_non_local() {
        let mut router = JobRouter::new("tower");
        router.update_gate(tower_gpu());
        router.update_gate(gate2_gpu());

        assert!(!router.is_remote_gate("tower"));
        assert!(router.is_remote_gate("gate2"));
        assert!(router.is_remote_gate("unknown"));
    }

    #[test]
    fn test_is_remote_gate_returns_false_for_local() {
        let router = JobRouter::new("local");
        assert!(!router.is_remote_gate("local"));
    }

    #[test]
    fn test_gate_endpoint_returns_endpoint_for_known_gates() {
        let mut router = JobRouter::new("tower");
        let mut gate2 = gate2_gpu();
        gate2.endpoint = Some("/tmp/gate2.sock".to_string());
        router.update_gate(tower_gpu());
        router.update_gate(gate2);

        assert_eq!(router.gate_endpoint("tower"), None);
        assert_eq!(
            router.gate_endpoint("gate2"),
            Some("/tmp/gate2.sock".to_string())
        );
        assert_eq!(router.gate_endpoint("unknown"), None);
    }

    #[test]
    fn test_gate_gpu_info_endpoint_serializes() {
        let info = GateGpuInfo {
            gate_id: Arc::from("remote"),
            gpu_model: "RTX 4090".to_string(),
            vram_total_mb: 24576,
            vram_available_mb: 20000,
            loaded_models: vec![],
            queue_depth: 0,
            reachable: true,
            endpoint: Some("127.0.0.1:9999".to_string()),
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("127.0.0.1:9999"));
        let parsed: GateGpuInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.endpoint.as_deref(), Some("127.0.0.1:9999"));
    }

    #[tokio::test]
    async fn test_remote_dispatcher_forward_unix_invalid_path_returns_transport_error() {
        // /tmp exists but is a directory, not a socket — UnixStream::connect fails
        let result =
            super::RemoteDispatcher::forward("/tmp", "compute.submit", serde_json::json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, super::RemoteDispatchError::Transport(_)),
            "expected Transport error, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_remote_dispatcher_forward_nonexistent_tcp_returns_transport_error() {
        // Use localhost with port 1 — nothing listens, connection refused quickly
        let result = super::RemoteDispatcher::forward(
            "127.0.0.1:1",
            "compute.submit",
            serde_json::json!({}),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, super::RemoteDispatchError::Transport(_)),
            "expected Transport error, got {err:?}"
        );
    }
}
