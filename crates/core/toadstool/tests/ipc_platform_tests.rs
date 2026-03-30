// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for IPC platform layer
//!
//! Tests the platform abstraction layer for universal IPC including
//! Unix sockets, abstract sockets (Linux), and TCP sockets.

use tempfile::TempDir;
use toadstool::ipc::platform::{Endpoint, bind_unix, connect_unix};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_endpoint_serialization() {
    // Test that Endpoint can be serialized/deserialized
    use serde_json;

    let unix_endpoint = Endpoint::Unix {
        path: std::path::PathBuf::from("/tmp/test.sock"),
    };

    let json = serde_json::to_string(&unix_endpoint).unwrap();
    let deserialized: Endpoint = serde_json::from_str(&json).unwrap();

    assert_eq!(unix_endpoint, deserialized);
}

#[tokio::test]
async fn test_unix_socket_basic() {
    let temp_dir = TempDir::new().unwrap();
    let socket_path = temp_dir.path().join("test.sock");

    // Bind server
    let listener = bind_unix(&socket_path).await.unwrap();

    // Spawn server task
    let server_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");
        stream.write_all(b"world").await.unwrap();
    });

    // Connect client
    let mut client = connect_unix(&socket_path).await.unwrap();
    client.write_all(b"hello").await.unwrap();

    let mut buf = [0u8; 5];
    client.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"world");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_endpoint_equality() {
    let path1 = std::path::PathBuf::from("/tmp/test1.sock");
    let path2 = std::path::PathBuf::from("/tmp/test2.sock");

    let ep1 = Endpoint::Unix {
        path: path1.clone(),
    };
    let ep2 = Endpoint::Unix { path: path1 };
    let ep3 = Endpoint::Unix { path: path2 };

    assert_eq!(ep1, ep2);
    assert_ne!(ep1, ep3);
}

#[tokio::test]
async fn test_tcp_endpoint() {
    let endpoint = Endpoint::Tcp {
        host: "127.0.0.1".to_string(),
        port: 0, // Use 0 for OS-assigned port
    };

    let json = serde_json::to_string(&endpoint).unwrap();
    assert!(json.contains("127.0.0.1"));
    assert!(json.contains("tcp"));

    let deserialized: Endpoint = serde_json::from_str(&json).unwrap();
    assert_eq!(endpoint, deserialized);
}
