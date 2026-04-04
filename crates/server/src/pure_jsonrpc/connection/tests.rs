// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for pure JSON-RPC connection handling (`process_request`, TCP, Unix).

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixStream};

use super::process_request;
use super::serve_unix;
use super::tcp::handle_tcp_connection;
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::tarpc_server::StandaloneExecutor;

fn test_handler() -> JsonRpcHandler {
    let executor = Arc::new(StandaloneExecutor::new());
    JsonRpcHandler::new(executor, "test-conn-1.0.0".to_string(), None)
}

#[tokio::test]
async fn test_process_request_valid_health() {
    let handler = test_handler();
    let body = br#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp["result"]["healthy"].as_bool().is_some());
    assert_eq!(resp["result"]["version"], "test-conn-1.0.0");
}

#[tokio::test]
async fn test_process_request_invalid_json() {
    let handler = test_handler();
    let body = b"this is not json";
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32700);
}

#[tokio::test]
async fn test_process_request_empty_body() {
    let handler = test_handler();
    let body = b"";
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32700);
}

#[tokio::test]
async fn test_process_request_method_not_found() {
    let handler = test_handler();
    let body = br#"{"jsonrpc":"2.0","method":"nonexistent.method","id":42}"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32601);
}

#[tokio::test]
async fn test_serve_tcp_accepts_raw_json() {
    let handler = Arc::new(test_handler());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server_handler = Arc::clone(&handler);
    let _server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let handler = server_handler;
        handle_tcp_connection(handler, stream).await.expect("ok");
    });

    let mut client = TcpStream::connect(addr).await.expect("connect");
    let request = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n";
    client.write_all(request).await.expect("write");
    client.shutdown().await.ok();

    let mut buf = Vec::new();
    client.read_to_end(&mut buf).await.expect("read");
    let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
    assert!(resp["result"]["healthy"].as_bool().is_some());
}

#[tokio::test]
async fn test_serve_tcp_accepts_http_post() {
    let handler = Arc::new(test_handler());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server_handler = Arc::clone(&handler);
    let _server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        handle_tcp_connection(server_handler, stream)
            .await
            .expect("ok");
    });

    let body = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
    let http = format!(
        "POST /jsonrpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    let mut client = TcpStream::connect(addr).await.expect("connect");
    client.write_all(http.as_bytes()).await.expect("write");
    client.shutdown().await.ok();

    let mut buf = Vec::new();
    client.read_to_end(&mut buf).await.expect("read");
    let response = String::from_utf8_lossy(&buf);
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("test-conn-1.0.0"));
}

#[tokio::test]
async fn test_serve_tcp_accepts_http_get_with_body() {
    let handler = Arc::new(test_handler());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server_handler = Arc::clone(&handler);
    let _server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        handle_tcp_connection(server_handler, stream)
            .await
            .expect("ok");
    });

    let body = r#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
    let http = format!(
        "GET /jsonrpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    let mut client = TcpStream::connect(addr).await.expect("connect");
    client.write_all(http.as_bytes()).await.expect("write");
    client.shutdown().await.ok();

    let mut buf = Vec::new();
    client.read_to_end(&mut buf).await.expect("read");
    let response = String::from_utf8_lossy(&buf);
    assert!(response.contains("HTTP/1.1 200 OK"));
}

async fn await_unix_socket(path: &std::path::Path) -> UnixStream {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    let mut backoff = tokio::time::Duration::from_millis(1);
    loop {
        match UnixStream::connect(path).await {
            Ok(stream) => return stream,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::task::yield_now().await;
                backoff = (backoff * 2).min(tokio::time::Duration::from_millis(50));
                tokio::time::sleep(backoff).await;
            }
            Err(e) => panic!("Unix socket not ready within deadline: {e}"),
        }
    }
}

#[tokio::test]
async fn test_serve_unix_accepts_raw_json() {
    let handler = Arc::new(test_handler());
    let dir = tempfile::tempdir().expect("tempdir");
    let socket_path = dir.path().join("jsonrpc.sock");

    let server_handler = Arc::clone(&handler);
    let sock_path = socket_path.clone();
    let _server = tokio::spawn(async move {
        serve_unix(server_handler, sock_path).await.expect("serve");
    });

    let mut stream = await_unix_socket(&socket_path).await;
    let request = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n";
    stream.write_all(request).await.expect("write");
    stream.shutdown().await.ok();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read");
    let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
    assert!(resp["result"]["healthy"].as_bool().is_some());
}

#[tokio::test]
async fn test_serve_unix_accepts_http_post() {
    let handler = Arc::new(test_handler());
    let dir = tempfile::tempdir().expect("tempdir");
    let socket_path = dir.path().join("jsonrpc.sock");

    let server_handler = Arc::clone(&handler);
    let sock_path = socket_path.clone();
    let _server = tokio::spawn(async move {
        serve_unix(server_handler, sock_path).await.expect("serve");
    });

    let body = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
    let http = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    let mut stream = await_unix_socket(&socket_path).await;
    stream.write_all(http.as_bytes()).await.expect("write");
    stream.shutdown().await.ok();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read");
    let response = String::from_utf8_lossy(&buf);
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("test-conn-1.0.0"));
}

#[tokio::test]
async fn test_process_request_partial_json() {
    let handler = test_handler();
    let body = b"{\"jsonrpc\":\"2.0\",\"method\":";
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32700);
}

#[tokio::test]
async fn test_process_request_null_id() {
    let handler = test_handler();
    let body = br#"{"jsonrpc":"2.0","method":"toadstool.health","id":null}"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp["result"]["healthy"].as_bool().is_some());
}

#[tokio::test]
async fn test_process_request_string_id() {
    let handler = test_handler();
    let body = br#"{"jsonrpc":"2.0","method":"toadstool.health","id":"req-1"}"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(resp["id"], "req-1");
}

#[tokio::test]
async fn test_process_request_invalid_method_params() {
    let handler = test_handler();
    let body = br#"{"jsonrpc":"2.0","method":"toadstool.health","params":{"bad":true},"id":1}"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp["result"]["healthy"].as_bool().is_some());
}

#[tokio::test]
async fn test_process_request_whitespace_only() {
    let handler = test_handler();
    let body = b"   \t\n  ";
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
}

#[tokio::test]
async fn test_process_request_array_not_supported() {
    let handler = test_handler();
    let body = br#"[{"jsonrpc":"2.0","method":"toadstool.health","id":1}]"#;
    let result = process_request(&handler, body).await;
    assert!(result.is_ok());
    let bytes = result.expect("ok");
    let resp: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert!(resp.get("error").is_some());
}
