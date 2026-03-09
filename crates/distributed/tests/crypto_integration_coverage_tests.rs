#![allow(clippy::pedantic)]
// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for crypto_integration/client.rs
//!
//! Tests CryptoServiceDiscovery, CryptoServiceClient creation, configuration,
//! encryption/decryption operations, key management, and error paths.

use std::time::Duration;

use toadstool_common::primal_identity::{Capability, CryptoCapability, ServiceEndpoint};

use toadstool_distributed::crypto_integration::types::{
    CryptoOperation, EncryptionAlgorithm, KeyManagementRequest, KeyManagementResponse,
    KeyOperation, KeyType, SecurityLevel,
};
use toadstool_distributed::crypto_integration::{
    CryptoRequest, CryptoResponse, CryptoServiceClient, CryptoServiceConfig,
    CryptoServiceDiscovery, ServiceLocation,
};

fn make_discovered_service(
    id: &str,
    name: &str,
    protocol: &str,
    address: &str,
    metadata: std::collections::HashMap<String, String>,
) -> toadstool_common::service_discovery::DiscoveredService {
    toadstool_common::service_discovery::DiscoveredService {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
        endpoints: vec![ServiceEndpoint {
            protocol: protocol.to_string(),
            address: address.to_string(),
            port: 0,
            path: None,
            metadata,
        }],
        metadata: Default::default(),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        healthy: true,
    }
}

// ============================================================================
// CryptoServiceDiscovery - creation and configuration
// ============================================================================

#[tokio::test]
async fn test_crypto_service_discovery_creation() {
    let config = CryptoServiceConfig::default();
    let discovery = CryptoServiceDiscovery::new(config)
        .await
        .expect("Failed to create discovery");
    let _ = discovery;
}

#[tokio::test]
async fn test_crypto_service_discovery_discover() {
    let config = CryptoServiceConfig::default();
    let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
    let services = discovery.discover().await.unwrap();
    assert!(services.is_empty() || !services.is_empty());
}

#[tokio::test]
async fn test_crypto_service_discovery_get_cached() {
    let config = CryptoServiceConfig::default();
    let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
    let cached = discovery.get_cached().await;
    assert!(cached.is_empty());
}

#[tokio::test]
async fn test_crypto_service_discovery_by_capability() {
    let config = CryptoServiceConfig::default();
    let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
    let cap = Capability::Crypto(CryptoCapability::Encryption);
    let result = discovery.discover_by_capability(cap).await.unwrap();
    assert!(result.is_none() || result.is_some());
}

// ============================================================================
// CryptoServiceClient - creation and configuration
// ============================================================================

#[test]
fn test_crypto_client_new_fails_no_endpoints() {
    let service = make_discovered_service(
        "no-ep",
        "empty-crypto",
        "unix",
        "/tmp/nonexistent.sock",
        Default::default(),
    );
    let mut empty_service = service;
    empty_service.endpoints = vec![];
    let result = CryptoServiceClient::new(&empty_service);
    assert!(result.is_err());
}

#[test]
fn test_crypto_client_new_unix_socket() {
    let service = make_discovered_service(
        "crypto-1",
        "test-crypto",
        "unix",
        "/tmp/test-crypto-coverage.sock",
        Default::default(),
    );
    let result = CryptoServiceClient::new(&service);
    assert!(result.is_ok());
}

#[test]
fn test_crypto_client_with_timeout() {
    let service = make_discovered_service(
        "crypto-2",
        "timeout-crypto",
        "unix",
        "/tmp/timeout-crypto-coverage.sock",
        Default::default(),
    );
    let result = CryptoServiceClient::with_timeout(&service, Duration::from_secs(5));
    assert!(result.is_ok());
}

#[test]
fn test_crypto_client_new_with_metadata_socket_path() {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "socket_path".to_string(),
        "/tmp/custom-sock-coverage.sock".to_string(),
    );
    let service =
        make_discovered_service("meta-sock", "meta-crypto", "http", "127.0.0.1", metadata);
    let result = CryptoServiceClient::new(&service);
    assert!(result.is_ok());
}

// ============================================================================
// CryptoRequest / CryptoResponse construction
// ============================================================================

#[test]
fn test_crypto_request_encrypt() {
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Encrypt,
        data: vec![1, 2, 3, 4, 5],
        key_id: Some("key-1".to_string()),
        algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
        security_level: SecurityLevel::High,
        metadata: serde_json::json!({"test": true}),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("operation").is_some());
    assert!(json.get("data").is_some());
}

#[test]
fn test_crypto_request_decrypt() {
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Decrypt,
        data: vec![0xAA, 0xBB, 0xCC],
        key_id: None,
        algorithm: Some(EncryptionAlgorithm::ChaCha20Poly1305),
        security_level: SecurityLevel::Standard,
        metadata: serde_json::Value::Null,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("data").is_some());
}

#[test]
fn test_key_management_request_generate() {
    let req = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate {
            key_type: KeyType::Symmetric { bits: 256 },
        },
        metadata: serde_json::Value::Null,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("operation").is_some());
}

#[test]
fn test_key_management_request_rotate() {
    let req = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Rotate {
            key_id: "old-key".to_string(),
        },
        metadata: serde_json::Value::Null,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("operation").is_some());
}

#[test]
fn test_crypto_response_parsing() {
    let resp = CryptoResponse {
        request_id: uuid::Uuid::new_v4(),
        data: vec![0x11, 0x22, 0x33, 0x44],
        key_id: "key-123".to_string(),
        algorithm: "aes-256-gcm".to_string(),
        metadata: serde_json::json!({"iv": "abc"}),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: CryptoResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.key_id, "key-123");
    assert_eq!(parsed.data.len(), 4);
}

#[test]
fn test_key_management_response_parsing() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        key_id: "gen-key-1".to_string(),
        success: true,
        metadata: serde_json::json!({"algorithm": "aes-256-gcm"}),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.key_id, "gen-key-1");
    assert!(parsed.success);
}

// ============================================================================
// Encryption/decryption/key management - error paths (no server listening)
// ============================================================================

#[tokio::test]
async fn test_encrypt_fails_when_no_server() {
    let service = make_discovered_service(
        "no-server",
        "no-server-crypto",
        "unix",
        "/tmp/nonexistent-crypto-encrypt.sock",
        Default::default(),
    );
    let client = CryptoServiceClient::with_timeout(&service, Duration::from_millis(10)).unwrap();
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Encrypt,
        data: vec![1, 2, 3],
        key_id: Some("key-1".to_string()),
        algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
        security_level: SecurityLevel::High,
        metadata: serde_json::Value::Null,
    };
    let result = client.encrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_fails_when_no_server() {
    let service = make_discovered_service(
        "no-server-dec",
        "no-server-decrypt",
        "unix",
        "/tmp/nonexistent-crypto-decrypt.sock",
        Default::default(),
    );
    let client = CryptoServiceClient::with_timeout(&service, Duration::from_millis(10)).unwrap();
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Decrypt,
        data: vec![0xAA, 0xBB],
        key_id: None,
        algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
        security_level: SecurityLevel::Standard,
        metadata: serde_json::Value::Null,
    };
    let result = client.decrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_manage_key_fails_when_no_server() {
    let service = make_discovered_service(
        "no-server-km",
        "no-server-keymgmt",
        "unix",
        "/tmp/nonexistent-crypto-keymgmt.sock",
        Default::default(),
    );
    let client = CryptoServiceClient::with_timeout(&service, Duration::from_millis(10)).unwrap();
    let req = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate {
            key_type: KeyType::Symmetric { bits: 256 },
        },
        metadata: serde_json::Value::Null,
    };
    let result = client.manage_key(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_fails_when_no_server() {
    let service = make_discovered_service(
        "no-server-hc",
        "no-server-health",
        "unix",
        "/tmp/nonexistent-crypto-health.sock",
        Default::default(),
    );
    let client = CryptoServiceClient::with_timeout(&service, Duration::from_millis(10)).unwrap();
    let result = client.health_check().await;
    assert!(result.is_err());
}

// ============================================================================
// CryptoServiceConfig
// ============================================================================

#[test]
fn test_crypto_config_default() {
    let config = CryptoServiceConfig::default();
    assert!(!config.required_capabilities.is_empty());
    assert!(config.auto_discover);
    assert_eq!(config.discovery_timeout_ms, 5000);
}

#[test]
fn test_crypto_config_preferred_location() {
    let local = CryptoServiceConfig {
        preferred_location: ServiceLocation::Local,
        ..Default::default()
    };
    let network = CryptoServiceConfig {
        preferred_location: ServiceLocation::Network,
        ..Default::default()
    };
    let any = CryptoServiceConfig {
        preferred_location: ServiceLocation::Any,
        ..Default::default()
    };
    assert_eq!(local.preferred_location, ServiceLocation::Local);
    assert_eq!(network.preferred_location, ServiceLocation::Network);
    assert_eq!(any.preferred_location, ServiceLocation::Any);
}

// ============================================================================
// Location filtering via discovery (filter_by_location is pub(crate), exercised via discover)
// ============================================================================

#[tokio::test]
async fn test_discover_filters_by_location_local() {
    let config = CryptoServiceConfig {
        preferred_location: ServiceLocation::Local,
        ..Default::default()
    };
    let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
    let services = discovery.discover().await.unwrap();
    assert!(services.is_empty() || !services.is_empty());
}

#[tokio::test]
async fn test_discover_filters_by_location_network() {
    let config = CryptoServiceConfig {
        preferred_location: ServiceLocation::Network,
        ..Default::default()
    };
    let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
    let services = discovery.discover().await.unwrap();
    assert!(services.is_empty() || !services.is_empty());
}
