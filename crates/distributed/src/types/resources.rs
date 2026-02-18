use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toadstool::IsolationLevel;

// Import canonical resource types for conversions
use toadstool::resources::{
    CpuRequirements as CoreCpuRequirements, GpuRequirements as CoreGpuRequirements,
    MemoryRequirements as CoreMemoryRequirements, NetworkRequirements as CoreNetworkRequirements,
    ResourceRequirements as CoreResourceRequirements,
    StorageRequirements as CoreStorageRequirements,
};

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

// ============================================================================
// CONVERSIONS TO/FROM CORE TYPES
// ============================================================================

impl From<ResourceRequirements> for CoreResourceRequirements {
    fn from(distributed: ResourceRequirements) -> Self {
        CoreResourceRequirements {
            cpu: CoreCpuRequirements {
                min_cores: distributed.cpu.min_cores,
                max_cores: distributed.cpu.max_cores,
                architecture: None,
            },
            memory: CoreMemoryRequirements {
                min_bytes: distributed.memory.min_bytes,
                max_bytes: distributed.memory.max_bytes,
            },
            storage: CoreStorageRequirements {
                min_bytes: distributed.storage.min_bytes,
                max_bytes: distributed.storage.max_bytes,
                storage_type: None,
            },
            gpu: distributed.gpu.map(|gpu| CoreGpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: gpu.compute_capability,
                min_memory_bytes: Some((gpu.min_memory_gb * 1024.0 * 1024.0 * 1024.0) as u64),
            }),
            network: CoreNetworkRequirements {
                min_bandwidth: distributed
                    .network
                    .bandwidth_mbps
                    .map(|mbps| mbps * 1024 * 1024),
                max_bandwidth: None,
                max_latency_ms: distributed.network.latency_ms,
            },
        }
    }
}

impl From<CoreResourceRequirements> for ResourceRequirements {
    fn from(core: CoreResourceRequirements) -> Self {
        ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: core.cpu.min_cores,
                max_cores: core.cpu.max_cores,
            },
            memory: MemoryRequirements {
                min_bytes: core.memory.min_bytes,
                max_bytes: core.memory.max_bytes,
            },
            storage: StorageRequirements {
                min_bytes: core.storage.min_bytes,
                max_bytes: core.storage.max_bytes,
            },
            network: NetworkRequirements {
                bandwidth_mbps: core
                    .network
                    .min_bandwidth
                    .map(|bytes_per_sec| bytes_per_sec / (1024 * 1024)),
                latency_ms: core.network.max_latency_ms,
            },
            gpu: core.gpu.map(|gpu| GpuRequirements {
                min_memory_gb: gpu
                    .min_memory_bytes
                    .map(|bytes| bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                    .unwrap_or(1.0),
                compute_capability: gpu.gpu_type,
            }),
        }
    }
}

/// Distributed execution retry configuration
///
/// Domain-specific retry configuration for distributed workload execution.
/// Includes execution-specific retry conditions and backoff strategies.
///
/// For simple retry logic, use `toadstool::config_bases::RetryConfig` instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Backoff strategy for retries
    pub backoff_strategy: BackoffStrategy,
    /// Conditions that trigger retries
    pub retry_conditions: Vec<RetryCondition>,
}

impl Default for DistributedRetryConfig {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_requirements_default_validation() {
        let req = ResourceRequirements::default();
        assert_eq!(req.cpu.min_cores, 1.0);
        assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
        assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024);
        assert!(req.network.bandwidth_mbps.is_none());
        assert!(req.gpu.is_none());
    }

    #[test]
    fn cpu_requirements_construction() {
        let cpu = CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
        };
        assert_eq!(cpu.min_cores, 4.0);
        assert_eq!(cpu.max_cores, Some(8.0));
    }

    #[test]
    fn memory_requirements_construction() {
        let mem = MemoryRequirements {
            min_bytes: 2 * 1024 * 1024 * 1024,
            max_bytes: Some(16 * 1024 * 1024 * 1024),
        };
        assert_eq!(mem.min_bytes, 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn storage_requirements_construction() {
        let st = StorageRequirements {
            min_bytes: 10 * 1024 * 1024 * 1024,
            max_bytes: None,
        };
        assert_eq!(st.min_bytes, 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn gpu_requirements_construction() {
        let gpu = GpuRequirements {
            min_memory_gb: 8.0,
            compute_capability: Some("8.0".to_string()),
        };
        assert_eq!(gpu.min_memory_gb, 8.0);
        assert_eq!(gpu.compute_capability.as_deref(), Some("8.0"));
    }

    #[test]
    fn resource_requirements_to_from_core() {
        let distributed = ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 2.0,
                max_cores: Some(4.0),
            },
            memory: MemoryRequirements {
                min_bytes: 4 * 1024 * 1024 * 1024,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 20 * 1024 * 1024 * 1024,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: Some(100),
                latency_ms: Some(50),
            },
            gpu: Some(GpuRequirements {
                min_memory_gb: 4.0,
                compute_capability: Some("7.5".to_string()),
            }),
        };
        let core_req: toadstool::resources::ResourceRequirements = distributed.clone().into();
        let back: ResourceRequirements = core_req.into();
        assert_eq!(back.cpu.min_cores, distributed.cpu.min_cores);
        assert_eq!(back.memory.min_bytes, distributed.memory.min_bytes);
        assert_eq!(back.gpu.as_ref().map(|g| g.min_memory_gb), Some(4.0));
    }

    #[test]
    fn distributed_retry_config_default() {
        let config = DistributedRetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert!(!config.retry_conditions.is_empty());
    }

    #[test]
    fn resource_allocation_default() {
        let alloc = ResourceAllocation::default();
        assert_eq!(alloc.cpu_cores, 1.0);
        assert_eq!(alloc.memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(alloc.storage_bytes, 10 * 1024 * 1024 * 1024);
        assert!(alloc.gpu_allocation.is_none());
    }

    #[test]
    fn network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.port_range.0, 8000);
        assert_eq!(config.port_range.1, 9000);
        assert!(matches!(
            config.security_level,
            NetworkSecurityLevel::Medium
        ));
    }

    #[test]
    fn resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_cpu_cores, 4.0);
        assert_eq!(limits.max_memory_bytes, 8 * 1024 * 1024 * 1024);
    }

    #[test]
    fn resource_value_variants() {
        let _i = ResourceValue::Integer(42);
        let _f = ResourceValue::Float(3.14);
        let _s = ResourceValue::String("test".to_string());
        let _b = ResourceValue::Boolean(true);
    }

    #[test]
    fn gpu_allocation_construction() {
        let alloc = GpuAllocation {
            device_id: 0,
            memory_bytes: 8 * 1024 * 1024 * 1024,
            compute_units: 40,
        };
        assert_eq!(alloc.device_id, 0);
        assert_eq!(alloc.memory_bytes, 8 * 1024 * 1024 * 1024);
    }
}
