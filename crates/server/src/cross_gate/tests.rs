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
