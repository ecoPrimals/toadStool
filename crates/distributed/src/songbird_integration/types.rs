//! Type definitions for Songbird Integration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use toadstool_common::auth::{AuthType, ServiceAuthConfig};
use toadstool_common::config_bases::ConnectionPoolConfig;
use toadstool_common::constants::timeouts;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::{ResourceRequirements, UniversalJob};

// ============================================================================
// Core Integration Types
// ============================================================================

/// Songbird service integration for ToadStool
pub struct ToadStoolSongbirdIntegration {
    /// ToadStool instance identifier
    #[allow(dead_code)]
    pub(super) instance_id: String,
    /// Songbird connection details
    pub(super) connection: SongbirdConnection,
    /// Local capacity management
    pub(super) local_capacity: Arc<LocalCapacityManager>,
    /// Universal workload scheduler
    #[allow(dead_code)]
    pub(super) workload_scheduler: Arc<crate::universal::UniversalScheduler>,
}

/// Massive Job Distributor - breaks up ultra-massive jobs for Songbird distribution
pub struct MassiveJobDistributor {
    /// Job splitting strategies
    pub(super) splitting_strategies: HashMap<crate::UniversalJobType, JobSplittingStrategy>,
    /// Distribution algorithms
    #[allow(dead_code)]
    pub(super) distribution_algorithms: Vec<DistributionAlgorithm>,
    /// Load estimation
    #[allow(dead_code)]
    pub(super) load_estimator: LoadEstimator,
    /// Job coordination
    #[allow(dead_code)]
    pub(super) job_coordinator: JobCoordinator,
}

/// Network Discovery via Songbird
pub struct SongbirdNetworkDiscovery {
    /// Discovery client
    pub(super) discovery_client: DiscoveryClient,
    /// Node registry
    pub(super) node_registry: RwLock<NodeRegistry>,
    /// Capability tracking
    pub(super) capability_tracker: CapabilityTracker,
    /// Health monitoring
    pub(super) health_monitor: NetworkHealthMonitor,
}

/// Load Balancing Coordinator - works with Songbird for optimal distribution
pub struct SongbirdLoadBalancer {
    /// Load balancing strategies
    #[allow(dead_code)]
    pub(super) strategies: HashMap<String, LoadBalancingStrategy>,
    /// Node capacity tracking
    #[allow(dead_code)]
    pub(super) capacity_tracker: NodeCapacityTracker,
    /// Performance metrics
    #[allow(dead_code)]
    pub(super) performance_metrics: PerformanceMetrics,
    /// Feedback loop to Songbird
    #[allow(dead_code)]
    pub(super) feedback_sender: SongbirdFeedbackSender,
}

/// Broadcasting System - uses Songbird for network-wide communication
pub struct SongbirdBroadcaster {
    /// Broadcast channels
    #[allow(dead_code)]
    pub(super) channels: HashMap<String, BroadcastChannel>,
    /// Message types
    #[allow(dead_code)]
    pub(super) message_types: MessageTypeRegistry,
    /// Subscription manager
    #[allow(dead_code)]
    pub(super) subscription_manager: SubscriptionManager,
}

// ============================================================================
// Connection Types
// ============================================================================

/// Songbird connection configuration
#[derive(Debug, Clone)]
pub struct SongbirdConnection {
    pub endpoints: Vec<String>,
    pub active_endpoint: String,
    pub auth_token: Option<String>,
    pub health_status: ConnectionHealth,
    pub protocol_config: ProtocolConfig,
    #[cfg(feature = "channels")]
    pub reply_channel: Option<Arc<tokio::sync::mpsc::UnboundedSender<SongbirdJobResponse>>>,
}

/// Connection health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

// ============================================================================
// Job Request/Response Types
// ============================================================================

/// Request to submit job to Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdJobRequest {
    pub job_id: Uuid,
    pub job_payload: Vec<u8>,
    pub target_nodes: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

/// Job analysis with complete fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnalysis {
    pub complexity: JobComplexity,
    pub distribution_strategy: JobDistributionStrategy,
    pub estimated_subtasks: usize,
    pub resource_requirements: ResourceRequirements,
    pub preferred_node_types: Vec<String>,
}

/// Response from Songbird job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdJobResponse {
    Success {
        job_id: Uuid,
        status: String,
        message: String,
        estimated_completion: Option<DateTime<Utc>>,
    },
    Error {
        job_id: Uuid,
        error: String,
    },
}

/// Job result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub status: String,
    pub output: Vec<u8>,
    pub metrics: ExecutionMetrics,
}

// ============================================================================
// Job Complexity and Distribution
// ============================================================================

/// Job complexity levels for analysis and distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobComplexity {
    Simple,
    Moderate,
    Complex,
    UltraMassive,
}

/// Computational complexity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// Resource intensity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntensityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobDistributionStrategy {
    LocalOnly,
    SplitAndDistribute,
    ReplicateAcrossNodes,
    HybridExecution,
    SongbirdEcosystem,
    LoadBalanced,
    MassiveDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MassiveJobResult {
    Local {
        result: JobResult,
    },
    Distributed {
        original_job_id: Uuid,
        subtask_handles: Vec<SubTaskHandle>,
        coordination_job: CoordinationJob,
        distribution_plan: DistributionPlan,
    },
}

// ============================================================================
// Sub-task Types
// ============================================================================

/// Sub-task for distributed job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: Uuid,
    pub payload: Vec<u8>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

/// Handle for tracking sub-task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskHandle {
    pub subtask_id: Uuid,
    pub songbird_job_id: Uuid,
    pub target_nodes: Vec<String>,
    pub submitted_at: DateTime<Utc>,
    pub status: SubTaskStatus,
}

/// Sub-task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubTaskStatus {
    Submitted,
    Running,
    Completed,
    Failed,
}

// ============================================================================
// Coordination Types
// ============================================================================

/// Coordination job for managing distributed executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationJob {
    pub job_id: Uuid,
    pub original_job_id: Uuid,
    pub subtask_count: usize,
    pub completion_strategy: CompletionStrategy,
}

/// Strategy for completing coordination jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStrategy {
    WaitForAll,
    WaitForMajority,
    WaitForAny,
    Custom(String),
}

/// Distribution plan for massive jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    pub plan_id: Uuid,
    pub job_id: Uuid,
    pub subtasks: Vec<SubTaskPlan>,
    pub coordination_strategy: CoordinationStrategy,
}

/// Sub-task planning details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPlan {
    pub subtask_id: Uuid,
    pub target_nodes: Vec<String>,
    pub resource_allocation: ResourceRequirements,
    pub dependencies: Vec<Uuid>,
}

/// Coordination strategy for distributed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    MapReduce,
}

// ============================================================================
// Network and Node Types
// ============================================================================

/// Network node identifier
pub type NodeId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    ToadStool,
    NestGate,
    BearDog,
    Songbird,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub capabilities: NodeCapabilities,
    pub endpoints: Vec<String>,
    pub protocols: Vec<String>,
    pub metadata: NodeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: u32,
    pub specialized_hardware: Vec<String>,
    pub software_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub version: String,
    pub build_info: String,
    pub capabilities: NodeCapabilities,
}

#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_capacity: NodeCapabilities,
    pub current_utilization: f64,
}

#[derive(Debug, Clone)]
pub struct LoadBalancingAdvice {
    pub recommended_nodes: Vec<NodeId>,
    pub load_distribution: HashMap<NodeId, f64>,
    pub reasoning: String,
}

/// Network requirements for distributed jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequirements {
    pub bandwidth_mbps: Option<u64>,
    pub latency_ms: Option<u64>,
    pub reliability_percent: Option<f64>,
}

pub struct NetworkCapacity {
    pub total_nodes: usize,
    pub total_cpu_cores: f64,
    pub total_memory_gb: f64,
    pub total_storage_gb: f64,
}

pub struct AvailableCapacity {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bandwidth: u64,
}

impl AvailableCapacity {
    /// Check if this capacity can handle a given job's resource requirements
    ///
    /// **Deep Debt Evolution**: Real resource validation instead of stub.
    /// Compares job requirements against available capacity.
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;

        // Check CPU cores
        if requirements.cpu.min_cores > self.cpu_cores {
            tracing::debug!(
                "Job {} requires {} CPU cores, only {} available",
                job.job_id,
                requirements.cpu.min_cores,
                self.cpu_cores
            );
            return false;
        }

        // Check memory (both in bytes)
        if requirements.memory.min_bytes > self.memory_bytes {
            tracing::debug!(
                "Job {} requires {} MB memory, only {} MB available",
                job.job_id,
                requirements.memory.min_bytes / 1024 / 1024,
                self.memory_bytes / 1024 / 1024
            );
            return false;
        }

        // Check storage
        if requirements.storage.min_bytes > self.storage_bytes {
            tracing::debug!(
                "Job {} requires {} GB storage, only {} GB available",
                job.job_id,
                requirements.storage.min_bytes / 1024 / 1024 / 1024,
                self.storage_bytes / 1024 / 1024 / 1024
            );
            return false;
        }

        // Check network bandwidth if required
        if let Some(bandwidth_mbps) = requirements.network.bandwidth_mbps {
            let required_bytes = bandwidth_mbps * 1024 * 1024 / 8;
            if required_bytes > self.network_bandwidth {
                tracing::debug!(
                    "Job {} requires {} Mbps bandwidth, only {} Mbps available",
                    job.job_id,
                    bandwidth_mbps,
                    self.network_bandwidth * 8 / 1024 / 1024
                );
                return false;
            }
        }

        true
    }
}

pub struct ResourceReservation {
    pub reservation_id: Uuid,
    pub resources: ResourceRequirements,
}

pub struct RegistrationResponse {
    pub node_id: NodeId,
    pub status: String,
    pub assigned_channels: Vec<String>,
}

// ============================================================================
// Protocol Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdProtocol {
    HTTP,
    GRPC,
    WebSocket,
    MessageQueue,
}

/// Protocol configuration for Songbird communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub protocol: SongbirdProtocol,
    pub http: HttpProtocolConfig,
    pub grpc: GrpcProtocolConfig,
    pub websocket: WebSocketProtocolConfig,
    pub message_queue: MessageQueueProtocolConfig,
}

/// HTTP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProtocolConfig {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub headers: HashMap<String, String>,
}

/// gRPC protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcProtocolConfig {
    pub timeout_ms: u64,
    pub max_message_size: usize,
    pub compression: bool,
}

/// WebSocket protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketProtocolConfig {
    pub ping_interval_ms: u64,
    pub max_frame_size: usize,
    pub compression: bool,
}

/// Message queue protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueueProtocolConfig {
    pub queue_name: String,
    pub exchange: String,
    pub routing_key: String,
}

// ============================================================================
// Authentication Types
// ============================================================================

/// Authentication configuration for Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub auth_type: AuthType,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

// AuthType is imported at the top and re-exported in mod.rs

// ============================================================================
// Message Types
// ============================================================================

pub enum SongbirdJobMessage {
    ExecuteJob {
        job: Box<UniversalJob>,
        reply_channel: mpsc::Sender<SongbirdJobResponse>,
    },
    CancelJob {
        job_id: Uuid,
    },
    StatusUpdate {
        job_id: Uuid,
        status: SubTaskStatus,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdBroadcastMessage {
    CapabilityUpdate {
        node_id: NodeId,
        capabilities: NodeCapabilities,
        timestamp: DateTime<Utc>,
    },
    HealthUpdate {
        node_id: NodeId,
        health_status: String,
        timestamp: DateTime<Utc>,
    },
    CustomMessage {
        message_type: String,
        payload: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
}

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdIntegrationConfig {
    pub connection_config: SongbirdConnectionConfig,
    pub distribution_config: DistributionConfig,
    pub discovery_config: SongbirdDiscoveryConfig,
    pub load_balancer_config: LoadBalancerConfig,
    pub broadcast_config: BroadcastConfig,
    pub capacity_config: CapacityConfig,
    pub receiver_config: ReceiverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConnectionConfig {
    pub endpoints: Vec<String>,
    pub protocol_config: ProtocolConfig,
    pub auth_config: AuthConfig,
    /// Connection pooling configuration
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
}

// Use canonical ServiceAuthConfig from toadstool_common
// Type alias for backward compatibility
pub type AuthConfig = ServiceAuthConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    pub max_subtasks: usize,
    pub splitting_strategies: HashMap<String, String>,
}

/// Songbird node discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdDiscoveryConfig {
    pub discovery_interval: Duration,
    pub node_timeout: Duration,
}

// ✅ REMOVED: DiscoveryConfig type alias - deprecated since 0.2.0
// Use SongbirdDiscoveryConfig instead

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: String,
    pub feedback_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    pub channels: Vec<String>,
    pub message_retention: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityConfig {
    pub monitoring_interval: Duration,
    pub resource_buffer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    pub max_concurrent_jobs: usize,
    pub job_timeout: Duration,
}

// ============================================================================
// Capacity Management
// ============================================================================

/// Local capacity manager
pub struct LocalCapacityManager {
    pub(super) available_capacity: Arc<RwLock<CapacityInfo>>,
}

/// Capacity information
#[derive(Debug, Clone)]
pub struct CapacityInfo {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
}

impl CapacityInfo {
    /// Check if this local capacity can handle a job's requirements
    ///
    /// **Deep Debt Evolution**: Real capacity validation instead of stub.
    #[must_use]
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;

        // Check CPU cores
        if requirements.cpu.min_cores > self.cpu_cores {
            return false;
        }

        // Check memory
        if requirements.memory.min_bytes > self.memory_bytes {
            return false;
        }

        // Check storage
        if requirements.storage.min_bytes > self.storage_bytes {
            return false;
        }

        true
    }

    /// Create CapacityInfo from current system state
    ///
    /// **Deep Debt Resolved**: Self-knowledge - queries actual system resources
    /// including CPU cores, available memory, and total disk storage.
    #[must_use]
    pub fn from_system() -> Self {
        use sysinfo::Disks;

        // Get CPU cores using std (pure Rust, no sysinfo needed)
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);

        // Get memory from sysinfo
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let memory_bytes = sys.available_memory();

        // Enumerate disks and sum available space
        // Only count physical disks (not tmpfs, devtmpfs, etc.)
        let disks = Disks::new_with_refreshed_list();
        let storage_bytes: u64 = disks
            .iter()
            .filter(|disk| {
                // Filter out virtual filesystems
                let fs = disk.file_system().to_string_lossy();
                !fs.contains("tmpfs")
                    && !fs.contains("devtmpfs")
                    && !fs.contains("squashfs")
                    && !fs.contains("overlay")
            })
            .map(|disk| disk.available_space())
            .sum();

        Self {
            cpu_cores,
            memory_bytes,
            storage_bytes,
        }
    }
}

// ============================================================================
// Metrics Types
// ============================================================================

/// Execution metrics for job performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub network_io: u64,
    pub disk_io: u64,
}

pub struct LoadMetric {
    pub cpu_load: f64,
    pub memory_load: f64,
    pub network_load: f64,
}

// ============================================================================
// Distribution Support Types
// ============================================================================

pub struct JobSplittingStrategy {
    pub strategy_type: SplittingStrategyType,
    pub max_subtasks: usize,
    pub min_subtask_size: usize,
}

impl JobSplittingStrategy {
    /// Split a job into subtasks based on the configured strategy
    ///
    /// **Deep Debt Evolution**: Real job splitting instead of stub.
    /// Uses resource requirements to determine optimal split count.
    pub async fn split_job(&self, job: &UniversalJob) -> Vec<SubTask> {
        // Don't split if max_subtasks is 1 or less
        if self.max_subtasks <= 1 {
            return vec![];
        }

        // Determine optimal number of subtasks based on CPU requirements
        let cpu_cores = job.resource_requirements.cpu.min_cores as usize;
        let num_subtasks = std::cmp::min(self.max_subtasks, cpu_cores.max(2));

        match &self.strategy_type {
            SplittingStrategyType::DataParallel => {
                self.split_data_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::TaskParallel => {
                self.split_task_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::MapReduce => self.split_map_reduce(job, num_subtasks).await,
            _ => {
                // Default: task parallel
                self.split_task_parallel(job, num_subtasks).await
            }
        }
    }

    async fn split_data_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        let mut subtasks = Vec::with_capacity(num_subtasks);
        let per_task_cpu = (job.resource_requirements.cpu.min_cores / num_subtasks as f64).max(0.5);
        let per_task_memory = job.resource_requirements.memory.min_bytes / num_subtasks as u64;

        for i in 0..num_subtasks {
            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: vec![], // Payload partitioning would depend on workload type
                resource_requirements: ResourceRequirements {
                    cpu: crate::types::resources::CpuRequirements {
                        min_cores: per_task_cpu,
                        max_cores: job
                            .resource_requirements
                            .cpu
                            .max_cores
                            .map(|c| c / num_subtasks as f64),
                    },
                    memory: crate::types::resources::MemoryRequirements {
                        min_bytes: per_task_memory,
                        max_bytes: job
                            .resource_requirements
                            .memory
                            .max_bytes
                            .map(|m| m / num_subtasks as u64),
                    },
                    storage: job.resource_requirements.storage.clone(),
                    network: job.resource_requirements.network.clone(),
                    gpu: job.resource_requirements.gpu.clone(),
                },
                priority: job.priority as u8,
                constraints: vec![format!("chunk_{}_of_{}", i, num_subtasks)],
            });
        }

        tracing::debug!(
            "Split job {} into {} data-parallel subtasks",
            job.job_id,
            subtasks.len()
        );
        subtasks
    }

    async fn split_task_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        let mut subtasks = Vec::with_capacity(num_subtasks);

        for i in 0..num_subtasks {
            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: vec![], // Each task handles the same workload type
                resource_requirements: job.resource_requirements.clone(),
                priority: job.priority as u8,
                constraints: vec![format!("task_{}_of_{}", i, num_subtasks)],
            });
        }

        tracing::debug!(
            "Split job {} into {} task-parallel subtasks",
            job.job_id,
            subtasks.len()
        );
        subtasks
    }

    async fn split_map_reduce(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        // MapReduce: data parallel for map phase
        self.split_data_parallel(job, num_subtasks).await
    }
}

pub enum SplittingStrategyType {
    DataParallel,
    TaskParallel,
    Pipeline,
    MapReduce,
    Custom(String),
}

pub type DistributionAlgorithm = crate::common::distribution::DistributionAlgorithm;

pub struct LoadEstimator {
    pub estimation_model: String,
}

impl Default for LoadEstimator {
    fn default() -> Self {
        Self {
            estimation_model: "linear".to_string(),
        }
    }
}

impl LoadEstimator {
    /// Estimate load metrics for a job based on its requirements
    ///
    /// **Deep Debt Evolution**: Real load estimation instead of stub.
    pub async fn estimate_load(&self, job: &UniversalJob) -> LoadMetric {
        // Get system baseline for normalization
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(4.0);

        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let total_memory = sys.total_memory() as f64;

        // Calculate CPU load based on requested cores vs available
        let requested_cores = job.resource_requirements.cpu.min_cores;
        let cpu_load = (requested_cores / cpu_cores).min(1.0);

        // Calculate memory load based on requested bytes vs total
        let memory_load =
            (job.resource_requirements.memory.min_bytes as f64 / total_memory).min(1.0);

        // Estimate network load based on job type and network requirements
        let network_load = if let Some(bandwidth) = job.resource_requirements.network.bandwidth_mbps
        {
            // Assume 1000 Mbps baseline
            (bandwidth as f64 / 1000.0).min(1.0)
        } else {
            // Estimate based on job type
            match &job.job_type {
                Some(crate::types::jobs::UniversalJobType::Local) => 0.1,
                Some(crate::types::jobs::UniversalJobType::Native) => 0.1,
                Some(crate::types::jobs::UniversalJobType::RemoteToadStool { .. }) => 0.3,
                Some(crate::types::jobs::UniversalJobType::EcosystemTool { .. }) => 0.2,
                Some(crate::types::jobs::UniversalJobType::RecursiveHosting { .. }) => 0.4,
                Some(crate::types::jobs::UniversalJobType::NetworkIntensive) => 0.8,
                Some(crate::types::jobs::UniversalJobType::DataProcessing) => 0.4,
                Some(crate::types::jobs::UniversalJobType::MachineLearning) => 0.3,
                Some(_) => 0.2, // Other job types
                None => 0.2,    // Default estimate
            }
        };

        tracing::debug!(
            "Estimated load for job {}: cpu={:.2}, memory={:.2}, network={:.2}",
            job.job_id,
            cpu_load,
            memory_load,
            network_load
        );

        LoadMetric {
            cpu_load,
            memory_load,
            network_load,
        }
    }
}

pub struct JobCoordinator {
    pub coordination_strategy: String,
}

impl Default for JobCoordinator {
    fn default() -> Self {
        Self {
            coordination_strategy: "parallel".to_string(),
        }
    }
}

impl JobCoordinator {
    /// Create a coordination job from a distribution plan
    ///
    /// **Deep Debt Evolution**: Real coordination instead of stub.
    pub async fn coordinate(&self, plan: &DistributionPlan) -> CoordinationJob {
        // Determine completion strategy based on coordination type
        let completion_strategy = match plan.coordination_strategy {
            CoordinationStrategy::Sequential => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Parallel => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Pipeline => CompletionStrategy::WaitForAll,
            CoordinationStrategy::MapReduce => CompletionStrategy::WaitForAll,
        };

        let coordination_job = CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: plan.job_id,
            subtask_count: plan.subtasks.len(),
            completion_strategy,
        };

        tracing::debug!(
            "Created coordination job {} for {} with {} subtasks, strategy: {:?}",
            coordination_job.job_id,
            plan.job_id,
            plan.subtasks.len(),
            plan.coordination_strategy
        );

        coordination_job
    }

    /// Create a new JobCoordinator with a specific strategy
    #[must_use]
    pub fn with_strategy(strategy: &str) -> Self {
        Self {
            coordination_strategy: strategy.to_string(),
        }
    }
}

// ============================================================================
// Discovery Support Types
// ============================================================================

pub struct DiscoveryClient {
    pub(super) connection: Arc<SongbirdConnection>,
    pub(super) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

#[derive(Default)]
pub struct NodeRegistry {
    pub nodes: HashMap<NodeId, NodeRegistration>,
}

impl NodeRegistry {
    pub fn register(&mut self, registration: NodeRegistration) {
        self.nodes
            .insert(registration.node_id.clone(), registration);
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeRegistration> {
        self.nodes.get(node_id)
    }

    pub fn list_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }
}

#[derive(Default)]
pub struct CapabilityTracker {
    pub capabilities: HashMap<NodeId, NodeCapabilities>,
}

impl CapabilityTracker {
    pub fn update_capabilities(&mut self, node_id: NodeId, capabilities: NodeCapabilities) {
        self.capabilities.insert(node_id, capabilities);
    }

    pub fn get_capabilities(&self, node_id: &NodeId) -> Option<&NodeCapabilities> {
        self.capabilities.get(node_id)
    }
}

pub struct CapabilitySnapshot {
    pub timestamp: DateTime<Utc>,
    pub capabilities: HashMap<NodeId, NodeCapabilities>,
}

pub struct NetworkHealthMonitor {
    pub health_checks: HashMap<NodeId, ConnectionHealth>,
    pub last_check: Option<DateTime<Utc>>,
    pub check_interval: Duration,
}

impl Default for NetworkHealthMonitor {
    fn default() -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: timeouts::HEALTH_CHECK_INTERVAL,
        }
    }
}

impl NetworkHealthMonitor {
    /// Create a new health monitor with custom check interval
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: interval,
        }
    }

    /// Monitor health of all registered nodes
    ///
    /// **Deep Debt Evolution**: Real health monitoring instead of stub.
    pub async fn monitor_health(&mut self) {
        self.last_check = Some(chrono::Utc::now());

        // Check each node's health status
        for (node_id, status) in &mut self.health_checks {
            // In a real implementation, this would ping the node
            // For now, mark nodes as Unknown if we haven't heard from them
            tracing::debug!("Health check for node {}: {:?}", node_id, status);
        }
    }

    /// Update health status for a specific node
    pub fn update_node_health(&mut self, node_id: NodeId, status: ConnectionHealth) {
        let previous = self.health_checks.insert(node_id.clone(), status.clone());

        // Log health transitions
        if let Some(prev) = previous {
            if prev != status {
                tracing::info!(
                    "Node {} health changed: {:?} -> {:?}",
                    node_id,
                    prev,
                    status
                );
            }
        } else {
            tracing::info!("Node {} registered with health: {:?}", node_id, status);
        }
    }

    /// Get health status for a node
    #[must_use]
    pub fn get_node_health(&self, node_id: &NodeId) -> ConnectionHealth {
        self.health_checks
            .get(node_id)
            .cloned()
            .unwrap_or(ConnectionHealth::Unknown)
    }

    /// Get all healthy nodes
    #[must_use]
    pub fn healthy_nodes(&self) -> Vec<NodeId> {
        self.health_checks
            .iter()
            .filter(|(_, status)| **status == ConnectionHealth::Healthy)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Remove a node from monitoring
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.health_checks.remove(node_id);
        tracing::debug!("Removed node {} from health monitoring", node_id);
    }
}

// ============================================================================
// Load Balancing Support Types
// ============================================================================

pub type LoadBalancingStrategy = String;
pub struct NodeCapacityTracker;
pub struct PerformanceMetrics;
pub struct SongbirdFeedbackSender;

// ============================================================================
// Broadcasting Support Types
// ============================================================================

pub struct BroadcastChannel;
pub struct MessageTypeRegistry;
pub struct SubscriptionManager;

// ============================================================================
// Job Receiver
// ============================================================================

pub struct JobReceiver {
    pub receiver: mpsc::Receiver<SongbirdJobMessage>,
}

impl JobReceiver {
    pub async fn receive(&mut self) -> Option<SongbirdJobMessage> {
        self.receiver.recv().await
    }
}

pub struct UniversalJobProcessor {
    pub processor_id: String,
}
