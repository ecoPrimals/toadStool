// ============================================================================
// Security Violation Tests
// ============================================================================

#[test]
fn test_violation_severity_low() {
    let severity = ViolationSeverity::Low;
    assert!(matches!(severity, ViolationSeverity::Low));
}

#[test]
fn test_violation_severity_medium() {
    let severity = ViolationSeverity::Medium;
    assert!(matches!(severity, ViolationSeverity::Medium));
}

#[test]
fn test_violation_severity_high() {
    let severity = ViolationSeverity::High;
    assert!(matches!(severity, ViolationSeverity::High));
}

#[test]
fn test_violation_severity_critical() {
    let severity = ViolationSeverity::Critical;
    assert!(matches!(severity, ViolationSeverity::Critical));
}

// ============================================================================
// Resource Usage Tests
// ============================================================================

#[test]
fn test_resource_usage_default() {
    let usage = ResourceUsage::default();

    assert_eq!(usage.memory_bytes, 0);
    assert_eq!(usage.cpu_percent, 0.0);
    assert_eq!(usage.file_descriptors, 0);
    assert_eq!(usage.processes, 0);
}

#[test]
fn test_resource_usage_memory() {
    let usage = ResourceUsage {
        memory_bytes: 256 * 1024 * 1024, // 256MB
        ..ResourceUsage::default()
    };

    assert_eq!(usage.memory_bytes, 256 * 1024 * 1024);
}

#[test]
fn test_resource_usage_cpu() {
    let usage = ResourceUsage {
        cpu_percent: 45.5,
        ..ResourceUsage::default()
    };

    assert_eq!(usage.cpu_percent, 45.5);
}

#[test]
fn test_resource_usage_file_descriptors() {
    let usage = ResourceUsage {
        file_descriptors: 250,
        ..ResourceUsage::default()
    };

    assert_eq!(usage.file_descriptors, 250);
}

#[test]
fn test_resource_usage_processes() {
    let usage = ResourceUsage {
        processes: 15,
        ..ResourceUsage::default()
    };

    assert_eq!(usage.processes, 15);
}

// ============================================================================
// Mount Type Tests
// ============================================================================

#[test]
fn test_mount_type_readonly_bind() {
    let mount_type = MountType::ReadOnlyBind;
    assert!(matches!(mount_type, MountType::ReadOnlyBind));
}

#[test]
fn test_mount_type_readwrite_bind() {
    let mount_type = MountType::ReadWriteBind;
    assert!(matches!(mount_type, MountType::ReadWriteBind));
}

#[test]
fn test_mount_type_tmpfs() {
    let mount_type = MountType::TmpFs;
    assert!(matches!(mount_type, MountType::TmpFs));
}

#[test]
fn test_mount_type_device() {
    let mount_type = MountType::Device;
    assert!(matches!(mount_type, MountType::Device));
}

#[test]
fn test_mount_type_proc() {
    let mount_type = MountType::Proc;
    assert!(matches!(mount_type, MountType::Proc));
}

// ============================================================================
// SandboxLifetime Tests (NEW)
// ============================================================================

#[test]
fn test_sandbox_lifetime_ephemeral() {
    let lifetime = SandboxLifetime::Ephemeral;
    assert!(matches!(lifetime, SandboxLifetime::Ephemeral));
}

#[test]
fn test_sandbox_lifetime_persistent() {
    let ttl = Duration::from_secs(3600); // 1 hour
    let lifetime = SandboxLifetime::Persistent { ttl };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert_eq!(ttl, Duration::from_secs(3600));
        }
        _ => panic!("Expected Persistent variant"),
    }
}

#[test]
fn test_sandbox_lifetime_persistent_short() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(60), // 1 minute
    };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert!(ttl < Duration::from_secs(120));
        }
        _ => panic!("Expected Persistent"),
    }
}

#[test]
fn test_sandbox_lifetime_persistent_long() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(86400), // 24 hours
    };

    match lifetime {
        SandboxLifetime::Persistent { ttl } => {
            assert!(ttl > Duration::from_secs(3600));
        }
        _ => panic!("Expected Persistent"),
    }
}

#[test]
fn test_sandbox_lifetime_manual() {
    let lifetime = SandboxLifetime::Manual;
    assert!(matches!(lifetime, SandboxLifetime::Manual));
}

#[test]
fn test_sandbox_lifetime_clone() {
    let lifetime1 = SandboxLifetime::Ephemeral;
    let lifetime2 = lifetime1.clone();
    assert!(matches!(lifetime2, SandboxLifetime::Ephemeral));
}

// ============================================================================
// FilesystemMount Tests (NEW - Additional)
// ============================================================================

#[test]
fn test_filesystem_mount_with_multiple_options() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec!["ro".to_string(), "noexec".to_string(), "nosuid".to_string()],
    };

    assert_eq!(mount.options.len(), 3);
    assert!(mount.options.contains(&"noexec".to_string()));
}

#[test]
fn test_filesystem_mount_clone() {
    let mount1 = FilesystemMount {
        source: PathBuf::from("/test"),
        target: PathBuf::from("/sandbox/test"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    let mount2 = mount1.clone();
    assert_eq!(mount1.source, mount2.source);
    assert_eq!(mount1.target, mount2.target);
}

// ============================================================================
// ResourceLimits Edge Cases (NEW)
// ============================================================================

#[test]
fn test_resource_limits_no_memory_limit() {
    let limits = ResourceLimits {
        max_memory_bytes: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_memory_bytes.is_none());
}

#[test]
fn test_resource_limits_no_cpu_limit() {
    let limits = ResourceLimits {
        max_cpu_percent: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_cpu_percent.is_none());
}

#[test]
fn test_resource_limits_very_high_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(64 * 1024 * 1024 * 1024), // 64GB
        ..ResourceLimits::default()
    };

    assert!(limits.max_memory_bytes.unwrap() > 1024 * 1024 * 1024);
}

#[test]
fn test_resource_limits_very_low_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(10.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(10.0));
}

#[test]
fn test_resource_limits_max_execution_time() {
    let limits = ResourceLimits {
        max_execution_time: Some(Duration::from_secs(600)), // 10 minutes
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_some());
}

#[test]
fn test_resource_limits_no_execution_time_limit() {
    let limits = ResourceLimits {
        max_execution_time: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_none());
}

#[test]
fn test_resource_limits_network_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(100 * 1024 * 1024), // 100 MB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(100 * 1024 * 1024));
}

#[test]
fn test_resource_limits_all_none() {
    let limits = ResourceLimits {
        max_memory_bytes: None,
        max_cpu_percent: None,
        max_file_descriptors: None,
        max_processes: None,
        max_disk_bytes: None,
        max_network_bps: None,
        max_execution_time: None,
    };

    assert!(limits.max_memory_bytes.is_none());
    assert!(limits.max_cpu_percent.is_none());
    assert!(limits.max_file_descriptors.is_none());
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits::default();
    let json = serde_json::to_string(&limits).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// SandboxConfig Edge Cases (NEW)
// ============================================================================

#[test]
fn test_sandbox_config_disabled_all_features() {
    let config = SandboxConfig {
        advanced_features_enabled: false,
        enable_seccomp: false,
        enable_capability_dropping: false,
        enable_namespace_isolation: false,
        enable_resource_limits: false,
        enable_monitoring: false,
        ..SandboxConfig::default()
    };

    assert!(!config.advanced_features_enabled);
    assert!(!config.enable_seccomp);
    assert!(!config.enable_capability_dropping);
}

#[test]
fn test_sandbox_config_very_high_concurrency() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1000);
}

#[test]
fn test_sandbox_config_very_low_concurrency() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1);
}

#[test]
fn test_sandbox_config_long_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 300, // 5 minutes
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 300);
}

#[test]
fn test_sandbox_config_short_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 5, // 5 seconds
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 5);
}

#[test]
fn test_sandbox_config_fast_monitoring() {
    let config = SandboxConfig {
        monitoring_interval_ms: 100, // 100ms
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 100);
}

#[test]
fn test_sandbox_config_slow_monitoring() {
    let config = SandboxConfig {
        monitoring_interval_ms: 10000, // 10 seconds
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 10000);
}

#[test]
fn test_sandbox_config_isolation_level_minimum() {
    let config = SandboxConfig {
        default_isolation_level: toadstool::security::IsolationLevel::None,
        ..SandboxConfig::default()
    };

    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::None
    ));
}

#[test]
fn test_sandbox_config_isolation_level_enhanced() {
    let config = SandboxConfig {
        default_isolation_level: toadstool::security::IsolationLevel::Enhanced,
        ..SandboxConfig::default()
    };

    assert!(matches!(
        config.default_isolation_level,
        toadstool::security::IsolationLevel::Enhanced
    ));
}

#[test]
fn test_sandbox_config_clone() {
    let config1 = SandboxConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.max_concurrent_sandboxes,
        config2.max_concurrent_sandboxes
    );
    assert_eq!(config1.cleanup_timeout_secs, config2.cleanup_timeout_secs);
}

#[test]
fn test_sandbox_config_serialization() {
    let config = SandboxConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("advanced_features_enabled"));
}

#[test]
fn test_sandbox_config_deserialization() {
    let config = SandboxConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: SandboxConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        config.max_concurrent_sandboxes,
        deserialized.max_concurrent_sandboxes
    );
}

// ============================================================================
// Extended Filesystem Mount Tests
// ============================================================================

#[test]
fn test_filesystem_mount_readonly_bind() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    assert_eq!(mount.source, PathBuf::from("/host/data"));
    assert_eq!(mount.target, PathBuf::from("/sandbox/data"));
    assert!(matches!(mount.mount_type, MountType::ReadOnlyBind));
}

#[test]
fn test_filesystem_mount_readwrite_bind() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/workspace"),
        target: PathBuf::from("/sandbox/workspace"),
        mount_type: MountType::ReadWriteBind,
        options: vec![],
    };

    assert!(matches!(mount.mount_type, MountType::ReadWriteBind));
}

#[test]
fn test_filesystem_mount_tmpfs_with_size_option() {
    let mount = FilesystemMount {
        source: PathBuf::from("none"),
        target: PathBuf::from("/sandbox/tmp"),
        mount_type: MountType::TmpFs,
        options: vec!["size=100m".to_string()],
    };

    assert!(matches!(mount.mount_type, MountType::TmpFs));
    assert_eq!(mount.options.len(), 1);
    assert_eq!(mount.options[0], "size=100m");
}

#[test]
fn test_filesystem_mount_with_options() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/lib"),
        target: PathBuf::from("/sandbox/lib"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![
            "nosuid".to_string(),
            "nodev".to_string(),
            "noexec".to_string(),
        ],
    };

    assert_eq!(mount.options.len(), 3);
    assert!(mount.options.contains(&"nosuid".to_string()));
    assert!(mount.options.contains(&"nodev".to_string()));
}

#[test]
fn test_filesystem_mount_nested_paths() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/app/config"),
        target: PathBuf::from("/sandbox/etc/app"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    assert!(mount.source.to_str().unwrap().contains("config"));
    assert!(mount.target.to_str().unwrap().contains("etc"));
}

// ============================================================================
// Extended Network Configuration Tests
// ============================================================================

#[test]
fn test_network_config_isolated() {
    let config = NetworkConfig {
        enabled: false,
        isolation_mode: NetworkIsolationMode::Isolated,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec![],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Isolated
    ));
    assert!(config.allowed_hosts.is_empty());
}

#[test]
fn test_network_config_firewall() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![
            "api.example.com".to_string(),
            "data.example.com".to_string(),
        ],
        allowed_ports: vec![443, 8443],
        dns_servers: vec!["8.8.8.8".to_string()],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Firewall
    ));
    assert_eq!(config.allowed_hosts.len(), 2);
    assert_eq!(config.allowed_ports.len(), 2);
}

#[test]
fn test_network_config_namespace() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Namespace,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()],
        bandwidth_limits: None,
    };

    assert!(matches!(
        config.isolation_mode,
        NetworkIsolationMode::Namespace
    ));
    assert_eq!(config.dns_servers.len(), 2);
}

#[test]
fn test_network_config_explicitly_enabled() {
    let config = NetworkConfig {
        enabled: true,
        ..Default::default()
    };

    assert!(config.enabled);
}

#[test]
fn test_network_config_custom_dns() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![],
        allowed_ports: vec![],
        dns_servers: vec![
            "192.168.1.1".to_string(),
            "192.168.1.2".to_string(),
            "192.168.1.3".to_string(),
        ],
        bandwidth_limits: None,
    };

    assert_eq!(config.dns_servers.len(), 3);
}

#[test]
fn test_network_config_allowed_ports_range() {
    let config = NetworkConfig {
        enabled: true,
        isolation_mode: NetworkIsolationMode::Firewall,
        allowed_hosts: vec![],
        allowed_ports: vec![80, 443, 8000, 8080, 8443],
        dns_servers: vec![],
        bandwidth_limits: None,
    };

    assert_eq!(config.allowed_ports.len(), 5);
    assert!(config.allowed_ports.contains(&443));
    assert!(config.allowed_ports.contains(&8443));
}

// ============================================================================
// Sandbox Lifetime Tests
// ============================================================================

#[test]
fn test_sandbox_lifetime_ephemeral_variant() {
    let lifetime = SandboxLifetime::Ephemeral;
    assert!(matches!(lifetime, SandboxLifetime::Ephemeral));
}

#[test]
fn test_sandbox_lifetime_persistent_with_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(300),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl, Duration::from_secs(300));
    } else {
        panic!("Expected Persistent lifetime");
    }
}

#[test]
fn test_sandbox_lifetime_manual_cleanup() {
    let lifetime = SandboxLifetime::Manual;
    assert!(matches!(lifetime, SandboxLifetime::Manual));
}

#[test]
fn test_sandbox_lifetime_persistent_short_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(10),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl.as_secs(), 10);
    } else {
        panic!("Expected Persistent lifetime");
    }
}

#[test]
fn test_sandbox_lifetime_persistent_long_ttl() {
    let lifetime = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(3600),
    };

    if let SandboxLifetime::Persistent { ttl } = lifetime {
        assert_eq!(ttl.as_secs(), 3600);
    } else {
        panic!("Expected Persistent lifetime");
    }
}

// ============================================================================
// Mount Type Variant Tests
// ============================================================================

#[test]
fn test_mount_type_equality() {
    let ro1 = MountType::ReadOnlyBind;
    let ro2 = MountType::ReadOnlyBind;

    assert!(matches!(ro1, MountType::ReadOnlyBind));
    assert!(matches!(ro2, MountType::ReadOnlyBind));
}

#[test]
fn test_mount_type_tmpfs_match() {
    let mount_type = MountType::TmpFs;
    assert!(matches!(mount_type, MountType::TmpFs));
}

// ============================================================================
// Network Isolation Mode Variant Tests
// ============================================================================

#[test]
fn test_network_isolation_mode_variants() {
    let isolated = NetworkIsolationMode::Isolated;
    let firewall = NetworkIsolationMode::Firewall;
    let namespace = NetworkIsolationMode::Namespace;
    let none = NetworkIsolationMode::None;

    assert!(matches!(isolated, NetworkIsolationMode::Isolated));
    assert!(matches!(firewall, NetworkIsolationMode::Firewall));
    assert!(matches!(namespace, NetworkIsolationMode::Namespace));
    assert!(matches!(none, NetworkIsolationMode::None));
}

// ============================================================================
// Resource Limits Edge Cases
// ============================================================================

#[test]
fn test_resource_limits_zero_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(0.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(0.0));
}

#[test]
fn test_resource_limits_hundred_cpu() {
    let limits = ResourceLimits {
        max_cpu_percent: Some(100.0),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_cpu_percent, Some(100.0));
}

#[test]
fn test_resource_limits_minimal_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(1024 * 1024), // 1MB
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_memory_bytes, Some(1024 * 1024));
}

#[test]
fn test_resource_limits_large_memory() {
    let limits = ResourceLimits {
        max_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_memory_bytes, Some(16 * 1024 * 1024 * 1024));
}

#[test]
fn test_resource_limits_unlimited_execution() {
    let limits = ResourceLimits {
        max_execution_time: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_execution_time.is_none());
}

#[test]
fn test_resource_limits_short_execution() {
    let limits = ResourceLimits {
        max_execution_time: Some(Duration::from_secs(1)),
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_execution_time, Some(Duration::from_secs(1)));
}

#[test]
fn test_resource_limits_unlimited_network() {
    let limits = ResourceLimits {
        max_network_bps: None,
        ..ResourceLimits::default()
    };

    assert!(limits.max_network_bps.is_none());
}

#[test]
fn test_resource_limits_low_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(128 * 1024), // 128 KB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(128 * 1024));
}

#[test]
fn test_resource_limits_high_bandwidth() {
    let limits = ResourceLimits {
        max_network_bps: Some(1024 * 1024 * 1024), // 1 GB/s
        ..ResourceLimits::default()
    };

    assert_eq!(limits.max_network_bps, Some(1024 * 1024 * 1024));
}

// ============================================================================
// Sandbox Configuration Edge Cases
// ============================================================================

#[test]
fn test_sandbox_config_minimal_concurrent() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1);
}

#[test]
fn test_sandbox_config_high_concurrent() {
    let config = SandboxConfig {
        max_concurrent_sandboxes: 1000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.max_concurrent_sandboxes, 1000);
}

#[test]
fn test_sandbox_config_very_short_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 5,
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 5);
}

#[test]
fn test_sandbox_config_very_long_cleanup_timeout() {
    let config = SandboxConfig {
        cleanup_timeout_secs: 300,
        ..SandboxConfig::default()
    };

    assert_eq!(config.cleanup_timeout_secs, 300);
}

#[test]
fn test_sandbox_config_very_fast_monitoring() {
    let config = SandboxConfig {
        enable_monitoring: true,
        monitoring_interval_ms: 100,
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 100);
}

#[test]
fn test_sandbox_config_very_slow_monitoring() {
    let config = SandboxConfig {
        enable_monitoring: true,
        monitoring_interval_ms: 10000,
        ..SandboxConfig::default()
    };

    assert_eq!(config.monitoring_interval_ms, 10000);
}

#[test]
fn test_sandbox_config_all_disabled() {
    let config = SandboxConfig {
        advanced_features_enabled: false,
        enable_seccomp: false,
        enable_capability_dropping: false,
        enable_namespace_isolation: false,
        enable_resource_limits: false,
        enable_monitoring: false,
        ..SandboxConfig::default()
    };

    assert!(!config.advanced_features_enabled);
    assert!(!config.enable_seccomp);
    assert!(!config.enable_monitoring);
}
