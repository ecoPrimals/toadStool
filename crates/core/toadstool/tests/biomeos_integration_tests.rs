//! Comprehensive tests for BiomeOS storage integration types

use toadstool::biomeos_integration::*;

// ============================================================================
// StorageProvisioningConfig Tests
// ============================================================================

#[test]
fn test_storage_provisioning_config_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.nestgate_endpoint, "http://localhost:9090");
    assert_eq!(config.storage_tier, "hot");
    assert!(config.backup_enabled);
    assert!(config.replication_enabled);
    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_storage_provisioning_config_clone() {
    let config1 = StorageProvisioningConfig {
        nestgate_endpoint: "http://nestgate:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 2,
    };

    let config2 = config1.clone();

    assert_eq!(config1.nestgate_endpoint, config2.nestgate_endpoint);
    assert_eq!(config1.storage_tier, config2.storage_tier);
    assert_eq!(config1.replication_factor, config2.replication_factor);
}

#[test]
fn test_storage_provisioning_config_serialization() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("nestgate_endpoint"));
}

// ============================================================================
// VolumeStatus Tests (7 variants)
// ============================================================================

#[test]
fn test_volume_status_creating() {
    let status = VolumeStatus::Creating;
    assert_eq!(status, VolumeStatus::Creating);
}

#[test]
fn test_volume_status_available() {
    let status = VolumeStatus::Available;
    assert_eq!(status, VolumeStatus::Available);
}

#[test]
fn test_volume_status_attaching() {
    let status = VolumeStatus::Attaching;
    assert_eq!(status, VolumeStatus::Attaching);
}

#[test]
fn test_volume_status_in_use() {
    let status = VolumeStatus::InUse;
    assert_eq!(status, VolumeStatus::InUse);
}

#[test]
fn test_volume_status_detaching() {
    let status = VolumeStatus::Detaching;
    assert_eq!(status, VolumeStatus::Detaching);
}

#[test]
fn test_volume_status_deleting() {
    let status = VolumeStatus::Deleting;
    assert_eq!(status, VolumeStatus::Deleting);
}

#[test]
fn test_volume_status_error() {
    let status = VolumeStatus::Error("Provisioning failed".to_string());

    match status {
        VolumeStatus::Error(msg) => {
            assert_eq!(msg, "Provisioning failed");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_volume_status_clone() {
    let status1 = VolumeStatus::Available;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

#[test]
fn test_volume_status_serialization() {
    let status = VolumeStatus::InUse;
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// Storage Tier Tests
// ============================================================================

#[test]
fn test_storage_tier_hot() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.storage_tier, "hot");
}

#[test]
fn test_storage_tier_warm() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 2,
    };

    assert_eq!(config.storage_tier, "warm");
}

#[test]
fn test_storage_tier_cold() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert_eq!(config.storage_tier, "cold");
}

// ============================================================================
// Replication Factor Tests
// ============================================================================

#[test]
fn test_replication_factor_one() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert_eq!(config.replication_factor, 1);
}

#[test]
fn test_replication_factor_three() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_replication_factor_five() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 5,
    };

    assert_eq!(config.replication_factor, 5);
}

// ============================================================================
// Backup Configuration Tests
// ============================================================================

#[test]
fn test_backup_enabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert!(config.backup_enabled);
}

#[test]
fn test_backup_disabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert!(!config.backup_enabled);
}

// ============================================================================
// Endpoint Configuration Tests
// ============================================================================

#[test]
fn test_nestgate_endpoint_localhost() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.nestgate_endpoint, "http://localhost:9090");
}

#[test]
fn test_nestgate_endpoint_custom() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "https://nestgate.biomeos.local:9443".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(
        config.nestgate_endpoint,
        "https://nestgate.biomeos.local:9443"
    );
}

// ============================================================================
// StorageProvisioningManager Tests
// ============================================================================

#[test]
fn test_storage_provisioning_manager_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Creation should succeed with in-memory backend
}

#[test]
fn test_storage_provisioning_manager_with_minimal_config() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Creation should succeed with in-memory backend
}

// ============================================================================
// Storage Async Method Tests (Mock Mode)
// ============================================================================

#[tokio::test]
async fn test_provision_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/mnt/data".to_string()),
        backup_policy: Some("daily".to_string()),
    };

    let result = manager.provision_volume(&volume_config).await;

    // In-memory backend returns test VolumeInfo
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-volume");
    assert!(volume_info.id.starts_with("test-"));
    assert_eq!(volume_info.status, "Available");
}

#[tokio::test]
async fn test_provision_persistent_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let pv_config = PersistentVolume {
        name: "test-pv".to_string(),
        capacity: "500Gi".to_string(),
        storage_class: "premium-ssd".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        host_path: None,
    };

    let result = manager.provision_persistent_volume(&pv_config).await;

    // In-memory backend returns test VolumeInfo
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-pv");
    assert!(volume_info.id.starts_with("test-pv-"));
    assert_eq!(volume_info.storage_class, "premium-ssd");
}

#[tokio::test]
async fn test_mount_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager
        .mount_volume("test-volume", "test-service", "/mnt/data")
        .await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unmount_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.unmount_volume("test-volume", "test-service").await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.delete_volume("test-volume").await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_volume_status_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.get_volume_status("test-volume").await;

    // In-memory backend returns Available status when volume exists
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), VolumeStatus::Available);
}

#[tokio::test]
async fn test_list_volumes_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let result = manager.list_volumes().await;

    // In-memory backend returns empty list initially
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_volume_lifecycle_provision_mount_unmount_delete() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // 1. Provision volume
    let volume_config = VolumeConfig {
        name: "lifecycle-test".to_string(),
        size: "10Gi".to_string(),
        storage_class: Some("standard".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/data".to_string()),
        backup_policy: None,
    };

    let provision_result = manager.provision_volume(&volume_config).await;
    assert!(provision_result.is_ok());

    // 2. Mount volume
    let mount_result = manager
        .mount_volume("lifecycle-test", "test-service", "/data")
        .await;
    assert!(mount_result.is_ok());

    // 3. Check status
    let status_result = manager.get_volume_status("lifecycle-test").await;
    assert!(status_result.is_ok());

    // 4. Unmount volume
    let unmount_result = manager
        .unmount_volume("lifecycle-test", "test-service")
        .await;
    assert!(unmount_result.is_ok());

    // 5. Delete volume
    let delete_result = manager.delete_volume("lifecycle-test").await;
    assert!(delete_result.is_ok());
}

#[tokio::test]
async fn test_multiple_volumes_provisioning() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 2,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Provision multiple volumes
    for i in 1..=5 {
        let volume_config = VolumeConfig {
            name: format!("volume-{}", i),
            size: format!("{}Gi", i * 10),
            storage_class: Some("standard".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: Some(format!("/mnt/vol{}", i)),
            backup_policy: None,
        };

        let result = manager.provision_volume(&volume_config).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_replication_enabled_provisioning() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "replicated-volume".to_string(),
        size: "200Gi".to_string(),
        storage_class: Some("replicated-ssd".to_string()),
        access_modes: vec!["ReadWriteMany".to_string()],
        mount_path: Some("/mnt/replicated".to_string()),
        backup_policy: Some("hourly".to_string()),
    };

    let result = manager.provision_volume(&volume_config).await;
    assert!(result.is_ok());
}

// ============================================================================
// Authentication Integration Tests
// ============================================================================

use std::time::Duration;

// ============================================================================
// AuthManagerConfig Tests
// ============================================================================

#[test]
fn test_auth_manager_config_creation() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
    };

    assert_eq!(config.beardog_endpoint, "http://localhost:8080");
    assert_eq!(config.token_refresh_interval, Duration::from_secs(300));
    assert!(config.signature_validation);
    assert_eq!(config.timestamp_window, Duration::from_secs(60));
    assert!(config.replay_protection);
}

#[test]
fn test_auth_manager_config_clone() {
    let config1 = AuthManagerConfig {
        beardog_endpoint: "http://beardog:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(120),
        replay_protection: false,
    };

    let config2 = config1.clone();

    assert_eq!(config1.beardog_endpoint, config2.beardog_endpoint);
    assert_eq!(
        config1.token_refresh_interval,
        config2.token_refresh_interval
    );
}

#[test]
fn test_auth_manager_config_serialization() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("beardog_endpoint"));
}

// ============================================================================
// TokenVerificationStatus Tests (5 variants)
// ============================================================================

#[test]
fn test_token_verification_status_valid() {
    let status = TokenVerificationStatus::Valid;
    assert_eq!(status, TokenVerificationStatus::Valid);
}

#[test]
fn test_token_verification_status_expired() {
    let status = TokenVerificationStatus::Expired;
    assert_eq!(status, TokenVerificationStatus::Expired);
}

#[test]
fn test_token_verification_status_invalid() {
    let status = TokenVerificationStatus::Invalid;
    assert_eq!(status, TokenVerificationStatus::Invalid);
}

#[test]
fn test_token_verification_status_not_found() {
    let status = TokenVerificationStatus::NotFound;
    assert_eq!(status, TokenVerificationStatus::NotFound);
}

#[test]
fn test_token_verification_status_error() {
    let status = TokenVerificationStatus::Error("Signature mismatch".to_string());

    match status {
        TokenVerificationStatus::Error(msg) => {
            assert_eq!(msg, "Signature mismatch");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_token_verification_status_clone() {
    let status1 = TokenVerificationStatus::Valid;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// TokenPropagationStatus Tests (4 variants)
// ============================================================================

#[test]
fn test_token_propagation_status_success() {
    let status = TokenPropagationStatus::Success;
    assert_eq!(status, TokenPropagationStatus::Success);
}

#[test]
fn test_token_propagation_status_failed() {
    let status = TokenPropagationStatus::Failed("Network timeout".to_string());

    match status {
        TokenPropagationStatus::Failed(msg) => {
            assert_eq!(msg, "Network timeout");
        }
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_token_propagation_status_pending() {
    let status = TokenPropagationStatus::Pending;
    assert_eq!(status, TokenPropagationStatus::Pending);
}

#[test]
fn test_token_propagation_status_skipped() {
    let status = TokenPropagationStatus::Skipped("Primal offline".to_string());

    match status {
        TokenPropagationStatus::Skipped(reason) => {
            assert_eq!(reason, "Primal offline");
        }
        _ => panic!("Expected Skipped variant"),
    }
}

#[test]
fn test_token_propagation_status_clone() {
    let status1 = TokenPropagationStatus::Success;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// AuthenticationToken Tests
// ============================================================================

#[test]
fn test_authentication_token_creation() {
    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted_token_value".to_string(),
        public_key: "ed25519_public_key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string(), "nestgate".to_string()],
        scope: vec!["read".to_string(), "write".to_string()],
        claims: std::collections::HashMap::new(),
    };

    assert_eq!(token.id, "token-123");
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.issuer, "beardog");
    assert_eq!(token.audience.len(), 2);
    assert_eq!(token.scope.len(), 2);
}

#[test]
fn test_authentication_token_clone() {
    let token1 = AuthenticationToken {
        id: "token-456".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted_token".to_string(),
        public_key: "public_key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["squirrel".to_string()],
        scope: vec!["read".to_string()],
        claims: std::collections::HashMap::new(),
    };

    let token2 = token1.clone();

    assert_eq!(token1.id, token2.id);
    assert_eq!(token1.issuer, token2.issuer);
}

// ============================================================================
// PropagationResult Tests
// ============================================================================

#[test]
fn test_propagation_result_creation() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenPropagationStatus::Success);
    results.insert("nestgate".to_string(), TokenPropagationStatus::Success);
    results.insert(
        "squirrel".to_string(),
        TokenPropagationStatus::Failed("timeout".to_string()),
    );

    let propagation = PropagationResult {
        total_primals: 3,
        successful_propagations: 2,
        results,
        token_id: "token-789".to_string(),
        propagation_time: chrono::Utc::now(),
    };

    assert_eq!(propagation.total_primals, 3);
    assert_eq!(propagation.successful_propagations, 2);
    assert_eq!(propagation.results.len(), 3);
}

#[test]
fn test_propagation_result_all_success() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenPropagationStatus::Success);
    results.insert("nestgate".to_string(), TokenPropagationStatus::Success);
    results.insert("squirrel".to_string(), TokenPropagationStatus::Success);
    results.insert("songbird".to_string(), TokenPropagationStatus::Success);
    results.insert("biomeos".to_string(), TokenPropagationStatus::Success);

    let propagation = PropagationResult {
        total_primals: 5,
        successful_propagations: 5,
        results,
        token_id: "token-all-success".to_string(),
        propagation_time: chrono::Utc::now(),
    };

    assert_eq!(propagation.total_primals, 5);
    assert_eq!(propagation.successful_propagations, 5);
}

// ============================================================================
// VerificationResult Tests
// ============================================================================

#[test]
fn test_verification_result_creation() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenVerificationStatus::Valid);
    results.insert("nestgate".to_string(), TokenVerificationStatus::Valid);
    results.insert("squirrel".to_string(), TokenVerificationStatus::Expired);

    let verification = VerificationResult {
        total_primals: 3,
        valid_tokens: 2,
        results,
        verification_time: chrono::Utc::now(),
    };

    assert_eq!(verification.total_primals, 3);
    assert_eq!(verification.valid_tokens, 2);
    assert_eq!(verification.results.len(), 3);
}

#[test]
fn test_verification_result_all_valid() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenVerificationStatus::Valid);
    results.insert("nestgate".to_string(), TokenVerificationStatus::Valid);
    results.insert("squirrel".to_string(), TokenVerificationStatus::Valid);

    let verification = VerificationResult {
        total_primals: 3,
        valid_tokens: 3,
        results,
        verification_time: chrono::Utc::now(),
    };

    assert_eq!(verification.total_primals, 3);
    assert_eq!(verification.valid_tokens, 3);
}

// ============================================================================
// AuthenticationManager Tests
// ============================================================================

#[test]
fn test_authentication_manager_creation() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
    };

    let _manager = AuthenticationManager::with_inmemory(config);
    // Creation should succeed
}

#[test]
fn test_authentication_manager_without_validation() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(120),
        replay_protection: false,
    };

    let _manager = AuthenticationManager::with_inmemory(config);
    // Creation should succeed
}

// ============================================================================
// Security Configuration Tests
// ============================================================================

#[test]
fn test_auth_config_short_refresh_interval() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(60),
        signature_validation: true,
        timestamp_window: Duration::from_secs(30),
        replay_protection: true,
    };

    assert_eq!(config.token_refresh_interval, Duration::from_secs(60));
}

#[test]
fn test_auth_config_long_refresh_interval() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(3600),
        signature_validation: true,
        timestamp_window: Duration::from_secs(300),
        replay_protection: true,
    };

    assert_eq!(config.token_refresh_interval, Duration::from_secs(3600));
}

#[test]
fn test_auth_config_wide_timestamp_window() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(300),
        replay_protection: true,
    };

    assert_eq!(config.timestamp_window, Duration::from_secs(300));
}

#[test]
fn test_auth_config_narrow_timestamp_window() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(10),
        replay_protection: true,
    };

    assert_eq!(config.timestamp_window, Duration::from_secs(10));
}

// ============================================================================
// Agent Deployment Integration Tests
// ============================================================================

// ============================================================================
// AgentDeploymentConfig Tests
// ============================================================================

#[test]
fn test_agent_deployment_config_creation() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.squirrel_endpoint, "http://localhost:7070");
    assert_eq!(config.model_registry, "huggingface");
    assert_eq!(config.agent_runtime, "container");
    assert!(config.mcp_enabled);
}

#[test]
fn test_agent_deployment_config_clone() {
    let config1 = AgentDeploymentConfig {
        squirrel_endpoint: "http://squirrel:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };

    let config2 = config1.clone();

    assert_eq!(config1.squirrel_endpoint, config2.squirrel_endpoint);
    assert_eq!(config1.model_registry, config2.model_registry);
}

// ============================================================================
// AgentStatus Tests (7 variants)
// ============================================================================

#[test]
fn test_agent_status_deploying() {
    let status = AgentStatus::Deploying;
    assert_eq!(status, AgentStatus::Deploying);
}

#[test]
fn test_agent_status_running() {
    let status = AgentStatus::Running;
    assert_eq!(status, AgentStatus::Running);
}

#[test]
fn test_agent_status_scaling() {
    let status = AgentStatus::Scaling;
    assert_eq!(status, AgentStatus::Scaling);
}

#[test]
fn test_agent_status_updating() {
    let status = AgentStatus::Updating;
    assert_eq!(status, AgentStatus::Updating);
}

#[test]
fn test_agent_status_terminating() {
    let status = AgentStatus::Terminating;
    assert_eq!(status, AgentStatus::Terminating);
}

#[test]
fn test_agent_status_failed() {
    let status = AgentStatus::Failed("Deployment error".to_string());

    match status {
        AgentStatus::Failed(msg) => {
            assert_eq!(msg, "Deployment error");
        }
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_agent_status_stopped() {
    let status = AgentStatus::Stopped;
    assert_eq!(status, AgentStatus::Stopped);
}

#[test]
fn test_agent_status_clone() {
    let status1 = AgentStatus::Running;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// ModelStatus Tests (5 variants)
// ============================================================================

#[test]
fn test_model_status_loading() {
    let status = ModelStatus::Loading;
    assert_eq!(status, ModelStatus::Loading);
}

#[test]
fn test_model_status_ready() {
    let status = ModelStatus::Ready;
    assert_eq!(status, ModelStatus::Ready);
}

#[test]
fn test_model_status_updating() {
    let status = ModelStatus::Updating;
    assert_eq!(status, ModelStatus::Updating);
}

#[test]
fn test_model_status_unloading() {
    let status = ModelStatus::Unloading;
    assert_eq!(status, ModelStatus::Unloading);
}

#[test]
fn test_model_status_error() {
    let status = ModelStatus::Error("Model not found".to_string());

    match status {
        ModelStatus::Error(msg) => {
            assert_eq!(msg, "Model not found");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_model_status_clone() {
    let status1 = ModelStatus::Ready;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// AgentResourceUsage Tests
// ============================================================================

#[test]
fn test_agent_resource_usage_creation() {
    let usage = AgentResourceUsage {
        cpu_millicores: 500,
        memory_bytes: 1024 * 1024 * 512, // 512 MB
        gpu_percent: Some(25.5),
        network_bytes_per_sec: 1024 * 100, // 100 KB/s
    };

    assert_eq!(usage.cpu_millicores, 500);
    assert_eq!(usage.memory_bytes, 1024 * 1024 * 512);
    assert_eq!(usage.gpu_percent, Some(25.5));
    assert_eq!(usage.network_bytes_per_sec, 1024 * 100);
}

#[test]
fn test_agent_resource_usage_no_gpu() {
    let usage = AgentResourceUsage {
        cpu_millicores: 1000,
        memory_bytes: 1024 * 1024 * 1024, // 1 GB
        gpu_percent: None,
        network_bytes_per_sec: 0,
    };

    assert!(usage.gpu_percent.is_none());
}

#[test]
fn test_agent_resource_usage_clone() {
    let usage1 = AgentResourceUsage {
        cpu_millicores: 750,
        memory_bytes: 1024 * 1024 * 256,
        gpu_percent: Some(50.0),
        network_bytes_per_sec: 1024,
    };

    let usage2 = usage1.clone();

    assert_eq!(usage1.cpu_millicores, usage2.cpu_millicores);
    assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
}

// ============================================================================
// ModelResourceRequirements Tests
// ============================================================================

#[test]
fn test_model_resource_requirements_creation() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 2.0,
        min_memory_gb: 4.0,
        gpu_required: true,
        min_gpu_memory_gb: Some(8.0),
    };

    assert_eq!(requirements.min_cpu_cores, 2.0);
    assert_eq!(requirements.min_memory_gb, 4.0);
    assert!(requirements.gpu_required);
    assert_eq!(requirements.min_gpu_memory_gb, Some(8.0));
}

#[test]
fn test_model_resource_requirements_no_gpu() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 1.0,
        min_memory_gb: 2.0,
        gpu_required: false,
        min_gpu_memory_gb: None,
    };

    assert!(!requirements.gpu_required);
    assert!(requirements.min_gpu_memory_gb.is_none());
}

#[test]
fn test_model_resource_requirements_minimal() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 0.5,
        min_memory_gb: 0.5,
        gpu_required: false,
        min_gpu_memory_gb: None,
    };

    assert_eq!(requirements.min_cpu_cores, 0.5);
    assert_eq!(requirements.min_memory_gb, 0.5);
}

#[test]
fn test_model_resource_requirements_large() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 16.0,
        min_memory_gb: 64.0,
        gpu_required: true,
        min_gpu_memory_gb: Some(24.0),
    };

    assert_eq!(requirements.min_cpu_cores, 16.0);
    assert_eq!(requirements.min_memory_gb, 64.0);
    assert_eq!(requirements.min_gpu_memory_gb, Some(24.0));
}

// ============================================================================
// ModelPerformanceMetrics Tests
// ============================================================================

#[test]
fn test_model_performance_metrics_creation() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 250,
        throughput_rps: 10.5,
        success_rate: 99.8,
    };

    assert_eq!(metrics.avg_inference_time_ms, 250);
    assert_eq!(metrics.throughput_rps, 10.5);
    assert_eq!(metrics.success_rate, 99.8);
}

#[test]
fn test_model_performance_metrics_fast() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 10,
        throughput_rps: 100.0,
        success_rate: 100.0,
    };

    assert_eq!(metrics.avg_inference_time_ms, 10);
    assert_eq!(metrics.throughput_rps, 100.0);
}

#[test]
fn test_model_performance_metrics_slow() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 5000,
        throughput_rps: 0.2,
        success_rate: 95.0,
    };

    assert_eq!(metrics.avg_inference_time_ms, 5000);
    assert_eq!(metrics.throughput_rps, 0.2);
}

// ============================================================================
// AgentDeploymentManager Tests
// ============================================================================

#[test]
fn test_agent_deployment_manager_creation() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    let _manager = AgentDeploymentManager::with_inmemory(config);
    // Creation should succeed
}

#[test]
fn test_agent_deployment_manager_with_local_registry() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };

    let _manager = AgentDeploymentManager::with_inmemory(config);
    // Creation should succeed
}

// ============================================================================
// Model Registry Tests
// ============================================================================

#[test]
fn test_model_registry_huggingface() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.model_registry, "huggingface");
}

#[test]
fn test_model_registry_local() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.model_registry, "local");
}

#[test]
fn test_model_registry_custom() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "custom".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.model_registry, "custom");
}

// ============================================================================
// Agent Runtime Tests
// ============================================================================

#[test]
fn test_agent_runtime_container() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.agent_runtime, "container");
}

#[test]
fn test_agent_runtime_process() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.agent_runtime, "process");
}

#[test]
fn test_agent_runtime_lambda() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "lambda".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.agent_runtime, "lambda");
}

// ============================================================================
// MCP (Model Control Protocol) Tests
// ============================================================================

#[test]
fn test_mcp_enabled() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert!(config.mcp_enabled);
}

#[test]
fn test_mcp_disabled() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };

    assert!(!config.mcp_enabled);
}
