// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Container Runtime Configuration

use std::path::PathBuf;
use std::time::Duration;
use toadstool_runtime_container::*;

// ============================================================================
// ContainerEngineType Tests
// ============================================================================

#[test]
fn test_container_engine_type_docker_default() {
    let engine = ContainerEngineType::default();

    match engine {
        ContainerEngineType::Docker {
            socket_path,
            api_version,
        } => {
            assert!(socket_path.is_none());
            assert!(!api_version.is_empty());
        }
        _ => panic!("Expected Docker engine type"),
    }
}

#[test]
fn test_container_engine_type_docker_custom() {
    let engine = ContainerEngineType::Docker {
        socket_path: Some("/var/run/docker.sock".to_string()),
        api_version: "1.41".to_string(),
    };

    match engine {
        ContainerEngineType::Docker {
            socket_path,
            api_version,
        } => {
            assert_eq!(socket_path.unwrap(), "/var/run/docker.sock");
            assert_eq!(api_version, "1.41");
        }
        _ => panic!("Expected Docker engine type"),
    }
}

#[test]
fn test_container_engine_type_containerd() {
    let engine = ContainerEngineType::Containerd {
        address: "/run/containerd/containerd.sock".to_string(),
        namespace: "default".to_string(),
    };

    match engine {
        ContainerEngineType::Containerd { address, namespace } => {
            assert_eq!(address, "/run/containerd/containerd.sock");
            assert_eq!(namespace, "default");
        }
        _ => panic!("Expected Containerd engine type"),
    }
}

#[test]
fn test_container_engine_type_podman() {
    let engine = ContainerEngineType::Podman {
        socket_path: "/run/podman/podman.sock".to_string(),
        remote_url: Some("ssh://user@host/run/podman/podman.sock".to_string()),
    };

    match engine {
        ContainerEngineType::Podman {
            socket_path,
            remote_url,
        } => {
            assert_eq!(socket_path, "/run/podman/podman.sock");
            assert!(remote_url.is_some());
        }
        _ => panic!("Expected Podman engine type"),
    }
}

// ============================================================================
// RegistryConfig Tests
// ============================================================================

#[test]
fn test_registry_config_default() {
    let config = RegistryConfig::default();

    assert_eq!(config.default_registry, "docker.io");
    assert!(config.registries.is_empty());
    assert!(matches!(config.pull_policy, ImagePullPolicy::IfNotPresent));
    assert_eq!(config.pull_timeout, Duration::from_secs(300));
}

#[test]
fn test_registry_config_custom_registry() {
    let config = RegistryConfig {
        default_registry: "ghcr.io".to_string(),
        ..Default::default()
    };

    assert_eq!(config.default_registry, "ghcr.io");
}

#[test]
fn test_registry_config_pull_policy_always() {
    let config = RegistryConfig {
        pull_policy: ImagePullPolicy::Always,
        ..Default::default()
    };

    assert!(matches!(config.pull_policy, ImagePullPolicy::Always));
}

#[test]
fn test_registry_config_pull_policy_never() {
    let config = RegistryConfig {
        pull_policy: ImagePullPolicy::Never,
        ..Default::default()
    };

    assert!(matches!(config.pull_policy, ImagePullPolicy::Never));
}

// ============================================================================
// NetworkPolicy Tests
// ============================================================================

#[test]
fn test_network_policy_default() {
    let policy = NetworkPolicy::default();

    assert!(matches!(policy.default_network, NetworkMode::Bridge));
    assert!(!policy.allow_custom_networks);
    assert_eq!(policy.allowed_port_ranges.len(), 2);
    // DNS defaults to empty — capability-based, resolved from host at runtime
    assert!(policy.dns_config.nameservers.is_empty());
}

#[test]
fn test_network_policy_host_network() {
    let policy = NetworkPolicy {
        default_network: NetworkMode::Host,
        ..Default::default()
    };

    assert!(matches!(policy.default_network, NetworkMode::Host));
}

#[test]
fn test_network_policy_no_network() {
    let policy = NetworkPolicy {
        default_network: NetworkMode::None,
        ..Default::default()
    };

    assert!(matches!(policy.default_network, NetworkMode::None));
}

#[test]
fn test_network_policy_custom_network() {
    let policy = NetworkPolicy {
        default_network: NetworkMode::Custom("my-network".to_string()),
        ..Default::default()
    };

    match policy.default_network {
        NetworkMode::Custom(name) => assert_eq!(name, "my-network"),
        _ => panic!("Expected custom network"),
    }
}

#[test]
fn test_network_policy_allow_custom_networks() {
    let policy = NetworkPolicy {
        allow_custom_networks: true,
        ..Default::default()
    };

    assert!(policy.allow_custom_networks);
}

// ============================================================================
// VolumePolicy Tests
// ============================================================================

#[test]
fn test_volume_policy_default() {
    let policy = VolumePolicy::default();

    assert!(!policy.allow_bind_mounts);
    assert_eq!(policy.allowed_host_paths.len(), 1);
    assert!(policy.allow_tmpfs);
    assert_eq!(policy.max_volume_size_mb, 1024);
}

#[test]
fn test_volume_policy_allow_bind_mounts() {
    let policy = VolumePolicy {
        allow_bind_mounts: true,
        ..Default::default()
    };

    assert!(policy.allow_bind_mounts);
}

#[test]
fn test_volume_policy_disable_tmpfs() {
    let policy = VolumePolicy {
        allow_tmpfs: false,
        ..Default::default()
    };

    assert!(!policy.allow_tmpfs);
}

#[test]
fn test_volume_policy_custom_max_size() {
    let policy = VolumePolicy {
        max_volume_size_mb: 2048,
        ..Default::default()
    };

    assert_eq!(policy.max_volume_size_mb, 2048);
}

#[test]
fn test_volume_policy_custom_allowed_paths() {
    let policy = VolumePolicy {
        allowed_host_paths: vec![PathBuf::from("/data"), PathBuf::from("/logs")],
        ..Default::default()
    };

    assert_eq!(policy.allowed_host_paths.len(), 2);
}

// ============================================================================
// ContainerSecurityConfig Tests
// ============================================================================

#[test]
fn test_container_security_config_default() {
    let config = ContainerSecurityConfig::default();

    assert!(config.non_root_required);
    assert!(config.drop_all_capabilities);
    assert!(config.allowed_capabilities.is_empty());
    assert_eq!(config.security_opts.len(), 1);
    assert!(!config.read_only_root_fs);
}

#[test]
fn test_container_security_config_allow_root() {
    let config = ContainerSecurityConfig {
        non_root_required: false,
        ..Default::default()
    };

    assert!(!config.non_root_required);
}

#[test]
fn test_container_security_config_allow_capabilities() {
    let config = ContainerSecurityConfig {
        drop_all_capabilities: false,
        allowed_capabilities: vec!["NET_BIND_SERVICE".to_string()],
        ..Default::default()
    };

    assert!(!config.drop_all_capabilities);
    assert_eq!(config.allowed_capabilities.len(), 1);
}

#[test]
fn test_container_security_config_read_only_rootfs() {
    let config = ContainerSecurityConfig {
        read_only_root_fs: true,
        ..Default::default()
    };

    assert!(config.read_only_root_fs);
}

#[test]
fn test_container_security_config_custom_security_opts() {
    let config = ContainerSecurityConfig {
        security_opts: vec![
            "seccomp=unconfined".to_string(),
            "apparmor=docker-default".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(config.security_opts.len(), 2);
}

// ============================================================================
// ContainerResourceLimits Tests
// ============================================================================

#[test]
fn test_container_resource_limits_default() {
    let limits = ContainerResourceLimits::default();

    assert_eq!(limits.max_memory_bytes, 512 * 1024 * 1024);
    assert_eq!(limits.max_cpu_millicores, 1000);
    assert_eq!(limits.max_execution_time, Duration::from_secs(3600));
    assert_eq!(limits.max_io_bps, 100 * 1024 * 1024);
}

#[test]
fn test_container_resource_limits_custom_memory() {
    let limits = ContainerResourceLimits {
        max_memory_bytes: 1024 * 1024 * 1024, // 1 GB
        ..Default::default()
    };

    assert_eq!(limits.max_memory_bytes, 1024 * 1024 * 1024);
}

#[test]
fn test_container_resource_limits_custom_cpu() {
    let limits = ContainerResourceLimits {
        max_cpu_millicores: 2000, // 2 cores
        ..Default::default()
    };

    assert_eq!(limits.max_cpu_millicores, 2000);
}

#[test]
fn test_container_resource_limits_high_io() {
    let limits = ContainerResourceLimits {
        max_io_bps: 500 * 1024 * 1024, // 500 MB/s
        ..Default::default()
    };

    assert_eq!(limits.max_io_bps, 500 * 1024 * 1024);
}

// ============================================================================
// ImageConfig Tests
// ============================================================================

#[test]
fn test_image_config_default() {
    let config = ImageConfig::default();

    assert!(config.cache_enabled);
    assert!(config.cache_dir.is_none());
    assert_eq!(config.max_cache_size_mb, 5120);
    assert_eq!(config.cleanup_interval, Duration::from_secs(3600));
}

#[test]
fn test_image_config_disable_cache() {
    let config = ImageConfig {
        cache_enabled: false,
        ..Default::default()
    };

    assert!(!config.cache_enabled);
}

#[test]
fn test_image_config_custom_cache_dir() {
    let config = ImageConfig {
        cache_dir: Some(PathBuf::from("/var/cache/containers")),
        ..Default::default()
    };

    assert_eq!(
        config.cache_dir.unwrap(),
        PathBuf::from("/var/cache/containers")
    );
}

#[test]
fn test_image_config_large_cache() {
    let config = ImageConfig {
        max_cache_size_mb: 20480, // 20 GB
        ..Default::default()
    };

    assert_eq!(config.max_cache_size_mb, 20480);
}

// ============================================================================
// ContainerRuntimeConfig Tests
// ============================================================================

#[test]
fn test_container_runtime_config_default() {
    let config = ContainerRuntimeConfig::default();

    match config.engine {
        ContainerEngineType::Docker { .. } => {}
        _ => panic!("Expected Docker engine"),
    }

    assert_eq!(config.registry_config.default_registry, "docker.io");
    assert!(matches!(
        config.network_policy.default_network,
        NetworkMode::Bridge
    ));
    assert!(!config.volume_policy.allow_bind_mounts);
    assert!(config.security_config.non_root_required);
    assert!(config.image_config.cache_enabled);
}

#[test]
fn test_container_runtime_config_with_containerd() {
    let config = ContainerRuntimeConfig {
        engine: ContainerEngineType::Containerd {
            address: "/run/containerd/containerd.sock".to_string(),
            namespace: "k8s.io".to_string(),
        },
        ..Default::default()
    };

    match config.engine {
        ContainerEngineType::Containerd { .. } => {}
        _ => panic!("Expected Containerd engine"),
    }
}

#[test]
fn test_container_runtime_config_with_podman() {
    let config = ContainerRuntimeConfig {
        engine: ContainerEngineType::Podman {
            socket_path: "/run/podman/podman.sock".to_string(),
            remote_url: None,
        },
        ..Default::default()
    };

    match config.engine {
        ContainerEngineType::Podman { .. } => {}
        _ => panic!("Expected Podman engine"),
    }
}

#[test]
fn test_container_runtime_config_serialization() {
    let config = ContainerRuntimeConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    assert!(json.contains("engine"));
    assert!(json.contains("registry_config"));
}

#[test]
fn test_container_runtime_config_deserialization() {
    let config = ContainerRuntimeConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: ContainerRuntimeConfig = serde_json::from_str(&json).unwrap();

    match deserialized.engine {
        ContainerEngineType::Docker { .. } => {}
        _ => panic!("Expected Docker engine"),
    }
}

// ============================================================================
// DNS Config Tests
// ============================================================================

#[test]
fn test_dns_config_default() {
    let config = DnsConfig::default();

    // Capability-based: no hardcoded nameservers — host/orchestrator provides them
    assert!(config.nameservers.is_empty());
    assert!(config.search_domains.is_empty());
    assert!(config.options.is_empty());
}

#[test]
fn test_dns_config_custom_nameservers() {
    let config = DnsConfig {
        nameservers: vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()],
        ..Default::default()
    };

    assert_eq!(config.nameservers.len(), 2);
    assert_eq!(config.nameservers[0], "1.1.1.1");
}

#[test]
fn test_dns_config_with_search_domains() {
    let config = DnsConfig {
        search_domains: vec!["example.com".to_string(), "local".to_string()],
        ..Default::default()
    };

    assert_eq!(config.search_domains.len(), 2);
}

// ============================================================================
// PortRange Tests
// ============================================================================

#[test]
fn test_port_range_creation() {
    let range = PortRange {
        start: 8000,
        end: 8999,
    };

    assert_eq!(range.start, 8000);
    assert_eq!(range.end, 8999);
}

#[test]
fn test_port_range_debug() {
    let range = PortRange {
        start: 3000,
        end: 3999,
    };

    let debug_str = format!("{range:?}");
    assert!(debug_str.contains("3000"));
    assert!(debug_str.contains("3999"));
}

// ============================================================================
// NetworkMode Tests
// ============================================================================

#[test]
fn test_network_mode_bridge() {
    let mode = NetworkMode::Bridge;
    assert!(matches!(mode, NetworkMode::Bridge));
}

#[test]
fn test_network_mode_host() {
    let mode = NetworkMode::Host;
    assert!(matches!(mode, NetworkMode::Host));
}

#[test]
fn test_network_mode_none() {
    let mode = NetworkMode::None;
    assert!(matches!(mode, NetworkMode::None));
}

#[test]
fn test_network_mode_custom() {
    let mode = NetworkMode::Custom("overlay".to_string());
    match mode {
        NetworkMode::Custom(name) => assert_eq!(name, "overlay"),
        _ => panic!("Expected custom network mode"),
    }
}

// ============================================================================
// ImagePullPolicy Tests
// ============================================================================

#[test]
fn test_image_pull_policy_always() {
    let policy = ImagePullPolicy::Always;
    assert_eq!(policy, ImagePullPolicy::Always);
}

#[test]
fn test_image_pull_policy_if_not_present() {
    let policy = ImagePullPolicy::IfNotPresent;
    assert_eq!(policy, ImagePullPolicy::IfNotPresent);
}

#[test]
fn test_image_pull_policy_never() {
    let policy = ImagePullPolicy::Never;
    assert_eq!(policy, ImagePullPolicy::Never);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_secure_config_scenario() {
    let mut config = ContainerRuntimeConfig::default();

    config.security_config.non_root_required = true;
    config.security_config.read_only_root_fs = true;
    config.security_config.drop_all_capabilities = true;

    config.network_policy.default_network = NetworkMode::None;
    config.volume_policy.allow_bind_mounts = false;

    assert!(config.security_config.non_root_required);
    assert!(config.security_config.read_only_root_fs);
    assert!(matches!(
        config.network_policy.default_network,
        NetworkMode::None
    ));
}

#[test]
fn test_resource_constrained_config() {
    let mut config = ContainerRuntimeConfig::default();

    config.resource_limits.max_memory_bytes = 128 * 1024 * 1024; // 128 MB
    config.resource_limits.max_cpu_millicores = 250; // 0.25 cores
    config.resource_limits.max_execution_time = Duration::from_secs(600);

    assert_eq!(config.resource_limits.max_memory_bytes, 128 * 1024 * 1024);
    assert_eq!(config.resource_limits.max_cpu_millicores, 250);
}

#[test]
fn test_high_performance_config() {
    let mut config = ContainerRuntimeConfig::default();

    config.resource_limits.max_memory_bytes = 16 * 1024 * 1024 * 1024; // 16 GB
    config.resource_limits.max_cpu_millicores = 8000; // 8 cores
    config.resource_limits.max_io_bps = 1024 * 1024 * 1024; // 1 GB/s

    config.network_policy.default_network = NetworkMode::Host;
    config.volume_policy.allow_bind_mounts = true;

    assert_eq!(
        config.resource_limits.max_memory_bytes,
        16 * 1024 * 1024 * 1024
    );
    assert!(config.volume_policy.allow_bind_mounts);
}
