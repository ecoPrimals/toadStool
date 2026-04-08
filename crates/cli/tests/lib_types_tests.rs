// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI library types and structures

// HashMap import removed - unused
use std::path::PathBuf;
use toadstool_cli::*;
use uuid::Uuid;

// ============================================================================
// CliError Tests
// ============================================================================

#[test]
fn test_cli_error_biome_not_found() {
    let error = CliError::BiomeNotFound("test-biome".to_string());
    assert!(error.to_string().contains("test-biome"));
    assert!(error.to_string().contains("not found"));
}

#[test]
fn test_cli_error_biome_already_exists() {
    let error = CliError::BiomeAlreadyExists("existing-biome".to_string());
    assert!(error.to_string().contains("existing-biome"));
    assert!(error.to_string().contains("already exists"));
}

#[test]
fn test_cli_error_invalid_config() {
    let error = CliError::InvalidConfig("bad config".to_string());
    assert!(error.to_string().contains("Invalid configuration"));
}

#[test]
fn test_cli_error_system() {
    let error = CliError::System("system failure".to_string());
    assert!(error.to_string().contains("system failure"));
}

#[test]
fn test_cli_error_other() {
    let error = CliError::Other("unexpected error".to_string());
    assert!(error.to_string().contains("unexpected error"));
}

// ============================================================================
// BiomeMetadata Tests
// ============================================================================

#[test]
fn test_biome_metadata_creation() {
    let metadata = BiomeMetadata {
        name: "my-biome".to_string(),
        version: "1.0.0".to_string(),
        description: Some("A test biome".to_string()),
        author: Some("Test Author".to_string()),
        created: std::time::SystemTime::now(),
        updated: std::time::SystemTime::now(),
        tags: vec!["web".to_string(), "api".to_string()],
    };

    assert_eq!(metadata.name, "my-biome");
    assert_eq!(metadata.version, "1.0.0");
    assert_eq!(metadata.tags.len(), 2);
}

#[test]
fn test_biome_metadata_no_description() {
    let metadata = BiomeMetadata {
        name: "minimal-biome".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        author: None,
        created: std::time::SystemTime::now(),
        updated: std::time::SystemTime::now(),
        tags: vec![],
    };

    assert!(metadata.description.is_none());
    assert!(metadata.author.is_none());
    assert!(metadata.tags.is_empty());
}

#[test]
fn test_biome_metadata_serialization() {
    let metadata = BiomeMetadata {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: Some("desc".to_string()),
        author: None,
        created: std::time::SystemTime::now(),
        updated: std::time::SystemTime::now(),
        tags: vec![],
    };

    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: BiomeMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "test");
}

// ============================================================================
// WorkloadSource Tests
// ============================================================================

#[test]
fn test_workload_source_container() {
    let source = WorkloadSource::Container {
        registry: "docker.io".to_string(),
        image: "nginx".to_string(),
        tag: "latest".to_string(),
        digest: None,
    };

    match source {
        WorkloadSource::Container {
            registry,
            image,
            tag,
            ..
        } => {
            assert_eq!(registry, "docker.io");
            assert_eq!(image, "nginx");
            assert_eq!(tag, "latest");
        }
        _ => panic!("Expected Container variant"),
    }
}

#[test]
fn test_workload_source_container_with_digest() {
    let source = WorkloadSource::Container {
        registry: "ghcr.io".to_string(),
        image: "myapp".to_string(),
        tag: "v1.2.3".to_string(),
        digest: Some("sha256:abc123".to_string()),
    };

    match source {
        WorkloadSource::Container { digest, .. } => {
            assert!(digest.is_some());
            assert_eq!(digest.unwrap(), "sha256:abc123");
        }
        _ => panic!("Expected Container variant"),
    }
}

#[test]
fn test_workload_source_wasm() {
    let source = WorkloadSource::Wasm {
        source: "https://example.com/module.wasm".to_string(),
        checksum: "sha256:def456".to_string(),
        wasi_config: None,
    };

    match source {
        WorkloadSource::Wasm {
            source, checksum, ..
        } => {
            assert!(source.contains("module.wasm"));
            assert!(checksum.starts_with("sha256:"));
        }
        _ => panic!("Expected Wasm variant"),
    }
}

#[test]
fn test_workload_source_git() {
    let source = WorkloadSource::Git {
        repository: "https://github.com/user/repo.git".to_string(),
        branch: Some("main".to_string()),
        commit: Some("abc123def456".to_string()),
        path: Some("services/api".to_string()),
    };

    match source {
        WorkloadSource::Git {
            repository,
            branch,
            commit,
            path,
        } => {
            assert!(repository.contains("github.com"));
            assert_eq!(branch.unwrap(), "main");
            assert_eq!(commit.unwrap(), "abc123def456");
            assert_eq!(path.unwrap(), "services/api");
        }
        _ => panic!("Expected Git variant"),
    }
}

#[test]
fn test_workload_source_ipfs() {
    let source = WorkloadSource::Ipfs {
        hash: "QmXyz123".to_string(),
        gateway: Some("https://ipfs.io".to_string()),
    };

    match source {
        WorkloadSource::Ipfs { hash, gateway } => {
            assert_eq!(hash, "QmXyz123");
            assert_eq!(gateway.unwrap(), "https://ipfs.io");
        }
        _ => panic!("Expected Ipfs variant"),
    }
}

#[test]
fn test_workload_source_local() {
    let source = WorkloadSource::Local {
        path: PathBuf::from("/opt/app"),
    };

    match source {
        WorkloadSource::Local { path } => {
            assert_eq!(path, PathBuf::from("/opt/app"));
        }
        _ => panic!("Expected Local variant"),
    }
}

#[test]
fn test_workload_source_serialization() {
    let source = WorkloadSource::Container {
        registry: "test.io".to_string(),
        image: "img".to_string(),
        tag: "v1".to_string(),
        digest: None,
    };

    let json = serde_json::to_string(&source).unwrap();
    let deserialized: WorkloadSource = serde_json::from_str(&json).unwrap();

    match deserialized {
        WorkloadSource::Container { image, .. } => assert_eq!(image, "img"),
        _ => panic!("Deserialization failed"),
    }
}

// ============================================================================
// BiomeResources Tests
// ============================================================================

#[test]
fn test_biome_resources_all_limits() {
    let resources = BiomeResources {
        cpu_limit: Some(4.0),
        memory_limit: Some("8GB".to_string()),
        storage_limit: Some("100GB".to_string()),
        gpu_limit: Some(2),
        network_bandwidth: Some("1Gbps".to_string()),
    };

    assert_eq!(resources.cpu_limit.unwrap(), 4.0);
    assert_eq!(resources.memory_limit.unwrap(), "8GB");
    assert_eq!(resources.gpu_limit.unwrap(), 2);
}

#[test]
fn test_biome_resources_no_limits() {
    let resources = BiomeResources {
        cpu_limit: None,
        memory_limit: None,
        storage_limit: None,
        gpu_limit: None,
        network_bandwidth: None,
    };

    assert!(resources.cpu_limit.is_none());
    assert!(resources.memory_limit.is_none());
    assert!(resources.gpu_limit.is_none());
}

#[test]
fn test_biome_resources_partial_limits() {
    let resources = BiomeResources {
        cpu_limit: Some(2.0),
        memory_limit: Some("4GB".to_string()),
        storage_limit: None,
        gpu_limit: None,
        network_bandwidth: None,
    };

    assert!(resources.cpu_limit.is_some());
    assert!(resources.memory_limit.is_some());
    assert!(resources.storage_limit.is_none());
}

#[test]
fn test_biome_resources_serialization() {
    let resources = BiomeResources {
        cpu_limit: Some(1.0),
        memory_limit: Some("2GB".to_string()),
        storage_limit: None,
        gpu_limit: None,
        network_bandwidth: None,
    };

    let json = serde_json::to_string(&resources).unwrap();
    let deserialized: BiomeResources = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cpu_limit.unwrap(), 1.0);
}

// ============================================================================
// BiomeSecurity Tests
// ============================================================================

#[test]
fn test_biome_security_high_isolation() {
    let security = BiomeSecurity {
        isolation_level: "high".to_string(),
        trust_level: "zero-trust".to_string(),
        security_required: true,
        crypto_policies: vec!["tls-1.3".to_string(), "aes-256".to_string()],
        allowed_networks: vec!["10.0.0.0/8".to_string()],
        forbidden_syscalls: vec!["ptrace".to_string(), "mount".to_string()],
    };

    assert_eq!(security.isolation_level, "high");
    assert!(security.security_required);
    assert_eq!(security.crypto_policies.len(), 2);
    assert_eq!(security.forbidden_syscalls.len(), 2);
}

#[test]
fn test_biome_security_minimal() {
    let security = BiomeSecurity {
        isolation_level: "low".to_string(),
        trust_level: "trusted".to_string(),
        security_required: false,
        crypto_policies: vec![],
        allowed_networks: vec![],
        forbidden_syscalls: vec![],
    };

    assert_eq!(security.isolation_level, "low");
    assert!(!security.security_required);
    assert!(security.crypto_policies.is_empty());
}

#[test]
fn test_biome_security_serialization() {
    let security = BiomeSecurity {
        isolation_level: "medium".to_string(),
        trust_level: "standard".to_string(),
        security_required: true,
        crypto_policies: vec![],
        allowed_networks: vec![],
        forbidden_syscalls: vec![],
    };

    let json = serde_json::to_string(&security).unwrap();
    let deserialized: BiomeSecurity = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.isolation_level, "medium");
}

// ============================================================================
// ServiceResources Tests
// ============================================================================

#[test]
fn test_service_resources_all_set() {
    let resources = ServiceResources {
        cpu_limit: Some(2.0),
        memory_limit: Some("4GB".to_string()),
        storage_limit: Some("50GB".to_string()),
    };

    assert_eq!(resources.cpu_limit.unwrap(), 2.0);
    assert_eq!(resources.memory_limit.unwrap(), "4GB");
    assert_eq!(resources.storage_limit.unwrap(), "50GB");
}

#[test]
fn test_service_resources_none() {
    let resources = ServiceResources {
        cpu_limit: None,
        memory_limit: None,
        storage_limit: None,
    };

    assert!(resources.cpu_limit.is_none());
}

// ============================================================================
// ServicePort Tests
// ============================================================================

#[test]
fn test_service_port_with_host_port() {
    let port = ServicePort {
        container_port: 8080,
        host_port: Some(80),
        protocol: "tcp".to_string(),
    };

    assert_eq!(port.container_port, 8080);
    assert_eq!(port.host_port.unwrap(), 80);
    assert_eq!(port.protocol, "tcp");
}

#[test]
fn test_service_port_no_host_port() {
    let port = ServicePort {
        container_port: 3000,
        host_port: None,
        protocol: "tcp".to_string(),
    };

    assert_eq!(port.container_port, 3000);
    assert!(port.host_port.is_none());
}

#[test]
fn test_service_port_udp() {
    let port = ServicePort {
        container_port: 53,
        host_port: Some(53),
        protocol: "udp".to_string(),
    };

    assert_eq!(port.protocol, "udp");
}

// ============================================================================
// ServiceVolume Tests
// ============================================================================

#[test]
fn test_service_volume_read_write() {
    let volume = ServiceVolume {
        source: "/host/data".to_string(),
        target: "/app/data".to_string(),
        read_only: false,
    };

    assert_eq!(volume.source, "/host/data");
    assert_eq!(volume.target, "/app/data");
    assert!(!volume.read_only);
}

#[test]
fn test_service_volume_read_only() {
    let volume = ServiceVolume {
        source: "/host/config".to_string(),
        target: "/app/config".to_string(),
        read_only: true,
    };

    assert!(volume.read_only);
}

// ============================================================================
// HealthCheck Tests
// ============================================================================

#[test]
fn test_health_check_http() {
    let health_check = HealthCheck {
        command: vec!["curl".to_string(), "http://localhost/health".to_string()],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };

    assert_eq!(health_check.command.len(), 2);
    assert_eq!(health_check.interval, 30);
    assert_eq!(health_check.retries, 3);
}

#[test]
fn test_health_check_tcp() {
    let health_check = HealthCheck {
        command: vec![
            "nc".to_string(),
            "-z".to_string(),
            "localhost".to_string(),
            "8080".to_string(),
        ],
        interval: 15,
        timeout: 3,
        retries: 5,
        start_period: 5,
    };

    assert_eq!(health_check.command.len(), 4);
    assert_eq!(health_check.timeout, 3);
}

#[test]
fn test_health_check_serialization() {
    let health_check = HealthCheck {
        command: vec!["echo".to_string(), "ok".to_string()],
        interval: 60,
        timeout: 10,
        retries: 2,
        start_period: 0,
    };

    let json = serde_json::to_string(&health_check).unwrap();
    let deserialized: HealthCheck = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.interval, 60);
}

// ============================================================================
// BiomeStatus Tests
// ============================================================================

#[test]
fn test_biome_status_starting() {
    let status = BiomeStatus::Starting;
    assert!(matches!(status, BiomeStatus::Starting));
}

#[test]
fn test_biome_status_running() {
    let status = BiomeStatus::Running;
    assert!(matches!(status, BiomeStatus::Running));
}

#[test]
fn test_biome_status_stopping() {
    let status = BiomeStatus::Stopping;
    assert!(matches!(status, BiomeStatus::Stopping));
}

#[test]
fn test_biome_status_stopped() {
    let status = BiomeStatus::Stopped;
    assert!(matches!(status, BiomeStatus::Stopped));
}

#[test]
fn test_biome_status_error() {
    let status = BiomeStatus::Error("failed to start".to_string());
    match status {
        BiomeStatus::Error(msg) => assert_eq!(msg, "failed to start"),
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_biome_status_migrating() {
    let status = BiomeStatus::Migrating;
    assert!(matches!(status, BiomeStatus::Migrating));
}

#[test]
fn test_biome_status_serialization() {
    let status = BiomeStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: BiomeStatus = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, BiomeStatus::Running));
}

// ============================================================================
// ResourceUsage Tests
// ============================================================================

#[test]
fn test_resource_usage_typical() {
    let usage = ResourceUsage {
        cpu_percent: 45.5,
        memory_bytes: 1024 * 1024 * 512,        // 512 MB
        storage_bytes: 1024 * 1024 * 1024 * 10, // 10 GB
        network_rx_bytes: 1024 * 1024,          // 1 MB
        network_tx_bytes: 512 * 1024,           // 512 KB
    };

    assert!((usage.cpu_percent - 45.5).abs() < f64::EPSILON);
    assert_eq!(usage.memory_bytes, 1024 * 1024 * 512);
}

#[test]
fn test_resource_usage_zero() {
    let usage = ResourceUsage {
        cpu_percent: 0.0,
        memory_bytes: 0,
        storage_bytes: 0,
        network_rx_bytes: 0,
        network_tx_bytes: 0,
    };

    assert_eq!(usage.cpu_percent, 0.0);
    assert_eq!(usage.memory_bytes, 0);
}

#[test]
fn test_resource_usage_high() {
    let usage = ResourceUsage {
        cpu_percent: 99.9,
        memory_bytes: u64::MAX / 2,
        storage_bytes: u64::MAX / 4,
        network_rx_bytes: 1024 * 1024 * 1024, // 1 GB
        network_tx_bytes: 1024 * 1024 * 1024, // 1 GB
    };

    assert!(usage.cpu_percent > 99.0);
    assert!(usage.memory_bytes > 0);
}

#[test]
fn test_resource_usage_serialization() {
    let usage = ResourceUsage {
        cpu_percent: 50.0,
        memory_bytes: 1024,
        storage_bytes: 2048,
        network_rx_bytes: 100,
        network_tx_bytes: 200,
    };

    let json = serde_json::to_string(&usage).unwrap();
    let deserialized: ResourceUsage = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.memory_bytes, 1024);
}

// ============================================================================
// ServiceInfo Tests
// ============================================================================

#[test]
fn test_service_info_healthy() {
    let info = ServiceInfo {
        name: "web-api".to_string(),
        status: "running".to_string(),
        replicas: 3,
        ports: vec![80, 443],
        health: "healthy".to_string(),
    };

    assert_eq!(info.name, "web-api");
    assert_eq!(info.replicas, 3);
    assert_eq!(info.ports.len(), 2);
    assert_eq!(info.health, "healthy");
}

#[test]
fn test_service_info_degraded() {
    let info = ServiceInfo {
        name: "database".to_string(),
        status: "running".to_string(),
        replicas: 1,
        ports: vec![5432],
        health: "degraded".to_string(),
    };

    assert_eq!(info.health, "degraded");
}

#[test]
fn test_service_info_no_ports() {
    let info = ServiceInfo {
        name: "worker".to_string(),
        status: "running".to_string(),
        replicas: 5,
        ports: vec![],
        health: "healthy".to_string(),
    };

    assert!(info.ports.is_empty());
    assert_eq!(info.replicas, 5);
}

// ============================================================================
// BiomeInfo Tests
// ============================================================================

#[test]
fn test_biome_info_creation() {
    let info = BiomeInfo {
        id: Uuid::new_v4(),
        name: "prod-biome".to_string(),
        status: BiomeStatus::Running,
        created: std::time::SystemTime::now(),
        started: Some(std::time::SystemTime::now()),
        manifest_path: PathBuf::from("/opt/biomes/prod.yaml"),
        resource_usage: ResourceUsage {
            cpu_percent: 25.0,
            memory_bytes: 1024 * 1024 * 1024,
            storage_bytes: 10 * 1024 * 1024 * 1024,
            network_rx_bytes: 1000,
            network_tx_bytes: 2000,
        },
        services: vec![],
    };

    assert_eq!(info.name, "prod-biome");
    assert!(info.started.is_some());
    assert!(info.services.is_empty());
}

#[test]
fn test_biome_info_with_services() {
    let info = BiomeInfo {
        id: Uuid::new_v4(),
        name: "test-biome".to_string(),
        status: BiomeStatus::Running,
        created: std::time::SystemTime::now(),
        started: Some(std::time::SystemTime::now()),
        manifest_path: PathBuf::from("/tmp/test.yaml"),
        resource_usage: ResourceUsage {
            cpu_percent: 10.0,
            memory_bytes: 512 * 1024 * 1024,
            storage_bytes: 1024 * 1024 * 1024,
            network_rx_bytes: 100,
            network_tx_bytes: 200,
        },
        services: vec![
            ServiceInfo {
                name: "api".to_string(),
                status: "running".to_string(),
                replicas: 2,
                ports: vec![8080],
                health: "healthy".to_string(),
            },
            ServiceInfo {
                name: "db".to_string(),
                status: "running".to_string(),
                replicas: 1,
                ports: vec![5432],
                health: "healthy".to_string(),
            },
        ],
    };

    assert_eq!(info.services.len(), 2);
    assert_eq!(info.services[0].name, "api");
}

#[test]
fn test_biome_info_not_started() {
    let info = BiomeInfo {
        id: Uuid::new_v4(),
        name: "new-biome".to_string(),
        status: BiomeStatus::Starting,
        created: std::time::SystemTime::now(),
        started: None,
        manifest_path: PathBuf::from("/tmp/new.yaml"),
        resource_usage: ResourceUsage {
            cpu_percent: 0.0,
            memory_bytes: 0,
            storage_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        },
        services: vec![],
    };

    assert!(info.started.is_none());
    assert!(matches!(info.status, BiomeStatus::Starting));
}
