// SPDX-License-Identifier: AGPL-3.0-or-later
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
async fn test_tcp_http_keepalive_multi_request() {
    let handler = Arc::new(test_handler());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server_handler = Arc::clone(&handler);
    let _server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        handle_tcp_connection(server_handler, stream).await.expect("ok");
    });

    let mut client = TcpStream::connect(addr).await.expect("connect");

    // First request (keep-alive default)
    let body1 = r#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
    let http1 = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body1.len(),
        body1
    );
    client.write_all(http1.as_bytes()).await.expect("write1");

    let mut buf = vec![0u8; 4096];
    let n = client.read(&mut buf).await.expect("read1");
    let resp1 = String::from_utf8_lossy(&buf[..n]);
    assert!(resp1.contains("HTTP/1.1 200 OK"), "first response ok");
    assert!(
        resp1.contains("Connection: keep-alive"),
        "keep-alive header present"
    );
    assert!(resp1.contains("healthy"), "first response has health data");

    // Second request on same connection (Connection: close to end)
    let body2 = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
    let http2 = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body2.len(),
        body2
    );
    client.write_all(http2.as_bytes()).await.expect("write2");

    let mut buf2 = Vec::new();
    client.read_to_end(&mut buf2).await.expect("read2");
    let resp2 = String::from_utf8_lossy(&buf2);
    assert!(resp2.contains("HTTP/1.1 200 OK"), "second response ok");
    assert!(
        resp2.contains("Connection: close"),
        "close header on final response"
    );
}

#[tokio::test]
async fn test_unix_http_keepalive_multi_request() {
    let handler = Arc::new(test_handler());
    let dir = tempfile::tempdir().expect("tempdir");
    let socket_path = dir.path().join("keepalive.sock");

    let server_handler = Arc::clone(&handler);
    let sock_path = socket_path.clone();
    let _server = tokio::spawn(async move {
        serve_unix(server_handler, sock_path).await.expect("serve");
    });

    let mut stream = await_unix_socket(&socket_path).await;

    // First request (keep-alive)
    let body1 = r#"{"jsonrpc":"2.0","method":"toadstool.health","id":1}"#;
    let http1 = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body1.len(),
        body1
    );
    stream.write_all(http1.as_bytes()).await.expect("write1");

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.expect("read1");
    let resp1 = String::from_utf8_lossy(&buf[..n]);
    assert!(resp1.contains("Connection: keep-alive"));

    // Second request on same connection
    let body2 = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
    let http2 = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body2.len(),
        body2
    );
    stream.write_all(http2.as_bytes()).await.expect("write2");

    let mut buf2 = Vec::new();
    stream.read_to_end(&mut buf2).await.expect("read2");
    let resp2 = String::from_utf8_lossy(&buf2);
    assert!(resp2.contains("HTTP/1.1 200 OK"));
    assert!(resp2.contains("Connection: close"));
}

#[tokio::test]
async fn test_ndjson_with_blank_lines_between_requests() {
    let handler = Arc::new(test_handler());
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server_handler = Arc::clone(&handler);
    let _server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        handle_tcp_connection(server_handler, stream).await.expect("ok");
    });

    let mut client = TcpStream::connect(addr).await.expect("connect");

    // Send two NDJSON requests with a blank line between them
    let requests = concat!(
        "{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n",
        "\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.version\",\"id\":2}\n",
    );
    client.write_all(requests.as_bytes()).await.expect("write");
    client.shutdown().await.ok();

    let mut buf = Vec::new();
    client.read_to_end(&mut buf).await.expect("read");
    let text = String::from_utf8_lossy(&buf);
    let responses: Vec<&str> = text.lines().collect();
    assert!(
        responses.len() >= 2,
        "expected 2 responses, got {}: {text}",
        responses.len()
    );

    let r1: serde_json::Value = serde_json::from_str(responses[0]).expect("json1");
    assert!(r1["result"]["healthy"].as_bool().is_some());
    let r2: serde_json::Value = serde_json::from_str(responses[1]).expect("json2");
    assert!(r2["result"]["version"].as_str().is_some());
}

#[tokio::test]
async fn test_ndjson_unix_persistent_multi_request() {
    let handler = Arc::new(test_handler());
    let dir = tempfile::tempdir().expect("tempdir");
    let socket_path = dir.path().join("ndjson-multi.sock");

    let server_handler = Arc::clone(&handler);
    let sock_path = socket_path.clone();
    let _server = tokio::spawn(async move {
        serve_unix(server_handler, sock_path).await.expect("serve");
    });

    let mut stream = await_unix_socket(&socket_path).await;

    // Send three requests on the same connection
    let r1 = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n";
    let r2 = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.version\",\"id\":2}\n";
    let r3 = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":3}\n";
    stream.write_all(r1).await.expect("w1");
    stream.write_all(r2).await.expect("w2");
    stream.write_all(r3).await.expect("w3");
    stream.shutdown().await.ok();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read");
    let text = String::from_utf8_lossy(&buf);
    assert_eq!(text.lines().count(), 3, "expected 3 responses: {text}");
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

#[test]
fn is_plaintext_protocol_byte_detects_json() {
    assert!(
        super::unix::is_plaintext_protocol_byte(b'{'),
        "JSON opening brace is plaintext"
    );
}

#[test]
fn is_plaintext_protocol_byte_detects_http() {
    assert!(super::unix::is_plaintext_protocol_byte(b'P'), "POST");
    assert!(super::unix::is_plaintext_protocol_byte(b'G'), "GET");
    assert!(super::unix::is_plaintext_protocol_byte(b'H'), "HTTP");
}

#[test]
fn is_plaintext_protocol_byte_rejects_btsp_prefix() {
    assert!(
        !super::unix::is_plaintext_protocol_byte(0x00),
        "0x00 is BTSP length prefix"
    );
    assert!(
        !super::unix::is_plaintext_protocol_byte(0x01),
        "0x01 is large BTSP frame"
    );
}

#[test]
fn is_plaintext_protocol_byte_whitespace() {
    assert!(super::unix::is_plaintext_protocol_byte(b'\t'), "tab");
    assert!(super::unix::is_plaintext_protocol_byte(b'\n'), "newline");
    assert!(super::unix::is_plaintext_protocol_byte(b' '), "space");
}

/// Verify that `handle_btsp_connection` auto-detects plain NDJSON
/// and degrades gracefully instead of rejecting the connection.
#[tokio::test]
async fn test_btsp_autodetect_plain_ndjson() {
    let handler = Arc::new(test_handler());
    let (server_stream, mut client_stream) = UnixStream::pair().expect("pair");

    let server_handle = tokio::spawn(async move {
        super::unix::handle_btsp_connection(handler, server_stream)
            .await
            .expect("btsp handler");
    });

    let request = b"{\"jsonrpc\":\"2.0\",\"method\":\"toadstool.health\",\"id\":1}\n";
    client_stream.write_all(request).await.expect("write");
    client_stream.shutdown().await.ok();

    let mut buf = Vec::new();
    client_stream.read_to_end(&mut buf).await.expect("read");
    server_handle.await.expect("join");

    let text = String::from_utf8_lossy(&buf);
    assert!(
        !text.is_empty(),
        "BTSP socket should serve plain JSON-RPC via auto-detect"
    );
    let resp: serde_json::Value = serde_json::from_slice(&buf).expect("json");
    assert!(
        resp["result"]["healthy"].as_bool().is_some(),
        "health response should be valid: {resp}"
    );
}

/// Verify that `handle_btsp_connection` auto-detects HTTP and serves it.
#[tokio::test]
async fn test_btsp_autodetect_plain_http() {
    let handler = Arc::new(test_handler());
    let (server_stream, mut client_stream) = UnixStream::pair().expect("pair");

    let server_handle = tokio::spawn(async move {
        super::unix::handle_btsp_connection(handler, server_stream)
            .await
            .expect("btsp handler");
    });

    let body = r#"{"jsonrpc":"2.0","method":"toadstool.version","id":2}"#;
    let http = format!(
        "POST /rpc HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    client_stream
        .write_all(http.as_bytes())
        .await
        .expect("write");

    let mut buf = Vec::new();
    client_stream.read_to_end(&mut buf).await.expect("read");
    server_handle.await.expect("join");

    let response = String::from_utf8_lossy(&buf);
    assert!(
        response.contains("HTTP/1.1 200 OK"),
        "BTSP socket should serve HTTP via auto-detect: {response}"
    );
    assert!(
        response.contains("test-conn-1.0.0"),
        "version in response: {response}"
    );
}

/// Verify that EOF on a BTSP socket is handled gracefully.
#[tokio::test]
async fn test_btsp_autodetect_eof() {
    let handler = Arc::new(test_handler());
    let (server_stream, client_stream) = UnixStream::pair().expect("pair");

    drop(client_stream);

    let result = super::unix::handle_btsp_connection(handler, server_stream).await;
    assert!(result.is_ok(), "EOF should be handled gracefully");
}
