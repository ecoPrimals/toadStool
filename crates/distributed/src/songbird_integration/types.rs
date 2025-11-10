//! Type definitions for Songbird Integration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use toadstool_common::auth::{AuthType, ServiceAuthConfig};
use toadstool_common::config_bases::ConnectionPoolConfig;
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
    pub fn can_handle_job(&self, _job: &UniversalJob) -> bool {
        // Stub implementation
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

// Type alias for backward compatibility
#[deprecated(since = "0.2.0", note = "Use SongbirdDiscoveryConfig instead")]
pub type DiscoveryConfig = SongbirdDiscoveryConfig;

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
    #[must_use]
    pub const fn can_handle_job(&self, _job: &UniversalJob) -> bool {
        // Simple capacity check stub
        true
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
    pub async fn split_job(&self, _job: &UniversalJob) -> Vec<SubTask> {
        // Stub implementation
        vec![]
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
    pub async fn estimate_load(&self, _job: &UniversalJob) -> LoadMetric {
        // Stub implementation
        LoadMetric {
            cpu_load: 0.5,
            memory_load: 0.5,
            network_load: 0.1,
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
    pub async fn coordinate(&self, _plan: &DistributionPlan) -> CoordinationJob {
        // Stub implementation
        CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: Uuid::new_v4(),
            subtask_count: 0,
            completion_strategy: CompletionStrategy::WaitForAll,
        }
    }
}

// ============================================================================
// Discovery Support Types
// ============================================================================

pub struct DiscoveryClient {
    pub(super) connection: Arc<SongbirdConnection>,
    pub(super) http_client: reqwest::Client,
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
}

impl NetworkHealthMonitor {
    pub async fn monitor_health(&mut self) {
        // Stub implementation
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
