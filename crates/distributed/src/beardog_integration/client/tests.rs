use super::*;
use toadstool::encryption::CryptoProvider;

#[test]
#[allow(deprecated)]
fn test_beardog_client_new_creates_client() {
    let config = BearDogConfig::default();
    let result = BearDogClient::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_provider_id_returns_beardog() {
    #[allow(deprecated)]
    let client = BearDogClient::new(BearDogConfig::default()).unwrap();
    assert_eq!(client.provider_id(), capabilities::CRYPTO);
}

#[test]
fn test_capabilities_returns_default() {
    #[allow(deprecated)]
    let client = BearDogClient::new(BearDogConfig::default()).unwrap();
    let caps = client.capabilities();
    assert!(!caps.algorithms.is_empty());
    assert!(
        caps.algorithms.contains(&"chacha20poly1305".to_string())
            || caps.algorithms.contains(&"aes-256-gcm".to_string())
    );
}

#[test]
fn test_parse_capabilities_security_level_standard() {
    let json = serde_json::json!({
        "algorithms": ["aes-256-gcm"],
        "security_level": "standard",
        "hardware_backed": false
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::Standard
    ));
}

#[test]
fn test_parse_capabilities_security_level_enhanced() {
    let json = serde_json::json!({
        "security_level": "enhanced"
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::Enhanced
    ));
}

#[test]
fn test_parse_capabilities_missing_algorithms_uses_default() {
    let json = serde_json::json!({"security_level": "standard"});
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert_eq!(
        cap.algorithms,
        vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()]
    );
}

#[test]
fn test_parse_capabilities_array_with_mixed_types() {
    let json = serde_json::json!({
        "algorithms": ["aes", 42, "gcm", null],
        "security_level": "standard"
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert_eq!(cap.algorithms, vec!["aes", "gcm"]);
}

#[test]
fn test_parse_capabilities_hardware_secured() {
    let json = serde_json::json!({
        "algorithms": ["aes-256-gcm"],
        "security_level": "hardware_secured",
        "hardware_backed": true
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::HardwareSecured
    ));
    assert!(cap.hardware_backed);
}

#[test]
fn test_parse_capabilities_hardware_variant() {
    let json = serde_json::json!({
        "security_level": "hardware"
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::HardwareSecured
    ));
}

#[test]
fn test_parse_capabilities_empty_response_uses_defaults() {
    let json = serde_json::json!({});
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert_eq!(
        cap.algorithms,
        vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()]
    );
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::Enhanced
    ));
    assert!(!cap.hardware_backed);
}

#[test]
fn test_parse_capabilities_unknown_security_level_defaults_to_enhanced() {
    let json = serde_json::json!({
        "security_level": "unknown_level"
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert!(matches!(
        cap.security_level,
        toadstool::encryption::SecurityLevel::Enhanced
    ));
}

#[test]
fn test_parse_capabilities_custom_algorithms() {
    let json = serde_json::json!({
        "algorithms": ["custom-algo-1", "custom-algo-2"],
        "security_level": "standard"
    });
    let cap = BearDogClient::parse_capabilities_from_json(&json);
    assert_eq!(cap.algorithms.len(), 2);
    assert!(cap.algorithms.contains(&"custom-algo-1".to_string()));
    assert!(cap.algorithms.contains(&"custom-algo-2".to_string()));
}

#[tokio::test]
async fn test_query_capabilities_service_unavailable() {
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-test-12345.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let result = client.query_capabilities_async().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Beardog") || err.to_string().contains("capabilities"),
        "expected beardog/capabilities error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_encrypt_service_unavailable() {
    use crate::beardog_integration::types::{EncryptionOperation, SecurityLevel};
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-encrypt.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let request = EncryptionRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: EncryptionOperation::Encrypt,
        data: b"secret".to_vec(),
        key_id: None,
        algorithm: None,
        security_level: SecurityLevel::Standard,
    };
    let result = client.encrypt(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_service_unavailable() {
    use crate::beardog_integration::types::{EncryptionOperation, SecurityLevel};
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-decrypt.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let request = EncryptionRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: EncryptionOperation::Decrypt,
        data: vec![],
        key_id: None,
        algorithm: None,
        security_level: SecurityLevel::Standard,
    };
    let result = client.decrypt(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_service_unavailable() {
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-health.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let result = client.health_check().await;
    assert!(result.is_ok());
    let endpoints = result.unwrap();
    assert!(endpoints.is_empty() || endpoints.iter().all(|e| !e.healthy));
}

#[tokio::test]
async fn test_sign_service_unavailable() {
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-sign.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let result = client.sign(b"data to sign").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_service_unavailable() {
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-verify.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let result = client.verify(b"data", b"sig", "key-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_key_management_service_unavailable() {
    use crate::beardog_integration::types::KeyOperation;
    let config = BearDogConfig::default();
    let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-beardog-km.sock");
    let client = BearDogClient::new_with_socket_path(config, nonexistent).unwrap();
    let request = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate,
        key_id: None,
        security_level: None,
    };
    let result = client.key_management(request).await;
    assert!(result.is_err());
}
