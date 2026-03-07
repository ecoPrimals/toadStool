// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for security sandbox library
//!
//! Tests focus on actual implemented types and functionality.
//! Obsolete tests for non-existent types have been removed.

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::types::*;
    use std::path::PathBuf;
    use std::time::Duration;
    use toadstool::error::ToadStoolError;
    use toadstool::security::IsolationLevel;

    // ============================================================================
    // Sandbox Configuration Tests
    // ============================================================================

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(config.advanced_features_enabled);
        assert_eq!(config.default_isolation_level, IsolationLevel::Standard);
        assert!(config.max_concurrent_sandboxes > 0);
    }

    #[test]
    fn test_sandbox_config_custom() {
        let config = SandboxConfig {
            advanced_features_enabled: false,
            default_isolation_level: IsolationLevel::Standard,
            enable_seccomp: true,
            enable_capability_dropping: true,
            enable_namespace_isolation: true,
            enable_resource_limits: true,
            sandbox_root: PathBuf::from("/custom/sandbox"),
            temp_dir: PathBuf::from("/custom/temp"),
            max_concurrent_sandboxes: 50,
            cleanup_timeout_secs: 60,
            enable_monitoring: true,
            monitoring_interval_ms: 500,
        };
        assert_eq!(config.max_concurrent_sandboxes, 50);
        assert_eq!(config.cleanup_timeout_secs, 60);
    }

    // ============================================================================
    // Resource Limits Tests
    // ============================================================================

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        // Default should have some reasonable limits
        assert!(limits.max_memory_bytes.is_some());
        assert!(limits.max_cpu_percent.is_some());
    }

    #[test]
    fn test_resource_limits_custom() {
        let limits = ResourceLimits {
            max_memory_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
            max_cpu_percent: Some(75.0),
            max_file_descriptors: Some(1024),
            max_processes: Some(10),
            max_disk_bytes: Some(10 * 1024 * 1024 * 1024), // 10GB
            max_network_bps: Some(100 * 1024 * 1024),      // 100 MB/s
            max_execution_time: Some(Duration::from_secs(300)),
        };
        assert_eq!(limits.max_memory_bytes, Some(2 * 1024 * 1024 * 1024));
        assert_eq!(limits.max_cpu_percent, Some(75.0));
        assert_eq!(limits.max_file_descriptors, Some(1024));
    }

    // ============================================================================
    // Filesystem Mount Tests
    // ============================================================================

    #[test]
    fn test_filesystem_mount() {
        let mount = FilesystemMount {
            source: PathBuf::from("/host/data"),
            target: PathBuf::from("/sandbox/data"),
            mount_type: MountType::ReadOnlyBind,
            options: vec![],
        };
        assert!(matches!(mount.mount_type, MountType::ReadOnlyBind));
        assert_eq!(mount.source, PathBuf::from("/host/data"));
    }

    // ============================================================================
    // Network Config Tests
    // ============================================================================

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        // Should have reasonable defaults
        assert!(!config.enabled);
    }

    #[test]
    fn test_network_config_custom() {
        let config = NetworkConfig {
            enabled: true,
            isolation_mode: NetworkIsolationMode::Firewall,
            allowed_hosts: vec!["api.example.com".to_string()],
            allowed_ports: vec![443, 8080],
            dns_servers: vec!["8.8.8.8".to_string()],
            bandwidth_limits: None,
        };
        assert_eq!(config.allowed_ports.len(), 2);
        assert!(config.enabled);
    }

    // ============================================================================
    // Sandbox Lifetime Tests
    // ============================================================================

    #[test]
    fn test_sandbox_lifetime_persistent() {
        let lifetime = SandboxLifetime::Persistent {
            ttl: Duration::from_secs(300),
        };
        match lifetime {
            SandboxLifetime::Persistent { ttl } => assert_eq!(ttl, Duration::from_secs(300)),
            _ => panic!("Expected Persistent variant"),
        }
    }

    #[test]
    fn test_sandbox_lifetime_ephemeral() {
        let lifetime = SandboxLifetime::Ephemeral;
        assert!(matches!(lifetime, SandboxLifetime::Ephemeral));
    }

    // ============================================================================
    // Sandbox Info Tests
    // ============================================================================

    #[test]
    fn test_sandbox_info_creation() {
        use std::time::SystemTime;

        let now = SystemTime::now();
        let info = SandboxInfo {
            sandbox_id: "sandbox-123".to_string(),
            status: SandboxStatus::Running,
            created_at: now,
            updated_at: now,
            process_id: Some(12345),
            resource_usage: ResourceUsage::default(),
            security_violations: vec![],
            metadata: std::collections::HashMap::new(),
        };

        assert_eq!(info.sandbox_id, "sandbox-123");
        assert_eq!(info.status, SandboxStatus::Running);
        assert_eq!(info.process_id, Some(12345));
    }

    // ============================================================================
    // Resource Usage Tests
    // ============================================================================

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.cpu_percent, 0.0);
        assert_eq!(usage.disk_bytes, 0);
    }

    #[test]
    fn test_resource_usage_custom() {
        let usage = ResourceUsage {
            memory_bytes: 512 * 1024 * 1024, // 512 MB
            cpu_percent: 45.5,
            file_descriptors: 42,
            processes: 5,
            disk_bytes: 1024 * 1024 * 1024, // 1 GB
            network_bytes_sent: 500000,
            network_bytes_received: 1000000,
            execution_time: Duration::from_secs(10),
        };

        assert_eq!(usage.memory_bytes, 512 * 1024 * 1024);
        assert_eq!(usage.cpu_percent, 45.5);
        assert_eq!(usage.file_descriptors, 42);
    }

    // ============================================================================
    // Error Handling Tests
    // ============================================================================

    #[test]
    fn test_error_handling_security() {
        let error = ToadStoolError::security("Sandbox violation");
        let error_msg = error.to_string();
        assert!(error_msg.contains("Sandbox violation"));
    }

    #[test]
    fn test_error_handling_runtime() {
        let error = ToadStoolError::runtime("Sandbox initialization failed");
        let error_msg = error.to_string();
        assert!(error_msg.contains("initialization failed"));
    }
}
