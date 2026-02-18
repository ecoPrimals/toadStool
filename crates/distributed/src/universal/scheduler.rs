use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool::ToadStoolResult;
use toadstool_common::constants::timeouts;

use crate::ecosystem::EcosystemCaller;
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
    /// Ecosystem caller for invoking other services
    ecosystem_caller: Arc<EcosystemCaller>,
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
    /// Songbird integration settings
    pub songbird_integration: SongbirdIntegrationConfig,
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

/// Songbird integration configuration
#[derive(Debug, Clone)]
pub struct SongbirdIntegrationConfig {
    /// Enable Songbird integration
    pub enabled: bool,
    /// Songbird endpoint
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

/// Scheduling algorithms
#[derive(Debug, Clone)]
pub enum SchedulingAlgorithm {
    FirstComeFirstServe,
    Priority,
    RoundRobin,
    ShortestJobFirst,
    ResourceAware,
    NetworkAware,
    EnergyOptimized,
}

/// Network load balancing configuration
#[derive(Debug, Clone)]
pub struct NetworkLoadBalancing {
    pub strategy: LoadBalancingStrategy,
    pub health_check_interval_ms: u64,
    pub failover_threshold: u32,
}

/// Resource sharing configuration
#[derive(Debug, Clone)]
pub struct ResourceSharingConfig {
    pub enabled: bool,
    pub sharing_ratio: f64,
    pub priority_boost: f64,
}

/// Fault tolerance configuration
#[derive(Debug, Clone)]
pub struct FaultToleranceConfig {
    pub enabled: bool,
    pub max_retries: u32,
    pub circuit_breaker_threshold: u32,
}

impl UniversalScheduler {
    pub async fn new(config: UniversalSchedulerConfig) -> ToadStoolResult<Self> {
        let local_queue = Arc::new(RwLock::new(UniversalJobQueue::new()));

        // Create NetworkDistributorConfig from NetworkEffectsConfig
        let network_config = crate::network::distributor::NetworkDistributorConfig {
            enabled: config.network_effects.enabled,
            max_concurrent_distributions: 10,
            distribution_timeout: timeouts::WORKLOAD_EXECUTION_TIMEOUT,
        };
        let network_distributor = Arc::new(NetworkDistributor::new(network_config));

        let ecosystem_caller = Arc::new(EcosystemCaller::new());
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
            ecosystem_caller,
            _recursive_hosting_manager: recursive_hosting_manager,
            _os_layer_manager: os_layer_manager,
            _metrics_collector: metrics_collector,
        })
    }

    pub async fn schedule_job(&self, job: UniversalJob) -> ToadStoolResult<()> {
        // Add job to local queue
        let mut queue = self.local_queue.write().await;
        queue.add_job(job.clone()).await?;

        // Process job based on target
        match &job.target {
            ExecutionTarget::Local => {
                // Schedule locally
                self.schedule_local_job(job).await?;
            }
            ExecutionTarget::ToadStool { .. } => {
                // Route to specific ToadStool
                self.network_distributor.distribute_job(job.clone())?;
            }
            ExecutionTarget::EcosystemService { .. } => {
                // Route to ecosystem service
                self.ecosystem_caller.call_service(&job).await?;
            }
            ExecutionTarget::BestAvailable { .. } => {
                // Find best available resource
                self.schedule_best_available(job).await?;
            }
            ExecutionTarget::LoadBalanced { .. } => {
                // Load balance across resources
                self.network_distributor.distribute_job(job.clone())?;
            }
        }

        Ok(())
    }

    async fn schedule_local_job(&self, job: UniversalJob) -> ToadStoolResult<()> {
        // This would implement local job scheduling
        // For now, just log it
        tracing::info!("Scheduling local job: {:?}", job.job_id);
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
            songbird_integration: SongbirdIntegrationConfig::default(),
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

impl Default for SongbirdIntegrationConfig {
    #[allow(deprecated)] // Using deprecated field during migration to capability-based discovery
    fn default() -> Self {
        // Use environment-aware configuration
        let port: u16 = std::env::var("TOADSTOOL_SONGBIRD_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| {
                let config = toadstool_config::env_config::EnvironmentConfig::from_env();
                config.network.songbird_port
            });
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let host = &config.network.bind_address;

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
            max_depth: 3,
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
            health_check_interval_ms: 5000,
            failover_threshold: 3,
        }
    }
}

impl Default for ResourceSharingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sharing_ratio: 0.8,
            priority_boost: 1.2,
        }
    }
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            circuit_breaker_threshold: 5,
        }
    }
}
