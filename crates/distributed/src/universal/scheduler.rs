// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool::ToadStoolResult;
use toadstool_common::constants::timeouts;
use toadstool_common::interned_strings::socket_env;

use crate::hosting::RecursiveHostingManager;
use crate::metrics::UniversalMetricsCollector;
use crate::network::NetworkDistributor;
use crate::os_layer::manager::OSLayerManager;
use crate::types::{
    CompatibilityMode, ExecutionTarget, LoadBalancingStrategy, ResourceAllocationStrategy,
    ResourceLimits, UniversalJob, UniversalJobQueue,
};

/// Universal scheduler for cross-platform job distribution
pub struct UniversalScheduler {
    /// Scheduler configuration
    _config: UniversalSchedulerConfig,
    /// Local job queue
    local_queue: Arc<RwLock<UniversalJobQueue>>,
    /// Network-aware job distribution
    network_distributor: Arc<NetworkDistributor>,
    /// Recursive hosting manager
    _recursive_hosting_manager: Arc<RecursiveHostingManager>,
    /// OS-layer manager
    _os_layer_manager: Arc<OSLayerManager>,
    /// Metrics collector
    _metrics_collector: Arc<UniversalMetricsCollector>,
}

/// Configuration for the universal scheduler
#[derive(Debug, Clone)]
pub struct UniversalSchedulerConfig {
    /// Scheduling algorithms to use
    pub scheduling_algorithms: Vec<SchedulingAlgorithm>,
    /// Network effect configuration
    pub network_effects: NetworkEffectsConfig,
    /// Coordination integration settings
    pub coordination: CoordinationSchedulerConfig,
    /// Recursive hosting settings
    pub recursive_hosting: RecursiveHostingConfig,
    /// OS-layer settings
    pub os_layer: OSLayerConfig,
}

/// Network effects configuration
#[derive(Debug, Clone)]
pub struct NetworkEffectsConfig {
    /// Enable network effects
    pub enabled: bool,
    /// Load balancing across network
    pub load_balancing: NetworkLoadBalancing,
    /// Resource sharing configuration
    pub resource_sharing: ResourceSharingConfig,
    /// Fault tolerance configuration
    pub fault_tolerance: FaultToleranceConfig,
}

/// High-level coordination settings for the universal scheduler (endpoint, auth).
#[derive(Debug, Clone)]
pub struct CoordinationSchedulerConfig {
    /// Enable Coordination integration
    pub enabled: bool,
    /// Coordination endpoint
    pub endpoint: String,
    /// Authentication token
    pub auth_token: Option<String>,
}

/// Recursive hosting configuration
#[derive(Debug, Clone)]
pub struct RecursiveHostingConfig {
    /// Enable recursive hosting
    pub enabled: bool,
    /// Current depth level
    pub current_depth: u32,
    /// Maximum depth allowed
    pub max_depth: u32,
    /// Parent `ToadStool` if hosted
    pub parent_toadstool: Option<String>,
    /// Child `ToadStools` being hosted
    pub child_toadstools: Vec<String>,
    /// Resource allocation for children
    pub child_resource_allocation: ResourceAllocationStrategy,
}

/// OS-layer configuration
#[derive(Debug, Clone, Default)]
pub struct OSLayerConfig {
    /// Enable virtual filesystem
    pub virtual_filesystem_enabled: bool,
    /// Enable process virtualization
    pub process_virtualization_enabled: bool,
    /// Enable network virtualization
    pub network_virtualization_enabled: bool,
    /// Compatibility modes
    pub compatibility_modes: Vec<CompatibilityMode>,
    /// Resource limits for OS layer
    pub os_layer_resource_limits: ResourceLimits,
}

/// Scheduling algorithms for job ordering.
#[derive(Debug, Clone)]
pub enum SchedulingAlgorithm {
    /// First-come first-serve.
    FirstComeFirstServe,
    /// Priority-based scheduling.
    Priority,
    /// Round-robin across nodes.
    RoundRobin,
    /// Shortest job first.
    ShortestJobFirst,
    /// Resource-aware placement.
    ResourceAware,
    /// Network-aware placement.
    NetworkAware,
    /// Energy-optimized placement.
    EnergyOptimized,
}

/// Network load balancing configuration.
#[derive(Debug, Clone)]
pub struct NetworkLoadBalancing {
    /// Load balancing strategy.
    pub strategy: LoadBalancingStrategy,
    /// Health check interval in ms.
    pub health_check_interval_ms: u64,
    /// Failover threshold (failures before marking down).
    pub failover_threshold: u32,
}

/// Resource sharing configuration.
#[derive(Debug, Clone)]
pub struct ResourceSharingConfig {
    /// Enable resource sharing.
    pub enabled: bool,
    /// Sharing ratio (0.0–1.0).
    pub sharing_ratio: f64,
    /// Priority boost for shared resources.
    pub priority_boost: f64,
}

/// Fault tolerance configuration.
#[derive(Debug, Clone)]
pub struct FaultToleranceConfig {
    /// Enable fault tolerance.
    pub enabled: bool,
    /// Max retries per job.
    pub max_retries: u32,
    /// Circuit breaker failure threshold.
    pub circuit_breaker_threshold: u32,
}

impl UniversalScheduler {
    /// Creates a universal scheduler with the given config.
    pub async fn new(config: UniversalSchedulerConfig) -> ToadStoolResult<Self> {
        let local_queue = Arc::new(RwLock::new(UniversalJobQueue::new()));

        // Create NetworkDistributorConfig from NetworkEffectsConfig
        let network_config = crate::network::distributor::NetworkDistributorConfig {
            enabled: config.network_effects.enabled,
            max_concurrent_distributions: 10,
            distribution_timeout: timeouts::WORKLOAD_EXECUTION_TIMEOUT,
        };
        let network_distributor = Arc::new(NetworkDistributor::new(network_config));

        let recursive_hosting_manager =
            Arc::new(RecursiveHostingManager::new(config.recursive_hosting.clone()).await?);

        // Create OSLayerConfig with correct fields
        let os_layer_config = crate::os_layer::manager::OSLayerConfig {
            enabled: true,
            default_layer: "native".to_string(),
            available_layers: vec!["native".to_string(), "container".to_string()],
        };
        let mut os_layer_manager = OSLayerManager::new(os_layer_config);
        os_layer_manager.initialize().await?;
        let os_layer_manager = Arc::new(os_layer_manager);

        let metrics_collector = Arc::new(UniversalMetricsCollector::new());

        Ok(Self {
            _config: config,
            local_queue,
            network_distributor,
            _recursive_hosting_manager: recursive_hosting_manager,
            _os_layer_manager: os_layer_manager,
            _metrics_collector: metrics_collector,
        })
    }

    /// Schedules a job based on its execution target.
    pub async fn schedule_job(&self, job: UniversalJob) -> ToadStoolResult<()> {
        // Add job to local queue
        self.local_queue.write().await.add_job(job.clone()).await?;

        // Process job based on target
        match &job.target {
            ExecutionTarget::Local => {
                // Schedule locally
                self.schedule_local_job(job).await?;
            }
            ExecutionTarget::ToadStool { .. } => {
                // Route to specific ToadStool
                self.network_distributor.distribute_job(job.clone()).await?;
            }
            ExecutionTarget::EcosystemService { .. } => {
                return Err(toadstool::ToadStoolError::not_supported(
                    "EcosystemService target: use Unix socket primal integrations instead of HTTP",
                ));
            }
            ExecutionTarget::BestAvailable { .. } => {
                // Find best available resource
                self.schedule_best_available(job).await?;
            }
            ExecutionTarget::LoadBalanced { .. } => {
                // Load balance across resources
                self.network_distributor.distribute_job(job.clone()).await?;
            }
        }

        Ok(())
    }

    async fn schedule_local_job(&self, job: UniversalJob) -> ToadStoolResult<()> {
        // The job is already registered and enqueued on `local_queue` by `schedule_job` via
        // `UniversalJobQueue::add_job`. Record structured scheduling telemetry for local execution.
        let job_id = job.job_id;
        let priority = job.priority;
        let target = job.target.clone();
        let job_type = job.job_type.clone();
        let local_queue_depth = self.local_queue.read().await.total_jobs();

        tracing::info!(
            job_id = %job_id,
            ?priority,
            ?target,
            ?job_type,
            local_queue_depth,
            "local job scheduled (enqueued on universal job queue)"
        );
        Ok(())
    }

    async fn schedule_best_available(&self, job: UniversalJob) -> ToadStoolResult<()> {
        // This would implement best available resource selection
        // For now, just schedule locally
        self.schedule_local_job(job).await
    }
}

impl Default for UniversalSchedulerConfig {
    fn default() -> Self {
        Self {
            scheduling_algorithms: vec![
                SchedulingAlgorithm::Priority,
                SchedulingAlgorithm::ResourceAware,
            ],
            network_effects: NetworkEffectsConfig::default(),
            coordination: CoordinationSchedulerConfig::default(),
            recursive_hosting: RecursiveHostingConfig::default(),
            os_layer: OSLayerConfig::default(),
        }
    }
}

impl Default for NetworkEffectsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            load_balancing: NetworkLoadBalancing::default(),
            resource_sharing: ResourceSharingConfig::default(),
            fault_tolerance: FaultToleranceConfig::default(),
        }
    }
}

impl Default for CoordinationSchedulerConfig {
    #[expect(deprecated, reason = "reads legacy TOADSTOOL_SONGBIRD_PORT as backward-compat fallback")]
    fn default() -> Self {
        let port: u16 = std::env::var(socket_env::TOADSTOOL_COORDINATION_PORT)
            .or_else(|_| std::env::var(socket_env::TOADSTOOL_SONGBIRD_PORT)) // legacy env alias
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(toadstool_config::ports::capability_fallback::COORDINATION);

        let host = std::env::var(socket_env::TOADSTOOL_BIND_ADDRESS)
            .unwrap_or_else(|_| String::from(toadstool_config::defaults::network::LOCALHOST));

        Self {
            enabled: false,
            endpoint: format!("http://{host}:{port}"),
            auth_token: None,
        }
    }
}

impl Default for RecursiveHostingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            current_depth: 0,
            max_depth: crate::common::defaults::MAX_HOSTING_DEPTH,
            parent_toadstool: None,
            child_toadstools: Vec::new(),
            child_resource_allocation: ResourceAllocationStrategy::Fair,
        }
    }
}

impl Default for NetworkLoadBalancing {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::RoundRobin,
            health_check_interval_ms: crate::common::defaults::HEALTH_CHECK_INTERVAL_MS,
            failover_threshold: crate::common::defaults::FAILOVER_THRESHOLD,
        }
    }
}

impl Default for ResourceSharingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sharing_ratio: crate::common::defaults::SHARING_RATIO,
            priority_boost: crate::common::defaults::PRIORITY_BOOST,
        }
    }
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: crate::common::defaults::MAX_RETRIES,
            circuit_breaker_threshold: crate::common::defaults::CIRCUIT_BREAKER_THRESHOLD,
        }
    }
}

#[cfg(test)]
mod tests {
    // SPDX-License-Identifier: AGPL-3.0-or-later

    use toadstool_config::defaults::network;
    use toadstool_config::ports::capability_fallback;

    use crate::types::{LoadBalancingStrategy, ResourceAllocationStrategy, ResourceLimits};

    use super::{
        CoordinationSchedulerConfig, FaultToleranceConfig, NetworkEffectsConfig,
        NetworkLoadBalancing, OSLayerConfig, RecursiveHostingConfig, ResourceSharingConfig,
        SchedulingAlgorithm, UniversalSchedulerConfig,
    };

    #[test]
    fn universal_scheduler_config_default() {
        let c = UniversalSchedulerConfig::default();
        assert_eq!(c.scheduling_algorithms.len(), 2);
        match c.scheduling_algorithms.as_slice() {
            [
                SchedulingAlgorithm::Priority,
                SchedulingAlgorithm::ResourceAware,
            ] => {}
            other => panic!("expected Priority + ResourceAware, got {other:?}"),
        }
    }

    #[test]
    fn network_effects_config_default() {
        let c = NetworkEffectsConfig::default();
        assert!(c.enabled);
    }

    #[test]
    fn coordination_scheduler_config_default() {
        let c = CoordinationSchedulerConfig::default();
        assert!(!c.enabled);
        assert!(c.auth_token.is_none());
        assert_eq!(
            c.endpoint,
            format!(
                "http://{}:{}",
                network::LOCALHOST,
                capability_fallback::COORDINATION
            )
        );
    }

    #[test]
    fn recursive_hosting_config_default() {
        let c = RecursiveHostingConfig::default();
        assert!(!c.enabled);
        assert_eq!(c.current_depth, 0);
        assert_eq!(c.max_depth, 3);
        assert!(c.parent_toadstool.is_none());
        assert!(c.child_toadstools.is_empty());
        assert!(matches!(
            c.child_resource_allocation,
            ResourceAllocationStrategy::Fair
        ));
    }

    #[test]
    fn network_load_balancing_default() {
        let c = NetworkLoadBalancing::default();
        assert!(matches!(c.strategy, LoadBalancingStrategy::RoundRobin));
        assert_eq!(c.health_check_interval_ms, 5000);
        assert_eq!(c.failover_threshold, 3);
    }

    #[test]
    fn resource_sharing_config_default() {
        let c = ResourceSharingConfig::default();
        assert!(c.enabled);
        assert!((c.sharing_ratio - 0.8).abs() < f64::EPSILON);
        assert!((c.priority_boost - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn fault_tolerance_config_default() {
        let c = FaultToleranceConfig::default();
        assert!(c.enabled);
        assert_eq!(c.max_retries, 3);
        assert_eq!(c.circuit_breaker_threshold, 5);
    }

    #[test]
    fn os_layer_config_default() {
        let c = OSLayerConfig::default();
        assert!(!c.virtual_filesystem_enabled);
        assert!(!c.process_virtualization_enabled);
        assert!(!c.network_virtualization_enabled);
        assert!(c.compatibility_modes.is_empty());
        let expected_limits = ResourceLimits::default();
        assert!(
            (c.os_layer_resource_limits.max_cpu_cores - expected_limits.max_cpu_cores).abs()
                < f64::EPSILON
        );
        assert_eq!(
            c.os_layer_resource_limits.max_memory_bytes,
            expected_limits.max_memory_bytes
        );
        assert_eq!(
            c.os_layer_resource_limits.max_storage_bytes,
            expected_limits.max_storage_bytes
        );
        assert_eq!(
            c.os_layer_resource_limits.max_network_bandwidth_mbps,
            expected_limits.max_network_bandwidth_mbps
        );
    }
}
