// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_provider_creation() {
    let provider = TcpSecurityProvider::new("localhost", 9090);
    assert_eq!(provider.host, "localhost");
    assert_eq!(provider.port, 9090);
}

#[test]
fn test_provider_with_timeout() {
    let provider = TcpSecurityProvider::with_timeout("example.com", 443, 60);
    assert_eq!(provider.timeout_secs, 60);
}

#[test]
fn test_addr_format() {
    let provider = TcpSecurityProvider::new("host.example", 8080);
    assert_eq!(provider.addr(), "host.example:8080");
}

#[test]
fn test_request_id_increment() {
    let provider = TcpSecurityProvider::new("localhost", 9090);
    assert_eq!(provider.next_id(), 1);
    assert_eq!(provider.next_id(), 2);
    assert_eq!(provider.next_id(), 3);
}

#[tokio::test]
async fn test_connection_refused() {
    let provider = TcpSecurityProvider::new("127.0.0.1", 1);
    let result = provider.capabilities().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_connection_refused() {
    let provider = TcpSecurityProvider::new("127.0.0.1", 1);
    let result = provider.health_check().await;
    assert!(result.is_err());
}

#[test]
fn test_json_rpc_request_serialization() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "test.method".to_string(),
        params: serde_json::json!({"key": "value"}),
        id: 1,
    };
    let json = serde_json::to_string(&req);
    assert!(json.is_ok());
    assert!(json.unwrap().contains("test.method"));
}

#[test]
fn test_json_rpc_response_deserialization_with_result() {
    let json = r#"{"jsonrpc":"2.0","result":{"healthy":true},"id":1}"#;
    let parsed: Result<JsonRpcResponse<serde_json::Value>, _> = serde_json::from_str(json);
    assert!(parsed.is_ok());
    let resp = parsed.unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn test_json_rpc_response_deserialization_with_error() {
    let json =
        r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid request"},"id":1}"#;
    let parsed: Result<JsonRpcResponse<serde_json::Value>, _> = serde_json::from_str(json);
    assert!(parsed.is_ok());
    let resp = parsed.unwrap();
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32600);
    assert!(err.message.contains("Invalid"));
}

#[test]
fn test_encrypt_request_structure() {
    let data = [1u8, 2, 3, 4, 5];
    let req = EncryptRequest {
        data: &data,
        options: None,
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[test]
fn test_decrypt_request_structure() {
    use crate::security_provider::types::EncryptionMetadata;
    use std::time::SystemTime;
    let ciphertext = [0xaau8, 0xbb];
    let metadata = EncryptionMetadata {
        algorithm: "AES-256-GCM".to_string(),
        key_id: "key-1".to_string(),
        encrypted_at: SystemTime::now(),
    };
    let req = DecryptRequest {
        ciphertext: &ciphertext,
        metadata: &metadata,
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[test]
fn test_verify_request_structure() {
    let data = [1u8, 2, 3];
    let sig = [0x11u8, 0x22];
    let req = VerifyRequest {
        data: &data,
        signature: &sig,
        public_key_id: "pk-1",
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}

#[test]
fn test_revoke_request_structure() {
    let perm_id = uuid::Uuid::new_v4();
    let req = RevokeRequest {
        permission_id: &perm_id,
        reason: "test revocation",
    };
    let json = serde_json::to_value(&req);
    assert!(json.is_ok());
}
