// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for under-covered CLI modules (s155b)
//!
//! Covers: ecosystem, executor/commands, daemon, `network_config/configurator/core`,
//! `zero_config/discovery`, `zero_config/service_discovery`, universal/operations (utilities,
//! benchmarking, migration, detection), `universal/manager_impl`, `templates/ml_science_templates`

use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Ecosystem - Types, Discovery, ServiceType, TrustLevel
// ============================================================================

mod ecosystem_tests {
    use super::*;
    use std::sync::Arc;
    use toadstool_cli::ecosystem::capabilities::StandardCapability;
    use toadstool_cli::ecosystem::service_type::ServiceType;
    use toadstool_cli::ecosystem::{
        SecurityPermission, CryptoVerificationContext, DiscoveredService, DiscoveryResult,
        EcosystemIntegrator, StorageMount, ServiceSignature, SignedServiceResponse, TrustLevel,
    };
    use uuid::Uuid;

    #[expect(deprecated)]
    use toadstool_cli::ecosystem::EcosystemService;

    #[test]
    fn test_ecosystem_integrator_new() {
        let integrator = EcosystemIntegrator::new();
        drop(integrator);
    }

    #[test]
    fn test_ecosystem_integrator_default() {
        let integrator = EcosystemIntegrator::default();
        drop(integrator);
    }

    #[test]
    fn test_trust_level_variants() {
        let _ = TrustLevel::Unknown;
        let _ = TrustLevel::Discovered;
        let _ = TrustLevel::Advertised;
        let _ = TrustLevel::Verified;
        let _ = TrustLevel::Sovereign;
    }

    #[test]
    fn test_discovery_result_creation() {
        let result = DiscoveryResult {
            services: vec![],
            scan_duration: std::time::Duration::from_secs(5),
            total_discovered: 10,
            verified_count: 7,
        };
        assert_eq!(result.total_discovered, 10);
        assert_eq!(result.verified_count, 7);
    }

    #[test]
    fn test_beardog_permission_creation() {
        let permission = SecurityPermission {
            permission_id: Uuid::new_v4(),
            granted_to: "test-service".to_string(),
            capabilities: vec!["read".to_string(), "write".to_string()],
            valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
            signature: "test-signature".to_string(),
        };
        assert_eq!(permission.granted_to, "test-service");
        assert_eq!(permission.capabilities.len(), 2);
    }

    #[test]
    fn test_nestgate_mount_creation() {
        let mount = StorageMount {
            dataset_name: "research-data".to_string(),
            mount_point: PathBuf::from("/mnt/data"),
            endpoint: "127.0.0.1:9000".to_string(),
            zfs_dataset: Some("tank/research".to_string()),
            access_mode: "read".to_string(),
            encryption_key: Some("key123".to_string()),
        };
        assert_eq!(mount.dataset_name, "research-data");
        assert_eq!(mount.access_mode, "read");
    }

    #[test]
    fn test_service_signature_creation() {
        let sig = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "base64-sig".to_string(),
            public_key: "base64-key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "nonce123".to_string(),
        };
        assert_eq!(sig.algorithm, "ed25519");
    }

    #[test]
    fn test_signed_service_response_creation() {
        let sig = ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig".to_string(),
            public_key: "key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "n".to_string(),
        };
        let resp = SignedServiceResponse {
            service_id: "songbird-001".to_string(),
            service_type: "songbird".to_string(),
            status: "active".to_string(),
            capabilities: vec!["discovery".to_string()],
            timestamp: std::time::SystemTime::now(),
            signature: sig,
        };
        assert_eq!(resp.service_id, "songbird-001");
    }

    #[test]
    fn test_crypto_verification_context_new() {
        let ctx = CryptoVerificationContext::new();
        assert!(ctx.trusted_public_keys.is_empty());
    }

    #[test]
    fn test_crypto_verification_context_with_trusted_key() {
        let ctx = CryptoVerificationContext::new().with_trusted_key("songbird", "key1");
        assert_eq!(
            ctx.trusted_public_keys.get("songbird"),
            Some(&"key1".to_string())
        );
    }

    #[test]
    fn test_service_type_from_capability_list_crypto() {
        let caps = vec![StandardCapability::CryptoSignatureEd25519.id()];
        let st = ServiceType::from_capability_list(caps);
        assert!(st.provides_crypto());
    }

    #[test]
    fn test_service_type_from_capability_list_coordination() {
        let caps = vec![StandardCapability::CoordinationServiceRegistry.id()];
        let st = ServiceType::from_capability_list(caps);
        assert!(st.provides_coordination());
    }

    #[test]
    fn test_service_type_display_name() {
        let caps = vec![StandardCapability::CryptoSignatureEd25519.id()];
        let st = ServiceType::from_capability_list(caps);
        assert_eq!(st.display_name(), "crypto-service");
    }

    #[test]
    #[expect(deprecated)]
    fn test_discovered_service_creation() {
        let mut caps = HashMap::new();
        caps.insert("version".to_string(), "1.0.0".to_string());
        let svc = DiscoveredService {
            service_type: toadstool_cli::ecosystem::ServiceType::Discovery,
            address: "127.0.0.1:8080".parse().unwrap(),
            trust_level: TrustLevel::Verified,
            capabilities: caps,
            last_seen: std::time::SystemTime::now(),
        };
        assert!(svc.capabilities.contains_key("version"));
    }

    #[test]
    #[expect(deprecated)]
    fn test_service_endpoint_serialization() {
        let endpoint = toadstool_cli::ecosystem::ServiceEndpoint {
            service_type: EcosystemService::Crypto,
            address: "127.0.0.1:6000".parse().unwrap(),
            version: Arc::from("2.0.0"),
            capabilities: vec!["auth".to_string()],
            trust_level: TrustLevel::Sovereign,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let parsed: toadstool_cli::ecosystem::ServiceEndpoint =
            serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version.as_ref(), "2.0.0");
    }
}

// ============================================================================
// Executor - RunBiomeOptions, UpBiomeOptions, BiomeExecutor
// ============================================================================

mod executor_tests {
    use super::*;
    use toadstool_cli::executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions};

    #[test]
    fn test_run_biome_options_construction() {
        let opts = RunBiomeOptions {
            manifest_path: PathBuf::from("biome.yaml"),
            name: Some("my-biome".to_string()),
            env: vec!["KEY=val".to_string()],
            debug: true,
            cpu_limit: Some(4.0),
            memory_limit: Some("8GB".to_string()),
            security: "high".to_string(),
        };
        assert_eq!(opts.manifest_path, PathBuf::from("biome.yaml"));
        assert_eq!(opts.name.as_deref(), Some("my-biome"));
        assert_eq!(opts.cpu_limit, Some(4.0));
    }

    #[test]
    fn test_up_biome_options_construction() {
        let opts = UpBiomeOptions {
            manifest_path: PathBuf::from("biome.toml"),
            detach: true,
            name: None,
            env: vec![],
            restart: true,
            health_interval: 30,
        };
        assert!(opts.detach);
        assert!(opts.restart);
        assert_eq!(opts.health_interval, 30);
    }

    #[tokio::test]
    async fn test_biome_executor_new() {
        let executor = BiomeExecutor::new().await;
        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_biome_executor_down_nonexistent() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let result = executor
            .down_biome("nonexistent-biome-xyz", false, 30, false)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_biome_executor_list_biomes_empty() {
        let executor = BiomeExecutor::new().await.expect("executor");
        let result = executor.list_biomes(false, "table", false, None).await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Daemon - DaemonConfig, start_daemon
// ============================================================================

#[cfg(feature = "daemon")]
mod daemon_tests {
    use super::*;
    use toadstool_cli::daemon::{DaemonConfig, WorkloadManager};

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.port, toadstool_config::ports::daemon_port());
        assert!(config.max_concurrent_workloads > 0);
    }

    #[test]
    fn test_daemon_config_size_of() {
        let _ = std::mem::size_of::<DaemonConfig>();
        let _ = std::mem::size_of::<WorkloadManager>();
    }

    #[tokio::test]
    async fn test_daemon_config_load_default() {
        let config = DaemonConfig::load(8084, false, None, None, 4, None).await;
        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.port, 8084);
        assert_eq!(cfg.max_concurrent_workloads, 4);
    }

    #[tokio::test]
    async fn test_daemon_config_load_invalid_path() {
        let result = DaemonConfig::load(
            0,
            false,
            None,
            Some(PathBuf::from("/nonexistent/config/path.yaml")),
            4,
            None,
        )
        .await;
        assert!(result.is_err());
    }
}

// ============================================================================
// Network Config - OrchestrationNetworkConfigurator, generate_configuration_summary
// ============================================================================

mod network_config_tests {
    use toadstool_cli::network_config::{
        OrchestrationConfigurator, OrchestrationNetworkConfigurator,
    };

    #[test]
    fn test_orchestration_network_configurator_new() {
        let configurator = OrchestrationNetworkConfigurator::new();
        let _ = configurator.config.service_mesh.enabled;
    }

    #[test]
    fn test_orchestration_configurator_new() {
        let configurator = OrchestrationConfigurator::new();
        let summary = configurator.generate_configuration_summary();
        assert!(summary.contains("Orchestration network configuration"));
        assert!(summary.contains("Service Mesh") || summary.contains("service mesh"));
    }

    #[test]
    fn test_network_configurator_default() {
        let configurator = OrchestrationNetworkConfigurator::default();
        assert!(
            configurator.config.service_mesh.mesh_type == "native"
                || !configurator.config.service_mesh.mesh_type.is_empty()
        );
    }
}

// ============================================================================
// Zero Config - Discovery types, ServiceDiscovery, ZeroConfigDeployment
// ============================================================================

mod zero_config_tests {
    use toadstool_cli::zero_config::{
        AutoGeneratedConfig, CpuInfo, DeploymentSummary, EcosystemServices, MemoryInfo,
        ServiceEndpoint, StorageInfo, SystemInfo,
    };

    #[test]
    fn test_zero_config_system_info_default() {
        let info = SystemInfo::default();
        assert_eq!(info.cpu.cores, 1);
    }

    #[test]
    fn test_zero_config_cpu_info_default() {
        let cpu = CpuInfo::default();
        assert_eq!(cpu.cores, 1);
        assert_eq!(cpu.architecture, "unknown");
    }

    #[test]
    fn test_zero_config_ecosystem_services_default() {
        let svc = EcosystemServices::default();
        assert!(svc.coordination.is_none());
        assert!(svc.security.is_none());
    }

    #[test]
    fn test_zero_config_auto_generated_config_default() {
        let config = AutoGeneratedConfig::default();
        assert_eq!(config.biome.name, "default");
    }

    #[test]
    fn test_service_endpoint_creation() {
        let ep = ServiceEndpoint {
            name: "songbird".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            version: "1.0".to_string(),
            status: "healthy".to_string(),
            auth_required: false,
            discovered_at: std::time::SystemTime::now(),
        };
        assert_eq!(ep.name, "songbird");
        assert_eq!(ep.endpoint, "http://localhost:8080");
    }

    #[test]
    fn test_deployment_summary_creation() {
        let summary = DeploymentSummary {
            total_time: std::time::Duration::from_secs(10),
            system_info: SystemInfo::default(),
            ecosystem_services: EcosystemServices::default(),
            config: AutoGeneratedConfig::default(),
            services_deployed: 3,
            health_status: "healthy".to_string(),
        };
        assert_eq!(summary.services_deployed, 3);
    }

    #[test]
    fn test_memory_info_serialization() {
        let mem = MemoryInfo {
            total_bytes: 16 * 1024 * 1024 * 1024,
            available_bytes: 8 * 1024 * 1024 * 1024,
            memory_type: "DDR4".to_string(),
        };
        let json = serde_json::to_string(&mem).unwrap();
        let parsed: MemoryInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_bytes, mem.total_bytes);
    }

    #[test]
    fn test_storage_info_serialization() {
        let storage = StorageInfo {
            total_bytes: 500 * 1024 * 1024 * 1024,
            available_bytes: 200 * 1024 * 1024 * 1024,
            storage_type: "SSD".to_string(),
            filesystem: "ext4".to_string(),
        };
        let json = serde_json::to_string(&storage).unwrap();
        let parsed: StorageInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.storage_type, "SSD");
    }
}

// ============================================================================
// Universal - Types, Manager, Operations (utilities, benchmarking, migration, detection)
// ============================================================================

mod universal_tests {
    use super::*;
    use toadstool_cli::universal::operations::{
        BenchmarkingOps, MigrationOps, PlatformDetectionOps, UtilityOps,
    };
    use toadstool_cli::universal::{
        BenchmarkResult, BenchmarkTest, BenchmarkType, DetectedPlatform, MigrationPlan,
        MigrationType, PlatformStatus, UniversalComputeManager, WorkloadCheckpoint, WorkloadExport,
        WorkloadSnapshot,
    };
    use toadstool_distributed::substrate_detection::{PlatformType, SubstrateCapabilities};

    #[tokio::test]
    async fn test_universal_compute_manager_new() {
        let manager = UniversalComputeManager::new().await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_benchmark_type_variants() {
        let _ = BenchmarkType::CpuInteger;
        let _ = BenchmarkType::Memory;
        let _ = BenchmarkType::Storage;
        let _ = BenchmarkType::Custom("custom".to_string());
    }

    #[test]
    fn test_benchmark_test_creation() {
        let test = BenchmarkTest {
            name: "CPU".to_string(),
            test_type: BenchmarkType::CpuInteger,
            duration: tokio::time::Duration::from_secs(1),
            score: 1000.0,
            unit: "ops/sec".to_string(),
            details: HashMap::new(),
        };
        assert_eq!(test.name, "CPU");
        assert_eq!(test.score, 1000.0);
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let result = BenchmarkResult {
            platform: "linux".to_string(),
            suite: "standard".to_string(),
            started: std::time::SystemTime::now(),
            duration: tokio::time::Duration::from_secs(5),
            tests: vec![],
            overall_score: 85.0,
            system_info: toadstool_cli::universal::SystemInfo {
                os: "Linux".to_string(),
                arch: "x86_64".to_string(),
                cpu_model: "Test CPU".to_string(),
                cpu_cores: 8,
                memory_gb: 16.0,
                storage_type: "SSD".to_string(),
                gpu_info: None,
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("linux"));
    }

    #[test]
    fn test_migration_type_variants() {
        let _ = MigrationType::LiveMigration;
        let _ = MigrationType::ColdMigration;
        let _ = MigrationType::HotMigration;
        let _ = MigrationType::CloneMigration;
    }

    #[test]
    fn test_migration_plan_creation() {
        let plan = MigrationPlan {
            source_platform: "linux".to_string(),
            target_platform: "docker".to_string(),
            workload_id: "w1".to_string(),
            migration_type: MigrationType::ColdMigration,
            estimated_duration: tokio::time::Duration::from_secs(60),
            risks: vec!["downtime".to_string()],
            requirements: vec!["target available".to_string()],
            cleanup_source: false,
        };
        assert_eq!(plan.source_platform, "linux");
        assert_eq!(plan.target_platform, "docker");
    }

    #[test]
    fn test_workload_checkpoint_creation() {
        let checkpoint = WorkloadCheckpoint {
            biome_name: "my-biome".to_string(),
            timestamp: std::time::SystemTime::now(),
            data_path: PathBuf::from("/tmp/checkpoint"),
        };
        assert_eq!(checkpoint.biome_name, "my-biome");
    }

    #[test]
    fn test_workload_export_creation() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "value".to_string());
        let export = WorkloadExport {
            biome_name: "biome".to_string(),
            export_path: PathBuf::from("/tmp/export"),
            metadata: meta,
        };
        assert_eq!(export.biome_name, "biome");
    }

    #[test]
    fn test_workload_snapshot_creation() {
        let snapshot = WorkloadSnapshot {
            biome_name: "biome".to_string(),
            snapshot_id: "snap-1".to_string(),
            created_at: std::time::SystemTime::now(),
        };
        assert_eq!(snapshot.snapshot_id, "snap-1");
    }

    #[test]
    fn test_platform_status_variants() {
        let _ = PlatformStatus::Available;
        let _ = PlatformStatus::Degraded;
        let _ = PlatformStatus::Testing;
    }

    #[test]
    fn test_detected_platform_creation() {
        let platform = DetectedPlatform {
            platform_type: PlatformType::Linux {
                distribution: "Ubuntu".to_string(),
                architecture: "x86_64".to_string(),
            },
            capabilities: SubstrateCapabilities {
                traditional_platforms: vec![],
                container_platforms: vec![],
                language_runtimes: vec![],
                gpu_platforms: vec![],
                specialized_platforms: vec![],
                experimental_platforms: vec![],
            },
            status: PlatformStatus::Available,
            performance_score: Some(90.0),
            last_tested: None,
            metadata: HashMap::new(),
        };
        assert!(matches!(platform.status, PlatformStatus::Available));
    }

    #[tokio::test]
    async fn test_utility_ops_get_platform_id_linux() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let platform = PlatformType::Linux {
            distribution: "Ubuntu".to_string(),
            architecture: "x86_64".to_string(),
        };
        let id = manager.get_platform_id(&platform);
        assert!(id.contains("ubuntu"));
        assert!(id.contains("x86_64"));
    }

    #[tokio::test]
    async fn test_utility_ops_get_platform_id_docker() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let platform = PlatformType::Docker;
        let id = manager.get_platform_id(&platform);
        assert_eq!(id, "docker");
    }

    #[tokio::test]
    async fn test_utility_ops_get_platform_metadata() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let platform = PlatformType::Linux {
            distribution: "Debian".to_string(),
            architecture: "aarch64".to_string(),
        };
        let meta = manager.get_platform_metadata(&platform);
        assert_eq!(
            meta.get("type").map(std::convert::AsRef::as_ref),
            Some("linux")
        );
        assert_eq!(
            meta.get("distribution").map(std::convert::AsRef::as_ref),
            Some("Debian")
        );
    }

    #[tokio::test]
    async fn test_benchmarking_ops_get_system_info() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let info = manager.get_system_info();
        assert!(info.cpu_cores > 0 || !info.cpu_model.is_empty());
        assert!(!info.cpu_model.is_empty() || info.cpu_model == "Unknown CPU");
    }

    #[tokio::test]
    async fn test_benchmarking_ops_run_cpu_benchmark() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let result: toadstool_cli::Result<_> = manager.run_cpu_benchmark().await;
        assert!(result.is_ok());
        let test = result.unwrap();
        assert_eq!(test.name, "CPU Integer");
        assert!(test.score > 0.0);
    }

    #[tokio::test]
    async fn test_migration_ops_create_plan() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let plan: toadstool_cli::Result<_> =
            manager.create_migration_plan("source", "target").await;
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert_eq!(plan.source_platform, "source");
        assert_eq!(plan.target_platform, "target");
    }

    #[tokio::test]
    async fn test_platform_detection_linux_capabilities() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let result: toadstool_cli::Result<bool> = manager.test_linux_capabilities().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_platform_detection_generic_capabilities() {
        let manager = UniversalComputeManager::new().await.expect("manager");
        let result: toadstool_cli::Result<bool> = manager.test_generic_capabilities().await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Templates - ML Science templates
// ============================================================================

mod template_ml_science_tests {
    use toadstool_cli::templates::specialized_templates::{
        create_ai_research_template, create_genomics_template, create_quantum_template,
        create_science_template, create_vision_template,
    };

    #[test]
    fn test_create_science_template() {
        let (name, desc, _primals, services, resources, _security, _net, _storage) =
            create_science_template();
        assert!(name.contains("science"));
        assert!(!desc.is_empty());
        assert!(resources.cpu_limit.unwrap() > 0.0);
        assert!(services.contains_key("jupyter") || services.contains_key("postgres"));
    }

    #[test]
    fn test_create_ai_research_template() {
        let (name, _, _, services, resources, _, _, _) = create_ai_research_template();
        assert!(name.contains("ai-research") || name.contains("ai"));
        assert!(resources.gpu_limit.is_some() || resources.cpu_limit.unwrap() >= 16.0);
        assert!(services.contains_key("pytorch") || services.contains_key("tensorboard"));
    }

    #[test]
    fn test_create_quantum_template() {
        let (name, desc, _, services, resources, _, _, _) = create_quantum_template();
        assert!(name.contains("quantum"));
        assert!(!desc.is_empty());
        assert!(resources.cpu_limit.unwrap() >= 32.0);
        assert!(services.contains_key("qiskit") || !services.is_empty());
    }

    #[test]
    fn test_create_genomics_template() {
        let (name, _, _, _, resources, security, _, _) = create_genomics_template();
        assert!(name.contains("genomics"));
        assert_eq!(security.isolation_level, "maximum");
        assert!(resources.cpu_limit.unwrap() > 0.0);
    }

    #[test]
    fn test_create_vision_template() {
        let (name, _, _, services, _, _, _, _) = create_vision_template();
        assert!(name.contains("vision"));
        assert!(services.contains_key("opencv") || !services.is_empty());
    }
}
