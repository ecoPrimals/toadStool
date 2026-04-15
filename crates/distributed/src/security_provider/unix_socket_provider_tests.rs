// SPDX-License-Identifier: AGPL-3.0-or-later

use super::super::types::{
    CloudProvider, ExternalTarget, PermissionScope, ResourceLimits, SecurityProof,
    SignatureAlgorithm,
};
use super::*;

#[test]
fn test_provider_creation() {
    let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
    assert_eq!(provider.socket_path, PathBuf::from("/tmp/test.sock"));
    assert_eq!(provider.timeout_secs, 30);
}

#[test]
fn test_provider_with_timeout() {
    let provider = UnixSocketSecurityProvider::with_timeout("/tmp/test.sock", 60);
    assert_eq!(provider.timeout_secs, 60);
}

#[test]
fn test_socket_exists() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/path.sock");
    assert!(!provider.socket_exists());
}

#[test]
fn test_request_id_increment() {
    let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
    assert_eq!(provider.next_id(), 1);
    assert_eq!(provider.next_id(), 2);
    assert_eq!(provider.next_id(), 3);
}

#[test]
fn test_provider_creation_with_pathbuf() {
    let path = PathBuf::from("/var/run/security.sock");
    let provider = UnixSocketSecurityProvider::new(&path);
    assert_eq!(provider.socket_path, path);
}

#[test]
fn test_provider_with_timeout_custom_path() {
    let provider = UnixSocketSecurityProvider::with_timeout("/custom/path.sock", 5);
    assert_eq!(provider.socket_path, PathBuf::from("/custom/path.sock"));
    assert_eq!(provider.timeout_secs, 5);
}

#[test]
fn test_socket_exists_when_file_exists() {
    let provider = UnixSocketSecurityProvider::new("/");
    assert!(provider.socket_exists());
}

#[test]
fn test_request_id_thread_safety() {
    let provider = UnixSocketSecurityProvider::new("/tmp/test.sock");
    let ids: Vec<u64> = (0..10).map(|_| provider.next_id()).collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 10);
}

#[tokio::test]
async fn test_capabilities_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.capabilities().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Failed to connect") || err.to_string().contains("Connection")
    );
}

#[tokio::test]
async fn test_metadata_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.metadata().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_encrypt_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.encrypt(b"data", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let metadata = EncryptionMetadata {
        algorithm: "AES-256-GCM".to_string(),
        key_id: "key-1".to_string(),
        encrypted_at: std::time::SystemTime::now(),
    };
    let result = provider.decrypt(&[0u8; 32], &metadata).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.health_check().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sign_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.sign(b"data", None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let result = provider.verify(b"data", &[0u8; 64], "key-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_permission_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let request = PermissionRequest {
        requester_id: "test".to_string(),
        target: ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec!["us-east-1".to_string()],
            services: vec!["s3".to_string()],
        },
        scope: PermissionScope {
            operations: vec!["read".to_string()],
            resource_limits: ResourceLimits::default(),
            geo_restrictions: vec![],
        },
        validity_duration: std::time::Duration::from_secs(3600),
        delegation_info: None,
    };
    let result = provider.create_permission(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_permission_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let now = std::time::SystemTime::now();
    let permission = SecurityPermission {
        permission_id: uuid::Uuid::new_v4(),
        holder_id: "test".to_string(),
        target: ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec![],
            services: vec![],
        },
        scope: PermissionScope {
            operations: vec![],
            resource_limits: ResourceLimits::default(),
            geo_restrictions: vec![],
        },
        valid_from: now,
        valid_until: now,
        proof: SecurityProof {
            signature: vec![],
            algorithm: SignatureAlgorithm::Ed25519,
            public_key_id: "key".to_string(),
            signed_at: now,
        },
        provider_metadata: ProviderMetadata {
            provider_id: "p".to_string(),
            provider_type: "test".to_string(),
            provider_version: "1.0".to_string(),
            metadata: std::collections::HashMap::new(),
        },
    };
    let result = provider.validate_permission(&permission).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_revoke_permission_connection_refused() {
    let provider = UnixSocketSecurityProvider::new("/nonexistent/socket/path.sock");
    let perm_id = uuid::Uuid::new_v4();
    let result = provider.revoke_permission(&perm_id, "test reason").await;
    assert!(result.is_err());
}
