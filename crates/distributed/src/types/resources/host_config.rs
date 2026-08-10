// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use toadstool_core::security::IsolationLevel;

/// Network configuration for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Allowed port range (min, max).
    pub port_range: (u16, u16),
    /// Security level for network isolation.
    pub security_level: NetworkSecurityLevel,
    /// Allowed protocols (http, https, etc.).
    pub protocols: Vec<String>,
}

impl NetworkConfig {
    const DEFAULT_PORT_MIN: u16 = 8000;
    const DEFAULT_PORT_MAX: u16 = 9000;
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port_range: (Self::DEFAULT_PORT_MIN, Self::DEFAULT_PORT_MAX),
            security_level: NetworkSecurityLevel::Medium,
            protocols: vec!["http".to_string(), "https".to_string()],
        }
    }
}

/// Network security levels for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSecurityLevel {
    /// Minimal isolation (dev/test).
    Low,
    /// Standard isolation.
    Medium,
    /// Strict isolation (production).
    High,
    /// Maximum isolation (compliance).
    Maximum,
}

/// Security configuration for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Process isolation level.
    pub isolation_level: IsolationLevel,
    /// Whether sandboxing is enabled.
    pub sandboxing_enabled: bool,
    /// Whether resource limits are enforced.
    pub resource_limits_enforced: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            sandboxing_enabled: true,
            resource_limits_enforced: true,
        }
    }
}

/// Startup configuration for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    /// Whether to auto-start on creation.
    pub auto_start: bool,
    /// Timeout for startup completion in ms.
    pub startup_timeout_ms: u64,
    /// Interval for health checks in ms.
    pub health_check_interval_ms: u64,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            startup_timeout_ms: 30000,
            health_check_interval_ms: 5000,
        }
    }
}

/// Resource limits for OS layer and hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU cores.
    pub max_cpu_cores: f64,
    /// Maximum memory in bytes.
    pub max_memory_bytes: u64,
    /// Maximum storage in bytes.
    pub max_storage_bytes: u64,
    /// Maximum network bandwidth in Mbps.
    pub max_network_bandwidth_mbps: u64,
}

impl ResourceLimits {
    const DEFAULT_CPU_CORES: f64 = 4.0;
    const DEFAULT_MEMORY_GB: u64 = 8;
    const DEFAULT_STORAGE_GB: u64 = 100;
    const DEFAULT_BANDWIDTH_MBPS: u64 = 1000;
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_cores: Self::DEFAULT_CPU_CORES,
            max_memory_bytes: Self::DEFAULT_MEMORY_GB * 1024 * 1024 * 1024,
            max_storage_bytes: Self::DEFAULT_STORAGE_GB * 1024 * 1024 * 1024,
            max_network_bandwidth_mbps: Self::DEFAULT_BANDWIDTH_MBPS,
        }
    }
}

/// Instance status for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceStatus {
    /// Instance is starting up.
    Starting,
    /// Instance is running and accepting work.
    Running,
    /// Instance is shutting down.
    Stopping,
    /// Instance has stopped.
    Stopped,
    /// Instance encountered an error.
    Error,
}

/// Process handle for hosted instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
    /// OS process ID.
    pub process_id: u32,
    /// When the process was started.
    pub started_at: std::time::SystemTime,
    /// Current instance status.
    pub status: InstanceStatus,
}

impl Default for ProcessHandle {
    fn default() -> Self {
        Self {
            process_id: 0,
            started_at: std::time::SystemTime::now(),
            status: InstanceStatus::Starting,
        }
    }
}
