//! Comprehensive tests for container runtime types

use std::collections::HashMap;
use std::time::Duration;
use toadstool::workload::RegistryAuth;
use toadstool_runtime_container::*;

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

#[test]
fn test_image_pull_policy_equality() {
    assert_eq!(ImagePullPolicy::Always, ImagePullPolicy::Always);
    assert_ne!(ImagePullPolicy::Always, ImagePullPolicy::Never);
}

#[test]
fn test_image_pull_policy_clone() {
    let policy1 = ImagePullPolicy::IfNotPresent;
    let policy2 = policy1.clone();
    assert_eq!(policy1, policy2);
}

// ============================================================================
// ContainerRuntimeConfig Tests
// ============================================================================

#[test]
fn test_container_runtime_config_default() {
    let config = ContainerRuntimeConfig::default();

    assert!(matches!(config.engine, ContainerEngineType::Docker { .. }));
    assert_eq!(config.registry_config.default_registry, "docker.io");
    assert_eq!(
        config.registry_config.pull_policy,
        ImagePullPolicy::IfNotPresent
    );
}

#[test]
fn test_container_runtime_config_clone() {
    let config1 = ContainerRuntimeConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.registry_config.default_registry,
        config2.registry_config.default_registry
    );
}

// ============================================================================
// RegistryConfig Tests
// ============================================================================

#[test]
fn test_registry_config_default() {
    let config = RegistryConfig::default();

    assert_eq!(config.default_registry, "docker.io");
    assert_eq!(config.pull_policy, ImagePullPolicy::IfNotPresent);
    assert_eq!(config.pull_timeout, Duration::from_secs(300));
    assert!(config.registries.is_empty());
}

#[test]
fn test_registry_config_clone() {
    let config1 = RegistryConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.default_registry, config2.default_registry);
    assert_eq!(config1.pull_policy, config2.pull_policy);
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
}

#[test]
fn test_network_policy_port_ranges() {
    let policy = NetworkPolicy::default();

    // Default ranges: 8000-8999 and 3000-3999
    assert_eq!(policy.allowed_port_ranges[0].start, 8000);
    assert_eq!(policy.allowed_port_ranges[0].end, 8999);
    assert_eq!(policy.allowed_port_ranges[1].start, 3000);
    assert_eq!(policy.allowed_port_ranges[1].end, 3999);
}

#[test]
fn test_network_policy_clone() {
    let policy1 = NetworkPolicy::default();
    let policy2 = policy1.clone();

    assert_eq!(policy1.allow_custom_networks, policy2.allow_custom_networks);
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
    let mode = NetworkMode::Custom("custom-network".to_string());

    match mode {
        NetworkMode::Custom(name) => {
            assert_eq!(name, "custom-network");
        }
        _ => panic!("Expected Custom network mode"),
    }
}

#[test]
fn test_network_mode_clone() {
    let mode1 = NetworkMode::Bridge;
    let mode2 = mode1.clone();

    match (mode1, mode2) {
        (NetworkMode::Bridge, NetworkMode::Bridge) => {} // OK - clone works
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// DnsConfig Tests
// ============================================================================

#[test]
fn test_dns_config_default() {
    let config = DnsConfig::default();

    assert_eq!(config.nameservers.len(), 2);
    assert_eq!(config.nameservers[0], "8.8.8.8");
    assert_eq!(config.nameservers[1], "8.8.4.4");
    assert!(config.search_domains.is_empty());
    assert!(config.options.is_empty());
}

#[test]
fn test_dns_config_clone() {
    let config1 = DnsConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.nameservers, config2.nameservers);
}

// ============================================================================
// VolumePolicy Tests
// ============================================================================

#[test]
fn test_volume_policy_default() {
    let policy = VolumePolicy::default();

    assert!(!policy.allow_bind_mounts);
    assert!(policy.allow_tmpfs);
    assert_eq!(policy.max_volume_size_mb, 1024);
    assert_eq!(policy.allowed_host_paths.len(), 1);
}

#[test]
fn test_volume_policy_clone() {
    let policy1 = VolumePolicy::default();
    let policy2 = policy1.clone();

    assert_eq!(policy1.allow_bind_mounts, policy2.allow_bind_mounts);
    assert_eq!(policy1.max_volume_size_mb, policy2.max_volume_size_mb);
}

// ============================================================================
// ContainerSecurityConfig Tests
// ============================================================================

#[test]
fn test_container_security_config_default() {
    let config = ContainerSecurityConfig::default();

    assert!(config.non_root_required);
    assert!(config.drop_all_capabilities);
    assert!(!config.read_only_root_fs);
    assert!(config.allowed_capabilities.is_empty());
    assert_eq!(config.security_opts.len(), 1);
    assert_eq!(config.security_opts[0], "no-new-privileges:true");
}

#[test]
fn test_container_security_config_clone() {
    let config1 = ContainerSecurityConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.non_root_required, config2.non_root_required);
    assert_eq!(config1.drop_all_capabilities, config2.drop_all_capabilities);
}

// ============================================================================
// ContainerResourceLimits Tests
// ============================================================================

#[test]
fn test_container_resource_limits_default() {
    let limits = ContainerResourceLimits::default();

    assert_eq!(limits.max_memory_bytes, 512 * 1024 * 1024); // 512 MB
    assert_eq!(limits.max_cpu_millicores, 1000); // 1 CPU core
    assert_eq!(limits.max_execution_time, Duration::from_secs(3600));
    assert_eq!(limits.max_io_bps, 100 * 1024 * 1024); // 100 MB/s
}

#[test]
fn test_container_resource_limits_memory() {
    let limits = ContainerResourceLimits::default();
    assert_eq!(limits.max_memory_bytes, 536_870_912);
}

#[test]
fn test_container_resource_limits_cpu_millicores() {
    let limits = ContainerResourceLimits::default();
    assert_eq!(limits.max_cpu_millicores, 1000);
}

#[test]
fn test_container_resource_limits_clone() {
    let limits1 = ContainerResourceLimits::default();
    let limits2 = limits1.clone();

    assert_eq!(limits1.max_memory_bytes, limits2.max_memory_bytes);
    assert_eq!(limits1.max_cpu_millicores, limits2.max_cpu_millicores);
}

// ============================================================================
// ImageConfig Tests
// ============================================================================

#[test]
fn test_image_config_default() {
    let config = ImageConfig::default();

    assert!(config.cache_enabled);
    assert_eq!(config.max_cache_size_mb, 5120); // 5 GB
    assert_eq!(config.cleanup_interval, Duration::from_secs(3600));
    assert!(config.cache_dir.is_none());
}

#[test]
fn test_image_config_clone() {
    let config1 = ImageConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.cache_enabled, config2.cache_enabled);
    assert_eq!(config1.max_cache_size_mb, config2.max_cache_size_mb);
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
fn test_port_range_clone() {
    let range1 = PortRange {
        start: 3000,
        end: 3999,
    };

    let range2 = range1.clone();

    assert_eq!(range1.start, range2.start);
    assert_eq!(range1.end, range2.end);
}

// ============================================================================
// ContainerEngineType Tests
// ============================================================================

#[test]
fn test_container_engine_type_docker() {
    let engine = ContainerEngineType::Docker {
        socket_path: None,
        api_version: "1.41".to_string(),
    };

    match engine {
        ContainerEngineType::Docker {
            socket_path,
            api_version,
        } => {
            assert!(socket_path.is_none());
            assert_eq!(api_version, "1.41");
        }
        _ => panic!("Expected Docker engine"),
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
        _ => panic!("Expected Containerd engine"),
    }
}

#[test]
fn test_container_engine_type_podman() {
    let engine = ContainerEngineType::Podman {
        socket_path: "/run/podman/podman.sock".to_string(),
        remote_url: Some("ssh://user@host".to_string()),
    };

    match engine {
        ContainerEngineType::Podman {
            socket_path,
            remote_url,
        } => {
            assert_eq!(socket_path, "/run/podman/podman.sock");
            assert_eq!(remote_url, Some("ssh://user@host".to_string()));
        }
        _ => panic!("Expected Podman engine"),
    }
}

#[test]
fn test_container_engine_type_default() {
    let engine = ContainerEngineType::default();

    assert!(matches!(engine, ContainerEngineType::Docker { .. }));
}

#[test]
fn test_container_engine_type_clone() {
    let engine1 = ContainerEngineType::default();
    let engine2 = engine1.clone();

    match (engine1, engine2) {
        (ContainerEngineType::Docker { .. }, ContainerEngineType::Docker { .. }) => {} // OK - clone works
        _ => panic!("Clone failed"),
    }
}

// ============================================================================
// Additional Comprehensive Tests (Session 19)
// ============================================================================

#[test]
fn test_port_range_single_port() {
    let port_range = PortRange {
        start: 8080,
        end: 8080,
    };

    assert_eq!(port_range.start, port_range.end);
}

#[test]
fn test_port_range_wide() {
    let port_range = PortRange {
        start: 1024,
        end: 65535,
    };

    assert!(port_range.end > port_range.start);
    assert!(port_range.end - port_range.start > 60000);
}

#[test]
fn test_network_mode_custom_empty() {
    let mode = NetworkMode::Custom(String::new());

    match mode {
        NetworkMode::Custom(name) => assert!(name.is_empty()),
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_network_policy_default_ports() {
    let policy = NetworkPolicy::default();

    assert_eq!(policy.allowed_port_ranges.len(), 2);
    assert_eq!(policy.allowed_port_ranges[0].start, 8000);
    assert_eq!(policy.allowed_port_ranges[0].end, 8999);
}

#[test]
fn test_network_policy_custom_networks_disabled() {
    let policy = NetworkPolicy::default();
    assert!(!policy.allow_custom_networks);
}

#[test]
fn test_network_policy_custom_networks_enabled() {
    let policy = NetworkPolicy {
        allow_custom_networks: true,
        ..Default::default()
    };

    assert!(policy.allow_custom_networks);
}

#[test]
fn test_network_policy_custom_ports() {
    let policy = NetworkPolicy {
        default_network: NetworkMode::Bridge,
        allow_custom_networks: true,
        allowed_port_ranges: vec![
            PortRange { start: 80, end: 80 },
            PortRange {
                start: 443,
                end: 443,
            },
        ],
        dns_config: DnsConfig::default(),
    };

    assert_eq!(policy.allowed_port_ranges.len(), 2);
    assert_eq!(policy.allowed_port_ranges[0].start, 80);
}

#[test]
fn test_registry_config_custom_timeout() {
    let config = RegistryConfig {
        default_registry: "ghcr.io".to_string(),
        registries: HashMap::new(),
        pull_policy: ImagePullPolicy::Always,
        pull_timeout: Duration::from_secs(600),
    };

    assert_eq!(config.pull_timeout, Duration::from_secs(600));
}

#[test]
fn test_registry_config_multiple_registries() {
    let mut registries = HashMap::new();
    registries.insert(
        "docker.io".to_string(),
        RegistryAuth {
            server_url: "https://docker.io".to_string(),
            username: "user1".to_string(),
            password: "pass1".to_string(),
        },
    );
    registries.insert(
        "ghcr.io".to_string(),
        RegistryAuth {
            server_url: "https://ghcr.io".to_string(),
            username: "user2".to_string(),
            password: "pass2".to_string(),
        },
    );

    let config = RegistryConfig {
        default_registry: "docker.io".to_string(),
        registries: registries.clone(),
        pull_policy: ImagePullPolicy::IfNotPresent,
        pull_timeout: Duration::from_secs(300),
    };

    assert_eq!(config.registries.len(), 2);
    assert!(config.registries.contains_key("docker.io"));
    assert!(config.registries.contains_key("ghcr.io"));
}

#[test]
fn test_image_pull_policy_serialization() {
    let policy = ImagePullPolicy::Always;
    let json = serde_json::to_string(&policy).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_image_pull_policy_deserialization() {
    let json = "\"Always\"";
    let policy: ImagePullPolicy = serde_json::from_str(json).unwrap();
    assert_eq!(policy, ImagePullPolicy::Always);
}

#[test]
fn test_container_engine_docker_custom_socket() {
    let engine = ContainerEngineType::Docker {
        socket_path: Some("/custom/docker.sock".to_string()),
        api_version: "1.41".to_string(),
    };

    match engine {
        ContainerEngineType::Docker {
            socket_path,
            api_version,
        } => {
            assert_eq!(socket_path, Some("/custom/docker.sock".to_string()));
            assert_eq!(api_version, "1.41");
        }
        _ => panic!("Expected Docker variant"),
    }
}

#[test]
fn test_container_engine_docker_default_socket() {
    let engine = ContainerEngineType::Docker {
        socket_path: None,
        api_version: "1.40".to_string(),
    };

    match engine {
        ContainerEngineType::Docker { socket_path, .. } => {
            assert!(socket_path.is_none());
        }
        _ => panic!("Expected Docker variant"),
    }
}

#[test]
fn test_container_runtime_config_serialization() {
    let config = ContainerRuntimeConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("registry_config"));
}

#[test]
fn test_container_runtime_config_custom_registry() {
    let mut config = ContainerRuntimeConfig::default();
    config.registry_config.default_registry = "custom.io".to_string();

    assert_eq!(config.registry_config.default_registry, "custom.io");
}

#[test]
fn test_container_runtime_config_pull_policy_always() {
    let mut config = ContainerRuntimeConfig::default();
    config.registry_config.pull_policy = ImagePullPolicy::Always;

    assert_eq!(config.registry_config.pull_policy, ImagePullPolicy::Always);
}

#[test]
fn test_container_runtime_config_pull_policy_never() {
    let mut config = ContainerRuntimeConfig::default();
    config.registry_config.pull_policy = ImagePullPolicy::Never;

    assert_eq!(config.registry_config.pull_policy, ImagePullPolicy::Never);
}

#[test]
fn test_port_range_serialization() {
    let range = PortRange {
        start: 8080,
        end: 8090,
    };

    let json = serde_json::to_string(&range).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_network_mode_serialization() {
    let mode = NetworkMode::Bridge;
    let json = serde_json::to_string(&mode).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_network_policy_serialization() {
    let policy = NetworkPolicy::default();
    let json = serde_json::to_string(&policy).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_registry_config_empty_registries() {
    let config = RegistryConfig::default();
    assert!(config.registries.is_empty());
}

#[test]
fn test_registry_config_serialization() {
    let config = RegistryConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_registry_auth_creation() {
    let auth = RegistryAuth {
        server_url: "https://registry.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };

    assert_eq!(auth.username, "testuser");
    assert_eq!(auth.password, "testpass");
    assert_eq!(auth.server_url, "https://registry.example.com");
}

#[test]
fn test_registry_auth_empty_credentials() {
    let auth = RegistryAuth {
        server_url: String::new(),
        username: String::new(),
        password: String::new(),
    };

    assert!(auth.username.is_empty());
    assert!(auth.password.is_empty());
    assert!(auth.server_url.is_empty());
}
