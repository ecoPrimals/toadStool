// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn sandbox_config_default() {
    let config = SandboxConfig::default();
    assert!(config.advanced_features_enabled);
    assert!(config.enable_resource_limits);
    assert!(config.enable_monitoring);
    assert_eq!(config.max_concurrent_sandboxes, 100);
}

#[test]
fn resource_limits_default() {
    let limits = ResourceLimits::default();
    assert!(limits.max_memory_bytes.is_some());
    assert!(limits.max_cpu_percent.is_some());
    assert!(limits.max_execution_time.is_some());
    assert_eq!(limits.max_file_descriptors, Some(1024));
    assert_eq!(limits.max_processes, Some(100));
}

#[test]
fn network_config_default_disabled() {
    let config = NetworkConfig::default();
    assert!(!config.enabled);
    assert!(config.allowed_hosts.is_empty());
    assert!(config.allowed_ports.is_empty());
    assert!(config.dns_servers.is_empty());
    assert!(config.bandwidth_limits.is_none());
}

#[test]
fn sandbox_status_equality() {
    assert_eq!(SandboxStatus::Creating, SandboxStatus::Creating);
    assert_ne!(SandboxStatus::Running, SandboxStatus::Completed);
}

#[test]
fn sandbox_status_all_variants() {
    let variants = [
        SandboxStatus::Creating,
        SandboxStatus::Ready,
        SandboxStatus::Running,
        SandboxStatus::Completed,
        SandboxStatus::Failed,
        SandboxStatus::Destroying,
        SandboxStatus::Destroyed,
    ];
    for v in &variants {
        assert!(!format!("{v:?}").is_empty());
    }
}

#[test]
fn mount_type_serde() {
    let types = [
        MountType::ReadOnlyBind,
        MountType::ReadWriteBind,
        MountType::TmpFs,
        MountType::Device,
        MountType::Proc,
        MountType::Sys,
    ];
    for mt in &types {
        let json = serde_json::to_string(mt).unwrap();
        let deser: MountType = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mt:?}"), format!("{deser:?}"));
    }
}

#[test]
fn network_isolation_mode_serde() {
    let modes = [
        NetworkIsolationMode::None,
        NetworkIsolationMode::Firewall,
        NetworkIsolationMode::Namespace,
        NetworkIsolationMode::Isolated,
    ];
    for mode in &modes {
        let json = serde_json::to_string(mode).unwrap();
        let deser: NetworkIsolationMode = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{mode:?}"), format!("{deser:?}"));
    }
}

#[test]
fn sandbox_lifetime_serde() {
    let ephemeral = SandboxLifetime::Ephemeral;
    let json = serde_json::to_string(&ephemeral).unwrap();
    assert!(json.contains("Ephemeral"));

    let persistent = SandboxLifetime::Persistent {
        ttl: Duration::from_secs(3600),
    };
    let json = serde_json::to_string(&persistent).unwrap();
    assert!(json.contains("Persistent"));

    let manual = SandboxLifetime::Manual;
    let json = serde_json::to_string(&manual).unwrap();
    assert!(json.contains("Manual"));
}

#[test]
fn violation_severity_serde() {
    let severities = [
        ViolationSeverity::Low,
        ViolationSeverity::Medium,
        ViolationSeverity::High,
        ViolationSeverity::Critical,
    ];
    for s in &severities {
        let json = serde_json::to_string(s).unwrap();
        let deser: ViolationSeverity = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{s:?}"), format!("{deser:?}"));
    }
}

#[test]
fn resource_usage_default_zeroed() {
    let usage = ResourceUsage::default();
    assert_eq!(usage.memory_bytes, 0);
    assert!((usage.cpu_percent - 0.0).abs() < f64::EPSILON);
    assert_eq!(usage.file_descriptors, 0);
    assert_eq!(usage.processes, 0);
    assert_eq!(usage.execution_time, Duration::ZERO);
}

#[test]
fn filesystem_mount_serde() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec!["nosuid".to_string()],
    };
    let json = serde_json::to_string(&mount).unwrap();
    let deser: FilesystemMount = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.source, PathBuf::from("/host/data"));
    assert_eq!(deser.options.len(), 1);
}

#[test]
fn bandwidth_limits_serde() {
    let limits = BandwidthLimits {
        upload_bps: 1_000_000,
        download_bps: 10_000_000,
    };
    let json = serde_json::to_string(&limits).unwrap();
    let deser: BandwidthLimits = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.upload_bps, 1_000_000);
}
