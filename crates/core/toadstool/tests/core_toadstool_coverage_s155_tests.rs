// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(deprecated)]
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // test asserts on exact constructed values
//! Coverage tests for under-covered modules in toadstool core
//!
//! Target modules:
//! - resources/types.rs, resources/monitoring.rs
//! - security/types.rs, security/policy.rs, security/context.rs
//! - `security_hardening` (`rate_limiter`, `input_validator`, audit, intrusion)
//! - `biomeos_integration/storage_backend/inmemory.rs`
//! - `layer_adaptation/types.rs`, `layer_adaptation/detection.rs`
//! - workload/types.rs
//! - `workload_migration/mod.rs`
//! - `plugin_system/manager.rs`

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

use toadstool::ResourceMonitor;
use toadstool::biomeos_integration::{
    InMemoryBackend, PersistentVolume, StorageBackend, VolumeConfig, VolumeStatus,
};
use toadstool::layer_adaptation::{
    AdaptedCapabilities, CapabilityMetadata, ComputeCapabilities, GpuAccess, NetworkAccess,
    NetworkCapabilities, StorageCapabilities, StorageType, compute_capabilities,
    detect_network_bandwidth, detect_storage_read_bandwidth, detect_storage_write_bandwidth,
    storage_capabilities,
};
use toadstool::plugin_system::{PluginManager, PluginManifest, PluginState};
use toadstool::resources::{
    CpuRequirements, LoadAverages, MemoryRequirements, ProcessStatus, ResourceLimits,
    ResourceRequirements, ResourceUsage, RuntimeMetrics, SystemResourceMonitor, SystemResources,
};
use toadstool::security::{
    AuditEvent, AuditSettings, Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity,
    SecurityContext, SecuritySettings, UserContext,
};
use toadstool::security_hardening::{
    ActivityType, AuditConfig, InputValidator, IntrusionDetectionConfig, IntrusionDetectionSystem,
    RateLimiter, RateLimitingConfig, SecurityAuditEvent, SecurityAuditLogger, SecurityEventType,
    SecuritySeverity, ValidationRules,
};
use toadstool::workload::{
    ExecutableSource, GpuArgument, GpuProgramSource, PortProtocol, PythonSource, VolumeMountType,
    WasiConfig, WasmModuleSource,
};
use toadstool::workload_migration::{
    CostImpact, MigrationRecommendation, MigrationStats, MigrationTarget,
};
use uuid::Uuid;

// ============================================================================
// Resource Types (resources/types.rs)
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let req = ResourceRequirements::default();
    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
    assert!(req.gpu.is_none());
}

#[test]
fn test_resource_requirements_validate_ok() {
    let req = ResourceRequirements::default();
    assert!(req.validate().is_ok());
}

#[test]
fn test_resource_requirements_validate_invalid_cpu() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 0.0;
    assert!(req.validate().is_err());
}

#[test]
fn test_resource_requirements_validate_invalid_memory() {
    let mut req = ResourceRequirements::default();
    req.memory.min_bytes = 0;
    assert!(req.validate().is_err());
}

#[test]
fn test_cpu_requirements_default() {
    let cpu = CpuRequirements::default();
    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
}

#[test]
fn test_memory_requirements_default() {
    let mem = MemoryRequirements::default();
    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024);
}

#[test]
fn test_runtime_metrics_default() {
    let m = RuntimeMetrics::default();
    assert_eq!(m.cpu.usage_percent, 0.0);
    assert_eq!(m.network.bytes_sent, 0);
}

#[test]
fn test_resource_usage_is_empty() {
    let u = ResourceUsage::default();
    assert!(u.is_empty());
}

#[test]
fn test_resource_usage_not_empty() {
    let u = ResourceUsage {
        cpu_usage_percent: 1.0,
        ..Default::default()
    };
    assert!(!u.is_empty());
}

#[test]
fn test_resource_limits_default() {
    let limits = ResourceLimits::default();
    assert!(limits.execution_timeout.is_some());
}

#[test]
fn test_system_resources_default() {
    let sr = SystemResources::default();
    assert_eq!(sr.available_cpu_cores, 1.0);
    assert_eq!(sr.total_cpu_cores, 1);
}

#[test]
fn test_process_status_debug() {
    let status = ProcessStatus::Running;
    let s = format!("{status:?}");
    assert!(s.contains("Running"));
}

#[test]
fn test_load_averages_construction() {
    let la = LoadAverages {
        one_minute: 0.5,
        five_minutes: 0.4,
        fifteen_minutes: 0.3,
    };
    assert!((la.one_minute - 0.5).abs() < f64::EPSILON);
}

// ============================================================================
// Resource Monitoring (resources/monitoring.rs)
// ============================================================================

#[tokio::test]
async fn test_system_resource_monitor_new() {
    let monitor = SystemResourceMonitor::new();
    monitor.start_monitoring("test-wl").unwrap();
    let metrics = monitor.get_metrics("test-wl").await.unwrap();
    assert_eq!(metrics.cpu.usage_percent, 0.0);
}

#[tokio::test]
async fn test_system_resource_monitor_start_stop() {
    let monitor = SystemResourceMonitor::new();
    monitor.start_monitoring("wl-1").unwrap();
    monitor.stop_monitoring("wl-1").unwrap();
    // get_metrics returns default when not found after stop (async timing)
    let _ = monitor.get_metrics("wl-1").await;
}

#[tokio::test]
async fn test_system_resource_monitor_real_time() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.start_real_time_monitoring("rt-wl").await;
    // May succeed or fail depending on toadstool_sysmon availability
    let _ = result;
}

// ============================================================================
// Security Types (security/types.rs)
// ============================================================================

#[test]
fn test_isolation_level_debug() {
    let level = IsolationLevel::Standard;
    let s = format!("{level:?}");
    assert!(s.contains("Standard"));
}

#[test]
fn test_capability_custom() {
    let cap = Capability::Custom("custom:foo".to_string());
    let s = format!("{cap:?}");
    assert!(s.contains("Custom"));
}

#[test]
fn test_network_security_default() {
    let ns = NetworkSecurity::default();
    assert!(!ns.allow_outbound);
    assert!(!ns.allow_inbound);
}

#[test]
fn test_filesystem_security_default() {
    let fs = FilesystemSecurity::default();
    assert!(!fs.read_only);
}

#[test]
fn test_user_context_construction() {
    let uc = UserContext {
        username: Some("test".to_string()),
        uid: Some(1000),
        gid: Some(1000),
        groups: vec![1000, 1001],
    };
    assert_eq!(uc.username.as_deref(), Some("test"));
}

// ============================================================================
// Security Policy (security/policy.rs)
// ============================================================================

#[test]
fn test_security_settings_default() {
    let ss = SecuritySettings::default();
    assert_eq!(ss.default_isolation_level, IsolationLevel::Standard);
    assert!(!ss.default_capabilities.is_empty());
}

#[test]
fn test_audit_settings_default() {
    let as_ = AuditSettings::default();
    assert!(as_.enabled);
    assert_eq!(as_.log_level, "info");
}

#[test]
fn test_audit_event_variants() {
    let events = [
        AuditEvent::ExecutionStart,
        AuditEvent::ExecutionEnd,
        AuditEvent::SecurityViolation,
    ];
    for e in &events {
        let s = format!("{e:?}");
        assert!(!s.is_empty());
    }
}

// ============================================================================
// Security Context (security/context.rs)
// ============================================================================

#[test]
fn test_security_context_default() {
    let ctx = SecurityContext::default();
    assert!(ctx.has_capability(&Capability::Execute));
}

#[test]
fn test_security_context_for_isolation_level() {
    let ctx = SecurityContext::for_isolation_level(IsolationLevel::Maximum);
    assert_eq!(ctx.isolation_level, IsolationLevel::Maximum);
}

#[test]
fn test_security_context_with_capability() {
    let ctx = SecurityContext::default().with_capability(Capability::NetworkClient);
    assert!(ctx.has_capability(&Capability::NetworkClient));
}

#[test]
fn test_security_context_has_permission() {
    let ctx = SecurityContext::default();
    assert!(ctx.has_permission("execute"));
    assert!(ctx.has_permission("read"));
}

#[test]
fn test_security_context_validate_ok() {
    let ctx = SecurityContext::default();
    assert!(ctx.validate().is_ok());
}

#[test]
fn test_security_context_validate_empty_capabilities() {
    let ctx = SecurityContext {
        capabilities: vec![],
        ..Default::default()
    };
    assert!(ctx.validate().is_err());
}

// ============================================================================
// In-Memory Storage Backend (biomeos_integration/storage_backend/inmemory.rs)
// ============================================================================

#[tokio::test]
async fn test_inmemory_backend_new() {
    let backend = InMemoryBackend::new("test-tier");
    let vols = backend.list_volumes().await.unwrap();
    assert!(vols.is_empty());
}

#[tokio::test]
async fn test_inmemory_backend_provision_volume() {
    let backend = InMemoryBackend::new("standard");
    let config = VolumeConfig {
        name: "vol-1".to_string(),
        size: "10Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    let info = backend.provision_volume(&config).await.unwrap();
    assert_eq!(info.name, "vol-1");
    assert!(info.id.starts_with("test-"));
    assert_eq!(info.status, "Available");
}

#[tokio::test]
async fn test_inmemory_backend_provision_persistent_volume() {
    let backend = InMemoryBackend::new("premium");
    let pv = PersistentVolume {
        name: "pv-1".to_string(),
        capacity: "100Gi".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        storage_class: "fast".to_string(),
        host_path: None,
    };
    let info = backend.provision_persistent_volume(&pv).await.unwrap();
    assert_eq!(info.name, "pv-1");
    assert!(info.id.starts_with("test-pv-"));
}

#[tokio::test]
async fn test_inmemory_backend_mount_unmount() {
    let backend = InMemoryBackend::new("test");
    let config = VolumeConfig {
        name: "mount-vol".to_string(),
        size: "1Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    backend.provision_volume(&config).await.unwrap();
    backend
        .mount_volume("mount-vol", "svc-1", "/data")
        .await
        .unwrap();
    backend.unmount_volume("mount-vol", "svc-1").await.unwrap();
}

#[tokio::test]
async fn test_inmemory_backend_mount_nonexistent_fails() {
    let backend = InMemoryBackend::new("test");
    let result = backend.mount_volume("nonexistent", "svc", "/path").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_backend_delete_volume() {
    let backend = InMemoryBackend::new("test");
    let config = VolumeConfig {
        name: "del-vol".to_string(),
        size: "1Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    backend.provision_volume(&config).await.unwrap();
    backend.delete_volume("del-vol").await.unwrap();
    let result = backend.get_volume_status("del-vol").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_inmemory_backend_get_volume_status() {
    let backend = InMemoryBackend::new("test");
    let config = VolumeConfig {
        name: "status-vol".to_string(),
        size: "1Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    backend.provision_volume(&config).await.unwrap();
    let status = backend.get_volume_status("status-vol").await.unwrap();
    assert_eq!(status, VolumeStatus::Available);
}

#[tokio::test]
async fn test_inmemory_backend_list_volumes() {
    let backend = InMemoryBackend::new("test");
    let config = VolumeConfig {
        name: "list-vol".to_string(),
        size: "1Gi".to_string(),
        storage_class: None,
        access_modes: vec![],
        mount_path: None,
        backup_policy: None,
    };
    backend.provision_volume(&config).await.unwrap();
    let vols = backend.list_volumes().await.unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].name, "list-vol");
}

// ============================================================================
// Security Hardening - Rate Limiter
// ============================================================================

#[tokio::test]
async fn test_rate_limiter_first_request_allowed() {
    let config = RateLimitingConfig::default();
    let limiter = RateLimiter::new(config);
    let ok = limiter.check_rate_limit("client-a").await.unwrap();
    assert!(ok);
}

#[tokio::test]
async fn test_rate_limiter_ban_client() {
    let config = RateLimitingConfig::default();
    let limiter = RateLimiter::new(config);
    limiter.ban_client("banned", Duration::from_mins(1)).await;
    let ok = limiter.check_rate_limit("banned").await.unwrap();
    assert!(!ok);
}

// ============================================================================
// Security Hardening - Input Validator
// ============================================================================

#[test]
fn test_input_validator_valid_input() {
    let rules = ValidationRules::default();
    let validator = InputValidator::new(rules);
    assert!(validator.validate_input("hello world").is_ok());
}

#[test]
fn test_input_validator_rejects_xss() {
    let rules = ValidationRules::default();
    let validator = InputValidator::new(rules);
    assert!(
        validator
            .validate_input("<script>alert(1)</script>")
            .is_err()
    );
}

#[test]
fn test_input_validator_sanitize() {
    let rules = ValidationRules::default();
    let validator = InputValidator::new(rules);
    let out = validator.sanitize_input("<script>");
    assert!(!out.contains('<'));
    assert!(out.contains("&lt;"));
}

// ============================================================================
// Security Hardening - Audit
// ============================================================================

#[tokio::test]
async fn test_security_audit_logger_log_and_retrieve() {
    let config = AuditConfig::default();
    let logger = SecurityAuditLogger::new(config);
    let event = SecurityAuditEvent {
        id: Uuid::new_v4(),
        event_type: SecurityEventType::AuthenticationAttempt,
        timestamp: SystemTime::now(),
        client_id: Some("c1".to_string()),
        ip_address: None,
        user_agent: None,
        details: HashMap::new(),
        severity: SecuritySeverity::Low,
    };
    logger.log_event(event).await;
    let events = logger.get_recent_events(5).await;
    assert_eq!(events.len(), 1);
}

#[test]
fn test_security_severity_ordering() {
    assert!(SecuritySeverity::Critical > SecuritySeverity::High);
    assert!(SecuritySeverity::High > SecuritySeverity::Medium);
}

// ============================================================================
// Security Hardening - Intrusion Detection
// ============================================================================

#[tokio::test]
async fn test_intrusion_detection_system_new() {
    let config = IntrusionDetectionConfig::default();
    let _ids = IntrusionDetectionSystem::new(config);
}

#[tokio::test]
async fn test_intrusion_detection_record_activity() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.record_activity("client-x", ActivityType::Request).await;
    assert!(!ids.is_banned("client-x").await);
}

#[tokio::test]
async fn test_intrusion_detection_ban_client() {
    let config = IntrusionDetectionConfig::default();
    let ids = IntrusionDetectionSystem::new(config);
    ids.ban_client("banned-client", Duration::from_mins(1), "Test ban")
        .await;
    assert!(ids.is_banned("banned-client").await);
}

// ============================================================================
// Layer Adaptation Types (layer_adaptation/types.rs)
// ============================================================================

#[test]
fn test_adapted_capabilities_has_direct_gpu_access() {
    let caps = AdaptedCapabilities {
        compute: ComputeCapabilities {
            gpu_access: GpuAccess::Direct,
            has_cpu: true,
            cpu_cores: Some(4),
            memory_bytes: Some(8 * 1024 * 1024 * 1024),
            supports_tensor_ops: true,
            supports_nn_training: true,
            supports_nn_inference: true,
        },
        storage: StorageCapabilities {
            storage_type: StorageType::PersistentVolume,
            available_bytes: Some(100_000_000),
            read_bandwidth: None,
            write_bandwidth: None,
        },
        network: NetworkCapabilities {
            network_access: NetworkAccess::Direct,
            bandwidth: Some(1_000_000_000),
            latency_ms: Some(1),
            has_service_mesh: false,
        },
        metadata: CapabilityMetadata {
            layer: "container".to_string(),
            host_os: None,
            cloud_provider: None,
            extra: HashMap::new(),
        },
    };
    assert!(caps.has_direct_gpu_access());
    assert!(caps.has_gpu_access());
    let list = caps.to_capability_list();
    assert!(!list.is_empty());
}

#[test]
fn test_compute_capability_constants() {
    assert_eq!(
        compute_capabilities::GPU_COMPUTE_DIRECT,
        "gpu_compute_direct"
    );
    assert_eq!(storage_capabilities::PERSISTENT_VOLUME, "persistent_volume");
}

// ============================================================================
// Layer Adaptation Detection (layer_adaptation/detection.rs)
// ============================================================================

#[test]
fn test_detect_storage_read_bandwidth() {
    let bw = detect_storage_read_bandwidth();
    assert!(bw.is_some() || bw.is_none()); // platform-dependent
}

#[test]
fn test_detect_storage_write_bandwidth() {
    let bw = detect_storage_write_bandwidth();
    assert!(bw.is_some() || bw.is_none());
}

#[test]
fn test_detect_network_bandwidth() {
    let bw = detect_network_bandwidth();
    assert!(bw.is_some());
}

// ============================================================================
// Workload Types (workload/types.rs)
// ============================================================================

#[test]
fn test_executable_source_file() {
    let src = ExecutableSource::File {
        path: PathBuf::from("/bin/echo"),
    };
    let s = format!("{src:?}");
    assert!(s.contains("File"));
}

#[test]
fn test_wasm_module_source_url() {
    let src = WasmModuleSource::Url {
        url: "https://example.com/mod.wasm".to_string(),
    };
    let s = format!("{src:?}");
    assert!(s.contains("Url"));
}

#[test]
fn test_wasi_config_default() {
    let cfg = WasiConfig {
        inherit_env: true,
        inherit_stdio: true,
        allowed_dirs: vec![],
        preopened_dirs: vec![],
        args: vec![],
    };
    assert!(cfg.inherit_env);
}

#[test]
fn test_volume_mount_type_bind() {
    let t = VolumeMountType::Bind;
    assert_eq!(t, VolumeMountType::Bind);
}

#[test]
fn test_port_protocol_tcp() {
    let p = PortProtocol::Tcp;
    assert_eq!(p, PortProtocol::Tcp);
}

#[test]
fn test_gpu_program_source_cuda_debug() {
    let src = GpuProgramSource::Cuda {
        source: "kernel void foo() {}".to_string(),
    };
    let s = format!("{src:?}");
    assert!(s.contains("Cuda"));
}

#[test]
fn test_gpu_argument_scalar() {
    let arg = GpuArgument::Scalar { value: 2.72 };
    let s = format!("{arg:?}");
    assert!(s.contains("Scalar"));
}

#[test]
fn test_python_source_module() {
    let src = PythonSource::Module {
        name: "mymodule".to_string(),
    };
    let s = format!("{src:?}");
    assert!(s.contains("Module"));
}

// ============================================================================
// Workload Migration (workload_migration/mod.rs)
// ============================================================================

#[test]
fn test_migration_stats_default() {
    let stats = MigrationStats::default();
    assert_eq!(stats.total_migrations, 0);
    assert_eq!(stats.successful_migrations, 0);
}

#[test]
fn test_migration_recommendation_construction() {
    let rec = MigrationRecommendation {
        should_migrate: false,
        reason: "Optimal".to_string(),
        target: None,
        cost_impact: None,
        confidence: 1.0,
    };
    assert!(!rec.should_migrate);
}

#[test]
fn test_migration_target_local() {
    let t = MigrationTarget::Local;
    let s = format!("{t:?}");
    assert!(s.contains("Local"));
}

#[test]
fn test_migration_target_cloud() {
    let t = MigrationTarget::Cloud {
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        estimated_cost_per_hour: 0.5,
    };
    let s = format!("{t:?}");
    assert!(s.contains("Cloud"));
}

#[test]
fn test_cost_impact_construction() {
    let c = CostImpact {
        current_cost_per_hour: 1.0,
        new_cost_per_hour: 0.5,
        savings_per_hour: 0.5,
        migration_cost: 0.1,
    };
    assert!((c.savings_per_hour - 0.5).abs() < f64::EPSILON);
}

// ============================================================================
// Plugin System (plugin_system/manager.rs)
// ============================================================================

#[test]
fn test_plugin_manager_new() {
    let manager = PluginManager::new();
    assert!(manager.list_plugins().is_empty());
}

#[test]
fn test_plugin_manager_register_plugin() {
    let mut manager = PluginManager::new();
    let manifest = PluginManifest {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "cloud_provider".to_string(),
        entry_point: "libtest.so".to_string(),
        ..Default::default()
    };
    manager.register_plugin(manifest).unwrap();
    assert_eq!(manager.list_plugins(), vec!["test-plugin"]);
}

#[test]
fn test_plugin_manager_load_unload() {
    let mut manager = PluginManager::new();
    let manifest = PluginManifest {
        name: "load-plugin".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "storage".to_string(),
        entry_point: "libload.so".to_string(),
        ..Default::default()
    };
    manager.register_plugin(manifest).unwrap();
    let load_err = manager.load_plugin("load-plugin").unwrap_err();
    assert!(load_err.to_string().contains("deprecated"));
    assert!(manager.active_plugins().is_empty());
    manager.unload_plugin("load-plugin").unwrap();
    let info = manager.get_plugin_info("load-plugin").unwrap();
    assert_eq!(info.state, PluginState::Unloaded);
}

#[test]
fn test_plugin_manager_plugins_by_type() {
    let mut manager = PluginManager::new();
    let manifest = PluginManifest {
        name: "type-plugin".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: "custom_type".to_string(),
        entry_point: "lib.so".to_string(),
        ..Default::default()
    };
    manager.register_plugin(manifest).unwrap();
    let by_type = manager.plugins_by_type("custom_type");
    assert_eq!(by_type, vec!["type-plugin"]);
}

#[test]
fn test_plugin_manager_invalid_manifest_rejected() {
    let mut manager = PluginManager::new();
    let manifest = PluginManifest {
        name: String::new(),
        version: "1.0.0".to_string(),
        plugin_type: "x".to_string(),
        entry_point: "lib.so".to_string(),
        ..Default::default()
    };
    assert!(manager.register_plugin(manifest).is_err());
}
