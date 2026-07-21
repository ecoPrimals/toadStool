// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use super::{
    GateGpuInfo, JobRouter, RemoteDispatchError, RemoteDispatcher, RoutingDecision, RoutingReason,
};

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
        is_owner: false,
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
        is_owner: false,
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
        is_owner: false,
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
fn gate_gpu_info_is_owner_defaults_false() {
    let json = serde_json::json!({
        "gate_id": "test-gate",
        "gpu_model": "RTX 4070",
        "vram_total_mb": 12288,
        "vram_available_mb": 8000,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true,
        "endpoint": "unix:///tmp/test.sock"
    });
    let info: GateGpuInfo = serde_json::from_value(json).expect("deserialize");
    assert!(!info.is_owner);
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
        is_owner: false,
    };
    let json = serde_json::to_string(&info).expect("serialize");
    assert!(json.contains("127.0.0.1:9999"));
    let parsed: GateGpuInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.endpoint.as_deref(), Some("127.0.0.1:9999"));
}

#[tokio::test]
async fn test_remote_dispatcher_forward_unix_invalid_path_returns_transport_error() {
    // /tmp exists but is a directory, not a socket — UnixStream::connect fails
    let result = RemoteDispatcher::forward("/tmp", "compute.submit", serde_json::json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, RemoteDispatchError::Transport(_)),
        "expected Transport error, got {err:?}"
    );
}

#[tokio::test]
async fn test_remote_dispatcher_forward_nonexistent_tcp_returns_transport_error() {
    // Use localhost with port 1 — nothing listens, connection refused quickly
    let result =
        RemoteDispatcher::forward("127.0.0.1:1", "compute.submit", serde_json::json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, RemoteDispatchError::Transport(_)),
        "expected Transport error, got {err:?}"
    );
}

// ============================================================================
// Success-path mock tests
// ============================================================================

/// Spawn a mock JSON-RPC Unix server that speaks the riboCipher protocol:
/// 1. Read 2-byte prefix [0xEC, 0x01]
/// 2. Read NDJSON request line
/// 3. Write NDJSON response line
fn mock_jsonrpc_unix_server(
    socket_path: &std::path::Path,
    expected_method: &'static str,
    response_result: serde_json::Value,
) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::UnixListener::bind(socket_path).expect("bind mock socket");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = tokio::io::BufReader::new(stream);
        let mut prefix = [0u8; 2];
        reader
            .read_exact(&mut prefix)
            .await
            .expect("read riboCipher prefix");
        assert_eq!(prefix, [0xEC, 0x01], "expected riboCipher NDJSON signal");

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read request line");
        let request: serde_json::Value =
            serde_json::from_str(line.trim()).expect("parse JSON-RPC request");
        assert_eq!(
            request["method"].as_str().unwrap(),
            expected_method,
            "method mismatch"
        );
        assert!(
            request["params"]["_dispatch_trust"]["source_gate_id"]
                .as_str()
                .is_some(),
            "provenance metadata missing"
        );
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "result": response_result,
            "id": request["id"]
        });
        let mut resp_bytes = serde_json::to_vec(&response).expect("serialize response");
        resp_bytes.push(b'\n');
        reader
            .get_mut()
            .write_all(&resp_bytes)
            .await
            .expect("write response");
    })
}

#[tokio::test]
async fn test_remote_dispatcher_forward_unix_success_path() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let sock = dir.path().join("test-dispatch.sock");

    let expected_result = serde_json::json!({"job_id": "j-42", "status": "queued"});
    let handle = mock_jsonrpc_unix_server(&sock, "compute.submit", expected_result.clone());

    let params = serde_json::json!({"model": "tinyllama:latest", "prompt": "hello"});
    let result = RemoteDispatcher::forward(sock.to_str().unwrap(), "compute.submit", params).await;

    assert!(result.is_ok(), "forward failed: {result:?}");
    let value = result.unwrap();
    assert_eq!(value["job_id"], "j-42");
    assert_eq!(value["status"], "queued");
    handle.await.ok();
}

#[tokio::test]
async fn test_remote_dispatcher_forward_unix_preserves_provenance() {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

    let dir = tempfile::TempDir::new().expect("tempdir");
    let sock = dir.path().join("provenance-check.sock");

    let listener = tokio::net::UnixListener::bind(&sock).expect("bind");
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = tokio::io::BufReader::new(stream);
        let mut prefix = [0u8; 2];
        reader.read_exact(&mut prefix).await.expect("read prefix");
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read line");
        let req: serde_json::Value = serde_json::from_str(line.trim()).expect("parse");
        let trust = &req["params"]["_dispatch_trust"];
        assert!(trust.is_object(), "_dispatch_trust should be object");
        let gate_id = trust["source_gate_id"].as_str().unwrap();
        assert!(!gate_id.is_empty(), "source_gate_id should not be empty");

        let resp = serde_json::json!({"jsonrpc":"2.0","result":{"gate": gate_id},"id":req["id"]});
        let mut body = serde_json::to_vec(&resp).unwrap();
        body.push(b'\n');
        reader.get_mut().write_all(&body).await.unwrap();
    });

    let result = RemoteDispatcher::forward(
        sock.to_str().unwrap(),
        "compute.status",
        serde_json::json!({"job_id": "j-1"}),
    )
    .await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert!(val["gate"].as_str().is_some());
    handle.await.ok();
}

#[tokio::test]
async fn test_remote_dispatcher_forward_unix_remote_error() {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

    let dir = tempfile::TempDir::new().expect("tempdir");
    let sock = dir.path().join("error-path.sock");

    let listener = tokio::net::UnixListener::bind(&sock).expect("bind");
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = tokio::io::BufReader::new(stream);
        let mut prefix = [0u8; 2];
        reader.read_exact(&mut prefix).await.expect("read prefix");
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read line");
        let req: serde_json::Value = serde_json::from_str(line.trim()).expect("parse");
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {"code": -32603, "message": "GPU OOM"},
            "id": req["id"]
        });
        let mut body = serde_json::to_vec(&resp).unwrap();
        body.push(b'\n');
        reader.get_mut().write_all(&body).await.unwrap();
    });

    let result = RemoteDispatcher::forward(
        sock.to_str().unwrap(),
        "compute.submit",
        serde_json::json!({}),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{err:?}");
    assert!(
        err_msg.contains("GPU OOM"),
        "error should contain the remote error message, got {err_msg}"
    );
    handle.await.ok();
}

#[tokio::test]
async fn test_remote_dispatcher_forward_tcp_success_path() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind TCP");
    let addr = listener.local_addr().expect("local_addr");

    let expected_result = serde_json::json!({"status": "completed", "output": [1, 2, 3]});
    let result_clone = expected_result.clone();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut reader = tokio::io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read line");
        let req: serde_json::Value = serde_json::from_str(line.trim()).expect("parse request");
        assert_eq!(req["method"], "compute.submit");
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "result": result_clone,
            "id": req["id"]
        });
        reader
            .get_mut()
            .write_all(&serde_json::to_vec(&resp).unwrap())
            .await
            .unwrap();
    });

    let result = RemoteDispatcher::forward(
        &addr.to_string(),
        "compute.submit",
        serde_json::json!({"data": "test"}),
    )
    .await;

    assert!(result.is_ok(), "TCP forward failed: {result:?}");
    let val = result.unwrap();
    assert_eq!(val["status"], "completed");
    assert_eq!(val["output"], serde_json::json!([1, 2, 3]));
    handle.await.ok();
}

#[tokio::test]
async fn test_enrich_params_preserves_existing_dispatch_trust() {
    use super::dispatcher::enrich_params_with_gate_provenance;

    let params = serde_json::json!({
        "model": "test",
        "_dispatch_trust": {"source_gate_id": "original-gate"}
    });
    let enriched = enrich_params_with_gate_provenance(params, "new-gate");
    assert_eq!(
        enriched["_dispatch_trust"]["source_gate_id"], "original-gate",
        "should not overwrite existing _dispatch_trust"
    );
}

#[tokio::test]
async fn test_enrich_params_wraps_non_object_payload() {
    use super::dispatcher::enrich_params_with_gate_provenance;

    let params = serde_json::json!("just-a-string");
    let enriched = enrich_params_with_gate_provenance(params, "gate-1");
    assert_eq!(enriched["payload"], "just-a-string");
    assert_eq!(enriched["_dispatch_trust"]["source_gate_id"], "gate-1");
}
