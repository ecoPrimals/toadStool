//! BiomeOS Integration Types Functional Tests - Week 19 Sprint 13
//!
//! Focus: Type construction, serialization, defaults, enums
//! Target: 0% → 50%+ coverage
//! Tests: ~25 focused functional tests

use std::collections::HashMap;
use std::time::Duration;
use toadstool::biomeos_integration::types::*;

// ============================================================================
// BiomeManifest Tests (5 tests)
// ============================================================================

#[test]
fn test_biome_manifest_default() {
    let manifest = BiomeManifest::default();
    assert_eq!(manifest.api_version, "biomeOS/v1");
    assert_eq!(manifest.kind, "Biome");
    assert_eq!(manifest.metadata.name, "default-biome");
    assert!(manifest.primals.toadstool.is_some());
}

#[test]
fn test_biome_manifest_clone() {
    let manifest = BiomeManifest::default();
    let cloned = manifest.clone();
    assert_eq!(manifest.metadata.name, cloned.metadata.name);
    assert_eq!(manifest.api_version, cloned.api_version);
}

#[test]
fn test_biome_manifest_serialization() {
    let manifest = BiomeManifest::default();
    let json = serde_json::to_string(&manifest);
    assert!(json.is_ok());
}

#[test]
fn test_biome_manifest_deserialization() {
    let manifest = BiomeManifest::default();
    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: Result<BiomeManifest, _> = serde_json::from_str(&json);
    assert!(deserialized.is_ok());
}

#[test]
fn test_biome_manifest_with_custom_metadata() {
    let mut manifest = BiomeManifest::default();
    manifest.metadata.name = "custom-biome".to_string();
    manifest.metadata.environment = Some("production".to_string());
    assert_eq!(manifest.metadata.name, "custom-biome");
    assert_eq!(
        manifest.metadata.environment,
        Some("production".to_string())
    );
}

// ============================================================================
// VolumeConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_volume_config_creation() {
    let config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/mnt/data".to_string()),
        backup_policy: Some("daily".to_string()),
    };
    assert_eq!(config.name, "test-volume");
    assert_eq!(config.size, "100Gi");
}

#[test]
fn test_volume_config_clone() {
    let config = VolumeConfig {
        name: "vol1".to_string(),
        size: "50Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    let cloned = config.clone();
    assert_eq!(config.name, cloned.name);
}

#[test]
fn test_volume_config_serialization() {
    let config = VolumeConfig {
        name: "serialize-test".to_string(),
        size: "200Gi".to_string(),
        storage_class: Some("standard".to_string()),
        access_modes: vec!["ReadWriteMany".to_string()],
        mount_path: Some("/mnt/shared".to_string()),
        backup_policy: None,
    };
    let json = serde_json::to_string(&config);
    assert!(json.is_ok());
}

#[test]
fn test_volume_config_with_multiple_access_modes() {
    let config = VolumeConfig {
        name: "multi-access".to_string(),
        size: "1TB".to_string(),
        storage_class: Some("nfs".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string(), "ReadWriteMany".to_string()],
        mount_path: Some("/mnt/multi".to_string()),
        backup_policy: Some("hourly".to_string()),
    };
    assert_eq!(config.access_modes.len(), 2);
}

#[test]
fn test_volume_config_minimal() {
    let config = VolumeConfig {
        name: "minimal".to_string(),
        size: "10Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    assert!(config.storage_class.is_none());
    assert!(config.mount_path.is_none());
}

// ============================================================================
// PersistentVolume Tests (3 tests)
// ============================================================================

#[test]
fn test_persistent_volume_creation() {
    let pv = PersistentVolume {
        name: "pv-1".to_string(),
        capacity: "500Gi".to_string(),
        access_modes: vec!["ReadWriteOnce".to_string()],
        storage_class: "premium".to_string(),
        host_path: None,
    };
    assert_eq!(pv.name, "pv-1");
    assert_eq!(pv.capacity, "500Gi");
}

#[test]
fn test_persistent_volume_with_host_path() {
    let pv = PersistentVolume {
        name: "local-pv".to_string(),
        capacity: "1TB".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        storage_class: "local".to_string(),
        host_path: Some(std::path::PathBuf::from("/mnt/local-storage")),
    };
    assert!(pv.host_path.is_some());
}

#[test]
fn test_persistent_volume_serialization() {
    let pv = PersistentVolume {
        name: "serialize-pv".to_string(),
        capacity: "100Gi".to_string(),
        access_modes: vec!["ReadWriteOnce".to_string()],
        storage_class: "fast".to_string(),
        host_path: None,
    };
    let json = serde_json::to_string(&pv);
    assert!(json.is_ok());
}

// ============================================================================
// VolumeInfo Tests (4 tests)
// ============================================================================

#[test]
fn test_volume_info_creation() {
    let info = VolumeInfo {
        name: "vol-info".to_string(),
        id: "vol-12345".to_string(),
        size: "100Gi".to_string(),
        storage_class: "standard".to_string(),
        status: "Available".to_string(),
        created_at: chrono::Utc::now(),
    };
    assert_eq!(info.name, "vol-info");
    assert_eq!(info.id, "vol-12345");
}

#[test]
fn test_volume_info_equality() {
    let now = chrono::Utc::now();
    let info1 = VolumeInfo {
        name: "vol-1".to_string(),
        id: "id-1".to_string(),
        size: "50Gi".to_string(),
        storage_class: "fast".to_string(),
        status: "Active".to_string(),
        created_at: now,
    };
    let info2 = info1.clone();
    assert_eq!(info1, info2);
}

#[test]
fn test_volume_info_clone() {
    let info = VolumeInfo {
        name: "clone-test".to_string(),
        id: "id-clone".to_string(),
        size: "200Gi".to_string(),
        storage_class: "premium".to_string(),
        status: "Provisioning".to_string(),
        created_at: chrono::Utc::now(),
    };
    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
    assert_eq!(info.id, cloned.id);
}

#[test]
fn test_volume_info_serialization() {
    let info = VolumeInfo {
        name: "serialize".to_string(),
        id: "serialize-id".to_string(),
        size: "1TB".to_string(),
        storage_class: "archive".to_string(),
        status: "Available".to_string(),
        created_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&info);
    assert!(json.is_ok());
}

// ============================================================================
// Enum Tests (5 tests)
// ============================================================================

#[test]
fn test_mount_status_variants() {
    let mounting = MountStatus::Mounting;
    let mounted = MountStatus::Mounted;
    let unmounting = MountStatus::Unmounting;
    let failed = MountStatus::Failed("error".to_string());

    assert!(matches!(mounting, MountStatus::Mounting));
    assert!(matches!(mounted, MountStatus::Mounted));
    assert!(matches!(unmounting, MountStatus::Unmounting));
    assert!(matches!(failed, MountStatus::Failed(_)));
}

#[test]
fn test_mount_status_clone() {
    let status = MountStatus::Mounted;
    let cloned = status.clone();
    assert_eq!(status, cloned);
}

#[test]
fn test_primal_orchestration_status_variants() {
    let not_started = PrimalOrchestrationStatus::NotStarted;
    let _configuring = PrimalOrchestrationStatus::Configuring;
    let running = PrimalOrchestrationStatus::Running;
    let _failed = PrimalOrchestrationStatus::Failed("test error".to_string());
    let _stopped = PrimalOrchestrationStatus::Stopped;

    assert_eq!(not_started, PrimalOrchestrationStatus::NotStarted);
    assert_eq!(running, PrimalOrchestrationStatus::Running);
}

#[test]
fn test_volume_provisioning_status_serialization() {
    let info = VolumeInfo {
        name: "test".to_string(),
        id: "id".to_string(),
        size: "100Gi".to_string(),
        storage_class: "standard".to_string(),
        status: "Available".to_string(),
        created_at: chrono::Utc::now(),
    };
    let status = VolumeProvisioningStatus::Success(info);
    let json = serde_json::to_string(&status);
    assert!(json.is_ok());
}

#[test]
fn test_volume_cleanup_status_variants() {
    let success = VolumeCleanupStatus::Success;
    let failed = VolumeCleanupStatus::Failed("cleanup error".to_string());
    let skipped = VolumeCleanupStatus::Skipped("already clean".to_string());

    assert_eq!(success, VolumeCleanupStatus::Success);
    assert!(matches!(failed, VolumeCleanupStatus::Failed(_)));
    assert!(matches!(skipped, VolumeCleanupStatus::Skipped(_)));
}

// ============================================================================
// Configuration Type Tests (3 tests)
// ============================================================================

#[test]
fn test_primal_resources_creation() {
    let resources = PrimalResources {
        cpu_cores: Some(8.0),
        memory_gb: Some(16.0),
        storage_gb: Some(500.0),
        gpu: None,
        network_bandwidth: Some("10Gbps".to_string()),
    };
    assert_eq!(resources.cpu_cores, Some(8.0));
    assert_eq!(resources.memory_gb, Some(16.0));
}

#[test]
fn test_gpu_allocation() {
    let gpu = GpuAllocation {
        count: 4,
        gpu_type: Some("NVIDIA-A100".to_string()),
        memory_gb: Some(80.0),
    };
    assert_eq!(gpu.count, 4);
    assert_eq!(gpu.memory_gb, Some(80.0));
}

#[test]
fn test_health_check_config() {
    let health = BiomeHealthCheckConfig {
        interval: Duration::from_secs(30),
        timeout: Duration::from_secs(10),
        retries: 3,
        initial_delay: Duration::from_secs(15),
    };
    assert_eq!(health.retries, 3);
}

// ============================================================================
// Volume Mount Tests (3 tests)
// ============================================================================

#[test]
fn test_volume_mount_spec_creation() {
    let spec = VolumeMountSpec {
        volume_name: "data-volume".to_string(),
        mount_path: "/app/data".to_string(),
        read_only: false,
    };
    assert_eq!(spec.volume_name, "data-volume");
    assert!(!spec.read_only);
}

#[test]
fn test_volume_mount_spec_equality() {
    let spec1 = VolumeMountSpec {
        volume_name: "vol".to_string(),
        mount_path: "/mnt".to_string(),
        read_only: true,
    };
    let spec2 = spec1.clone();
    assert_eq!(spec1, spec2);
}

#[test]
fn test_volume_mount_info_creation() {
    let spec = VolumeMountSpec {
        volume_name: "vol".to_string(),
        mount_path: "/mnt/vol".to_string(),
        read_only: false,
    };
    let info = VolumeMountInfo {
        spec,
        mount_id: "mount-123".to_string(),
        status: MountStatus::Mounted,
        mounted_at: chrono::Utc::now(),
    };
    assert_eq!(info.mount_id, "mount-123");
    assert_eq!(info.status, MountStatus::Mounted);
}

// ============================================================================
// Result Types Tests (2 tests)
// ============================================================================

#[test]
fn test_storage_provisioning_result() {
    let result = StorageProvisioningResult {
        total_volumes: 5,
        provisioned_volumes: 4,
        results: HashMap::new(),
        provisioning_time: chrono::Utc::now(),
    };
    assert_eq!(result.total_volumes, 5);
    assert_eq!(result.provisioned_volumes, 4);
}

#[test]
fn test_volume_mount_result() {
    let result = VolumeMountResult {
        total_mounts: 3,
        successful_mounts: 2,
        results: HashMap::new(),
        mount_time: chrono::Utc::now(),
    };
    assert_eq!(result.total_mounts, 3);
    assert_eq!(result.successful_mounts, 2);
}

// ============================================================================
// Coverage Summary
// ============================================================================

#[test]
fn test_sprint13_coverage_summary() {
    println!("\n=== Week 19 Sprint 13: BiomeOS Types Functional Tests ===");
    println!("BiomeManifest:          5 tests");
    println!("VolumeConfig:           5 tests");
    println!("PersistentVolume:       3 tests");
    println!("VolumeInfo:             4 tests");
    println!("Enum Variants:          5 tests");
    println!("Configuration Types:    3 tests");
    println!("Volume Mounts:          3 tests");
    println!("Result Types:           2 tests");
    println!("──────────────────────────────────────────────────────────");
    println!("Total:                  30 functional tests");
    println!("Target:                 0% → 50%+ coverage");
    println!("Focus:                  Type construction, serialization, enums");
    println!("============================================================\n");
}
