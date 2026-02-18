//! Type definitions for Songbird Integration

mod capabilities;
mod discovery;
mod node;
mod protocols;

pub use capabilities::{CapabilitySnapshot, CapabilityTracker};
pub use discovery::{
    AvailableCapacity, DiscoveryClient, LoadBalancingAdvice, NetworkCapacity, NetworkHealthMonitor,
    NetworkRequirements, NetworkStatus, NodeMetadata, NodeRegistration, NodeRegistry, NodeType,
    RegistrationResponse, ResourceReservation,
};
pub use node::{NodeCapabilities, NodeId};
pub use protocols::{
    GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
    SongbirdProtocol,
};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use toadstool_common::auth::{AuthType, ServiceAuthConfig};
use toadstool_common::config_bases::ConnectionPoolConfig;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{ResourceRequirements, UniversalJob};

// ============================================================================
// Connection Types (required by discovery)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

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

// ============================================================================
// Core Integration Types
// ============================================================================

pub struct ToadStoolSongbirdIntegration {
    #[allow(dead_code)]
    pub(super) instance_id: String,
    pub(super) connection: SongbirdConnection,
    pub(super) local_capacity: Arc<LocalCapacityManager>,
    #[allow(dead_code)]
    pub(super) workload_scheduler: Arc<crate::universal::UniversalScheduler>,
}

pub struct MassiveJobDistributor {
    pub(super) splitting_strategies: HashMap<crate::UniversalJobType, JobSplittingStrategy>,
    #[allow(dead_code)]
    pub(super) distribution_algorithms: Vec<DistributionAlgorithm>,
    #[allow(dead_code)]
    pub(super) load_estimator: LoadEstimator,
    #[allow(dead_code)]
    pub(super) job_coordinator: JobCoordinator,
}

pub struct SongbirdNetworkDiscovery {
    pub(super) discovery_client: DiscoveryClient,
    pub(super) node_registry: RwLock<NodeRegistry>,
    pub(super) capability_tracker: CapabilityTracker,
    pub(super) health_monitor: NetworkHealthMonitor,
}

pub struct SongbirdLoadBalancer {
    pub(super) strategies: HashMap<String, LoadBalancingStrategy>,
    pub(super) capacity_tracker: NodeCapacityTracker,
    pub(super) performance_metrics: PerformanceMetrics,
    pub(super) feedback_sender: SongbirdFeedbackSender,
}

pub struct SongbirdBroadcaster {
    pub(super) channels: HashMap<String, BroadcastChannel>,
    pub(super) message_types: MessageTypeRegistry,
    pub(super) subscription_manager: SubscriptionManager,
}

// ============================================================================
// Job Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdJobRequest {
    pub job_id: Uuid,
    pub job_payload: Vec<u8>,
    pub target_nodes: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnalysis {
    pub complexity: JobComplexity,
    pub distribution_strategy: JobDistributionStrategy,
    pub estimated_subtasks: usize,
    pub resource_requirements: ResourceRequirements,
    pub preferred_node_types: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobComplexity {
    Simple,
    Moderate,
    Complex,
    UltraMassive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

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
// Sub-task and Coordination Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: Uuid,
    pub payload: Vec<u8>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskHandle {
    pub subtask_id: Uuid,
    pub songbird_job_id: Uuid,
    pub target_nodes: Vec<String>,
    pub submitted_at: DateTime<Utc>,
    pub status: SubTaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubTaskStatus {
    Submitted,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationJob {
    pub job_id: Uuid,
    pub original_job_id: Uuid,
    pub subtask_count: usize,
    pub completion_strategy: CompletionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStrategy {
    WaitForAll,
    WaitForMajority,
    WaitForAny,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    pub plan_id: Uuid,
    pub job_id: Uuid,
    pub subtasks: Vec<SubTaskPlan>,
    pub coordination_strategy: CoordinationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPlan {
    pub subtask_id: Uuid,
    pub target_nodes: Vec<String>,
    pub resource_allocation: ResourceRequirements,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    MapReduce,
}

// ============================================================================
// Authentication and Config
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub auth_type: AuthType,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub type AuthConfig = ServiceAuthConfig;

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
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    pub max_subtasks: usize,
    pub splitting_strategies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdDiscoveryConfig {
    pub discovery_interval: Duration,
    pub node_timeout: Duration,
}

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

impl SongbirdBroadcastMessage {
    /// Derive a routing channel name from the message variant.
    ///
    /// Used by `SongbirdBroadcaster::broadcast()` to route to the correct channel.
    pub fn channel_name(&self) -> &str {
        match self {
            Self::CapabilityUpdate { .. } => "capability-updates",
            Self::HealthUpdate { .. } => "health-updates",
            Self::CustomMessage { message_type, .. } => message_type.as_str(),
        }
    }
}

// ============================================================================
// Capacity Management
// ============================================================================

pub struct LocalCapacityManager {
    pub(super) available_capacity: Arc<RwLock<CapacityInfo>>,
}

#[derive(Debug, Clone)]
pub struct CapacityInfo {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
}

impl CapacityInfo {
    #[must_use]
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;
        requirements.cpu.min_cores <= self.cpu_cores
            && requirements.memory.min_bytes <= self.memory_bytes
            && requirements.storage.min_bytes <= self.storage_bytes
    }

    #[must_use]
    pub fn from_system() -> Self {
        use sysinfo::Disks;
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let memory_bytes = sys.available_memory();
        let disks = Disks::new_with_refreshed_list();
        let storage_bytes: u64 = disks
            .iter()
            .filter(|disk| {
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
// Metrics and Distribution Support
// ============================================================================

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

pub struct JobSplittingStrategy {
    pub strategy_type: SplittingStrategyType,
    pub max_subtasks: usize,
    pub min_subtask_size: usize,
}

impl JobSplittingStrategy {
    pub async fn split_job(&self, job: &UniversalJob) -> Vec<SubTask> {
        if self.max_subtasks <= 1 {
            return vec![];
        }
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
            _ => self.split_task_parallel(job, num_subtasks).await,
        }
    }

    async fn split_data_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        let mut subtasks = Vec::with_capacity(num_subtasks);
        let per_task_cpu = (job.resource_requirements.cpu.min_cores / num_subtasks as f64).max(0.5);
        let per_task_memory = job.resource_requirements.memory.min_bytes / num_subtasks as u64;
        for i in 0..num_subtasks {
            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: vec![],
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
        subtasks
    }

    async fn split_task_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        (0..num_subtasks)
            .map(|i| SubTask {
                id: Uuid::new_v4(),
                payload: vec![],
                resource_requirements: job.resource_requirements.clone(),
                priority: job.priority as u8,
                constraints: vec![format!("task_{}_of_{}", i, num_subtasks)],
            })
            .collect()
    }

    async fn split_map_reduce(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
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
    pub async fn estimate_load(&self, job: &UniversalJob) -> LoadMetric {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(4.0);
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let total_memory = sys.total_memory() as f64;
        let cpu_load = (job.resource_requirements.cpu.min_cores / cpu_cores).min(1.0);
        let memory_load =
            (job.resource_requirements.memory.min_bytes as f64 / total_memory).min(1.0);
        let network_load = if let Some(bandwidth) = job.resource_requirements.network.bandwidth_mbps
        {
            (bandwidth as f64 / 1000.0).min(1.0)
        } else {
            match &job.job_type {
                Some(crate::types::jobs::UniversalJobType::Local) => 0.1,
                Some(crate::types::jobs::UniversalJobType::Native) => 0.1,
                Some(crate::types::jobs::UniversalJobType::RemoteToadStool { .. }) => 0.3,
                Some(crate::types::jobs::UniversalJobType::EcosystemTool { .. }) => 0.2,
                Some(crate::types::jobs::UniversalJobType::RecursiveHosting { .. }) => 0.4,
                Some(crate::types::jobs::UniversalJobType::NetworkIntensive) => 0.8,
                Some(crate::types::jobs::UniversalJobType::DataProcessing) => 0.4,
                Some(crate::types::jobs::UniversalJobType::MachineLearning) => 0.3,
                Some(_) => 0.2,
                None => 0.2,
            }
        };
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
    pub async fn coordinate(&self, plan: &DistributionPlan) -> CoordinationJob {
        let completion_strategy = match plan.coordination_strategy {
            CoordinationStrategy::Sequential => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Parallel => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Pipeline => CompletionStrategy::WaitForAll,
            CoordinationStrategy::MapReduce => CompletionStrategy::WaitForAll,
        };
        CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: plan.job_id,
            subtask_count: plan.subtasks.len(),
            completion_strategy,
        }
    }

    #[must_use]
    pub fn with_strategy(strategy: &str) -> Self {
        Self {
            coordination_strategy: strategy.to_string(),
        }
    }
}

// ============================================================================
// Load Balancing Types
// ============================================================================

/// Named strategy handle — the string identifies which algorithm to apply
/// (e.g. "round-robin", "least-loaded", "capability-aware").
pub type LoadBalancingStrategy = String;

/// Per-node load tracking.
///
/// Records the most-recently observed load fraction (0.0–1.0) and when it
/// was updated. Used by `SongbirdLoadBalancer::request_advice` to select
/// the least-loaded eligible node.
pub struct NodeCapacityTracker {
    /// `node_id → (load_fraction, updated_at)`
    inner: Mutex<HashMap<NodeId, (f64, Instant)>>,
}

impl NodeCapacityTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Record a load observation for the given node (0.0 = idle, 1.0 = saturated).
    pub fn update(&self, node_id: &NodeId, load: f64) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(node_id.clone(), (load.clamp(0.0, 1.0), Instant::now()));
        }
    }

    /// Return the node with the lowest tracked load, or `None` if no data.
    pub fn least_loaded(&self) -> Option<NodeId> {
        self.inner
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .iter()
                    .min_by(|a, b| a.1 .0.partial_cmp(&b.1 .0).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(id, _)| id.clone())
            })
    }

    /// Current load snapshot: `node_id → load_fraction`.
    pub fn snapshot(&self) -> HashMap<NodeId, f64> {
        self.inner
            .lock()
            .ok()
            .map(|g| g.iter().map(|(k, (v, _))| (k.clone(), *v)).collect())
            .unwrap_or_default()
    }
}

impl Default for NodeCapacityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight in-process performance counters for a Songbird connection.
///
/// Tracks request count, error count, and cumulative latency so callers
/// can compute p50/p95 approximations or derive error rates.
pub struct PerformanceMetrics {
    inner: Mutex<PerformanceCounters>,
}

#[derive(Default)]
struct PerformanceCounters {
    requests: u64,
    errors: u64,
    total_latency_ms: u64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PerformanceCounters::default()),
        }
    }

    /// Record a completed request with its latency and whether it failed.
    pub fn record(&self, latency_ms: u64, is_error: bool) {
        if let Ok(mut g) = self.inner.lock() {
            g.requests += 1;
            g.total_latency_ms += latency_ms;
            if is_error {
                g.errors += 1;
            }
        }
    }

    /// Error rate in [0.0, 1.0]; 0.0 when no requests recorded.
    pub fn error_rate(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.errors as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    /// Mean latency in milliseconds; 0.0 when no requests recorded.
    pub fn mean_latency_ms(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.total_latency_ms as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    pub fn request_count(&self) -> u64 {
        self.inner.lock().ok().map(|g| g.requests).unwrap_or(0)
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Feedback messages sent back to Songbird about local node state.
#[derive(Debug, Clone)]
pub enum SongbirdFeedback {
    LoadUpdate { node_id: NodeId, load: f64 },
    ErrorReport { node_id: NodeId, error: String },
    CapacityAvailable { node_id: NodeId },
}

/// Sends feedback events to Songbird's coordination loop.
///
/// Backed by an unbounded mpsc channel. Callers `send()` feedback; a
/// background task (or the Songbird client) drains `SongbirdFeedbackReceiver`.
pub struct SongbirdFeedbackSender {
    tx: mpsc::UnboundedSender<SongbirdFeedback>,
}

pub struct SongbirdFeedbackReceiver {
    pub rx: mpsc::UnboundedReceiver<SongbirdFeedback>,
}

impl SongbirdFeedbackSender {
    pub fn new() -> (Self, SongbirdFeedbackReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, SongbirdFeedbackReceiver { rx })
    }

    /// Emit a feedback event. Returns `false` if the receiver has been dropped.
    pub fn send(&self, feedback: SongbirdFeedback) -> bool {
        self.tx.send(feedback).is_ok()
    }
}

impl Default for SongbirdFeedbackSender {
    fn default() -> Self {
        Self::new().0
    }
}

// ============================================================================
// Broadcasting Types
// ============================================================================

/// A named pub/sub broadcast channel backed by `tokio::sync::broadcast`.
///
/// `BroadcastChannel::subscribe()` returns a `broadcast::Receiver` that
/// receives all future messages sent on this channel.
pub struct BroadcastChannel {
    name: String,
    tx: broadcast::Sender<SongbirdBroadcastMessage>,
}

impl BroadcastChannel {
    pub fn new(name: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            name: name.into(),
            tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Publish a message to all current subscribers.
    ///
    /// Returns `Ok(n)` where `n` is the number of active receivers.
    /// Returns `Err(broadcast::error::SendError)` only when there are no
    /// subscribers (harmless — callers can ignore or log).
    pub fn publish(
        &self,
        msg: SongbirdBroadcastMessage,
    ) -> Result<usize, broadcast::error::SendError<SongbirdBroadcastMessage>> {
        self.tx.send(msg)
    }

    /// Subscribe to this channel.
    pub fn subscribe(&self) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        self.tx.subscribe()
    }
}

/// Registry of known message type names for routing / validation.
///
/// Prevents typos in channel names and provides a single source of truth
/// for which message types are in use.
pub struct MessageTypeRegistry {
    types: Mutex<HashSet<String>>,
}

impl MessageTypeRegistry {
    pub fn new() -> Self {
        Self {
            types: Mutex::new(HashSet::new()),
        }
    }

    /// Register a message type; idempotent.
    pub fn register(&self, type_name: impl Into<String>) {
        if let Ok(mut g) = self.types.lock() {
            g.insert(type_name.into());
        }
    }

    /// `true` if the type was previously registered.
    pub fn is_known(&self, type_name: &str) -> bool {
        self.types
            .lock()
            .ok()
            .is_some_and(|g| g.contains(type_name))
    }

    pub fn known_types(&self) -> Vec<String> {
        self.types
            .lock()
            .ok()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for MessageTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps channel names to their broadcast channels.
///
/// Callers call `get_or_create(name)` to obtain a subscriber handle.
/// The channel is created on first access; subsequent calls return a
/// new `broadcast::Receiver` from the same sender.
pub struct SubscriptionManager {
    channels: Mutex<HashMap<String, broadcast::Sender<SongbirdBroadcastMessage>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a named channel, creating it if it does not yet exist.
    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        let mut guard = self
            .channels
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .subscribe()
    }

    /// Publish a message to a named channel.
    ///
    /// Returns `0` if the channel does not exist or has no subscribers.
    pub fn publish(&self, channel: &str, msg: SongbirdBroadcastMessage) -> usize {
        self.channels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(channel)
            .and_then(|tx| tx.send(msg).ok())
            .unwrap_or(0)
    }

    /// Unsubscribe by dropping all senders for a channel (channel closes).
    pub fn close_channel(&self, channel: &str) {
        self.channels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(channel);
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

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
