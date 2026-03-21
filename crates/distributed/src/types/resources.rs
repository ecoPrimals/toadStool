// SPDX-License-Identifier: AGPL-3.0-only
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

/// CPU requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Minimum CPU cores required.
    pub min_cores: f64,
    /// Maximum CPU cores (optional cap).
    pub max_cores: Option<f64>,
}

/// Memory requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Minimum memory in bytes.
    pub min_bytes: u64,
    /// Maximum memory in bytes (optional cap).
    pub max_bytes: Option<u64>,
}

/// Storage requirements specification for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Minimum storage in bytes.
    pub min_bytes: u64,
    /// Maximum storage in bytes (optional cap).
    pub max_bytes: Option<u64>,
}

/// Network requirements specification for distributed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Minimum bandwidth in Mbps.
    pub bandwidth_mbps: Option<u64>,
    /// Maximum acceptable latency in ms.
    pub latency_ms: Option<u64>,
}

/// GPU requirements specification for accelerated workloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Minimum GPU memory in GB.
    pub min_memory_gb: f64,
    /// Required compute capability (e.g. CUDA 8.0).
    pub compute_capability: Option<String>,
}

// ============================================================================
// CONVERSIONS TO/FROM CORE TYPES
// ============================================================================

impl From<ResourceRequirements> for CoreResourceRequirements {
    fn from(distributed: ResourceRequirements) -> Self {
        Self {
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
        Self {
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
        const DEFAULT_BACKOFF_BASE_MS: u64 = 1_000;
        const DEFAULT_BACKOFF_MAX_MS: u64 = 30_000;
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                base_ms: DEFAULT_BACKOFF_BASE_MS,
                max_ms: DEFAULT_BACKOFF_MAX_MS,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::ResourceUnavailable,
                RetryCondition::TemporaryFailure,
            ],
        }
    }
}

/// Backoff strategies for retry logic in distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed {
        /// Delay in milliseconds.
        delay_ms: u64,
    },
    /// Linear increase: initial + n * increment.
    Linear {
        /// Initial delay in ms.
        initial_ms: u64,
        /// Increment per retry in ms.
        increment_ms: u64,
    },
    /// Exponential backoff with base and max.
    Exponential {
        /// Base delay in ms.
        base_ms: u64,
        /// Max delay in ms.
        max_ms: u64,
    },
    /// Exponential backoff with jitter to avoid thundering herd.
    ExponentialJittered {
        /// Base delay in ms.
        base_ms: u64,
        /// Max delay in ms.
        max_ms: u64,
    },
}

/// Conditions that trigger job retry in distributed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// Network connectivity or timeout error.
    NetworkError,
    /// Resource (CPU, memory, GPU) temporarily unavailable.
    ResourceUnavailable,
    /// Generic transient failure.
    TemporaryFailure,
    /// Remote service returned 503 or similar.
    ServiceUnavailable,
    /// Custom condition for extensibility.
    Custom(String),
}

/// Resource constraints for job placement and scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum CPU cores allowed for placement.
    pub max_cpu_cores: Option<f64>,
    /// Maximum memory in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Required hardware/software features (e.g. gpu, nvme).
    pub required_features: Vec<String>,
    /// Node IDs to exclude from placement.
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
        const GIB: u64 = 1024 * 1024 * 1024;
        Self {
            cpu_cores: 1.0,
            memory_bytes: GIB,
            storage_bytes: 10 * GIB,
            network_bandwidth: 100,
            gpu_allocation: None,
            custom_resources: HashMap::new(),
        }
    }
}

/// Resource allocation strategies for child instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceAllocationStrategy {
    /// Equal share across children.
    Fair,
    /// Proportional to workload size.
    Proportional,
    /// Priority-based allocation.
    Priority,
    /// Custom strategy (plugin name).
    Custom(String),
}

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

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            port_range: (8000, 9000),
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
            isolation_level: toadstool::IsolationLevel::Standard,
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

/// Typed value for custom resource allocations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceValue {
    /// Integer resource value.
    Integer(i64),
    /// Float resource value.
    Float(f64),
    /// String resource value.
    String(String),
    /// Boolean resource value.
    Boolean(bool),
}

#[cfg(test)]
mod tests {
    // SPDX-License-Identifier: AGPL-3.0-only
    use super::*;
    use proptest::prelude::*;

    fn arb_resource_allocation() -> impl Strategy<Value = ResourceAllocation> {
        (
            // Use integer-ish cpu_cores: JSON roundtrip can change float representation
            (1u32..1024u32).prop_map(|n| n as f64),
            (1024u64..(1u64 << 40)), // 1KB .. 1TB
            (1024u64..(1u64 << 40)),
            (100u64..(1u64 << 30)), // 100 B/s .. 1 GB/s
            prop::option::of(((0u32..16), (1024u64..(1u64 << 30)), (1u32..128))),
            // Exclude Float from custom_resources: JSON roundtrip can change float representation
            prop::collection::hash_map(
                "[a-z_]{1,20}",
                prop_oneof![
                    any::<i64>().prop_map(ResourceValue::Integer),
                    "[a-zA-Z0-9_]{0,50}".prop_map(ResourceValue::String),
                    any::<bool>().prop_map(ResourceValue::Boolean),
                ],
                0..5,
            ),
        )
            .prop_map(
                |(
                    cpu_cores,
                    memory_bytes,
                    storage_bytes,
                    network_bandwidth,
                    gpu,
                    custom_resources,
                )| {
                    ResourceAllocation {
                        cpu_cores,
                        memory_bytes,
                        storage_bytes,
                        network_bandwidth,
                        gpu_allocation: gpu.map(|(device_id, memory_bytes, compute_units)| {
                            GpuAllocation {
                                device_id,
                                memory_bytes,
                                compute_units,
                            }
                        }),
                        custom_resources,
                    }
                },
            )
    }

    fn arb_backoff_strategy() -> impl Strategy<Value = BackoffStrategy> {
        prop_oneof![
            (1u64..60_000u64).prop_map(|delay_ms| BackoffStrategy::Fixed { delay_ms }),
            ((100u64..5_000u64), (50u64..2_000u64)).prop_map(|(initial_ms, increment_ms)| {
                BackoffStrategy::Linear {
                    initial_ms,
                    increment_ms,
                }
            }),
            ((100u64..10_000u64), (1_000u64..60_000u64))
                .prop_map(|(base_ms, max_ms)| BackoffStrategy::Exponential { base_ms, max_ms }),
            ((100u64..10_000u64), (1_000u64..60_000u64)).prop_map(|(base_ms, max_ms)| {
                BackoffStrategy::ExponentialJittered { base_ms, max_ms }
            }),
        ]
    }

    fn arb_network_config() -> impl Strategy<Value = NetworkConfig> {
        (
            ((1u16..65535), (1u16..65535)).prop_filter("port_range ordered", |(a, b)| a <= b),
            prop_oneof![
                Just(NetworkSecurityLevel::Low),
                Just(NetworkSecurityLevel::Medium),
                Just(NetworkSecurityLevel::High),
                Just(NetworkSecurityLevel::Maximum),
            ],
            prop::collection::vec("[a-z]{4,10}", 0..5),
        )
            .prop_map(|(port_range, security_level, protocols)| NetworkConfig {
                port_range,
                security_level,
                protocols,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_resource_allocation_json_roundtrip(alloc in arb_resource_allocation()) {
            let json = serde_json::to_string(&alloc).unwrap();
            let restored: ResourceAllocation = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(alloc, restored);
        }

        #[test]
        fn prop_backoff_strategy_json_roundtrip(strategy in arb_backoff_strategy()) {
            let json = serde_json::to_string(&strategy).unwrap();
            let restored: BackoffStrategy = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(format!("{strategy:?}"), format!("{restored:?}"));
        }

        #[test]
        fn prop_network_config_json_roundtrip(config in arb_network_config()) {
            let json = serde_json::to_string(&config).unwrap();
            let restored: NetworkConfig = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(config.port_range, restored.port_range);
            prop_assert_eq!(format!("{:?}", config.security_level), format!("{:?}", restored.security_level));
            prop_assert_eq!(config.protocols, restored.protocols);
        }
    }

    #[test]
    fn resource_requirements_default_validation() {
        let req = ResourceRequirements::default();
        assert!((req.cpu.min_cores - 1.0).abs() < f64::EPSILON);
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
        assert!((cpu.min_cores - 4.0).abs() < f64::EPSILON);
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
        assert!((gpu.min_memory_gb - 8.0).abs() < f64::EPSILON);
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
        assert!((back.cpu.min_cores - distributed.cpu.min_cores).abs() < f64::EPSILON);
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
        assert!((alloc.cpu_cores - 1.0).abs() < f64::EPSILON);
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
        assert!((limits.max_cpu_cores - 4.0).abs() < f64::EPSILON);
        assert_eq!(limits.max_memory_bytes, 8 * 1024 * 1024 * 1024);
    }

    #[test]
    fn resource_value_variants() {
        let _i = ResourceValue::Integer(42);
        let _f = ResourceValue::Float(3.5_f64);
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
