// SPDX-License-Identifier: AGPL-3.0-or-later
//! Data structures, validation, and status handling.

use toadstool::byob::*;

// ============================================================================
// Data Structure Tests
// ============================================================================

#[test]
fn test_service_resource_requirements_default() {
    let reqs = ServiceResourceRequirements {
        cpu_cores: None,
        memory_bytes: None,
        storage_bytes: None,
        gpu_count: None,
    };

    assert!(reqs.cpu_cores.is_none());
    assert!(reqs.memory_bytes.is_none());
    assert!(reqs.storage_bytes.is_none());
    assert!(reqs.gpu_count.is_none());
}

#[test]
fn test_service_resource_requirements_with_values() {
    let reqs = ServiceResourceRequirements {
        cpu_cores: Some(4.0),
        memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
        storage_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        gpu_count: Some(2),
    };

    assert_eq!(reqs.cpu_cores, Some(4.0));
    assert_eq!(reqs.memory_bytes, Some(8_589_934_592));
    assert_eq!(reqs.storage_bytes, Some(107_374_182_400));
    assert_eq!(reqs.gpu_count, Some(2));
}

#[test]
fn test_service_resource_requirements_clone() {
    let reqs1 = ServiceResourceRequirements {
        cpu_cores: Some(2.0),
        memory_bytes: Some(4_000_000_000),
        storage_bytes: Some(50_000_000_000),
        gpu_count: Some(1),
    };

    let reqs2 = reqs1.clone();
    assert_eq!(reqs1.cpu_cores, reqs2.cpu_cores);
    assert_eq!(reqs1.memory_bytes, reqs2.memory_bytes);
}

#[test]
fn test_service_resource_requirements_debug() {
    let reqs = ServiceResourceRequirements {
        cpu_cores: Some(1.0),
        memory_bytes: Some(2_000_000_000),
        storage_bytes: None,
        gpu_count: None,
    };

    let debug_str = format!("{reqs:?}");
    assert!(debug_str.contains("ServiceResourceRequirements"));
}

#[test]
fn test_service_resource_requirements_serialization() {
    let reqs = ServiceResourceRequirements {
        cpu_cores: Some(2.5),
        memory_bytes: Some(4_000_000_000),
        storage_bytes: Some(10_000_000_000),
        gpu_count: Some(1),
    };

    let json = serde_json::to_string(&reqs).unwrap();
    let deserialized: ServiceResourceRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(reqs.cpu_cores, deserialized.cpu_cores);
    assert_eq!(reqs.memory_bytes, deserialized.memory_bytes);
}

// ============================================================================
// TeamResourceQuotas Tests
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_team_resource_quotas_creation() {
    let quotas = TeamResourceQuotas {
        max_cpu_cores: 16.0,
        max_memory_bytes: 32 * 1024 * 1024 * 1024,
        max_storage_bytes: 500 * 1024 * 1024 * 1024,
        max_gpu_count: 4,
        max_concurrent_services: 50,
    };

    assert_eq!(quotas.max_cpu_cores, 16.0);
    assert_eq!(quotas.max_gpu_count, 4);
    assert_eq!(quotas.max_concurrent_services, 50);
}

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_team_resource_quotas_clone() {
    let quotas1 = TeamResourceQuotas {
        max_cpu_cores: 8.0,
        max_memory_bytes: 16_000_000_000,
        max_storage_bytes: 200_000_000_000,
        max_gpu_count: 2,
        max_concurrent_services: 25,
    };

    let quotas2 = quotas1.clone();
    assert_eq!(quotas1.max_cpu_cores, quotas2.max_cpu_cores);
    assert_eq!(
        quotas1.max_concurrent_services,
        quotas2.max_concurrent_services
    );
}

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_team_resource_quotas_serialization() {
    let quotas = TeamResourceQuotas {
        max_cpu_cores: 10.0,
        max_memory_bytes: 20_000_000_000,
        max_storage_bytes: 300_000_000_000,
        max_gpu_count: 3,
        max_concurrent_services: 30,
    };

    let json = serde_json::to_string(&quotas).unwrap();
    let deserialized: TeamResourceQuotas = serde_json::from_str(&json).unwrap();

    assert_eq!(quotas.max_cpu_cores, deserialized.max_cpu_cores);
    assert_eq!(quotas.max_gpu_count, deserialized.max_gpu_count);
}

#[test]
fn test_team_resource_quotas_debug() {
    let quotas = TeamResourceQuotas {
        max_cpu_cores: 4.0,
        max_memory_bytes: 8_000_000_000,
        max_storage_bytes: 100_000_000_000,
        max_gpu_count: 1,
        max_concurrent_services: 10,
    };

    let debug_str = format!("{quotas:?}");
    assert!(debug_str.contains("TeamResourceQuotas"));
    assert!(debug_str.contains("max_cpu_cores"));
}

// ============================================================================
// TeamSecurityConfig Tests
// ============================================================================

#[test]
fn test_team_security_config_creation() {
    let config = TeamSecurityConfig {
        isolation_level: "high".to_string(),
        network_policies: vec!["deny-all".to_string(), "allow-internal".to_string()],
        volume_policies: vec!["read-only".to_string()],
        resource_policies: vec!["limited".to_string()],
    };

    assert_eq!(config.isolation_level, "high");
    assert_eq!(config.network_policies.len(), 2);
    assert_eq!(config.volume_policies.len(), 1);
}

#[test]
fn test_team_security_config_with_policies() {
    let config = TeamSecurityConfig {
        isolation_level: "medium".to_string(),
        network_policies: vec!["allow-egress".to_string()],
        volume_policies: vec!["read-write".to_string(), "mount-secret".to_string()],
        resource_policies: vec!["flexible".to_string()],
    };

    assert_eq!(config.network_policies.len(), 1);
    assert_eq!(config.volume_policies.len(), 2);
}

#[test]
fn test_team_security_config_clone() {
    let config1 = TeamSecurityConfig {
        isolation_level: "low".to_string(),
        network_policies: vec!["allow-all".to_string()],
        volume_policies: vec![],
        resource_policies: vec!["unrestricted".to_string()],
    };

    let config2 = config1.clone();
    assert_eq!(config1.isolation_level, config2.isolation_level);
    assert_eq!(config1.network_policies, config2.network_policies);
}

#[test]
fn test_team_security_config_serialization() {
    let config = TeamSecurityConfig {
        isolation_level: "high".to_string(),
        network_policies: vec!["strict".to_string()],
        volume_policies: vec!["no-mount".to_string()],
        resource_policies: vec!["quota".to_string()],
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: TeamSecurityConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.isolation_level, deserialized.isolation_level);
    assert_eq!(config.network_policies, deserialized.network_policies);
}

// ============================================================================
// TeamNetworkConfig Tests
// ============================================================================

#[test]
fn test_team_network_config_creation() {
    let config = TeamNetworkConfig {
        network_name: "team-network".to_string(),
        subnet_cidr: "10.0.0.0/24".to_string(),
        dns_config: None,
        load_balancer: None,
    };

    assert_eq!(config.network_name, "team-network");
    assert_eq!(config.subnet_cidr, "10.0.0.0/24");
    assert!(config.dns_config.is_none());
}

#[test]
fn test_team_network_config_clone() {
    let config1 = TeamNetworkConfig {
        network_name: "test-net".to_string(),
        subnet_cidr: "10.1.0.0/16".to_string(),
        dns_config: None,
        load_balancer: None,
    };

    let config2 = config1.clone();
    assert_eq!(config1.network_name, config2.network_name);
    assert_eq!(config1.subnet_cidr, config2.subnet_cidr);
}

#[test]
fn test_team_network_config_serialization() {
    let config = TeamNetworkConfig {
        network_name: "prod-network".to_string(),
        subnet_cidr: "172.16.0.0/12".to_string(),
        dns_config: None,
        load_balancer: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: TeamNetworkConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.network_name, deserialized.network_name);
    assert_eq!(config.subnet_cidr, deserialized.subnet_cidr);
}

// ============================================================================
// PortMapping Tests
// ============================================================================

#[test]
fn test_port_mapping_creation() {
    let mapping = PortMapping {
        container_port: 8080,
        host_port: Some(80),
        protocol: "TCP".to_string(),
    };

    assert_eq!(mapping.container_port, 8080);
    assert_eq!(mapping.host_port, Some(80));
    assert_eq!(mapping.protocol, "TCP");
}

#[test]
fn test_port_mapping_without_host_port() {
    let mapping = PortMapping {
        container_port: 3000,
        host_port: None,
        protocol: "TCP".to_string(),
    };

    assert!(mapping.host_port.is_none());
}

#[test]
fn test_port_mapping_udp_protocol() {
    let mapping = PortMapping {
        container_port: 53,
        host_port: Some(53),
        protocol: "UDP".to_string(),
    };

    assert_eq!(mapping.protocol, "UDP");
}

#[test]
fn test_port_mapping_clone() {
    let mapping1 = PortMapping {
        container_port: 443,
        host_port: Some(443),
        protocol: "TCP".to_string(),
    };

    let mapping2 = mapping1.clone();
    assert_eq!(mapping1.container_port, mapping2.container_port);
    assert_eq!(mapping1.host_port, mapping2.host_port);
}

#[test]
fn test_port_mapping_serialization() {
    let mapping = PortMapping {
        container_port: 5432,
        host_port: Some(5432),
        protocol: "TCP".to_string(),
    };

    let json = serde_json::to_string(&mapping).unwrap();
    let deserialized: PortMapping = serde_json::from_str(&json).unwrap();

    assert_eq!(mapping.container_port, deserialized.container_port);
    assert_eq!(mapping.protocol, deserialized.protocol);
}

// ============================================================================
// VolumeMount Tests
// ============================================================================

#[test]
fn test_volume_mount_creation() {
    let mount = VolumeMount {
        source: "/host/path".to_string(),
        target: "/container/path".to_string(),
        mount_type: "bind".to_string(),
        read_only: false,
    };

    assert_eq!(mount.source, "/host/path");
    assert_eq!(mount.target, "/container/path");
    assert_eq!(mount.mount_type, "bind");
    assert!(!mount.read_only);
}

#[test]
fn test_volume_mount_read_only() {
    let mount = VolumeMount {
        source: "/etc/config".to_string(),
        target: "/app/config".to_string(),
        mount_type: "bind".to_string(),
        read_only: true,
    };

    assert!(mount.read_only);
}

#[test]
fn test_volume_mount_clone() {
    let mount1 = VolumeMount {
        source: "/data".to_string(),
        target: "/app/data".to_string(),
        mount_type: "volume".to_string(),
        read_only: false,
    };

    let mount2 = mount1.clone();
    assert_eq!(mount1.source, mount2.source);
    assert_eq!(mount1.target, mount2.target);
    assert_eq!(mount1.mount_type, mount2.mount_type);
    assert_eq!(mount1.read_only, mount2.read_only);
}

#[test]
fn test_volume_mount_serialization() {
    let mount = VolumeMount {
        source: "/logs".to_string(),
        target: "/var/log".to_string(),
        mount_type: "tmpfs".to_string(),
        read_only: true,
    };

    let json = serde_json::to_string(&mount).unwrap();
    let deserialized: VolumeMount = serde_json::from_str(&json).unwrap();

    assert_eq!(mount.source, deserialized.source);
    assert_eq!(mount.target, deserialized.target);
    assert_eq!(mount.mount_type, deserialized.mount_type);
    assert_eq!(mount.read_only, deserialized.read_only);
}

// ============================================================================
// HealthCheck Tests
// ============================================================================

#[test]
fn test_health_check_http() {
    let health = HealthCheck {
        command: vec![
            "curl".to_string(),
            "-f".to_string(),
            "http://localhost:8080/health".to_string(),
        ],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };

    assert_eq!(health.interval, 30);
    assert_eq!(health.timeout, 5);
    assert_eq!(health.retries, 3);
}

#[test]
fn test_health_check_tcp() {
    let health = HealthCheck {
        command: vec![
            "nc".to_string(),
            "-z".to_string(),
            "localhost".to_string(),
            "5432".to_string(),
        ],
        interval: 10,
        timeout: 3,
        retries: 5,
        start_period: 0,
    };

    assert_eq!(health.interval, 10);
    assert_eq!(health.retries, 5);
}

#[test]
fn test_health_check_clone() {
    let health1 = HealthCheck {
        command: vec![
            "curl".to_string(),
            "-f".to_string(),
            "http://localhost:8080/ready".to_string(),
        ],
        interval: 15,
        timeout: 2,
        retries: 2,
        start_period: 0,
    };

    let health2 = health1.clone();
    assert_eq!(health1.command, health2.command);
    assert_eq!(health1.interval, health2.interval);
}

#[test]
fn test_health_check_serialization() {
    let health = HealthCheck {
        command: vec![
            "curl".to_string(),
            "-f".to_string(),
            "http://localhost:9000/healthz".to_string(),
        ],
        interval: 20,
        timeout: 4,
        retries: 3,
        start_period: 5,
    };

    let json = serde_json::to_string(&health).unwrap();
    let deserialized: HealthCheck = serde_json::from_str(&json).unwrap();

    assert_eq!(health.command, deserialized.command);
    assert_eq!(health.interval, deserialized.interval);
}
