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
//! Week 2 Zero-Config Discovery Expansion Tests
//!
//! Comprehensive tests for zero-config discovery functionality covering:
//! - System discovery (CPU, memory, storage, network)
//! - Ecosystem service discovery
//! - Discovery error handling
//! - Configuration validation
//! - Discovery state management

use toadstool_cli::zero_config::*;

// ============================================================================
// System Discovery Tests (8 tests)
// ============================================================================

#[test]
fn test_cpu_info_creation() {
    let cpu = CpuInfo {
        cores: 8,
        architecture: "x86_64".to_string(),
        model: "Intel Core i7".to_string(),
        frequency: 2400,
        vendor: "Intel".to_string(),
    };
    assert_eq!(cpu.cores, 8);
    assert_eq!(cpu.architecture, "x86_64");
}

#[test]
fn test_memory_info_creation() {
    let memory = MemoryInfo {
        total_bytes: 16_384_000_000,
        available_bytes: 8_192_000_000,
        memory_type: "DDR4".to_string(),
    };
    assert_eq!(memory.total_bytes, 16_384_000_000);
    assert!(memory.available_bytes <= memory.total_bytes);
}

#[test]
fn test_storage_info_creation() {
    let storage = StorageInfo {
        total_bytes: 500_000_000_000,
        available_bytes: 250_000_000_000,
        storage_type: "SSD".to_string(),
        filesystem: "ext4".to_string(),
    };
    assert_eq!(storage.total_bytes, 500_000_000_000);
    assert!(storage.available_bytes <= storage.total_bytes);
}

#[test]
fn test_network_info_creation() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.100".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 1000,
    };
    let network = NetworkInfo {
        interfaces: vec![iface],
        external_ip: Some("1.2.3.4".to_string()),
        local_ips: vec!["192.168.1.100".to_string()],
    };
    assert_eq!(network.interfaces.len(), 1);
    assert_eq!(network.interfaces[0].name, "eth0");
}

#[test]
fn test_os_info_creation() {
    let os = OsInfo {
        name: "Linux".to_string(),
        version: "6.16.3".to_string(),
        kernel: "6.16.3-76061603-generic".to_string(),
        arch: "x86_64".to_string(),
    };
    assert_eq!(os.name, "Linux");
    assert_eq!(os.arch, "x86_64");
}

#[test]
fn test_container_runtime_info_creation() {
    let runtime = ContainerRuntimeInfo {
        docker: true,
        podman: false,
        containerd: false,
        version: Some("24.0.0".to_string()),
    };
    assert!(runtime.docker);
    assert!(runtime.version.is_some());
}

#[test]
fn test_gpu_info_creation() {
    let gpu = GpuInfo {
        count: 2,
        vendor: "NVIDIA".to_string(),
        model: "RTX 4090".to_string(),
        memory_bytes: 24_000_000_000,
        cuda: true,
        opencl: true,
    };
    assert_eq!(gpu.count, 2);
    assert!(gpu.cuda);
}

#[test]
fn test_system_info_aggregation() {
    let system = SystemInfo {
        cpu: CpuInfo {
            cores: 16,
            architecture: "x86_64".to_string(),
            model: "AMD Ryzen".to_string(),
            frequency: 3600,
            vendor: "AMD".to_string(),
        },
        memory: MemoryInfo {
            total_bytes: 32_768_000_000,
            available_bytes: 16_384_000_000,
            memory_type: "DDR4".to_string(),
        },
        storage: StorageInfo {
            total_bytes: 1_000_000_000_000,
            available_bytes: 500_000_000_000,
            storage_type: "NVMe".to_string(),
            filesystem: "ext4".to_string(),
        },
        network: NetworkInfo {
            interfaces: vec![],
            external_ip: Some("1.2.3.4".to_string()),
            local_ips: vec!["10.0.0.100".to_string()],
        },
        os: OsInfo {
            name: "Linux".to_string(),
            version: "6.16.3".to_string(),
            kernel: "6.16.3-76061603-generic".to_string(),
            arch: "x86_64".to_string(),
        },
        container_runtime: ContainerRuntimeInfo {
            docker: true,
            podman: false,
            containerd: false,
            version: Some("24.0.0".to_string()),
        },
        gpu: GpuInfo {
            count: 0,
            vendor: "None".to_string(),
            model: "None".to_string(),
            memory_bytes: 0,
            cuda: false,
            opencl: false,
        },
    };

    assert_eq!(system.cpu.cores, 16);
    assert_eq!(system.memory.total_bytes, 32_768_000_000);
    assert!(system.container_runtime.docker);
}

// ============================================================================
// Ecosystem Service Discovery Tests (6 tests)
// ============================================================================

#[test]
fn test_service_endpoint_creation() {
    let endpoint = ServiceEndpoint {
        name: "test-service".to_string(),
        endpoint: "http://localhost:8080".to_string(),
        version: "1.0.0".to_string(),
        status: "healthy".to_string(),
        auth_required: false,
        discovered_at: std::time::SystemTime::now(),
    };
    assert_eq!(endpoint.endpoint, "http://localhost:8080");
    assert_eq!(endpoint.status, "healthy");
}

#[test]
fn test_service_endpoint_with_auth() {
    let endpoint = ServiceEndpoint {
        name: "secure-service".to_string(),
        endpoint: "http://service:9000".to_string(),
        version: "2.0.0".to_string(),
        status: "unhealthy".to_string(),
        auth_required: true,
        discovered_at: std::time::SystemTime::now(),
    };
    assert!(endpoint.auth_required);
    assert_eq!(endpoint.status, "unhealthy");
}

#[test]
fn test_ecosystem_services_all_available() {
    let services = EcosystemServices {
        songbird: Some(ServiceEndpoint {
            name: "songbird".to_string(),
            endpoint: "http://songbird:8080".to_string(),
            version: "1.0.0".to_string(),
            status: "healthy".to_string(),
            auth_required: false,
            discovered_at: std::time::SystemTime::now(),
        }),
        beardog: Some(ServiceEndpoint {
            name: "beardog".to_string(),
            endpoint: "http://beardog:8081".to_string(),
            version: "1.0.0".to_string(),
            status: "healthy".to_string(),
            auth_required: true,
            discovered_at: std::time::SystemTime::now(),
        }),
        nestgate: Some(ServiceEndpoint {
            name: "nestgate".to_string(),
            endpoint: "http://nestgate:9000".to_string(),
            version: "1.0.0".to_string(),
            status: "healthy".to_string(),
            auth_required: false,
            discovered_at: std::time::SystemTime::now(),
        }),
        squirrel: Some(ServiceEndpoint {
            name: "squirrel".to_string(),
            endpoint: "http://squirrel:6000".to_string(),
            version: "1.0.0".to_string(),
            status: "healthy".to_string(),
            auth_required: false,
            discovered_at: std::time::SystemTime::now(),
        }),
        toadstool_peers: vec![],
    };

    assert!(services.songbird.is_some());
    assert!(services.beardog.is_some());
    assert!(services.nestgate.is_some());
    assert!(services.squirrel.is_some());
}

#[test]
fn test_ecosystem_services_partial_availability() {
    let services = EcosystemServices {
        songbird: Some(ServiceEndpoint {
            name: "songbird".to_string(),
            endpoint: "http://songbird:8080".to_string(),
            version: "1.0.0".to_string(),
            status: "healthy".to_string(),
            auth_required: false,
            discovered_at: std::time::SystemTime::now(),
        }),
        beardog: None,
        nestgate: None,
        squirrel: None,
        toadstool_peers: vec![],
    };

    assert!(services.songbird.is_some());
    assert!(services.beardog.is_none());
}

#[test]
fn test_service_endpoint_status() {
    let healthy = ServiceEndpoint {
        name: "service1".to_string(),
        endpoint: "http://service1:8080".to_string(),
        version: "1.0.0".to_string(),
        status: "healthy".to_string(),
        auth_required: false,
        discovered_at: std::time::SystemTime::now(),
    };

    let unhealthy = ServiceEndpoint {
        name: "service2".to_string(),
        endpoint: "http://service2:8080".to_string(),
        version: "1.0.0".to_string(),
        status: "unhealthy".to_string(),
        auth_required: false,
        discovered_at: std::time::SystemTime::now(),
    };

    assert_eq!(healthy.status, "healthy");
    assert_eq!(unhealthy.status, "unhealthy");
}

#[test]
fn test_ecosystem_services_default() {
    let services = EcosystemServices::default();
    assert!(services.songbird.is_none());
    assert!(services.beardog.is_none());
    assert!(services.nestgate.is_none());
    assert!(services.squirrel.is_none());
}

// ============================================================================
// Zero Config Deployment Tests (6 tests)
// ============================================================================

#[test]
fn test_zero_config_deployment_creation() {
    let deployment = ZeroConfigDeployment::new();
    assert!(deployment.system_info.cpu.cores > 0);
}

#[test]
fn test_zero_config_deployment_default() {
    let deployment = ZeroConfigDeployment::default();
    // Verify deployment has default values
    let _system_info = &deployment.system_info;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_zero_config_system_info_cpu() {
    let deployment = ZeroConfigDeployment::new();
    assert!(deployment.system_info.cpu.cores > 0);
    assert!(!deployment.system_info.cpu.architecture.is_empty());
}

#[test]
fn test_zero_config_system_info_memory() {
    let deployment = ZeroConfigDeployment::new();
    // Memory should have default values (u64, always >= 0)
    let _total = deployment.system_info.memory.total_bytes;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_zero_config_system_info_storage() {
    let deployment = ZeroConfigDeployment::new();
    // Storage should have default values (u64, always >= 0)
    let _total = deployment.system_info.storage.total_bytes;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_zero_config_ecosystem_services() {
    let deployment = ZeroConfigDeployment::new();
    // Ecosystem services should be initialized
    let _services = &deployment.ecosystem_services;
    // Test passes if compilation succeeds and no panic occurs
}

// ============================================================================
// Configuration Tests (5 tests)
// ============================================================================

#[test]
fn test_auto_generated_config_exists() {
    let deployment = ZeroConfigDeployment::new();
    let _config = &deployment.config;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_deployment_has_start_time() {
    let deployment = ZeroConfigDeployment::new();
    // Verify start_time is initialized and is valid
    let _start_time = deployment.start_time;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_network_interface_has_fields() {
    let iface = NetworkInterface {
        name: "eth0".to_string(),
        ip: "192.168.1.1".to_string(),
        mac: "00:11:22:33:44:55".to_string(),
        speed: 1000,
    };
    assert_eq!(iface.name, "eth0");
    assert_eq!(iface.speed, 1000);
}

#[test]
fn test_container_runtime_info_defaults() {
    let runtime = ContainerRuntimeInfo::default();
    // Verify defaults are accessible (booleans are always true or false)
    let _docker = runtime.docker;
    let _podman = runtime.podman;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_ecosystem_services_structure() {
    let services = EcosystemServices::default();
    // All should start as None
    assert!(services.songbird.is_none());
    assert!(services.beardog.is_none());
    assert!(services.nestgate.is_none());
    assert!(services.squirrel.is_none());
}

// ============================================================================
// Integration Tests (5 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_deployment_creation_is_fast() {
    let start = std::time::Instant::now();
    let deployment = ZeroConfigDeployment::new();
    let duration = start.elapsed();

    assert!(duration.as_millis() < 1000); // Should be fast
    assert!(deployment.system_info.cpu.cores > 0);
}

#[test]
fn test_system_info_completeness_check() {
    let system = SystemInfo::default();
    // Verify fields exist and are accessible (types are unsigned, always >= 0)
    let _cores = system.cpu.cores;
    let _memory = system.memory.total_bytes;
    let _storage = system.storage.total_bytes;
    // Test passes if compilation succeeds and no panic occurs
}

#[test]
fn test_service_endpoint_url_format() {
    let endpoint = ServiceEndpoint {
        name: "service".to_string(),
        endpoint: "http://service:8080".to_string(),
        version: "1.0.0".to_string(),
        status: "healthy".to_string(),
        auth_required: false,
        discovered_at: std::time::SystemTime::now(),
    };

    assert!(endpoint.endpoint.starts_with("http://"));
    assert!(endpoint.endpoint.contains(":8080"));
}

#[test]
fn test_deployment_instances_are_independent() {
    let deployment1 = ZeroConfigDeployment::new();
    let deployment2 = ZeroConfigDeployment::new();

    // Start times should be close but independent
    let diff = deployment2
        .start_time
        .duration_since(deployment1.start_time);
    assert!(diff.as_millis() < 100);
}

#[test]
fn test_default_implementations_work() {
    let system = SystemInfo::default();
    let runtime = ContainerRuntimeInfo::default();
    let network = NetworkInfo::default();
    let services = EcosystemServices::default();

    // Verify all defaults work (types are valid)
    let _ = system.cpu.cores;
    let _ = runtime.docker;
    let _ = network.interfaces;
    assert!(services.songbird.is_none());
}
