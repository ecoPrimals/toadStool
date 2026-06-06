// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use serde_json::json;
use std::path::PathBuf;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::timeout;

/// Spawns a test server and returns (temp_dir, socket_path, state).
/// The temp_dir must be kept in scope for the socket to remain valid.
async fn spawn_test_server(test_name: &str) -> (tempfile::TempDir, PathBuf, Arc<ServerState>) {
    let dir = tempfile::tempdir().expect("temp dir");
    let socket_path = dir.path().join(format!("{}.sock", test_name));

    let workload_manager = Arc::new(
        WorkloadManager::new(2)
            .await
            .expect("create workload manager"),
    );
    let state = Arc::new(ServerState {
        start_time: Instant::now(),
        workload_manager,
    });

    if socket_path.exists() {
        std::fs::remove_file(&socket_path).expect("remove existing");
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    let listener = UnixListener::bind(&socket_path).expect("bind");

    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let state_clone = Arc::clone(&state_clone);
                tokio::spawn(async move {
                    let _ = super::handle_connection(stream, state_clone.as_ref().clone()).await;
                });
            }
        }
    });

    tokio::task::yield_now().await;

    (dir, socket_path, state)
}

fn jsonrpc_request(method: &str, params: &Value, id: &Value) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    }))
    .expect("serialize request")
}

async fn connect_and_send(socket_path: &std::path::Path, request: &str) -> String {
    let stream = timeout(std::time::Duration::from_secs(2), async {
        for _ in 0..50 {
            match UnixStream::connect(socket_path).await {
                Ok(s) => return Ok(s),
                Err(_) => tokio::task::yield_now().await,
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "could not connect",
        ))
    })
    .await
    .expect("connect timeout")
    .expect("connect");

    let (reader, mut writer) = stream.into_split();
    writer
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    writer.write_all(b"\n").await.expect("write newline");
    writer.flush().await.expect("flush");

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    timeout(
        std::time::Duration::from_secs(2),
        reader.read_line(&mut line),
    )
    .await
    .expect("read timeout")
    .expect("read");
    line
}

#[test]
fn test_jsonrpc_request_parsing() {
    let json = r#"{"jsonrpc":"2.0","method":"daemon.health","params":{},"id":1}"#;
    let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.method, "daemon.health");
}

#[test]
fn test_jsonrpc_response_serialization() {
    let response = JsonRpcResponse {
        jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
        result: Some(json!({"status": "ok"})),
        error: None,
        id: Some(json!(1)),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("2.0"));
    assert!(json.contains("result"));
}

#[tokio::test]
async fn test_server_construct_and_health() {
    let (_dir, socket_path, _state) = spawn_test_server("test").await;

    let req = jsonrpc_request("daemon.health", &json!({}), &json!(1));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert_eq!(parsed["result"]["status"], "ok");
    assert!(parsed["result"]["uptime_secs"].as_u64().is_some());
    assert_eq!(parsed["id"], 1);
}

#[tokio::test]
async fn test_method_routing_metrics() {
    let (_dir, socket_path, _state) = spawn_test_server("test_metrics").await;

    let req = jsonrpc_request("daemon.metrics", &json!({}), &json!(2));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["result"]["workloads"].is_object());
    assert!(parsed["result"]["uptime_secs"].as_u64().is_some());
}

#[tokio::test]
async fn test_method_routing_list_workloads() {
    let (_dir, socket_path, _state) = spawn_test_server("test_list").await;

    let req = jsonrpc_request("daemon.list_workloads", &json!({}), &json!(3));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["result"]["workloads"].is_array());
    assert!(parsed["result"]["count"].as_u64().is_some());
}

#[tokio::test]
async fn test_submit_workload_request_response() {
    let (_dir, socket_path, _state) = spawn_test_server("test_submit").await;

    let params = json!({
        "biome_yaml": "version: 1.0",
        "requester": "test-client",
        "environment": {},
        "timeout_secs": 60,
        "persistent": false
    });
    let req = jsonrpc_request("daemon.submit_workload", &params, &json!(4));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    if let Some(err) = parsed.get("error") {
        unreachable!("submit_workload failed: {err}");
    }
    // JSON-RPC handler returns workload_id string directly from WorkloadManager
    let workload_id = parsed["result"]
        .as_str()
        .or_else(|| parsed["result"]["workload_id"].as_str());
    assert!(
        workload_id.is_some_and(|id| !id.is_empty()),
        "expected workload_id in result: {}",
        parsed
    );
}

#[tokio::test]
async fn test_get_workload_not_found() {
    let (_dir, socket_path, _state) = spawn_test_server("test_get").await;

    let params = json!({"id": "nonexistent-uuid"});
    let req = jsonrpc_request("daemon.get_workload", &params, &json!(5));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::WORKLOAD_NOT_FOUND);
}

#[tokio::test]
async fn test_parse_error_invalid_json() {
    let (_dir, socket_path, _state) = spawn_test_server("test_parse").await;

    let resp = connect_and_send(&socket_path, "not valid json\n").await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::PARSE_ERROR);
}

#[tokio::test]
async fn test_invalid_jsonrpc_version() {
    let (_dir, socket_path, _state) = spawn_test_server("test_version").await;

    let req = jsonrpc_request("daemon.health", &json!({}), &json!(1));
    let bad_req = req.replace("\"2.0\"", "\"1.0\"");
    let resp = connect_and_send(&socket_path, &bad_req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::INVALID_REQUEST);
}

#[tokio::test]
async fn test_method_not_found() {
    let (_dir, socket_path, _state) = spawn_test_server("test_method").await;

    let req = jsonrpc_request("daemon.nonexistent", &json!({}), &json!(6));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_params_submit_workload() {
    let (_dir, socket_path, _state) = spawn_test_server("test_invalid_submit").await;

    let params = json!({"invalid": "params"});
    let req = jsonrpc_request("daemon.submit_workload", &params, &json!(7));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::INVALID_PARAMS);
}

#[tokio::test]
async fn test_invalid_params_get_workload_missing_id() {
    let (_dir, socket_path, _state) = spawn_test_server("test_get_missing").await;

    let params = json!({});
    let req = jsonrpc_request("daemon.get_workload", &params, &json!(8));
    let resp = connect_and_send(&socket_path, &req).await;
    let parsed: Value = serde_json::from_str(resp.trim()).expect("parse response");
    assert!(parsed["error"].is_object());
    assert_eq!(parsed["error"]["code"], error_codes::INVALID_PARAMS);
}
