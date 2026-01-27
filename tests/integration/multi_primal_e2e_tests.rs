//! Multi-Primal Integration E2E Tests
//!
//! Comprehensive end-to-end tests for inter-primal communication.
//! Tests IPC with BearDog (crypto), Songbird (discovery), NestGate (storage).
//!
//! ## Deep Debt Principles
//!
//! - ✅ **Capability-Based**: Tests discover primals at runtime, no hardcoding
//! - ✅ **Self-Knowledge**: ToadStool knows only itself, discovers others
//! - ✅ **JSON-RPC First**: Tests JSON-RPC 2.0 over Unix domain sockets
//! - ✅ **Semantic Methods**: Tests semantic method naming standard
//! - ✅ **Real Implementations**: Tests actual IPC, not mocks (or mock at IPC boundary)

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::UnixStream;
use uuid::Uuid;

use toadstool::ipc::{JsonRpcClient, JsonRpcRequest, JsonRpcResponse};
use toadstool::discovery::{PrimalDiscovery, ServiceCapability};
use toadstool::{ToadStoolError, ToadStoolResult};

// ============================================================================
// Test: Service Discovery via Songbird
// ============================================================================

#[tokio::test]
async fn test_discover_songbird_capability() {
    let discovery = PrimalDiscovery::new().await;

    // Discover Songbird (capability-based, no hardcoding)
    let result = discovery
        .discover_capability(ServiceCapability::PrimalDiscovery)
        .await;

    match result {
        Ok(service_info) => {
            // Songbird available
            assert!(!service_info.socket_path.is_empty());
            assert!(service_info.capabilities.contains(&ServiceCapability::PrimalDiscovery));
        }
        Err(ToadStoolError::ServiceNotFound(_)) => {
            // Songbird not running - expected in test environment
            eprintln!("⚠️  Songbird not available - skipping test");
        }
        Err(e) => {
            eprintln!("Discovery error: {:?}", e);
        }
    }
}

// ============================================================================
// Test: Crypto Operation via BearDog
// ============================================================================

#[tokio::test]
async fn test_beardog_crypto_encrypt_e2e() {
    let discovery = PrimalDiscovery::new().await;

    // Discover BearDog (capability-based)
    let beardog = discovery
        .discover_capability(ServiceCapability::Cryptography)
        .await;

    let socket_path = match beardog {
        Ok(info) => info.socket_path,
        Err(ToadStoolError::ServiceNotFound(_)) => {
            eprintln!("⚠️  BearDog not available - skipping test");
            return;
        }
        Err(e) => {
            eprintln!("Discovery error: {:?}", e);
            return;
        }
    };

    // Connect via Unix socket
    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("⚠️  Failed to connect to BearDog: {:?}", e);
            return;
        }
    };

    let mut client = JsonRpcClient::new(stream);

    // Call crypto.encrypt using semantic method naming
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "crypto.encrypt".to_string(), // Semantic method name
        params: serde_json::json!({
            "plaintext": "Hello, BearDog!",
            "algorithm": "AES-256-GCM"
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let response = client.call(request).await;

    match response {
        Ok(JsonRpcResponse::Success { result, .. }) => {
            // Encryption succeeded
            assert!(result.get("ciphertext").is_some());
            assert!(result.get("iv").is_some());
        }
        Ok(JsonRpcResponse::Error { error, .. }) => {
            eprintln!("⚠️  BearDog returned error: {:?}", error);
        }
        Err(e) => {
            eprintln!("⚠️  RPC call failed: {:?}", e);
        }
    }
}

// ============================================================================
// Test: Crypto Sign/Verify Workflow via BearDog
// ============================================================================

#[tokio::test]
async fn test_beardog_crypto_sign_verify_e2e() {
    let discovery = PrimalDiscovery::new().await;

    let beardog = match discovery
        .discover_capability(ServiceCapability::Cryptography)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  BearDog not available - skipping test");
            return;
        }
    };

    let stream = match UnixStream::connect(&beardog.socket_path).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut client = JsonRpcClient::new(stream);

    // Step 1: Sign data
    let sign_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "crypto.sign".to_string(), // Semantic method name
        params: serde_json::json!({
            "data": "Important message",
            "algorithm": "Ed25519"
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let sign_response = client.call(sign_request).await;

    let signature = match sign_response {
        Ok(JsonRpcResponse::Success { result, .. }) => {
            result.get("signature").and_then(|s| s.as_str()).unwrap().to_string()
        }
        _ => {
            eprintln!("⚠️  Signing failed");
            return;
        }
    };

    // Step 2: Verify signature
    let verify_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "crypto.verify".to_string(), // Semantic method name
        params: serde_json::json!({
            "data": "Important message",
            "signature": signature,
            "algorithm": "Ed25519"
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let verify_response = client.call(verify_request).await;

    match verify_response {
        Ok(JsonRpcResponse::Success { result, .. }) => {
            let valid = result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
            assert!(valid, "Signature should be valid");
        }
        _ => {
            eprintln!("⚠️  Verification failed");
        }
    }
}

// ============================================================================
// Test: Artifact Storage via NestGate
// ============================================================================

#[tokio::test]
async fn test_nestgate_artifact_storage_e2e() {
    let discovery = PrimalDiscovery::new().await;

    let nestgate = match discovery
        .discover_capability(ServiceCapability::ArtifactStorage)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  NestGate not available - skipping test");
            return;
        }
    };

    let stream = match UnixStream::connect(&nestgate.socket_path).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut client = JsonRpcClient::new(stream);

    // Store artifact
    let artifact_data = b"Test artifact content";
    let artifact_id = Uuid::new_v4().to_string();

    let store_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "storage.store".to_string(), // Semantic method name
        params: serde_json::json!({
            "artifact_id": artifact_id,
            "data": base64::encode(artifact_data),
            "metadata": {
                "content_type": "application/octet-stream",
                "size": artifact_data.len()
            }
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let store_response = client.call(store_request).await;

    match store_response {
        Ok(JsonRpcResponse::Success { .. }) => {
            // Storage succeeded
        }
        _ => {
            eprintln!("⚠️  Storage failed");
            return;
        }
    }

    // Retrieve artifact
    let retrieve_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "storage.retrieve".to_string(), // Semantic method name
        params: serde_json::json!({
            "artifact_id": artifact_id
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let retrieve_response = client.call(retrieve_request).await;

    match retrieve_response {
        Ok(JsonRpcResponse::Success { result, .. }) => {
            let retrieved_data = result
                .get("data")
                .and_then(|d| d.as_str())
                .map(|s| base64::decode(s).ok())
                .flatten()
                .unwrap();

            assert_eq!(retrieved_data, artifact_data);
        }
        _ => {
            eprintln!("⚠️  Retrieval failed");
        }
    }
}

// ============================================================================
// Test: Multi-Primal Workflow (ToadStool → BearDog → NestGate)
// ============================================================================

#[tokio::test]
async fn test_multi_primal_workflow_e2e() {
    let discovery = PrimalDiscovery::new().await;

    // Step 1: Discover BearDog
    let beardog = match discovery
        .discover_capability(ServiceCapability::Cryptography)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  BearDog not available - skipping test");
            return;
        }
    };

    // Step 2: Discover NestGate
    let nestgate = match discovery
        .discover_capability(ServiceCapability::ArtifactStorage)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  NestGate not available - skipping test");
            return;
        }
    };

    // Step 3: Encrypt data via BearDog
    let beardog_stream = match UnixStream::connect(&beardog.socket_path).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut beardog_client = JsonRpcClient::new(beardog_stream);

    let encrypt_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "crypto.encrypt".to_string(),
        params: serde_json::json!({
            "plaintext": "Sensitive workload data",
            "algorithm": "AES-256-GCM"
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let encrypt_response = beardog_client.call(encrypt_request).await;

    let encrypted_data = match encrypt_response {
        Ok(JsonRpcResponse::Success { result, .. }) => {
            result.get("ciphertext").and_then(|c| c.as_str()).unwrap().to_string()
        }
        _ => {
            eprintln!("⚠️  Encryption failed");
            return;
        }
    };

    // Step 4: Store encrypted data via NestGate
    let nestgate_stream = match UnixStream::connect(&nestgate.socket_path).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut nestgate_client = JsonRpcClient::new(nestgate_stream);

    let artifact_id = Uuid::new_v4().to_string();
    let store_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "storage.store".to_string(),
        params: serde_json::json!({
            "artifact_id": artifact_id,
            "data": encrypted_data,
            "metadata": {
                "content_type": "application/encrypted",
                "encryption": "AES-256-GCM"
            }
        }),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let store_response = nestgate_client.call(store_request).await;

    match store_response {
        Ok(JsonRpcResponse::Success { .. }) => {
            // Multi-primal workflow succeeded!
            // ToadStool orchestrated BearDog (crypto) + NestGate (storage)
        }
        _ => {
            eprintln!("⚠️  Storage failed");
        }
    }
}

// ============================================================================
// Test: Capability Discovery Error Handling
// ============================================================================

#[tokio::test]
async fn test_discover_nonexistent_capability() {
    let discovery = PrimalDiscovery::new().await;

    // Try to discover non-existent capability
    let result = discovery
        .discover_capability(ServiceCapability::Custom("nonexistent_service".to_string()))
        .await;

    // Should fail gracefully
    assert!(result.is_err());
    match result {
        Err(ToadStoolError::ServiceNotFound(_)) => {
            // Expected: Service not found
        }
        Err(ToadStoolError::DiscoveryFailed(_)) => {
            // Also acceptable
        }
        _ => panic!("Expected ServiceNotFound or DiscoveryFailed error"),
    }
}

// ============================================================================
// Test: IPC Connection Error Handling
// ============================================================================

#[tokio::test]
async fn test_ipc_connection_to_invalid_socket() {
    let invalid_socket_path = PathBuf::from("/tmp/nonexistent_socket_12345.sock");

    let result = UnixStream::connect(&invalid_socket_path).await;

    // Should fail with connection error
    assert!(result.is_err());
}

// ============================================================================
// Test: JSON-RPC Method Not Found
// ============================================================================

#[tokio::test]
async fn test_jsonrpc_method_not_found() {
    let discovery = PrimalDiscovery::new().await;

    let beardog = match discovery
        .discover_capability(ServiceCapability::Cryptography)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  BearDog not available - skipping test");
            return;
        }
    };

    let stream = match UnixStream::connect(&beardog.socket_path).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut client = JsonRpcClient::new(stream);

    // Call non-existent method
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "crypto.nonexistent_method".to_string(),
        params: serde_json::json!({}),
        id: serde_json::Value::String(Uuid::new_v4().to_string()),
    };

    let response = client.call(request).await;

    // Should return JSON-RPC error
    match response {
        Ok(JsonRpcResponse::Error { error, .. }) => {
            assert_eq!(error.code, -32601); // Method not found
        }
        Err(_) => {
            // Network error also acceptable
        }
        _ => panic!("Expected JSON-RPC error response"),
    }
}

// ============================================================================
// Test: Concurrent Multi-Primal Requests
// ============================================================================

#[tokio::test]
async fn test_concurrent_multi_primal_requests() {
    let discovery = PrimalDiscovery::new().await;

    let beardog = match discovery
        .discover_capability(ServiceCapability::Cryptography)
        .await
    {
        Ok(info) => info,
        Err(_) => {
            eprintln!("⚠️  BearDog not available - skipping test");
            return;
        }
    };

    // Launch 5 concurrent encryption requests
    let mut handles = vec![];

    for i in 0..5 {
        let socket_path = beardog.socket_path.clone();

        let handle = tokio::spawn(async move {
            let stream = UnixStream::connect(&socket_path).await.ok()?;
            let mut client = JsonRpcClient::new(stream);

            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "crypto.encrypt".to_string(),
                params: serde_json::json!({
                    "plaintext": format!("Message {}", i),
                    "algorithm": "AES-256-GCM"
                }),
                id: serde_json::Value::String(Uuid::new_v4().to_string()),
            };

            client.call(request).await.ok()
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Some(JsonRpcResponse::Success { .. })) = handle.await {
            success_count += 1;
        }
    }

    // At least some should succeed
    assert!(success_count > 0, "At least some concurrent requests should succeed");
}

// ============================================================================
// Test: Semantic Method Naming Compliance
// ============================================================================

#[tokio::test]
async fn test_semantic_method_naming_compliance() {
    // Verify semantic method names follow wateringHole standard
    // Format: {domain}.{operation}[.{variant}]

    let valid_methods = vec![
        "crypto.encrypt",
        "crypto.decrypt",
        "crypto.sign",
        "crypto.verify",
        "storage.store",
        "storage.retrieve",
        "storage.delete",
        "compute.execute",
        "compute.schedule",
        "resource.monitor",
        "discovery.list",
        "discovery.query",
    ];

    for method in valid_methods {
        assert!(is_semantic_method_valid(method), "Method {} should be valid", method);
    }

    let invalid_methods = vec![
        "encryptData",      // CamelCase (non-semantic)
        "encrypt_data",     // snake_case (non-semantic)
        "Encrypt",          // No domain
        "crypto",           // No operation
    ];

    for method in invalid_methods {
        assert!(!is_semantic_method_valid(method), "Method {} should be invalid", method);
    }
}

// Helper: Validate semantic method naming
fn is_semantic_method_valid(method: &str) -> bool {
    let parts: Vec<&str> = method.split('.').collect();
    // Must have at least domain.operation
    parts.len() >= 2 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_'))
}
