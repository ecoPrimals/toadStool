use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toadstool::IsolationLevel;

/// Resource requirements for job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: CpuRequirements,
    /// Memory requirements
    pub memory: MemoryRequirements,
    /// Storage requirements
    pub storage: StorageRequirements,
    /// Network requirements
    pub network: NetworkRequirements,
    /// GPU requirements
    pub gpu: Option<GpuRequirements>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu: CpuRequirements {
                min_cores: 1.0,
                max_cores: None,
            },
            memory: MemoryRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1GB
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 1024 * 1024 * 1024, // 1GB
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: None,
                latency_ms: None,
            },
            gpu: None,
        }
    }
}

/// CPU requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    pub min_cores: f64,
    pub max_cores: Option<f64>,
}

/// Memory requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    pub min_bytes: u64,
    pub max_bytes: Option<u64>,
}

/// Storage requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    pub min_bytes: u64,
    pub max_bytes: Option<u64>,
}

/// Network requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    pub bandwidth_mbps: Option<u64>,
    pub latency_ms: Option<u64>,
}

/// GPU requirements specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    pub min_memory_gb: f64,
    pub compute_capability: Option<String>,
}

/// Retry configuration for failed executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<RetryCondition>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                base_ms: 1000,
                max_ms: 30000,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::ResourceUnavailable,
                RetryCondition::TemporaryFailure,
            ],
        }
    }
}

/// Backoff strategies for retry logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay_ms: u64 },
    Linear { initial_ms: u64, increment_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64 },
    ExponentialJittered { base_ms: u64, max_ms: u64 },
}

/// Conditions that trigger job retry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    NetworkError,
    ResourceUnavailable,
    TemporaryFailure,
    ServiceUnavailable,
    Custom(String),
}

/// Resource constraints for job placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    pub max_cpu_cores: Option<f64>,
    pub max_memory_bytes: Option<u64>,
    pub required_features: Vec<String>,
    pub excluded_nodes: Vec<String>,
}

/// Resource allocation for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceAllocation {
    /// CPU allocation in cores
    pub cpu_cores: f64,
    /// Memory allocation in bytes
    pub memory_bytes: u64,
    /// Storage allocation in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bytes per second
    pub network_bandwidth: u64,
    /// GPU allocation (if available)
    pub gpu_allocation: Option<GpuAllocation>,
    /// Custom resource allocations
    pub custom_resources: HashMap<String, ResourceValue>,
}

impl Eq for ResourceAllocation {}

impl std::hash::Hash for ResourceAllocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cpu_cores.to_bits().hash(state);
        self.memory_bytes.hash(state);
        self.storage_bytes.hash(state);
        self.network_bandwidth.hash(state);
    }
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        Self {
            cpu_cores: 1.0,
            memory_bytes: 1024 * 1024 * 1024,       // 1GB
            storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            network_bandwidth: 100,
            gpu_allocation: None,
            custom_resources: HashMap::new(),
        }
    }
}

/// Resource allocation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceAllocationStrategy {
    Fair,
    Proportional,
    Priority,
    Custom(String),
}

/// Network configuration for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub port_range: (u16, u16),
    pub security_level: NetworkSecurityLevel,
    pub protocols: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port_range: (8000, 9000),
            security_level: NetworkSecurityLevel::Medium,
            protocols: vec!["http".to_string(), "https".to_string()],
        }
    }
}

/// Network security levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkSecurityLevel {
    Low,
    Medium,
    High,
    Maximum,
}

/// Security configuration for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub isolation_level: IsolationLevel,
    pub sandboxing_enabled: bool,
    pub resource_limits_enforced: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            isolation_level: toadstool::IsolationLevel::Standard,
            sandboxing_enabled: true,
            resource_limits_enforced: true,
        }
    }
}

/// Startup configuration for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupConfig {
    pub auto_start: bool,
    pub startup_timeout_ms: u64,
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

/// Resource limits for OS layer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_cores: f64,
    pub max_memory_bytes: u64,
    pub max_storage_bytes: u64,
    pub max_network_bandwidth_mbps: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_cores: 4.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            max_storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            max_network_bandwidth_mbps: 1000,
        }
    }
}

/// Instance status for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
}

/// Process handle for hosted instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
    pub process_id: u32,
    pub started_at: std::time::SystemTime,
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

/// GPU allocation information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuAllocation {
    /// GPU device ID
    pub device_id: u32,
    /// Memory allocation in bytes
    pub memory_bytes: u64,
    /// Compute units allocated
    pub compute_units: u32,
}

/// Resource value type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}
