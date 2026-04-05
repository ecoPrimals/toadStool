// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sandbox types and structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use toadstool::security::{IsolationLevel, SecurityContext};
use toadstool::workload::WorkloadSpec;

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable advanced sandboxing features
    pub advanced_features_enabled: bool,
    /// Default isolation level
    pub default_isolation_level: IsolationLevel,
    /// Enable seccomp filtering (Linux only)
    pub enable_seccomp: bool,
    /// Enable capability dropping
    pub enable_capability_dropping: bool,
    /// Enable namespace isolation
    pub enable_namespace_isolation: bool,
    /// Enable resource limits enforcement
    pub enable_resource_limits: bool,
    /// Sandbox root directory
    pub sandbox_root: PathBuf,
    /// Temporary directory for sandbox operations
    pub temp_dir: PathBuf,
    /// Maximum number of concurrent sandboxes
    pub max_concurrent_sandboxes: u32,
    /// Sandbox cleanup timeout in seconds
    pub cleanup_timeout_secs: u64,
    /// Enable sandbox monitoring
    pub enable_monitoring: bool,
    /// Monitoring interval in milliseconds
    pub monitoring_interval_ms: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let primal_name = toadstool_common::constants::primal_identity::PRIMAL_NAME;

        // Platform-agnostic path resolution (ecoBin v2.0 compliant)
        let sandbox_root = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share"))
                    .unwrap_or_else(|_| std::env::temp_dir())
            })
            .join(format!("{primal_name}/sandbox"));

        let temp_dir = std::env::temp_dir().join(primal_name);

        Self {
            advanced_features_enabled: true,
            default_isolation_level: IsolationLevel::Standard,
            enable_seccomp: cfg!(target_os = "linux"),
            enable_capability_dropping: true,
            enable_namespace_isolation: cfg!(target_os = "linux"),
            enable_resource_limits: true,
            sandbox_root,
            temp_dir,
            max_concurrent_sandboxes: 100,
            cleanup_timeout_secs: toadstool_common::constants::timeouts::BIOME_SHUTDOWN_TIMEOUT
                .as_secs(),
            enable_monitoring: true,
            monitoring_interval_ms: 1000,
        }
    }
}

/// Sandbox specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSpec {
    /// Unique sandbox identifier
    pub sandbox_id: String,
    /// Workload to be sandboxed
    pub workload: WorkloadSpec,
    /// Security context
    pub security_context: SecurityContext,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// File system mounts
    pub filesystem_mounts: Vec<FilesystemMount>,
    /// Network configuration
    pub network_config: NetworkConfig,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Working directory inside sandbox
    pub working_directory: Option<PathBuf>,
    /// Sandbox lifetime
    pub lifetime: SandboxLifetime,
}

/// Resource limits for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: Option<f64>,
    /// Maximum number of file descriptors
    pub max_file_descriptors: Option<u32>,
    /// Maximum number of processes/threads
    pub max_processes: Option<u32>,
    /// Maximum disk usage in bytes
    pub max_disk_bytes: Option<u64>,
    /// Maximum network bandwidth in bytes/second
    pub max_network_bps: Option<u64>,
    /// Maximum execution time
    pub max_execution_time: Option<Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024), // 512MB
            max_cpu_percent: Some(80.0),
            max_file_descriptors: Some(1024),
            max_processes: Some(100),
            max_disk_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_network_bps: Some(10 * 1024 * 1024),  // 10MB/s
            max_execution_time: Some(Duration::from_secs(300)), // 5 minutes
        }
    }
}

/// Filesystem mount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemMount {
    /// Source path (host)
    pub source: PathBuf,
    /// Target path (sandbox)
    pub target: PathBuf,
    /// Mount type
    pub mount_type: MountType,
    /// Mount options
    pub options: Vec<String>,
}

/// Mount type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MountType {
    /// Read-only bind mount
    ReadOnlyBind,
    /// Read-write bind mount
    ReadWriteBind,
    /// Temporary filesystem
    TmpFs,
    /// Device mount
    Device,
    /// Proc filesystem
    Proc,
    /// Sys filesystem
    Sys,
}

/// Network configuration for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable network access
    pub enabled: bool,
    /// Network isolation mode
    pub isolation_mode: NetworkIsolationMode,
    /// Allowed outbound hosts
    pub allowed_hosts: Vec<String>,
    /// Allowed outbound ports
    pub allowed_ports: Vec<u16>,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Network bandwidth limits
    pub bandwidth_limits: Option<BandwidthLimits>,
}

/// Network isolation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkIsolationMode {
    /// No network isolation
    None,
    /// Basic firewall rules
    Firewall,
    /// Network namespace isolation
    Namespace,
    /// Complete network isolation
    Isolated,
}

/// Network bandwidth limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthLimits {
    /// Upload limit in bytes/second
    pub upload_bps: u64,
    /// Download limit in bytes/second
    pub download_bps: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            isolation_mode: NetworkIsolationMode::Firewall,
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
            // Empty by default: network is disabled. When enabled, operators provide
            // DNS resolvers via config or they are discovered from the host system's
            // /etc/resolv.conf. Never hardcode external DNS addresses.
            dns_servers: Vec::new(),
            bandwidth_limits: None,
        }
    }
}

/// Sandbox lifetime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxLifetime {
    /// Ephemeral sandbox (destroyed after execution)
    Ephemeral,
    /// Persistent sandbox with TTL
    Persistent {
        /// Time-to-live duration.
        ttl: Duration,
    },
    /// Manual cleanup required
    Manual,
}

/// Sandbox status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxStatus {
    /// Sandbox is being created
    Creating,
    /// Sandbox is ready for execution
    Ready,
    /// Sandbox is running workload
    Running,
    /// Sandbox execution completed
    Completed,
    /// Sandbox failed
    Failed,
    /// Sandbox is being destroyed
    Destroying,
    /// Sandbox has been destroyed
    Destroyed,
}

/// Sandbox information
#[derive(Debug, Clone)]
pub struct SandboxInfo {
    /// Sandbox identifier
    pub sandbox_id: String,
    /// Current status
    pub status: SandboxStatus,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last updated timestamp
    pub updated_at: SystemTime,
    /// Process ID (if running)
    pub process_id: Option<u32>,
    /// Resource usage statistics
    pub resource_usage: ResourceUsage,
    /// Security violations (if any)
    pub security_violations: Vec<SecurityViolation>,
    /// Sandbox metadata
    pub metadata: HashMap<String, String>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: u64,
    /// Current CPU usage percentage
    pub cpu_percent: f64,
    /// Number of open file descriptors
    pub file_descriptors: u32,
    /// Number of running processes
    pub processes: u32,
    /// Disk usage in bytes
    pub disk_bytes: u64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Execution time
    pub execution_time: Duration,
}

/// Security violation information
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Violation type
    pub violation_type: String,
    /// Violation description
    pub description: String,
    /// Timestamp of violation
    pub timestamp: SystemTime,
    /// Severity level
    pub severity: ViolationSeverity,
    /// Action taken
    pub action_taken: String,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Low severity violation.
    Low,
    /// Medium severity violation.
    Medium,
    /// High severity violation.
    High,
    /// Critical severity violation.
    Critical,
}

#[cfg(test)]
mod tests {
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
}
