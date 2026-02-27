//! Connection flow tests: Unix raw JSON, HTTP, TCP, mock-based integration.

use super::super::{ManualJsonRpcServer, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND};
use crate::tarpc_server::StandaloneExecutor;
use std::sync::Arc;

fn test_server() -> ManualJsonRpcServer {
    let executor = Arc::new(StandaloneExecutor::new());
    ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
}

// ─── Connection flow tests: Unix raw JSON and HTTP, TCP raw JSON ─────────

#[tokio::test]
async fn test_handle_connection_unix_raw_json() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":1}"#;
    client.write_all(req.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert!(parsed.get("result").is_some());
    assert!(parsed["result"]["healthy"].as_bool().unwrap());
}

#[tokio::test]
async fn test_handle_connection_unix_http_post() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let body = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":3}"#;
    let req = format!(
            "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    client.write_all(req.as_bytes()).await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    assert!(buf.contains("HTTP/1.1 200"));
    assert!(buf.contains("Content-Type: application/json"));
    let body_start = buf.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let json_part = &buf[body_start..];
    let parsed: serde_json::Value = serde_json::from_str(json_part.trim()).unwrap();
    assert!(parsed.get("result").is_some());
}

#[tokio::test]
async fn test_handle_connection_unix_parse_error() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    client.write_all(b"not json at all\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = vec![0u8; 512];
    let n = client.read(&mut buf).await.unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("error") || resp.contains("Parse error"));
}

#[tokio::test]
async fn test_handle_tcp_connection_raw_json() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = std::sync::Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":4}"#;
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert!(parsed.get("result").is_some());
}

// ─── Mock-based integration tests: temp Unix socket, TCP port 0, error paths ─

#[cfg(unix)]
#[tokio::test]
async fn test_unix_socket_temp_path_connect_client() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let socket_path = temp_dir.path().join("jsonrpc.sock");

    let server = test_server();
    let path_for_server = socket_path.clone();
    tokio::spawn(async move {
        let _ = server.serve(path_for_server).await;
    });
    // Wait for server to bind
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }

    let mut client = UnixStream::connect(&socket_path).await.expect("connect");
    let req = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":100}"#;
    client.write_all(req.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert!(parsed.get("result").is_some());
    assert!(parsed["result"]["healthy"].as_bool().unwrap());
}

#[tokio::test]
async fn test_tcp_port_zero_connect_client() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = test_server();
    let server_arc = Arc::new(server);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });
    // Wait for server to bind
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = r#"{"jsonrpc":"2.0","method":"toadstool.version","params":{},"id":101}"#;
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert!(parsed.get("result").is_some());
    assert!(!parsed["result"]["version"]
        .as_str()
        .unwrap_or("")
        .is_empty());
}

#[tokio::test]
async fn test_handle_tcp_connection_http_post() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let body = r#"{"jsonrpc":"2.0","method":"toadstool.health","params":{},"id":5}"#;
    let req = format!(
            "POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    assert!(buf.contains("HTTP/1.1 200"));
    let body_start = buf.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let json_part = buf[body_start..].trim();
    let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
    assert!(parsed.get("result").is_some());
}

#[tokio::test]
async fn test_handle_connection_unix_unknown_method_returns_method_not_found() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let req = r#"{"jsonrpc":"2.0","method":"nonexistent.unknown","params":{},"id":50}"#;
    client.write_all(req.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert_eq!(parsed["error"]["code"], METHOD_NOT_FOUND);
    assert!(parsed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Method not found"));
}

#[tokio::test]
async fn test_handle_tcp_connection_unknown_method() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = r#"{"jsonrpc":"2.0","method":"fake.method","params":null,"id":51}"#;
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert_eq!(parsed["error"]["code"], METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_handle_tcp_connection_parse_error() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(b"{ invalid json }\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = vec![0u8; 512];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("error") || resp.contains("Parse error"));
}

#[tokio::test]
async fn test_handle_connection_unix_invalid_version() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let req = r#"{"jsonrpc":"1.0","method":"toadstool.health","params":{},"id":52}"#;
    client.write_all(req.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert_eq!(parsed["error"]["code"], INVALID_REQUEST);
}

#[cfg(unix)]
#[tokio::test]
async fn test_serve_tcp_accepts_connection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = test_server();
    tokio::spawn(async move {
        let _ = server.serve_tcp(listener).await;
    });
    // Wait for server to bind
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = r#"{"jsonrpc":"2.0","method":"gate.list","params":{},"id":200}"#;
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert!(parsed.is_object());
    assert!(parsed.get("result").is_some() || parsed.get("error").is_some());
}

#[tokio::test]
async fn test_handle_connection_http_content_length_zero() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let req = "POST /rpc HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    client.write_all(req.as_bytes()).await.unwrap();
    client.flush().await.unwrap();

    let mut buf = vec![0u8; 512];
    let n = client.read(&mut buf).await.unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("error") || resp.contains("Parse error"));
}

#[tokio::test]
async fn test_handle_tcp_connection_first_line_http_prefix() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let body = r#"{"jsonrpc":"2.0","method":"compute.health","params":{},"id":60}"#;
    let req = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    assert!(!buf.is_empty());
}

#[tokio::test]
async fn test_handle_connection_unix_response_serialization() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    let req = r#"{"jsonrpc":"2.0","method":"toadstool.query_capabilities","params":{},"id":70}"#;
    client.write_all(req.as_bytes()).await.unwrap();
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();

    let mut buf = String::new();
    client.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert!(parsed.get("result").is_some() || parsed.get("error").is_some());
}

#[tokio::test]
async fn test_handle_connection_unix_malformed_json_trailing() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let (mut client, server) = UnixStream::pair().unwrap();
    let server_instance = test_server();
    tokio::spawn(async move {
        let _ = server_instance.handle_connection(server).await;
    });

    client
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"x\"\n")
        .await
        .unwrap();
    client.flush().await.unwrap();

    let mut buf = vec![0u8; 256];
    let n = client.read(&mut buf).await.unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    assert!(resp.contains("error"));
}

#[tokio::test]
async fn test_handle_tcp_connection_compute_submit_validation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_instance = test_server();
    let server_arc = Arc::new(server_instance);
    let server_clone = Arc::clone(&server_arc);
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ = server_clone.handle_tcp_connection(stream).await;
    });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req =
        r#"{"jsonrpc":"2.0","method":"compute.submit","params":{"job_id":"bad-uuid"},"id":80}"#;
    stream.write_all(req.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();
    stream.shutdown().await.ok();

    let mut buf = String::new();
    stream.read_to_string(&mut buf).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(buf.trim()).unwrap();
    assert_eq!(parsed["error"]["code"], INVALID_PARAMS);
}
